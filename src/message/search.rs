use std::borrow::Cow;
use std::fmt::Debug;
use std::net::{ToSocketAddrs, SocketAddr, SocketAddrV6, IpAddr, Ipv6Addr};
use std::str::FromStr;
use std::time::Duration;
use std::io;

use hyper::header::{Header, HeaderFormat};

use error::{SSDPResult, MsgError};
use header::{HeaderRef, HeaderMut, MX};
use message::{self, MessageType};
use message::ssdp::SSDPMessage;
use receiver::{SSDPReceiver, FromRawSSDP};
use net;

/// Overhead to add to device response times to account for transport time.
const NETWORK_TIMEOUT_OVERHEAD: u8 = 1;

/// Devices are required to respond within 1 second of receiving unicast message.
const DEFAULT_UNICAST_TIMEOUT: u8 = 1 + NETWORK_TIMEOUT_OVERHEAD;

/// Search request that can be sent via unicast or multicast to devices on the network.
#[derive(Debug, Clone)]
pub struct SearchRequest {
    message: SSDPMessage,
}

impl SearchRequest {
    /// Construct a new SearchRequest.
    pub fn new() -> SearchRequest {
        SearchRequest { message: SSDPMessage::new(MessageType::Search) }
    }

    /// Send this search request to a single host.
    ///
    /// Currently this sends the unicast message on all available network
    /// interfaces. This assumes that the network interfaces are operating
    /// on either different subnets or different ip address ranges.
    pub fn unicast<A: ToSocketAddrs>(&mut self, dst_addr: A) -> SSDPResult<SSDPReceiver<SearchResponse>> {
        let mode = try!(net::IpVersionMode::from_addr(&dst_addr));
        let mut connectors = try!(message::all_local_connectors(None, mode));

        // Send On All Connectors
        for connector in &mut connectors {
            try!(self.message.send(connector, &dst_addr));
        }

        let mut raw_connectors = Vec::with_capacity(connectors.len());
        raw_connectors.extend(connectors.into_iter().map(|conn| conn.deconstruct()));

        let opt_timeout = opt_unicast_timeout(self.get::<MX>());

        Ok(try!(SSDPReceiver::new(raw_connectors, opt_timeout)))
    }

    /// Send this search request to the standard multicast address:port.
    pub fn multicast(&mut self) -> SSDPResult<SSDPReceiver<SearchResponse>> {
        self.multicast_with_port(message::UPNP_MULTICAST_PORT)
    }

    /// Send this search request to the standard multicast address but a custom port
    pub fn multicast_with_port(&mut self, port: u16) -> SSDPResult<SSDPReceiver<SearchResponse>> {
        let mcast_timeout = try!(multicast_timeout(self.get::<MX>()));
        let mcast_ttl = Some(message::UPNP_MULTICAST_TTL);

        let mut connectors = try!(message::all_local_connectors(mcast_ttl, net::IpVersionMode::Any));

        // Send On All Connectors
        for conn in &mut connectors {
            match try!(conn.local_addr()) {
                SocketAddr::V4(_) => {
                    try!(self.message.send(conn, &(message::UPNP_MULTICAST_IPV4_ADDR, port)))
                }
                SocketAddr::V6(n) => {
                    try!(self.message.send(conn,
                                           &SocketAddrV6::new(try!(
                    FromStr::from_str(message::UPNP_MULTICAST_IPV6_LINK_LOCAL_ADDR)),
                                                              port,
                                                              n.flowinfo(),
                                                              n.scope_id())))
                }
            }
        }

        let mut raw_connectors = Vec::with_capacity(connectors.len());
        raw_connectors.extend(connectors.into_iter().map(|conn| conn.deconstruct()));

        Ok(try!(SSDPReceiver::new(raw_connectors, Some(mcast_timeout))))
    }
}

impl Default for SearchRequest {
    fn default() -> Self {
        SearchRequest::new()
    }
}

/// Get the require timeout to use for a multicast search request.
fn multicast_timeout(mx: Option<&MX>) -> SSDPResult<Duration> {
    match mx {
        Some(&MX(n)) => Ok(Duration::new((n + NETWORK_TIMEOUT_OVERHEAD) as u64, 0)),
        None => try!(Err(MsgError::new("Multicast Searches Require An MX Header"))),
    }
}

/// Get the default timeout to use for a unicast search request.
fn opt_unicast_timeout(mx: Option<&MX>) -> Option<Duration> {
    match mx {
        Some(&MX(n)) => Some(Duration::new((n + NETWORK_TIMEOUT_OVERHEAD) as u64, 0)),
        None => Some(Duration::new(DEFAULT_UNICAST_TIMEOUT as u64, 0)),
    }
}

impl FromRawSSDP for SearchRequest {
    fn raw_ssdp(bytes: &[u8]) -> SSDPResult<SearchRequest> {
        let message = try!(SSDPMessage::raw_ssdp(bytes));

        if message.message_type() != MessageType::Search {
            try!(Err(MsgError::new("SSDP Message Received Is Not A SearchRequest")))
        } else {
            Ok(SearchRequest { message: message })
        }
    }
}

impl HeaderRef for SearchRequest {
    fn get<H>(&self) -> Option<&H>
        where H: Header + HeaderFormat
    {
        self.message.get::<H>()
    }

