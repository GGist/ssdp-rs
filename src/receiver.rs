//! Primitives for non-blocking SSDP message receiving.

use std::io::{self};
use std::result::{self};
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

/// Wrapper for decorating an SSDPReceiver with the Iterator trait.
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
    recv: Receiver<T>,
    sock: UdpSocket,
    addr: SocketAddr,
    kill: Arc<AtomicBool>
}

impl<T> SSDPReceiver<T> where T: FromRawSSDP + Send + 'static {
    pub fn new(sock: UdpSocket, time: Option<Duration>) -> io::Result<SSDPReceiver<T>> {
        // Self Arguments
        let self_flag = Arc::new(AtomicBool::new(false));
        let self_sock = sock;
        let self_addr = try!(self_sock.local_addr());
        
        // Timer Thread Arguments
        let timer_sock = try!(self_sock.try_clone());
        
        let (send, recv) = mpsc::channel();
        
        // Spawn Receiver Thread
        let pckt_recv = PacketReceiver::new(try!(self_sock.try_clone()));
        let recv_flag = self_flag.clone();
        thread::spawn(move || {
            receive_packets::<T>(pckt_recv, recv_flag, send);
        });
        
        // Check If A Timer Thread Should Be Spawned
        if let Some(n) = time {
            let timer_addr = self_addr;
            let timer_flag = self_flag.clone();
            
            thread::spawn(move || {
                udp_timer(n, timer_flag, timer_sock, timer_addr);
            });
        }
        
        Ok(SSDPReceiver{ recv: recv, sock: self_sock, kill: self_flag, addr: self_addr })
    }
}

impl<T> SSDPReceiver<T> {
    pub fn try_recv(&self) -> result::Result<T, TryRecvError> {
        self.recv.try_recv()
    }
    
    pub fn recv(&self) -> result::Result<T, RecvError> {
        self.recv.recv()
    }
}

impl<'a, T> IntoIterator for &'a SSDPReceiver<T> {
    type Item = T;
    type IntoIter = Iter<'a, T>;
    
    fn into_iter(self) -> Self::IntoIter {
        self.recv.iter()
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
        syncronize_kill(&*self.kill, &self.sock, self.addr);
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
/// the kill flag and send a message on the UdpSocket to unblock whatever thread is
/// blocking on it so that it can see that the kill flag has been activated.
///
/// This should be run in it's own thread.
fn udp_timer(time: Duration, kill: Arc<AtomicBool>, sock: UdpSocket, addr: SocketAddr) {
    thread::sleep_ms(time.num_milliseconds() as u32);
    
    syncronize_kill(&*kill, &sock, addr);
}

/// Sets the kill flag and sends a one byte message through the UdpSocket, making
/// sure that the operations are sequentially consistent (not re-ordered).
fn syncronize_kill(kill: &AtomicBool, sock: &UdpSocket, local_addr: SocketAddr) {
    kill.store(true, Ordering::SeqCst);
    
    sock.send_to(&[0], local_addr).unwrap();
}