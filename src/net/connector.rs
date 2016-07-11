use std::io;
use std::net::{UdpSocket, ToSocketAddrs, SocketAddr, SocketAddrV4, SocketAddrV6};
use std::str::FromStr;

use hyper::error;
use hyper::net::NetworkConnector;

use net::sender::UdpSender;
use net;

/// A `UdpConnector` allows Hyper to obtain `NetworkStream` objects over `UdpSockets`
/// so that Http messages created by Hyper can be sent over UDP instead of TCP.
pub struct UdpConnector(UdpSocket);

impl UdpConnector {
    /// Create a new UdpConnector that will be bound to the given local address.
    pub fn new<A: ToSocketAddrs>(local_addr: A, _: Option<u32>) -> io::Result<UdpConnector> {
        let addr = try!(net::addr_from_trait(local_addr));
        debug!("Attempting to connect to {}", addr);

        let udp = try!(UdpSocket::bind(addr));

        // TODO: This throws an invalid argument error
        // if let Some(n) = multicast_ttl {
        //     trace!("Setting ttl to {}", n);
        //     try!(udp.set_multicast_ttl_v4(n));
        // }

        Ok(UdpConnector(udp))
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.0.local_addr()
    }

    /// Destroy the UdpConnector and return the underlying UdpSocket.
    pub fn deconstruct(self) -> UdpSocket {
        self.0
    }
}

impl NetworkConnector for UdpConnector {
    type Stream = UdpSender;

    fn connect(&self, host: &str, port: u16, _: &str) -> error::Result<<Self as NetworkConnector>::Stream> {
        let udp_sock = try!(self.0.try_clone());
        let sock_addr = match try!(self.local_addr()) {
            SocketAddr::V4(_) => {
                SocketAddr::V4(SocketAddrV4::new(try!(FromStr::from_str(host).map_err(|err| {
                                                     io::Error::new(io::ErrorKind::InvalidInput, err)
                                                 })),
                                                 port))
            }
            SocketAddr::V6(n) => {
                let mut addr: SocketAddrV6 = if host.find('[') == Some(0) &&
                                                host.rfind(']') == Some(host.len() - 1) {
                    try!(FromStr::from_str(format!("{}:{}", host, port).as_str())
                        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err)))
                } else {
                    try!(FromStr::from_str(format!("[{}]:{}", host, port).as_str())
                        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err)))
                };
                addr.set_flowinfo(n.flowinfo());
                addr.set_scope_id(n.scope_id());
                SocketAddr::V6(addr)
            }
        };

        Ok(UdpSender::new(udp_sock, sock_addr))
    }
}
