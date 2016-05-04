//! Messaging primitives for discovering devices and services.

use std::io::{self};
#[cfg(windows)]
use ::std::net;
use ::std::net::{SocketAddr, Ipv4Addr};

use net::connector::{UdpConnector};

mod notify;
mod search;
mod ssdp;

pub use message::search::{SearchRequest, SearchResponse};
pub use message::notify::{NotifyMessage, NotifyListener};

#[cfg(not(windows))]
use ::ifaces;

/// Multicast Socket Information
const UPNP_MULTICAST_ADDR: &'static str = "239.255.255.250";
const UPNP_MULTICAST_PORT: u16          = 1900;

/// Default TTL For Multicast
const UPNP_MULTICAST_TTL: i32 = 2;

/// Enumerates different types of SSDP messages.
#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub enum MessageType {
    /// A notify message.
    Notify,
    /// A search message.
    Search,
    /// A response to a search message.
    Response
}

/// Generate UdpConnector objects for all local IPv4 interfaces.
fn all_local_connectors(multicast_ttl: Option<i32>) -> io::Result<Vec<UdpConnector>> {
    map_local_ipv4(|&addr| UdpConnector::new((addr, 0), multicast_ttl))
}

/// Generate a list of some object R constructed from all local Ipv4Addr objects.
///
/// If any of the SocketAddrs fail to resolve, this function will not return an error.
#[cfg(windows)]
fn map_local_ipv4<F, R>(mut f: F) -> io::Result<Vec<R>>
    where F: FnMut(&Ipv4Addr) -> io::Result<R> {
    let host_iter = try!(net::lookup_host(""));
    let mut obj_list = match host_iter.size_hint() {
        (_, Some(n)) => Vec::with_capacity(n),
        (_, None)    => Vec::new()
    };

    for host in host_iter.filter_map(|host| host.ok()) {
        match host {
            SocketAddr::V4(n) => obj_list.push(try!(f(n.ip()))),
            _ => ()
        }
    }

    Ok(obj_list)
}

/// Generate a list of some object R constructed from all local Ipv4Addr objects.
///
/// If any of the SocketAddrs fail to resolve, this function will not return an error.
#[cfg(not(windows))]
fn map_local_ipv4<F, R>(mut f: F) -> io::Result<Vec<R>>
    where F: FnMut(&Ipv4Addr) -> io::Result<R>
{
    let iface_iter = try!(ifaces::Interface::get_all()).into_iter();

    let mut obj_list = match iface_iter.size_hint() {
        (_, Some(n)) => Vec::with_capacity(n),
        (_, None)    => Vec::new()
    };

    for iface in iface_iter.filter(|iface| iface.kind == ifaces::Kind::Ipv4) {
        match iface.addr {
            Some(SocketAddr::V4(n)) => {
                if !n.ip().is_loopback() {
                    obj_list.push(try!(f(n.ip())))
                }
            },
            _ => ()
        }
    }

    Ok(obj_list)
}
