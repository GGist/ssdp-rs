use std::io::{Error, ErrorKind, Result};
use std::net::{UdpSocket, ToSocketAddrs};

use hyper::net::{NetworkConnector};

use net::receiver::{PacketReceiver};
use net::sender::{UdpSender};

/// A UdpConnector allows Hyper to obtain NetworkStream objects over UdpSockets 
/// so that Http messages created by Hyper can be sent over UDP instead of TCP.
pub struct UdpConnector(UdpSocket);

impl UdpConnector {
    /// Create a new UdpConnector that will be bound to the given local address.
    pub fn new<A: ToSocketAddrs>(local_addr: A) -> Result<UdpConnector> {
        let udp = try!(UdpSocket::bind(local_addr));
        udp.set_multicast_loop(false).unwrap();
        //udp.set_time_to_live(100).unwrap();
        udp.set_multicast_time_to_live(255).unwrap();
        Ok(UdpConnector(udp))
    }
    
    /// Creates a PacketReceiver that can be used to receive packets from the
    /// underlying UdpSocket of the current UdpConnector.
    ///
    /// For semantical information as to what constitutes a packet, see 
    /// net::receiver::PacketReceiver.
    pub fn receiver(&self) -> Result<PacketReceiver> {
        let udp_clone = try!(self.0.try_clone());
    
        Ok(PacketReceiver::new(udp_clone))
    }
}

impl NetworkConnector for UdpConnector {
    type Stream = UdpSender;
    
    fn connect(&mut self, host: &str, port: u16, _: &str) -> Result<<Self as NetworkConnector>::Stream> {
        let udp_clone = try!(self.0.try_clone());
        let mut socket_iter = try!((host, port).to_socket_addrs());
        
        match socket_iter.next() {
            Some(addr) => Ok(UdpSender::new(udp_clone, addr)),
            None       => Err(Error::new(ErrorKind::InvalidInput, 
                                         "Couldn't Convert host:port To SocketAddr"))
        }
    }
}