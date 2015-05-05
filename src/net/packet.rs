use std::io::{Error, ErrorKind, Result};
use std::net::{UdpSocket, SocketAddr};

/// Maximum length for packets received on a PacketReceiver.
pub const MAX_PCKT_LEN: usize = 600;

/// A PacketReceiver that abstract over a network socket and reads full packets
/// from the connection. Packets received from this connection are assumed to
/// be no larger than what the typical MTU would be on a standard router.
///
/// See net::packet::MAX_PCKT_LEN.
pub struct PacketReceiver(UdpSocket);

impl PacketReceiver {
    /// Create a new PacketReceiver from the given UdpSocket.
    pub fn new(udp: UdpSocket) -> PacketReceiver {
        PacketReceiver(udp)
    }
    
    /// Receive a packet from the underlying connection.
    pub fn recv_pckt(&self) -> Result<(Vec<u8>, SocketAddr)> {
        let mut pckt_buf = vec![0u8; MAX_PCKT_LEN];
        
        let (size, addr) = try!(self.0.recv_from(&mut pckt_buf));
        
        // Check For Something That SHOULD NEVER Occur.
        if size > pckt_buf.len() {
            Err(Error::new(ErrorKind::Other, 
                           "UdpSocket Reported Receive Length Greater Than Buffer"))
        } else {
            unsafe{ pckt_buf.set_len(size) };
            
            Ok((pckt_buf, addr))
        }
    }
}