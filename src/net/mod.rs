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

#[cfg(windows)]
pub type Socket = libc::SOCKET;

#[cfg(not(windows))]
pub type Socket = libc::c_int;

/// Bind A UdpSocket To The Given Address With The SO_REUSEADDR Option Set.
pub fn reuse_socket<A: ToSocketAddrs>(addr: A) -> io::Result<UdpSocket> {
    let mut ret;
    
    // Dummy UdpSocket Will Run Socket Initialization Code For Process Since
    // We Can't Access The init() Function Ourselves (Private Visibility)
    let _ = try!(UdpSocket::bind(("0.0.0.0", 0)));
    let socket_addr = try!(try!(addr.to_socket_addrs()).next().ok_or(
        Error::new(ErrorKind::InvalidInput, "Error With Addr Passed In")
    ));
    
    // Create Socket
    let family = match socket_addr {
        SocketAddr::V4(..) => libc::AF_INET,
        SocketAddr::V6(..) => libc::AF_INET6
    };
    let sock: Socket = unsafe{ libc::socket(family, libc::SOCK_DGRAM, 0) };
    try!(check_sock(sock));
    
    // Set SO_REUSEADDR On Socket
    ret = unsafe{ libc::setsockopt(sock, libc::SOL_SOCKET, libc::SO_REUSEADDR,
            &1i32 as &libc::c_int as *const libc::c_int as *const libc::c_void,
            mem::size_of::<libc::c_int>() as libc::socklen_t)
    };
    
    if ret != 0 {
        return Err(Error::last_os_error())
    }
    
    // Bind Address On Socket
    let (sock_addr, len) = match socket_addr {
        SocketAddr::V4(ref a) => 
            (a as *const _ as *const _, mem::size_of_val(a) as libc::socklen_t),
        SocketAddr::V6(ref a) =>
            (a as *const _ as *const _, mem::size_of_val(a) as libc::socklen_t)
    };
    ret = unsafe{ libc::bind(sock, sock_addr, len) };
    
    if ret != 0 {
        return Err(Error::last_os_error())
    }
    
    // LOL I HAVE NO IDEA WHY THIS WORKS!!!!!!!
    // Joking Aside, I Looked At The Bit Patterns For A Bunch Of UdpSockets I
    // Created And Even Though They All Had A 32 Bit Representation Under The
    // Hood (Either SOCKET Or c_int), The Size Of The UdpSocket Was 64 Bits.
    // All Of The Upper Bits For The UdpSocket Had The Same Bit Pattern Which Was
    // 0000 0000 0000 0000 0000 0000 1101 0100 .... Or In Decimal 910533066752
    // So That Is What We Are Setting The Upper Bits To Below.
    let size_correction: u64 = 910533066752 | (sock as u64);
    
    // Return New Socket
    Ok(unsafe{ mem::transmute::<u64, UdpSocket>(size_correction) })
}

/// Check The Return Value Of A Call To libc::socket().
#[cfg(windows)]
fn check_sock(sock: Socket) -> io::Result<()> {
    if sock == libc::INVALID_SOCKET {
        // Dont Have Access To Private Function WSAGetLastError()
        Err(Error::new(ErrorKind::Other, "Error With Socket Creation"))
    } else {
        Ok(())
    }
}

/// Check The Return Value Of A Call To libc::socket().
#[cfg(not(windows))]
fn check_sock(sock: Socket) -> io::Result<()> {
    if sock == -1i32 {
        Err(Error::last_os_error())
    } else {
        Ok(())
    }
}
