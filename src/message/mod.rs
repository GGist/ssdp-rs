//! Messaging primitives for discovering devices and services.

use std::io;
use std::net::SocketAddr;

use net::connector::UdpConnector;
use net::IpVersionMode;

mod notify;
mod search;
mod ssdp;
pub mod listen;
pub mod multicast;

use get_if_addrs;

pub use message::multicast::Multicast;
pub use message::search::{SearchRequest, SearchResponse, SearchListener};
pub use message::notify::{NotifyMessage, NotifyListener};
pub use message::listen::Listen;

/// Multicast Socket Information
pub const UPNP_MULTICAST_IPV4_ADDR: &'static str = "239.255.255.250";
pub const UPNP_MULTICAST_IPV6_LINK_LOCAL_ADDR: &'static str = "FF02::C";
pub const UPNP_MULTICAST_PORT: u16 = 1900;

/// Default TTL For Multicast
pub const UPNP_MULTICAST_TTL: u32 = 2;

/// Enumerates different types of SSDP messages.
#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub enum MessageType {
    /// A notify message.
    Notify,
    /// A search message.
    Search,
    /// A response to a search message.
    Response,
}

#[derive(Clone)]
pub struct Config {
    pub ipv4_addr: String,
    pub ipv6_addr: String,
    pub port: u16,
    pub ttl: u32,
    pub mode: IpVersionMode,
}

impl Config {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn set_ipv4_addr<S: Into<String>>(mut self, value: S) -> Self {
        self.ipv4_addr = value.into();
        self
    }

    pub fn set_ipv6_addr<S: Into<String>>(mut self, value: S) -> Self {
        self.ipv6_addr = value.into();
        self
    }

    pub fn set_port(mut self, value: u16) -> Self {
        self.port = value;
        self
    }

    pub fn set_ttl(mut self, value: u32) -> Self {
        self.ttl = value;
        self
    }

    pub fn set_mode(mut self, value: IpVersionMode) -> Self {
        self.mode = value;
        self
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            ipv4_addr: UPNP_MULTICAST_IPV4_ADDR.to_string(),
            ipv6_addr: UPNP_MULTICAST_IPV6_LINK_LOCAL_ADDR.to_string(),
            port: UPNP_MULTICAST_PORT,
            ttl: UPNP_MULTICAST_TTL,
            mode: IpVersionMode::Any,
        }
    }
}

/// Generate `UdpConnector` objects for all local `IPv4` interfaces.
fn all_local_connectors(multicast_ttl: Option<u32>, filter: &IpVersionMode) -> io::Result<Vec<UdpConnector>> {
    trace!("Fetching all local connectors");
    map_local(|&addr| match (filter, addr) {
        (&IpVersionMode::V4Only, SocketAddr::V4(n)) |
        (&IpVersionMode::Any, SocketAddr::V4(n)) => {
            Ok(Some(try!(UdpConnector::new((*n.ip(), 0), multicast_ttl))))
        }
        (&IpVersionMode::V6Only, SocketAddr::V6(n)) |
        (&IpVersionMode::Any, SocketAddr::V6(n)) => Ok(Some(try!(UdpConnector::new(n, multicast_ttl)))),
        _ => Ok(None),
    })
}

/// Invoke the closure for every local address found on the system
///
/// This method filters out _loopback_ and _global_ addresses.
fn map_local<F, R>(mut f: F) -> io::Result<Vec<R>>
    where F: FnMut(&SocketAddr) -> io::Result<Option<R>>
{
    let addrs_iter = try!(get_local_addrs());

    let mut obj_list = Vec::with_capacity(addrs_iter.len());

    for addr in addrs_iter {
        trace!("Found {}", addr);
        match addr {
            SocketAddr::V4(n) if !n.ip().is_loopback() => {
                if let Some(x) = try!(f(&addr)) {
                    obj_list.push(x);
                }
            }
            // Filter all loopback and global IPv6 addresses
            SocketAddr::V6(n) if !n.ip().is_loopback() && !n.ip().is_global() => {
                if let Some(x) = try!(f(&addr)) {
                    obj_list.push(x);
                }
            }
            _ => (),
        }
    }

    Ok(obj_list)
}

/// Generate a list of some object R constructed from all local `Ipv4Addr` objects.
///
/// If any of the `SocketAddr`'s fail to resolve, this function will not return an error.
fn get_local_addrs() -> io::Result<Vec<SocketAddr>> {
    let iface_iter = try!(get_if_addrs::get_if_addrs()).into_iter();
    Ok(iface_iter.filter_map(|iface| Some(SocketAddr::new(iface.addr.ip(), 0)))
        .collect())
}