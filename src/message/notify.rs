use std::borrow::Cow;
use std::fmt::Debug;

use hyper::header::{Header, HeaderFormat};

use error::SSDPResult;
use header::{HeaderRef, HeaderMut};
use message::{MessageType, Listen, Config};
use message::ssdp::SSDPMessage;
use message::multicast::{self, Multicast};
use receiver::FromRawSSDP;


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
}

impl Multicast for NotifyMessage {
    type Item = ();

    fn multicast_with_config(&self, config: &Config) -> SSDPResult<Self::Item> {
        multicast::send(&self.message, config)?;
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
            try!(Err("SSDP Message Received Is Not A NotifyMessage"))
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

impl Listen for NotifyListener {
    type Message = NotifyMessage;
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
