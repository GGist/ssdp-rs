use std::io::{self, ErrorKind};
use std::net::{UdpSocket, ToSocketAddrs, SocketAddr};

use hyper::error::{self};
use hyper::net::{NetworkConnector};

use net::sender::{UdpSender};

/// A UdpConnector allows Hyper to obtain NetworkStream objects over UdpSockets 
/// so that Http messages created by Hyper can be sent over UDP instead of TCP.
pub struct UdpConnector(UdpSocket);

impl UdpConnector {
    /// Create a new UdpConnector that will be bound to the given local address.
    pub fn new<A: ToSocketAddrs>(local_addr: A, multicast_ttl: Option<i32>) -> io::Result<UdpConnector> {
        let udp = try!(UdpSocket::bind(local_addr));
        
        if let Some(n) = multicast_ttl {
            try!(udp.set_multicast_time_to_live(n));
        }
        
        Ok(UdpConnector(udp))
    }
    
    /// Destroy the UdpConnector and return the underlying UdpSocket.
    pub fn deconstruct(self) -> UdpSocket {
        self.0
    }
}

/// Accept a type implementing ToSocketAddrs and tries to extract the first address.
pub fn addr_from_trait<A: ToSocketAddrs>(addr: A) -> io::Result<SocketAddr> {
    let mut sock_iter = try!(addr.to_socket_addrs());
    
    match sock_iter.next() {
        Some(n) => Ok(n),
        None    => Err(io::Error::new(ErrorKind::InvalidInput, "Failed To Parse SocketAddr"))
    }
}

impl NetworkConnector for UdpConnector {
    type Stream = UdpSender;
    
    fn connect(&mut self, host: &str, port: u16, _: &str) -> error::Result<<Self as NetworkConnector>::Stream> {
        let udp_sock = try!(self.0.try_clone());
        let sock_addr = try!(addr_from_trait((host, port)));
        
        Ok(UdpSender::new(udp_sock, sock_addr))
    }
}