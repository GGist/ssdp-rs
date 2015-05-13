//! Implements the HTTPMU and lower layers of the UPnP standard.
//!
//! This module deals with primitives for working with external libraries to write
//! data to UDP sockets as a stream, and read data from UDP sockets as packets.

use std::io::{self, Error, ErrorKind};
use std::net::{ToSocketAddrs, UdpSocket, SocketAddr, IpAddr, Ipv4Addr};
use std::mem;

use libc;

pub mod connector;
pub mod packet;
pub mod sender;

// I found that most of the fd's for UdpSockets were 32 bits but the UdpSockets
// themselves were 64 bits. I created a bunch of UdpSockets and looked at the
// bit representation of the value as a u64. All 32 of the upper bits in all
// of the sockets were set to the upper bits of 910533066752 so we need to
// OR our 32 bit socket with this number in order to cast to a UdpSocket.
const MUCH_MAGIC_NUMBER_WOW: u64 = 910533066752;

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
    let udp_socket = try!(create_udp_socket(&local_addr));
    
    try!(reuse_addr(udp_socket));
    try!(bind_addr(udp_socket, &local_addr));
    
    let size_correction: u64 = MUCH_MAGIC_NUMBER_WOW | (udp_socket as u64);
    
    Ok(unsafe{ mem::transmute::<u64, UdpSocket>(size_correction) })
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
    let socket = unsafe{ mem::transmute_copy::<UdpSocket, u32>(sock) };
    
    set_membership_ipv4(socket, iface_ip, mcast_ip, libc::IP_ADD_MEMBERSHIP)
}

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
    let socket = unsafe{ mem::transmute_copy::<UdpSocket, u32>(sock) };
    
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