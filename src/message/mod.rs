use std::borrow::{Cow, IntoCow, ToOwned};
use std::io::{ErrorKind};
use std::io::Error as IoError;
use std::iter::{Iterator};
use std::net::{ToSocketAddrs, SocketAddr};

use hyper::buffer::{BufReader};
use hyper::client::request::{Request};
use hyper::header::{Headers, Header, HeaderFormat, Host};
use hyper::http::{self, Incoming, RawStatus};
use hyper::method::{Method};
use hyper::net::{NetworkConnector, NetworkStream};
use hyper::uri::{RequestUri};
use hyper::version::{HttpVersion};
use url::{Url};

use {SSDPResult, SSDPError};
use header::{HeaderRef, HeaderMut};
use receiver::{FromRawSSDP};

pub mod search;

/// Multicast Socket Information
const UPNP_MULTICAST_ADDR: (u8, u8, u8, u8) = (239, 255, 255, 250);
const UPNP_MULTICAST_PORT: u16 = 1900;

/// Only Valid SearchResponse Code
const VALID_RESPONSE_CODE: u16 = 200;

/// Appended To Destination Socket Addresses For URLs
const BASE_HOST_URL: &'static str = "http://";

/// Case-Sensitive Method Names
const NOTIFY_METHOD: &'static str = "NOTIFY";
const SEARCH_METHOD: &'static str = "M-SEARCH";

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

/// Represents an SSDP method combined with both SSDP and HTTP headers.
#[derive(Debug, Clone)]
pub struct SSDPMessage {
    method:  MessageType,
    headers: Headers
}

impl SSDPMessage {
    /// Construct a new SSDPMessage.
    pub fn new(message_type: MessageType) -> SSDPMessage {
        SSDPMessage{ method: message_type, headers: Headers::new() }
    }
    
    /// Get the type of this message.
    pub fn message_type(&self) -> MessageType {
        self.method
    }
    
    /// Send this request to the given destination address using the given connector.
    ///
    /// The host header field will be taken care of by the underlying library.
    pub fn send<A: ToSocketAddrs, C, S>(&self, connector: &mut C, dst_addr: A) -> SSDPResult<()>
        where C: NetworkConnector<Stream=S>, S: Into<Box<NetworkStream + Send>> {
        let dst_sock_addr = try!(addr_from_trait(dst_addr));
        
        match self.method {
            MessageType::Notify => {
                send_request(NOTIFY_METHOD, &self.headers, connector, dst_sock_addr)
            },
            MessageType::Search => {
                send_request(SEARCH_METHOD, &self.headers, connector, dst_sock_addr)
            },
            MessageType::Response => {
                panic!("Unimplemented")
            }
        }
    }
}

/// Accept a type implementing ToSocketAddrs and tries to extract the first address.
fn addr_from_trait<A: ToSocketAddrs>(addr: A) -> SSDPResult<SocketAddr> {
    let mut sock_iter = try!(addr.to_socket_addrs());
    
    match sock_iter.next() {
        Some(n) => Ok(n),
        None    => try!(Err(IoError::new(ErrorKind::InvalidInput, "Failed To Parse SocketAddr")))
    }
}

#[allow(unused)]
/// Send a request on the UdpConnector with the supplied method and headers.
fn send_request<C, S>(method: &str, headers: &Headers, connector: &mut C, dst_addr: SocketAddr)
    -> SSDPResult<()> where C: NetworkConnector<Stream=S>, S: Into<Box<NetworkStream + Send>> {
    let url = try!(url_from_addr(dst_addr));

    let mut request = try!(Request::with_connector(
        Method::Extension(method.to_owned()),
        url,
        connector
    ));

    copy_headers(&headers, request.headers_mut());

    // Send Will Always Fail Within The UdpConnector Which Is Intended So That
    // Hyper Does Not Block For A Response Since We Are Handling That Ourselves.
    try!(request.start()).send();

    Ok(())
}

/// Convert the given address to a Url with a base of "udp://".
fn url_from_addr(addr: SocketAddr) -> SSDPResult<Url> {
    let str_url = BASE_HOST_URL.chars()
        .chain(addr.to_string()[..].chars())
        .collect::<String>();
    
    Ok(try!(Url::parse(&str_url[..])))
}

/// Copy the headers from the source header to the destination header.
fn copy_headers(src_headers: &Headers, dst_headers: &mut Headers) {
    // Not the best solution since we are doing a lot of string
    // allocations for no benefit other than to transfer the headers.
    
    // TODO: See if there is a way around calling to_owned() since set_raw
    // requires a Cow<'static, _> and we only have access to Cow<'a, _>.
    let iter = src_headers.iter();
    for view in iter {
        dst_headers.set_raw(view.name().to_owned().into_cow(),
                            vec![view.value_string().into_bytes()]);
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

/// Attempts to construct an SSDPMessage from the given request pieces.
fn message_from_request(parts: Incoming<(Method, RequestUri)>) -> SSDPResult<SSDPMessage> {
    let headers = parts.headers;

    try!(validate_http_version(parts.version));
    
    match parts.subject {
        (Method::Extension(n), RequestUri::Star) => {
            match &n[..] {
                NOTIFY_METHOD => Ok(SSDPMessage{ method: MessageType::Notify, headers: headers }),
                SEARCH_METHOD => Ok(SSDPMessage{ method: MessageType::Search, headers: headers }),
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
    try!(validate_response_code(status_code));
    
    Ok(SSDPMessage{ method: MessageType::Response, headers: headers })
}

/// Validate the HTTP version for an SSDP message.
fn validate_http_version(version: HttpVersion) -> SSDPResult<()> {
    if version != HttpVersion::Http11 {
        Err(SSDPError::InvalidHttpVersion)
    } else { Ok(()) }
}

/// Validate the response code for an SSDP message.
fn validate_response_code(code: u16) -> SSDPResult<()> {
    if code != VALID_RESPONSE_CODE {
        Err(SSDPError::ResponseCode(code))
    } else { Ok(()) }
}