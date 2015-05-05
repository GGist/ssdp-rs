use std::borrow::{Cow};

use hyper::buffer::{BufReader};
use hyper::header::{Headers, Header, HeaderFormat};
use hyper::http::{self, Incoming, RawStatus};
use hyper::method::{Method};
use hyper::uri::{RequestUri};
use hyper::version::{HttpVersion};

use {SSDPResult, SSDPError};
use header::{HeaderRef, HeaderMut};
use receiver::{FromRawSSDP};

const NOTIFY_HEADER: &'static str = "NOTIFY";
const SEARCH_HEADER: &'static str = "M-SEARCH";

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub enum MessageType {
    /// A notify message.
    Notify,
    /// A search message.
    Search,
    /// A response to a search message.
    Response(u16)
}

#[derive(Debug, Clone)]
pub struct SSDPMessage {
    method:  MessageType,
    headers: Headers
}

impl SSDPMessage {
    pub fn new(message_type: MessageType) -> SSDPMessage {
        SSDPMessage{ method: message_type, headers: Headers::new() }
    }
    
    pub fn message_type(&self) -> MessageType {
        self.method
    }
    /*
    pub fn unicast<T>(dest: T) -> Result<SSDPReceiver<SSDPMessage>>
        where T: ToSocketAddrs {
        
    }
    
    pub fn multicast() -> Result<SSDPReceiver<SSDPMessage>> {
        
    }*/
}

impl FromRawSSDP for SSDPMessage {
    fn raw_ssdp(bytes: &[u8]) -> SSDPResult<SSDPMessage> {
        let mut buf_reader = BufReader::new(bytes);
        
        if let Ok(parts) = http::parse_request(&mut buf_reader) {
            message_from_request(parts)
        } else if let Ok(parts) = http::parse_response(&mut buf_reader) {
            message_from_response(parts)
        } else {
            Err(SSDPError::InvalidHttp(bytes.to_owned()))
        }
    }
}

impl HeaderRef for SSDPMessage {
    fn get<H>(&self) -> Option<&H> where H: Header + HeaderFormat {
        HeaderRef::get::<H>(&self.headers)
    }
    
    fn get_raw(&self, name: &str) -> Option<&[Vec<u8>]> {
        HeaderRef::get_raw(&self.headers, name)
    }
}

impl HeaderMut for SSDPMessage {
    fn set<H>(&mut self, value: H) where H: Header + HeaderFormat {
        HeaderMut::set(&mut self.headers, value)
    }
    
    fn set_raw<K>(&mut self, name: K, value: Vec<Vec<u8>>) where K: Into<Cow<'static, str>> {
        HeaderMut::set_raw(&mut self.headers, name, value)
    }
}

/// Attempts to construct an SSDPMessage from the given request pieces.
fn message_from_request(parts: Incoming<(Method, RequestUri)>) -> SSDPResult<SSDPMessage> {
    let headers = parts.headers;

    try!(validate_http_version(parts.version));
    
    match parts.subject {
        (Method::Extension(n), RequestUri::Star) => {
            match &n[..] {
                NOTIFY_HEADER => Ok(SSDPMessage{ method: MessageType::Notify, headers: headers }),
                SEARCH_HEADER => Ok(SSDPMessage{ method: MessageType::Search, headers: headers }),
                _ => Err(SSDPError::InvalidMethod(n))
            }
        },
        (n, RequestUri::Star)            => Err(SSDPError::InvalidMethod(n.to_string())),
        (_, RequestUri::AbsolutePath(n)) => Err(SSDPError::InvalidUri(n)),
        (_, RequestUri::Authority(n))    => Err(SSDPError::InvalidUri(n)),
        (_, RequestUri::AbsoluteUri(n))  => Err(SSDPError::InvalidUri(n.serialize()))
    }
}

/// Attempts to construct an SSDPMessage from the given response pieces.
fn message_from_response(parts: Incoming<RawStatus>) -> SSDPResult<SSDPMessage> {
    let RawStatus(status_code, _) = parts.subject;
    let headers = parts.headers;
    
    try!(validate_http_version(parts.version));
    
    Ok(SSDPMessage{ method: MessageType::Response(status_code), headers: headers })
}

/// Validate the HTTP version for an SSDP message.
fn validate_http_version(version: HttpVersion) -> SSDPResult<()> {
    if version != HttpVersion::Http11 {
        Err(SSDPError::InvalidHttpVersion)
    } else { Ok(()) }
}