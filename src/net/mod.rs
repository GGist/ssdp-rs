//! Implements the HTTPMU and lower layers of the UPnP standard.
//!
//! This module deals with primitives for working with external libraries to write
//! data to UDP sockets as a stream, and read data from UDP sockets as packets.

use std::io::{self, Error, ErrorKind};
use std::net::{ToSocketAddrs, UdpSocket, SocketAddr, Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6, AddrParseError};
use std::mem;
use std::string::ToString;
use std::str::{FromStr};

#[cfg(windows)]
use std::os::windows::io::{RawSocket, FromRawSocket};

#[cfg(not(windows))]
use std::os::unix::io::{RawFd, FromRawFd};

use libc;

pub mod connector;
pub mod packet;
pub mod sender;

#[cfg(windows)]
type Socket = libc::SOCKET;

#[cfg(not(windows))]
type Socket = libc::c_int;

/// Accept a type implementing ToSocketAddrs and tries to extract the first address.
pub fn addr_from_trait<A: ToSocketAddrs>(addr: A) -> io::Result<SocketAddr> {
    let mut sock_iter = try!(addr.to_socket_addrs());

    match sock_iter.next() {
        Some(n) => Ok(n),
        None    => Err(io::Error::new(ErrorKind::InvalidInput, "Failed To Parse SocketAddr"))
    }
}

/// Bind to a UdpSocket, setting SO_REUSEADDR on the underlying socket before binding.
pub fn bind_reuse<A: ToSocketAddrs>(local_addr: A) -> io::Result<UdpSocket> {
    try!(init_sock_api());

    let local_addr = try!(addr_from_trait(local_addr));
    let socket = try!(create_udp_socket(&local_addr));

    try!(reuse_addr(socket));
    try!(bind_addr(socket, &local_addr));

    Ok(udp_socket_from_socket(socket))
}

/// Join a multicast address on the current UdpSocket.
pub fn join_multicast(sock: &UdpSocket, iface_addr: &IpAddr, mcast_addr: &IpAddr)
    -> io::Result<()> {
    let (iface_ip, mcast_ip) = match (iface_addr, mcast_addr) {
        (&IpAddr::V4(ref i), &IpAddr::V4(ref m)) => (i, m),
        (&IpAddr::V6(..), &IpAddr::V6(..)) => {
            return Err(io::Error::new(ErrorKind::InvalidInput,
                "Ipv6Addr Multicast Not Currently Supported"))
        },
        _ => return Err(io::Error::new(ErrorKind::InvalidInput,
                 "Multicast And Interface Addresses Are Not The Same Version"))
    };
    let socket = unsafe{ mem::transmute_copy::<UdpSocket, Socket>(sock) };

    set_membership_ipv4(socket, iface_ip, mcast_ip, libc::IP_ADD_MEMBERSHIP)
}

#[allow(unused)]
/// Leave a multicast address on the current UdpSocket.
pub fn leave_multicast(sock: &UdpSocket, iface_addr: &IpAddr, mcast_addr: &IpAddr)
    -> io::Result<()> {
    let (iface_ip, mcast_ip) = match (iface_addr, mcast_addr) {
        (&IpAddr::V4(ref i), &IpAddr::V4(ref m)) => {
            (i, m)
        },
        (&IpAddr::V6(..), &IpAddr::V6(..)) => {
            return Err(io::Error::new(ErrorKind::InvalidInput,
                "Ipv6Addr Multicast Not Currently Supported"))
        },
        _ => return Err(io::Error::new(ErrorKind::InvalidInput,
                 "Multicast And Interface Addresses Are Not The Same Version"))
    };
    let socket = unsafe{ mem::transmute_copy::<UdpSocket, Socket>(sock) };

    set_membership_ipv4(socket, iface_ip, mcast_ip, libc::IP_DROP_MEMBERSHIP)
}

//----------------------------------------------------------------------------//

/// Run the initialization routine for the sockets API.
fn init_sock_api() -> io::Result<()> {
    // Since we do not have access to the socket initialization function in libstd,
    // we will have to create a UdpSocket so that it invokes that function for
    // our platform.
    let init_facade = UdpSocket::bind(("0.0.0.0", 0));

    init_facade.map(|_| ())
}

/// Create a socket that has been checked to be valid.
fn create_udp_socket(sock_addr: &SocketAddr) -> io::Result<Socket> {
    let family = match *sock_addr {
        SocketAddr::V4(..) => libc::AF_INET,
        SocketAddr::V6(..) => libc::AF_INET6
    };
    let socket = unsafe{ libc::socket(family, libc::SOCK_DGRAM, 0) };

    check_socket(socket).map(|_| socket)
}

