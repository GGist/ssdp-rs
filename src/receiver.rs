//! Primitives for non-blocking SSDP message receiving.

use std::io::{self};
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
    type Item = (T, SocketAddr);
    
    fn next(&mut self) -> Option<Self::Item> {
        self.recv.recv().ok()
    }
}

/// A non-blocking SSDP message receiver.
pub struct SSDPReceiver<T> {
    recvr: Receiver<(T, SocketAddr)>,
    socks: Vec<UdpSocket>,
    addrs: Vec<SocketAddr>,
    kill:  Arc<AtomicBool>
}

impl<T> SSDPReceiver<T> where T: FromRawSSDP + Send + 'static {
    /// Construct a receiver that receives bytes from a number of UdpSockets and
    /// tries to construct an object T from them. If a duration is provided, the
    /// channel will be shutdown after the specified duration.
    ///
    /// Due to implementation details, none of the UdpSockets should be bound to
    /// the default route, 0.0.0.0, address.
    pub fn new(socks: Vec<UdpSocket>, time: Option<Duration>) -> io::Result<SSDPReceiver<T>> {
        let (send, recv) = mpsc::channel();
        
        let send_socks = try!(clone_socks(&socks[..]));
        let recv_addrs = try!(clone_addrs(&socks[..])); 
        
        let self_kill = Arc::new(AtomicBool::new(false));

        // Spawn Receiver Threads
        spawn_receivers(send_socks, self_kill.clone(), send);
        
        // Spawn Single Kill Timer
        let spawn_result = maybe_spawn_timer(time, self_kill.clone(), &socks[..]);
        
        // If Timer Failed To Spawn, Kill Our Receivers
        if let Err(e) = spawn_result {
            syncronize_kill(&*self_kill, &socks[..], &recv_addrs[..]);
            
            return Err(e)
        }
        
        Ok(SSDPReceiver{ recvr: recv, socks: socks, addrs: recv_addrs, kill: self_kill })
    }
}

/// Attempt to clone all UdpSockets into a new vector.
fn clone_socks(socks: &[UdpSocket]) -> io::Result<Vec<UdpSocket>> {
    let mut clone_socks = Vec::with_capacity(socks.len());
    
    for sock in socks.iter() {
        clone_socks.push(try!(sock.try_clone()));
    }

    Ok(clone_socks)
}

/// Attempt to copy all SocketAddrs from the UdpSockets into a new vector.
fn clone_addrs(socks: &[UdpSocket]) -> io::Result<Vec<SocketAddr>> {
    let mut clone_addrs = Vec::with_capacity(socks.len());
    
    for sock in socks.iter() {
        clone_addrs.push(try!(sock.local_addr()));
    }
    
    Ok(clone_addrs)
}

/// Spawn a number of receiver threads that will receive packets, forward the
/// bytes on to T, and send successfully constructed objects through the sender.
fn spawn_receivers<T>(socks: Vec<UdpSocket>, kill_flag: Arc<AtomicBool>, sender: Sender<(T, SocketAddr)>)
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

/// Spawn a timer if a duration was passed in and link the timer with the given sockets.
///
/// If some of the sockets or socket addresses could not be cloned, an error is returned.
fn maybe_spawn_timer(time: Option<Duration>, kill: Arc<AtomicBool>, socks: &[UdpSocket]) -> io::Result<()> {
    match time {
        Some(n) => {
            let timer_socks = try!(clone_socks(socks));
            let timer_addrs = try!(clone_addrs(socks));
            
            thread::spawn(move || {
                kill_timer(n, kill, timer_socks, timer_addrs);
            });
        },
        None => ()
    };
    
    Ok(())
}

impl<T> SSDPReceiver<T> {
    /// Non-blocking method that attempts to read a value from the receiver.
    pub fn try_recv(&self) -> Result<(T, SocketAddr), TryRecvError> {
        self.recvr.try_recv()
    }
    
    /// Blocking method that reads a value from the receiver until one is available.
    pub fn recv(&self) -> Result<(T, SocketAddr), RecvError> {
        self.recvr.recv()
    }
}

impl<'a, T> IntoIterator for &'a SSDPReceiver<T> {
    type Item = (T, SocketAddr);
    type IntoIter = Iter<'a, (T, SocketAddr)>;
    
    fn into_iter(self) -> Self::IntoIter {
        self.recvr.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut SSDPReceiver<T> {
    type Item = (T, SocketAddr);
    type IntoIter = Iter<'a, (T, SocketAddr)>;
    
    fn into_iter(self) -> Self::IntoIter {
        self.recvr.iter()
    }
}

impl<T> IntoIterator for SSDPReceiver<T> {
    type Item = (T, SocketAddr);
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
fn receive_packets<T>(recv: PacketReceiver, kill: Arc<AtomicBool>, send: Sender<(T, SocketAddr)>)
    where T: FromRawSSDP + Send {
    // TODO: Add logging to this function. Maybe forward sender IP Address along
    // so that we can do some checks when we parse the http.
    loop {
        let (msg_bytes, addr) = match recv.recv_pckt() {
            Ok((bytes, addr)) => (bytes, addr),
            Err(_)       => { continue; }
        };
        
        // Check If We Were Unblocked Intentionally
        if kill.load(Ordering::Acquire) {
            // With acquire, there is a chance that the code below could
            // be moved up and our unblock message could be processed. This
            // should not affect execution but keep in mind for logging
            // purposes.
            return
        }
        
        // Unwrap Will Cause A Panic If Receiver Hung Up Which Is Desired
        match T::raw_ssdp(&msg_bytes[..]) {
            Ok(n)  => {
                send.send((n, addr)).unwrap()
            },
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

#[allow(unused)]
/// Sets the kill flag and sends a one byte message through the UdpSockets,
/// making sure that the operations are sequentially consistent (not re-ordered).
fn syncronize_kill(kill: &AtomicBool, socks: &[UdpSocket], local_addrs: &[SocketAddr]) {
    kill.store(true, Ordering::SeqCst);
    
    for (sock, addr) in socks.iter().zip(local_addrs.iter()) {
        sock.send_to(&[0], addr);
    }
}