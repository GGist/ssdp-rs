//! Primitives for non-blocking SSDP message receiving.

use std::io::Result as IoResult;
use std::result::{Result};
use std::thread::{self};
use std::sync::{Arc};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError, RecvError, Iter};
use std::sync::atomic::{AtomicBool, Ordering};
use std::net::{UdpSocket, SocketAddr};

use time::{Duration};

use {SSDPResult};
use net::packet::{PacketReceiver};

/// Trait for constructing an object from some serialized SSDP message.
pub trait FromRawSSDP {
    fn raw_ssdp(bytes: &[u8]) -> SSDPResult<Self>;
}

/// Iterator for an SSDPReceiver.
pub struct SSDPIter<T> {
    recv: SSDPReceiver<T>
}

impl<T> SSDPIter<T> {
    fn new(recv: SSDPReceiver<T>) -> SSDPIter<T> {
        SSDPIter{ recv: recv }
    }
}

impl<T> Iterator for SSDPIter<T> {
    type Item = T;
    
    fn next(&mut self) -> Option<Self::Item> {
        self.recv.recv().ok()
    }
}

/// A non-blocking SSDP message receiver for any message that implements FromRawSSDP.
pub struct SSDPReceiver<T> {
    recvr: Receiver<T>,
    socks: Vec<UdpSocket>,
    addrs: Vec<SocketAddr>,
    kill:  Arc<AtomicBool>
}

impl<T> SSDPReceiver<T> where T: FromRawSSDP + Send + 'static {
    pub fn new(socks: Vec<UdpSocket>, time: Option<Duration>) -> IoResult<SSDPReceiver<T>> {
        // Produce Two More Copies Of Sockets
        let (send_socks, time_socks) = (try!(clone_socks(&socks[..])), try!(clone_socks(&socks[..])));
        
        // Produce Two Copies Of Socket Addresses
        let (recv_addrs, time_addrs) = (try!(clone_addrs(&socks[..])), try!(clone_addrs(&socks[..]))); 

        // Create Channel And Shareable Kill Flag
        let (send, recv) = mpsc::channel();
        let self_kill = Arc::new(AtomicBool::new(false));
        
        // Spawn Receiver Threads
        spawn_receivers(send_socks, self_kill.clone(), send);
        
        // Spawn Single Kill Timer
        if let Some(n) = time {
            let timer_flag = self_kill.clone();
            thread::spawn(move || {
                kill_timer(n, timer_flag, time_socks, time_addrs);
            });
        }
        
        Ok(SSDPReceiver{ recvr: recv, socks: socks, addrs: recv_addrs, kill: self_kill })
    }
}

fn clone_socks(socks: &[UdpSocket]) -> IoResult<Vec<UdpSocket>> {
    let mut clone_socks = Vec::with_capacity(socks.len());
    
    for (sock, dst) in socks.iter().zip(clone_socks.iter_mut()) {
        *dst = try!(sock.try_clone());
    }
    
    Ok(clone_socks)
}

fn clone_addrs(socks: &[UdpSocket]) -> IoResult<Vec<SocketAddr>> {
    let mut clone_addrs = Vec::with_capacity(socks.len());
    
    for (sock, dst) in socks.iter().zip(clone_addrs.iter_mut()) {
        *dst = try!(sock.local_addr());
    }
    
    Ok(clone_addrs)
}

/// Spawn a number of receiver threads that will receive packets, forward the bytes
/// on to a constructor, and send successfully constructed objects through the sender.
fn spawn_receivers<T>(socks: Vec<UdpSocket>, kill_flag: Arc<AtomicBool>, sender: Sender<T>)
    where T: FromRawSSDP + Send + 'static {
    for sock in socks {
        let pckt_recv = PacketReceiver::new(sock);
        let kill_flag = kill_flag.clone();
        let sender = sender.clone();
        
        thread::spawn(move || {
            receive_packets(pckt_recv, kill_flag, sender);
        });
    }
}

impl<T> SSDPReceiver<T> {
    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        self.recvr.try_recv()
    }
    
    pub fn recv(&self) -> Result<T, RecvError> {
        self.recvr.recv()
    }
}

impl<'a, T> IntoIterator for &'a SSDPReceiver<T> {
    type Item = T;
    type IntoIter = Iter<'a, T>;
    
    fn into_iter(self) -> Self::IntoIter {
        self.recvr.iter()
    }
}

impl<T> IntoIterator for SSDPReceiver<T> {
    type Item = T;
    type IntoIter = SSDPIter<T>;
    
    fn into_iter(self) -> Self::IntoIter {
        SSDPIter::new(self)
    }
}

impl<T> Drop for SSDPReceiver<T> {
    fn drop(&mut self) {
        syncronize_kill(&*self.kill, &self.socks[..], &self.addrs[..]);
    }
}

/// Receives bytes and attempts to construct a T which will be sent through the
/// supplied channel.
///
/// This should almost always be run in it's own thread.
fn receive_packets<T>(recv: PacketReceiver, kill: Arc<AtomicBool>, send: Sender<T>)
    where T: FromRawSSDP + Send {
    // TODO: Add logging to this function. Maybe forward sender IP Address along
    // so that we can do some checks when we parse the http.
    loop {
        let msg_bytes = match recv.recv_pckt() {
            Ok((bytes, _)) => bytes,
            Err(_)       => { continue; }
        };
        
        // Check If We Were Unblocked Intentionally
        if kill.load(Ordering::Acquire) {
            // With acquire, there is a chance that we could process our 
            // one byte unblock message, if we add logging in the future 
            // we have to take that into account.
            return
        }
        
        // Unwrap Will Cause A Panic If Receiver Hung Up Which Is Desired
        match T::raw_ssdp(&msg_bytes[..]) {
            Ok(n)  => send.send(n).unwrap(),
            Err(_) => { continue; }
        };
    }
}

/// Sleeps the current thread for the specified duration, after which, it will activate
/// the kill flag and send a message on all UdpSockets to unblock whatever threads are
/// blocking on them so that they can see that the kill flag has been activated.
///
/// This should be run in it's own thread.
fn kill_timer(time: Duration, kill: Arc<AtomicBool>, socks: Vec<UdpSocket>, addrs: Vec<SocketAddr>) {
    thread::sleep_ms(time.num_milliseconds() as u32);

    syncronize_kill(&*kill, &socks[..], &addrs[..]);
}

/// Sets the kill flag and sends a one byte message through the UdpSocket, making
/// sure that the operations are sequentially consistent (not re-ordered).
fn syncronize_kill(kill: &AtomicBool, socks: &[UdpSocket], local_addrs: &[SocketAddr]) {
    kill.store(true, Ordering::SeqCst);
    
    for (sock, addr) in socks.iter().zip(local_addrs.iter()) {
        sock.send_to(&[0], addr);
    }
}