    fn get_raw(&self, name: &str) -> Option<&[Vec<u8>]> {
        self.message.get_raw(name)
    }
}

impl HeaderMut for SearchRequest {
    fn set<H>(&mut self, value: H)
        where H: Header + HeaderFormat
    {
        self.message.set(value)
    }

    fn set_raw<K>(&mut self, name: K, value: Vec<Vec<u8>>)
        where K: Into<Cow<'static, str>> + Debug
    {
        self.message.set_raw(name, value)
    }
}

/// Search response that can be received or sent via unicast to devices on the network.
#[derive(Debug, Clone)]
pub struct SearchResponse {
    message: SSDPMessage,
}

impl SearchResponse {
    /// Construct a new SearchResponse.
    pub fn new() -> SearchResponse {
        SearchResponse { message: SSDPMessage::new(MessageType::Response) }
    }

    /// Send this search response to a single host.
    ///
    /// Currently this sends the unicast message on all available network
    /// interfaces. This assumes that the network interfaces are operating
    /// on either different subnets or different ip address ranges.
    pub fn unicast<A: ToSocketAddrs>(&mut self, dst_addr: A) -> SSDPResult<()> {
        let mode = try!(net::IpVersionMode::from_addr(&dst_addr));
        let mut connectors = try!(message::all_local_connectors(None, mode));

        let mut success_count = 0;
        let mut error_count = 0;
        // Send On All Connectors
        for conn in &mut connectors {
            // Some routing errors are expected, not all interfaces can find the target addresses
            match self.message.send(conn, &dst_addr) {
                Ok(_) => success_count += 1,
                Err(_) => error_count += 1,
            }
        }

        if success_count == 0 && error_count > 0 {
            try!(Err(io::Error::last_os_error()));
        }

        Ok(())
    }
}

impl Default for SearchResponse {
    fn default() -> Self {
        SearchResponse::new()
    }
}

/// Search listener that can listen for search messages sent within the network.
pub struct SearchListener;

impl SearchListener {
    /// Listen for notify messages on all local network interfaces.
    pub fn listen() -> SSDPResult<SSDPReceiver<SearchRequest>> {
        SearchListener::listen_on_port(message::UPNP_MULTICAST_PORT)
    }

    /// Listen for notify messages on a custom port on all local network interfaces.
    pub fn listen_on_port(port: u16) -> SSDPResult<SSDPReceiver<SearchRequest>> {
        // Generate a list of reused sockets on the standard multicast address.
        let reuse_sockets = try!(message::map_local(|&addr| match addr {
            SocketAddr::V4(v4_addr) => {
                let sock = try!(net::bind_reuse((*v4_addr.ip(), port)));

                let mcast_ip = FromStr::from_str(message::UPNP_MULTICAST_IPV4_ADDR).unwrap();

                debug!("Joining ipv4 multicast {} at iface: {}", mcast_ip, addr);
                try!(net::join_multicast(&sock, &addr, &mcast_ip));

                Ok(Some(sock))
            }
            SocketAddr::V6(v6_addr) => {
                let mcast_ip: Ipv6Addr = FromStr::from_str(message::UPNP_MULTICAST_IPV6_LINK_LOCAL_ADDR)
                    .unwrap();

                // clone to preserve interface scope
                let mut x = v6_addr.clone();
                x.set_ip(mcast_ip);
                x.set_port(port);
                let sock = try!(net::bind_reuse(x));

                debug!("Joining ipv6 multicast {} at iface: {}", mcast_ip, addr);
                try!(net::join_multicast(&sock, &addr, &IpAddr::V6(mcast_ip)));

                Ok(Some(sock))
            }
        }));

        Ok(try!(SSDPReceiver::new(reuse_sockets, None)))
    }
}

impl FromRawSSDP for SearchResponse {
    fn raw_ssdp(bytes: &[u8]) -> SSDPResult<SearchResponse> {
        let message = try!(SSDPMessage::raw_ssdp(bytes));

        if message.message_type() != MessageType::Response {
            try!(Err(MsgError::new("SSDP Message Received Is Not A SearchResponse")))
        } else {
            Ok(SearchResponse { message: message })
        }
    }
}

impl HeaderRef for SearchResponse {
    fn get<H>(&self) -> Option<&H>
        where H: Header + HeaderFormat
    {
        self.message.get::<H>()
    }

    fn get_raw(&self, name: &str) -> Option<&[Vec<u8>]> {
        self.message.get_raw(name)
    }
}

impl HeaderMut for SearchResponse {
    fn set<H>(&mut self, value: H)
        where H: Header + HeaderFormat
    {
        self.message.set(value)
    }

    fn set_raw<K>(&mut self, name: K, value: Vec<Vec<u8>>)
        where K: Into<Cow<'static, str>> + Debug
    {
        self.message.set_raw(name, value)
    }
}

#[cfg(test)]
mod tests {
    use header::MX;

    #[test]
    fn positive_multicast_timeout() {
        super::multicast_timeout(Some(&MX(5))).unwrap();
    }

    #[test]
    fn positive_some_opt_multicast_timeout() {
        super::opt_unicast_timeout(Some(&MX(5))).unwrap();
    }

    #[test]
    fn positive_none_opt_multicast_timeout() {
        super::opt_unicast_timeout(None).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_multicast_timeout() {
        super::multicast_timeout(None).unwrap();
    }
}
