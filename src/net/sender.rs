use std::io::{Error, ErrorKind, Read, Write, Result};
use std::net::{UdpSocket, SocketAddr};

use hyper::net::{NetworkStream};

/// A type that wraps a UdpSocket and a SocketAddr and implements the NetworkStream
/// trait.
///
/// Note that reading from this stream will generate an error, this object is
/// used for intercepting Http messages from Hyper and sending them out via Udp.
/// The response(s) from client(s) are to be handled by some other object that
/// has a cloned handle to our internal UdpSocket handle.
pub struct UdpSender {
    udp: UdpSocket,
    dst: SocketAddr
}

impl UdpSender {
    /// Creates a new UdpSender object.
    pub fn new(udp: UdpSocket, dst: SocketAddr) -> UdpSender {
        UdpSender{ udp: udp, dst: dst }
    }
}

impl NetworkStream for UdpSender {
    fn peer_addr(&mut self) -> Result<SocketAddr> {
        Ok(self.dst)
    }
}

unsafe impl Send for UdpSender { }

impl Read for UdpSender {
    fn read(&mut self, _: &mut [u8]) -> Result<usize> {
        // Simulate Some Network Error So Our Process Doesnt Hang
        Err(Error::new(ErrorKind::ConnectionAborted, "UdpSender Can Not Be Read From"))
    }
}

impl Write for UdpSender {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let mut buffer = vec![0u8; buf.len()];
        let mut found = false;
        for (index, &item) in buf.iter().enumerate() {
            if item == '/' as u8 && !found {
                found = true;
                buffer[index] = '*' as u8;
            } else {
                buffer[index] = item;
            }
        }
        
        for &i in buffer.iter() {
            print!("{}", i as char);
        }
        print!("\n");
        
        self.udp.send_to(&buffer[..], self.dst)
    }
    
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl Clone for UdpSender {
    fn clone(&self) -> UdpSender {
        let udp_clone = self.udp.try_clone().unwrap();
        
        UdpSender{ udp: udp_clone, dst: self.dst }
    }
    
    fn clone_from(&mut self, source: &UdpSender) {
        let udp_clone = source.udp.try_clone().unwrap();
        
        self.udp = udp_clone;
        self.dst = source.dst;
    }
}