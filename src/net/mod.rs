//! Implements the HTTPMU and lower layers of the UPnP standard.
//!
//! This module deals with primitives for working with external libraries to write
//! data to UDP sockets as a stream, and read data from UDP sockets as packets.

use std::io::{self, Error, ErrorKind};
use std::net::{ToSocketAddrs, UdpSocket, SocketAddr};
use std::mem;

use libc;

pub mod connector;
pub mod packet;
pub mod sender;

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
    init_sock_api();
    
    let local_addr = try!(addr_from_trait(local_addr));
    let udp_socket = try!(create_udp_socket(&local_addr));
    
    try!(reuse_addr(udp_socket));
    try!(bind_addr(udp_socket, local_addr));
    
    let size_correction: u64 = MUCH_MAGIC_NUMBER_WOW | udp_socket;
    
    Ok(unsafe{ mem::transmute::<u64, UdpSocket>(size_correction) })
}

/// Join a multicast address on the current UdpSocket.
///
/// Unlike the libstd crate, this function uses the socket's address when
/// setting the source interface for the multicast subscription.
pub fn join_multicast(sock: &UdpSocket, mcast_addr: &IpAddr) -> io::Result<()> {
    
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

/// Set a multicast membership option on the given socket.
fn set_membership(socket: Socket, mcast_addr: &IpAddr, opt: c_int) -> io::Result<()> {
    
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