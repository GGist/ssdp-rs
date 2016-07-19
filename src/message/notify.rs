use std::borrow::Cow;
use std::fmt::Debug;
use std::str::FromStr;
use std::net::{SocketAddr, IpAddr, Ipv6Addr};

use hyper::header::{Header, HeaderFormat};

use error::{SSDPResult, MsgError};
use header::{HeaderRef, HeaderMut};
use message::{self, MessageType};
use message::ssdp::SSDPMessage;
use receiver::{SSDPReceiver, FromRawSSDP};
use net;

/// Notify message that can be sent via multicast to devices on the network.
#[derive(Debug, Clone)]
pub struct NotifyMessage {
    message: SSDPMessage,
}

impl NotifyMessage {
    /// Construct a new NotifyMessage.
    pub fn new() -> Self {
        NotifyMessage { message: SSDPMessage::new(MessageType::Notify) }
    }

    /// Send this notify message to the standard multicast address:port.
    pub fn multicast(&mut self) -> SSDPResult<()> {
        self.multicast_with_port(message::UPNP_MULTICAST_PORT)
    }

    /// Send this notify message to the standard multicast address but a custom port.
    pub fn multicast_with_port(&mut self, port: u16) -> SSDPResult<()> {
        let mcast_ttl = Some(message::UPNP_MULTICAST_TTL);

        let mut connectors = try!(message::all_local_connectors(mcast_ttl, net::IpVersionMode::Any));

        // Send On All Connectors
        for conn in &mut connectors {
            match try!(conn.local_addr()) {
                SocketAddr::V4(n) => {
                    let mcast_addr = (message::UPNP_MULTICAST_IPV4_ADDR, port);
                    debug!("Sending ipv4 multicast through {} to {:?}", n, mcast_addr);
                    try!(self.message.send(conn, &mcast_addr));
                }
                SocketAddr::V6(n) => {
                    let mcast_addr = (message::UPNP_MULTICAST_IPV6_LINK_LOCAL_ADDR, port);
                    debug!("Sending Ipv6 multicast through {} to {:?}", n, mcast_addr);
                    try!(self.message.send(conn, &mcast_addr));
                }
            }
        }

        Ok(())
    }
}

impl Default for NotifyMessage {
    fn default() -> Self {
        NotifyMessage::new()
    }
}

impl FromRawSSDP for NotifyMessage {
    fn raw_ssdp(bytes: &[u8]) -> SSDPResult<NotifyMessage> {
        let message = try!(SSDPMessage::raw_ssdp(bytes));

        if message.message_type() != MessageType::Notify {
            try!(Err(MsgError::new("SSDP Message Received Is Not A NotifyMessage")))
        } else {
            Ok(NotifyMessage { message: message })
        }
    }
}

impl HeaderRef for NotifyMessage {
    fn get<H>(&self) -> Option<&H>
        where H: Header + HeaderFormat
    {
        self.message.get::<H>()
    }

    fn get_raw(&self, name: &str) -> Option<&[Vec<u8>]> {
        self.message.get_raw(name)
    }
}

impl HeaderMut for NotifyMessage {
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

/// Notify listener that can listen to notify messages sent within the network.
pub struct NotifyListener;

impl NotifyListener {
    /// Listen for notify messages on all local network interfaces.
    pub fn listen() -> SSDPResult<SSDPReceiver<NotifyMessage>> {
        NotifyListener::listen_on_port(message::UPNP_MULTICAST_PORT)
    }

    /// Listen for notify messages on a custom port on all local network interfaces.
    pub fn listen_on_port(port: u16) -> SSDPResult<SSDPReceiver<NotifyMessage>> {
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

#[cfg(test)]
mod tests {
    use super::NotifyMessage;
    use receiver::FromRawSSDP;

    #[test]
    fn positive_notify_message_type() {
        let raw_message = "NOTIFY * HTTP/1.1\r\nHOST: 192.168.1.1\r\n\r\n";

        NotifyMessage::raw_ssdp(raw_message.as_bytes()).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_search_message_type() {
        let raw_message = "M-SEARCH * HTTP/1.1\r\nHOST: 192.168.1.1\r\n\r\n";

        NotifyMessage::raw_ssdp(raw_message.as_bytes()).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_response_message_type() {
        let raw_message = "HTTP/1.1 200 OK\r\n\r\n";

        NotifyMessage::raw_ssdp(raw_message.as_bytes()).unwrap();
    }
}
