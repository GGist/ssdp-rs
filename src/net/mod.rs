//! Implements the HTTPMU and lower layers of the `UPnP` standard.
//!
//! This module deals with primitives for working with external libraries to write
//! data to UDP sockets as a stream, and read data from UDP sockets as packets.

use std::io::{self, ErrorKind};
use std::net::{ToSocketAddrs, UdpSocket};
use std::net::{SocketAddr, IpAddr};

#[cfg(not(windows))]
use net2::unix::UnixUdpBuilderExt;
use net2::UdpBuilder;

pub mod connector;
pub mod packet;
pub mod sender;

#[derive(Copy, Clone)]
pub enum IpVersionMode {
    V4Only,
    V6Only,
    Any,
}

impl IpVersionMode {
    pub fn from_addr<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
        match try!(addr_from_trait(addr)) {
            SocketAddr::V4(_) => Ok(IpVersionMode::V4Only),
            SocketAddr::V6(_) => Ok(IpVersionMode::V6Only),
        }
    }
}

/// Accept a type implementing `ToSocketAddrs` and tries to extract the first address.
pub fn addr_from_trait<A: ToSocketAddrs>(addr: A) -> io::Result<SocketAddr> {
    let mut sock_iter = try!(addr.to_socket_addrs());

    match sock_iter.next() {
        Some(n) => Ok(n),
        None => Err(io::Error::new(ErrorKind::InvalidInput, "Failed To Parse SocketAddr")),
    }
}

/// Bind to a `UdpSocket`, setting `SO_REUSEADDR` on the underlying socket before binding.
pub fn bind_reuse<A: ToSocketAddrs>(local_addr: A) -> io::Result<UdpSocket> {
    let local_addr = try!(addr_from_trait(local_addr));

    let builder = match local_addr {
        SocketAddr::V4(_) => try!(UdpBuilder::new_v4()),
        SocketAddr::V6(_) => try!(UdpBuilder::new_v6()),
    };

    try!(reuse_port(&builder));
    builder.bind(local_addr)
}

#[cfg(windows)]
fn reuse_port(builder: &UdpBuilder) -> io::Result<()> {
    // Allow wildcards + specific to not overlap
    try!(builder.reuse_address(true));
    Ok(())
}

#[cfg(not(windows))]
fn reuse_port(builder: &UdpBuilder) -> io::Result<()> {
    // Allow wildcards + specific to not overlap
    try!(builder.reuse_address(true));
    // Allow multiple listeners on the same port
    try!(builder.reuse_port(true));
    Ok(())
}

/// Join a multicast address on the current `UdpSocket`.
pub fn join_multicast(sock: &UdpSocket, iface: &SocketAddr, mcast_addr: &IpAddr) -> io::Result<()> {
    match (iface, mcast_addr) {
        (&SocketAddr::V4(ref i), &IpAddr::V4(ref m)) => sock.join_multicast_v4(m, i.ip()),
        (&SocketAddr::V6(ref i), &IpAddr::V6(ref m)) => sock.join_multicast_v6(m, i.scope_id()),
        _ => {
            Err(io::Error::new(ErrorKind::InvalidInput,
                               "Multicast And Interface Addresses Are Not The Same Version"))
        }
    }
}

/// Leave a multicast address on the current `UdpSocket`.
#[allow(dead_code)] // TODO: call this from somewhere?
pub fn leave_multicast(sock: &UdpSocket, iface_addr: &SocketAddr, mcast_addr: &SocketAddr) -> io::Result<()> {
    match (iface_addr, mcast_addr) {
        (&SocketAddr::V4(ref i), &SocketAddr::V4(ref m)) => sock.leave_multicast_v4(m.ip(), i.ip()),
        (&SocketAddr::V6(ref i), &SocketAddr::V6(ref m)) => sock.leave_multicast_v6(m.ip(), i.scope_id()),
        _ => {
            Err(io::Error::new(ErrorKind::InvalidInput,
                               "Multicast And Interface Addresses Are Not The Same Version"))
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn positive_addr_from_trait() {
        super::addr_from_trait("192.168.0.1:0").unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_addr_from_trait() {
        super::addr_from_trait("192.168.0.1").unwrap();
    }
}
