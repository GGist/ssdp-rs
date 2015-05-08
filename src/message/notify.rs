use std::borrow::{Cow};
use std::net::{IpAddr};
use std::str::{FromStr};

use hyper::header::{Header, HeaderFormat};

use error::{SSDPResult, MsgError};
use header::{HeaderRef, HeaderMut};
use message::{self, MessageType};
use message::ssdp::{SSDPMessage};
use net::{self};
use receiver::{SSDPReceiver, FromRawSSDP};

/// Notify message that can be sent via unicast or multicast to devices on the network.
#[derive(Debug, Clone)]
pub struct NotifyMessage {
    message: SSDPMessage
}

impl NotifyMessage {
    /// Construct a new NotifyMessage.
    pub fn new() -> NotifyMessage {
        NotifyMessage{ message: SSDPMessage::new(MessageType::Notify) }
    }
    
    /// Send this notify message to the standard multicast address.
    pub fn multicast(&mut self) -> SSDPResult<()> {
        let mcast_addr = (message::UPNP_MULTICAST_ADDR, message::UPNP_MULTICAST_PORT);
        let mcast_ttl = Some(message::UPNP_MULTICAST_TTL);
        
        let mut connectors = try!(message::all_local_connectors(mcast_ttl));
        
        // Send On All Connectors
        for conn in connectors.iter_mut() {
            try!(self.message.send(conn, &mcast_addr));
        }
        
        Ok(())
    }
}
    
impl FromRawSSDP for NotifyMessage {
    fn raw_ssdp(bytes: &[u8]) -> SSDPResult<NotifyMessage> {
        let message = try!(SSDPMessage::raw_ssdp(bytes));
        
        if message.message_type() != MessageType::Notify {
            try!(Err(MsgError::new("SSDP Message Received Is Not A NotifyMessage")))
        } else { 
            Ok(NotifyMessage{ message: message })
        }
    }
}

impl HeaderRef for NotifyMessage {
    fn get<H>(&self) -> Option<&H> where H: Header + HeaderFormat {
        self.message.get::<H>()
    }
    
    fn get_raw(&self, name: &str) -> Option<&[Vec<u8>]> {
        self.message.get_raw(name)
    }
}

impl HeaderMut for NotifyMessage {
    fn set<H>(&mut self, value: H) where H: Header + HeaderFormat {
        self.message.set(value)
    }
    
    fn set_raw<K>(&mut self, name: K, value: Vec<Vec<u8>>) where K: Into<Cow<'static, str>> {
        self.message.set_raw(name, value)
    }
}

/// Notify message listener that can listen to notify messages sent within the network.
pub struct NotifyListener;

impl NotifyListener {
    /// Listen for notify messages on all local network interfaces.
    pub fn listen() -> SSDPResult<SSDPReceiver<NotifyMessage>> {
        // Generate a list of reused sockets on the standard multicast address.
        let mut reuse_sockets = try!(message::map_local_ipv4(|&addr| {
            net::reuse_socket((addr, message::UPNP_MULTICAST_PORT))
        }));
        
        let mcast_addr = try!(IpAddr::from_str(message::UPNP_MULTICAST_ADDR)
            .map_err(|_| MsgError::new("Could Not Parse UPNP_MULTICAST_ADDR") ));
        
        // Subscribe To Multicast On All Of Them
        for sock in reuse_sockets.iter_mut() {
            try!(sock.join_multicast(&mcast_addr));
        }
        
        Ok(try!(SSDPReceiver::new(reuse_sockets, None)))
    }
}