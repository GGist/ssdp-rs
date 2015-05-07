use std::borrow::{Cow};
use std::net::{ToSocketAddrs};

use hyper::header::{Header, HeaderFormat};
use time::{Duration};

use error::{SSDPResult, MsgError};
use header::{HeaderRef, HeaderMut, MX};
use message::{self, MessageType};
use message::message::{SSDPMessage};
use receiver::{SSDPReceiver, FromRawSSDP};

/// Overhead to add to device response times to account for transport time.
const NETWORK_TIMEOUT_OVERHEAD: u8 = 1;

/// Devices are required to respond within 1 second of receiving unicast message.
const DEFAULT_UNICAST_TIMEOUT: u8 = 1 + NETWORK_TIMEOUT_OVERHEAD;

/// Search request that can be sent via unicast or multicast to devices on the network.
#[derive(Debug, Clone)]
pub struct SearchRequest {
    message: SSDPMessage
}

impl SearchRequest {
    /// Construct a new SearchRequest.
    pub fn new() -> SearchRequest {
        SearchRequest{ message: SSDPMessage::new(MessageType::Search) }
    }
    
    /// Send this search request to a single host.
    ///
    /// Currently this sends the unicast message on all available network
    /// interfaces. This assumes that the network interfaces are operating
    /// on either different subnets or different ip address ranges.
    pub fn unicast<A: ToSocketAddrs>(&mut self, dst_addr: A) -> SSDPResult<SSDPReceiver<SearchResponse>> {
        let mut connectors = try!(message::all_local_connectors(None));
        
        // Send On All Connectors
        for connector in connectors.iter_mut() {
            try!(self.message.send(connector, &dst_addr));
        }
        
        let connectors = connectors.map_in_place(|conn| conn.deconstruct());
        let opt_timeout = opt_unicast_timeout(self.get::<MX>());
        
        Ok(try!(SSDPReceiver::new(connectors, opt_timeout)))
    }
    
    /// Send this search request to the standard multicast address.
    pub fn multicast<A: ToSocketAddrs>(&mut self) -> SSDPResult<SSDPReceiver<SearchResponse>> {
        let mcast_addr = (message::UPNP_MULTICAST_ADDR, message::UPNP_MULTICAST_PORT);
        let mcast_timeout = try!(multicast_timeout(self.get::<MX>()));
        let mcast_ttl = Some(message::UPNP_MULTICAST_TTL);
        
        let mut connectors = try!(message::all_local_connectors(mcast_ttl));
        
        // Send On All Connectors
        for conn in connectors.iter_mut() {
            try!(self.message.send(conn, &mcast_addr));
        }
        
        let connectors = connectors.map_in_place(|conn| conn.deconstruct());
    
        Ok(try!(SSDPReceiver::new(connectors, Some(mcast_timeout))))
    }
}

/// Get the require timeout to use for a multicast search request.
fn multicast_timeout(mx: Option<&MX>) -> SSDPResult<Duration> {
    match mx {
        Some(&MX(n)) => Ok(Duration::seconds((n + NETWORK_TIMEOUT_OVERHEAD) as i64)),
        None         => try!(Err(MsgError::new("Multicast Searches Require An MX Header")))
    }
}

/// Get the default timeout to use for a unicast search request.
fn opt_unicast_timeout(mx: Option<&MX>) -> Option<Duration> {
    match mx {
        Some(&MX(n)) => Some(Duration::seconds((n + NETWORK_TIMEOUT_OVERHEAD) as i64)),
        None         => Some(Duration::seconds(DEFAULT_UNICAST_TIMEOUT as i64))
    }
}

impl FromRawSSDP for SearchRequest {
    fn raw_ssdp(bytes: &[u8]) -> SSDPResult<SearchRequest> {
        let message = try!(SSDPMessage::raw_ssdp(bytes));
        
        if message.message_type() != MessageType::Search {
            try!(Err(MsgError::new("SSDP Message Received Is Not A SearchRequest")))
        } else { 
            Ok(SearchRequest{ message: message })
        }
    }
}

impl HeaderRef for SearchRequest {
    fn get<H>(&self) -> Option<&H> where H: Header + HeaderFormat {
        self.message.get::<H>()
    }
    
    fn get_raw(&self, name: &str) -> Option<&[Vec<u8>]> {
        self.message.get_raw(name)
    }
}

impl HeaderMut for SearchRequest {
    fn set<H>(&mut self, value: H) where H: Header + HeaderFormat {
        self.message.set(value)
    }
    
    fn set_raw<K>(&mut self, name: K, value: Vec<Vec<u8>>) where K: Into<Cow<'static, str>> {
        self.message.set_raw(name, value)
    }
}

/// Search response that can be received or sent via unicast to devices on the network.
#[derive(Debug, Clone)]
pub struct SearchResponse {
    message: SSDPMessage
}

impl SearchResponse {
    /// Construct a new SearchResponse.
    pub fn new() -> SearchResponse {
        SearchResponse{ message: SSDPMessage::new(MessageType::Response) }
    }
    
    /// Send this search response to a single host.
    ///
    /// Currently this sends the unicast message on all available network
    /// interfaces. This assumes that the network interfaces are operating
    /// on either different subnets or different ip address ranges.
    pub fn unicast<A: ToSocketAddrs>(&mut self, dst_addr: A) -> SSDPResult<()> {
        let mut connectors = try!(message::all_local_connectors(None));
        
        // Send On All Connectors
        for conn in connectors.iter_mut() {
            try!(self.message.send(conn, &dst_addr));
        }
        
        Ok(())
    }
}

impl FromRawSSDP for SearchResponse {
    fn raw_ssdp(bytes: &[u8]) -> SSDPResult<SearchResponse> {
        let message = try!(SSDPMessage::raw_ssdp(bytes));
        
        if message.message_type() != MessageType::Response {
            try!(Err(MsgError::new("SSDP Message Received Is Not A SearchResponse")))
        } else { 
            Ok(SearchResponse{ message: message })
        }
    }
}

impl HeaderRef for SearchResponse {
    fn get<H>(&self) -> Option<&H> where H: Header + HeaderFormat {
        self.message.get::<H>()
    }
    
    fn get_raw(&self, name: &str) -> Option<&[Vec<u8>]> {
        self.message.get_raw(name)
    }
}

impl HeaderMut for SearchResponse {
    fn set<H>(&mut self, value: H) where H: Header + HeaderFormat {
        self.message.set(value)
    }
    
    fn set_raw<K>(&mut self, name: K, value: Vec<Vec<u8>>) where K: Into<Cow<'static, str>> {
        self.message.set_raw(name, value)
    }
}