/// Set SO_REUSEADDR option on the socket.
fn reuse_addr(socket: Socket) -> io::Result<()> {
    let opt: libc::c_int = 1;

    let ret = unsafe {
        libc::setsockopt(socket, libc::SOL_SOCKET, libc::SO_REUSEADDR,
            &opt as *const libc::c_int as *const libc::c_void,
            mem::size_of::<libc::c_int>() as libc::socklen_t)
    };

    if ret != 0 {
        Err(Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Bind the socket to the given socket address.
fn bind_addr(socket: Socket, sock_addr: &SocketAddr) -> io::Result<()> {
    let (sock_addr, sock_len) = match *sock_addr {
        SocketAddr::V4(ref a) => {
            (a as *const _ as *const _, mem::size_of_val(a) as libc::socklen_t)
        },
        SocketAddr::V6(ref a) => {
            (a as *const _ as *const _, mem::size_of_val(a) as libc::socklen_t)
        }
    };

    let ret = unsafe{ libc::bind(socket, sock_addr, sock_len) };

    if ret != 0 {
        Err(Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Join the socket on the given interface to the given multicast address.
fn set_membership_ipv4(socket: Socket, iface_addr: &Ipv4Addr,
    mcast_addr: &Ipv4Addr, opt: libc::c_int) -> io::Result<()> {
    let mreq = libc::ip_mreq {
        imr_multiaddr: ipv4addr_as_in_addr(mcast_addr),
        imr_interface: ipv4addr_as_in_addr(iface_addr)
    };

    let ret = unsafe {
        libc::setsockopt(socket, libc::IPPROTO_IP, opt,
            &mreq as *const libc::ip_mreq as *const libc::c_void,
            mem::size_of::<libc::ip_mreq>() as libc::socklen_t)
    };

    if ret != 0 {
        Err(Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Convert the ipv4 address to a single number.
fn ipv4addr_as_in_addr(addr: &Ipv4Addr) -> libc::in_addr {
    unsafe{ mem::transmute_copy::<Ipv4Addr, libc::in_addr>(addr) }
}

// Fix for issue #27801
pub enum IpAddr {
    V4(Ipv4Addr),
    V6(Ipv6Addr),
}
pub trait SocketIp {
    fn new(ip: IpAddr, port: u16) -> SocketAddr;
    fn ip(&self) -> IpAddr;
}
impl SocketIp for SocketAddr {
    fn new(ip: IpAddr, port: u16) -> SocketAddr {
        match ip {
            IpAddr::V4(a) => SocketAddr::V4(SocketAddrV4::new(a, port)),
            IpAddr::V6(a) => SocketAddr::V6(SocketAddrV6::new(a, port, 0, 0)),
        }
    }
    fn ip(&self) -> IpAddr {
        match *self {
            SocketAddr::V4(ref a) => IpAddr::V4(*a.ip()),
            SocketAddr::V6(ref a) => IpAddr::V6(*a.ip()),
        }
    }
}

impl ToString for IpAddr {
    fn to_string(&self) -> String {
        match *self {
            IpAddr::V4(a) => a.to_string(),
            IpAddr::V6(a) => a.to_string()
        }
    }
}

impl FromStr for IpAddr {
    type Err = AddrParseError;
    fn from_str(s: &str) -> Result<IpAddr, AddrParseError> {
        s.parse().map(|ip| IpAddr::V4(ip)).or_else(|_| s.parse().map(|ip| IpAddr::V6(ip)))
    }
}

#[cfg(windows)]
fn udp_socket_from_socket(socket: Socket) -> UdpSocket {
    let raw_socket = unsafe{ mem::transmute::<Socket, RawSocket>(socket) };

    unsafe{ UdpSocket::from_raw_socket(raw_socket) }
}

#[cfg(not(windows))]
fn udp_socket_from_socket(socket: Socket) -> UdpSocket {
    let raw_fd = unsafe{ mem::transmute::<Socket, RawFd>(socket) };

    unsafe{ UdpSocket::from_raw_fd(raw_fd) }
}

/// Check The Return Value Of A Call To libc::socket().
#[cfg(windows)]
fn check_socket(sock: Socket) -> io::Result<()> {
    if sock == libc::INVALID_SOCKET {
        // Don't Have Access To Private Function WSAGetLastError()
        Err(Error::new(ErrorKind::Other, "Error With Socket Creation"))
    } else {
        Ok(())
    }
}

/// Check The Return Value Of A Call To libc::socket().
#[cfg(not(windows))]
fn check_socket(sock: Socket) -> io::Result<()> {
    if sock == -1i32 {
        Err(Error::last_os_error())
    } else {
        Ok(())
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
