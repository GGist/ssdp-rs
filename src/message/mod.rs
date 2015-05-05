use std::borrow::{Cow, IntoCow, ToOwned};
use std::error::{Error};
use std::net::{ToSocketAddrs, UdpSocket, SocketAddr};

use hyper::buffer::{BufReader};
use hyper::client::request::{Request};
use hyper::error::{HttpResult};
use hyper::header::{Headers, Header, HeaderFormat};
use hyper::http::{self, Incoming, RawStatus};
use hyper::method::{Method};
use hyper::uri::{RequestUri};
use hyper::version::{HttpVersion};
use url::{Url, SchemeData};

use {SSDPResult, SSDPError};
use header::{HeaderRef, HeaderMut};
use net::connector::{UdpConnector};
use receiver::{FromRawSSDP};

const NOTIFY_METHOD: &'static str = "NOTIFY";
const SEARCH_METHOD: &'static str = "M-SEARCH";

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
    
    pub fn send<A: ToSocketAddrs>(&self, dest_addr: A) -> Result<UdpSocket, Box<Error>> {
        let mut connector = try!(UdpConnector::new(dest_addr).map_err(|e|
            Box::new(e) as Box<Error>
        ));
        
        match self.method {
            MessageType::Notify => {
                try!(send_request(NOTIFY_METHOD, &self.headers, &mut connector))
            },
            MessageType::Search => {
                try!(send_request(SEARCH_METHOD, &self.headers, &mut connector))
            },
            MessageType::Response(n) => {
                panic!("Unimplemented")
            }
        }
        
        connector.clone_udp().map_err(|e| Box::new(e) as Box<Error>)
    }
    /*
    pub fn unicast<T>(dest: T) -> Result<SSDPReceiver<SSDPMessage>>
        where T: ToSocketAddrs {
        
    }
    
    pub fn multicast() -> Result<SSDPReceiver<SSDPMessage>> {
        
    }*/
}

/// Construct and send a request on the UdpConnector with the supplied method
/// and headers.
fn send_request(method: &str, headers: &Headers, connector: &mut UdpConnector)
    -> Result<(), Box<Error>> {
    let mut request = try!(Request::with_connector(
        Method::Extension(NOTIFY_METHOD.to_owned()),
        build_mock_url(),
        connector
    ).map_err(|e| Box::new(e) as Box<Error>));
    
    copy_headers(&headers, &mut request.headers_mut());
    
    // Send Will Always Fail As Per The UdpConnector So Ignore That Result
    try!(request.start().map_err(|e| Box::new(e) as Box<Error>)).send();
    
    Ok(())
}

/// Creates a mock Url in instances where we are using our custom UdpConnector
/// to send to a host on a SocketAddr where a Url is not necessary for us.
fn build_mock_url() -> Url {
    Url{ scheme: "".to_owned(),
         scheme_data: SchemeData::NonRelative("".to_owned()),
         query: None,
         fragment: None }
}

/// Copy the headers from the source header struct to the destination header struct.
fn copy_headers(src_headers: &Headers, dst_headers: &mut Headers) {
    // Not the best solution since we are doing a lot of string
    // allocations for no benefit other than to transfer the headers.
    
    // TODO: See if there is a way around calling to_owned() since set_raw
    // requires a Cow<'static, _> and we only have access to Cow<'a, _>.
    let iter = src_headers.iter();
    for view in iter {
        dst_headers.set_raw(view.name().to_owned().into_cow(), vec![view.value_string().into_bytes()]);
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
    
    Ok(SSDPMessage{ method: MessageType::Response(status_code), headers: headers })
}

/// Validate the HTTP version for an SSDP message.
fn validate_http_version(version: HttpVersion) -> SSDPResult<()> {
    if version != HttpVersion::Http11 {
        Err(SSDPError::InvalidHttpVersion)
    } else { Ok(()) }
}