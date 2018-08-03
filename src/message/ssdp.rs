use std::borrow::{Cow, ToOwned};
use std::fmt::Debug;
use std::io::Write;
use std::net::{ToSocketAddrs, SocketAddr};

use hyper::Url;
use hyper::buffer::BufReader;
use hyper::client::request::Request;
use hyper::header::{Headers, Header, HeaderFormat, ContentLength, Host};
use hyper::http::RawStatus;
use hyper::http::h1::{self, Incoming};
use hyper::method::Method;
use hyper::net::{NetworkConnector, NetworkStream};
use hyper::server::response::Response;
use hyper::status::StatusCode;
use hyper::uri::RequestUri;
use hyper::version::HttpVersion;

use {SSDPResult, SSDPErrorKind};
use header::{HeaderRef, HeaderMut};
use message::MessageType;
use net;
use receiver::FromRawSSDP;


/// Only Valid `SearchResponse` Code
const VALID_RESPONSE_CODE: u16 = 200;

/// Appended To Destination Socket Addresses For URLs
const BASE_HOST_URL: &'static str = "http://";

/// Case-Sensitive Method Names
const NOTIFY_METHOD: &'static str = "NOTIFY";
const SEARCH_METHOD: &'static str = "M-SEARCH";

/// Represents an SSDP method combined with both SSDP and HTTP headers.
#[derive(Debug, Clone)]
pub struct SSDPMessage {
    method: MessageType,
    headers: Headers,
}

impl SSDPMessage {
    /// Construct a new SSDPMessage.
    pub fn new(message_type: MessageType) -> SSDPMessage {
        SSDPMessage {
            method: message_type,
            headers: Headers::new(),
        }
    }

    /// Get the type of this message.
    pub fn message_type(&self) -> MessageType {
        self.method
    }

    /// Send this request to the given destination address using the given connector.
    ///
    /// The host header field will be taken care of by the underlying library.
    pub fn send<A: ToSocketAddrs, C, S>(&self, connector: &mut C, dst_addr: A) -> SSDPResult<()>
        where C: NetworkConnector<Stream = S>,
              S: Into<Box<NetworkStream + Send>>
    {
        let dst_sock_addr = try!(net::addr_from_trait(dst_addr));
        match self.method {
            MessageType::Notify => {
                trace!("Notify to: {:?}", dst_sock_addr);
                send_request(NOTIFY_METHOD, &self.headers, connector, dst_sock_addr)
            }
            MessageType::Search => {
                trace!("Sending search request...");
                send_request(SEARCH_METHOD, &self.headers, connector, dst_sock_addr)
            }
            MessageType::Response => {
                trace!("Sending response to: {:?}", dst_sock_addr);
                // This might need fixing for IPV6, passing down the IP loses the scope information
                let dst_ip_string = dst_sock_addr.ip().to_string();
                let dst_port = dst_sock_addr.port();

                let net_stream = try!(connector.connect(&dst_ip_string[..], dst_port, "")).into();

                send_response(&self.headers, net_stream)
            }
        }
    }
}

#[allow(unused)]
/// Send a request using the connector with the supplied method and headers.
fn send_request<C, S>(method: &str,
                      headers: &Headers,
                      connector: &mut C,
                      dst_addr: SocketAddr)
                      -> SSDPResult<()>
    where C: NetworkConnector<Stream = S>,
          S: Into<Box<NetworkStream + Send>>
{
    trace!("Trying to parse url...");
    let url = try!(url_from_addr(dst_addr));
    trace!("Url: {}", url);

    let mut request = try!(Request::with_connector(Method::Extension(method.to_owned()), url, connector));

    trace!("Copying headers...");
    copy_headers(headers, request.headers_mut());
    trace!("Setting length");
    request.headers_mut().set(ContentLength(0));

    // Send Will Always Fail Within The UdpConnector Which Is Intended So That
    // Hyper Does Not Block For A Response Since We Are Handling That Ourselves.

    trace!("actual .send ...");
    try!(request.start()).send();

    Ok(())
}

/// Send an Ok response on the Writer with the supplied headers.
fn send_response<W>(headers: &Headers, mut dst_writer: W) -> SSDPResult<()>
    where W: Write
{
    let mut temp_headers = Headers::new();

    copy_headers(headers, &mut temp_headers);
    temp_headers.set(ContentLength(0));

    let mut response = Response::new(&mut dst_writer as &mut Write, &mut temp_headers);
    *response.status_mut() = StatusCode::Ok;

    // Have to make sure response is destroyed here for lifetime issues with temp_headers
    try!(try!(response.start()).end());

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
        dst_headers.set_raw(Cow::Owned(view.name().to_owned()), vec![view.value_string().into_bytes()]);
    }
}

impl HeaderRef for SSDPMessage {
    fn get<H>(&self) -> Option<&H>
        where H: Header + HeaderFormat
    {
        HeaderRef::get::<H>(&self.headers)
    }

    fn get_raw(&self, name: &str) -> Option<&[Vec<u8>]> {
        HeaderRef::get_raw(&self.headers, name)
    }
}

impl HeaderMut for SSDPMessage {
    fn set<H>(&mut self, value: H)
        where H: Header + HeaderFormat
    {
        HeaderMut::set(&mut self.headers, value)
    }

    fn set_raw<K>(&mut self, name: K, value: Vec<Vec<u8>>)
        where K: Into<Cow<'static, str>> + Debug
    {
        HeaderMut::set_raw(&mut self.headers, name, value)
    }
}

impl FromRawSSDP for SSDPMessage {
    fn raw_ssdp(bytes: &[u8]) -> SSDPResult<SSDPMessage> {
        let mut buf_reader = BufReader::new(bytes);

        if let Ok(parts) = h1::parse_request(&mut buf_reader) {
            let message_result = message_from_request(parts);

            log_message_result(&message_result, bytes);
            message_result
        } else {
            match h1::parse_response(&mut buf_reader) {
                Ok(parts) => {
                    let message_result = message_from_response(parts);

                    log_message_result(&message_result, bytes);
                    message_result
                },
                Err(err) => {
                    debug!("Failed parsing http response: {}, data: {}", err, String::from_utf8_lossy(bytes));

                    Err(SSDPErrorKind::InvalidHttp(bytes.to_owned()).into())
                }
            }
        } 
    }
}

/// Logs a debug! message based on the value of the `SSDPResult`.
fn log_message_result(result: &SSDPResult<SSDPMessage>, message: &[u8]) {
    match *result {
        Ok(_) => debug!("Received Valid SSDPMessage:\n{}", String::from_utf8_lossy(message)),
        Err(ref e) => debug!("Received Invalid SSDPMessage Error: {}", e),
    }
}

/// Attempts to construct an `SSDPMessage` from the given request pieces.
fn message_from_request(parts: Incoming<(Method, RequestUri)>) -> SSDPResult<SSDPMessage> {
    let headers = parts.headers;

    try!(validate_http_version(parts.version));
    try!(validate_http_host(&headers));

    match parts.subject {
        (Method::Extension(n), RequestUri::Star) => {
            match &n[..] {
                NOTIFY_METHOD => {
                    Ok(SSDPMessage {
                        method: MessageType::Notify,
                        headers: headers,
                    })
                }
                SEARCH_METHOD => {
                    Ok(SSDPMessage {
                        method: MessageType::Search,
                        headers: headers,
                    })
                }
                _ => Err(SSDPErrorKind::InvalidMethod(n).into()),
            }
        }
        (n, RequestUri::Star) => Err(SSDPErrorKind::InvalidMethod(n.to_string()).into()),
        (_, RequestUri::AbsolutePath(n)) |
        (_, RequestUri::Authority(n)) => Err(SSDPErrorKind::InvalidUri(n).into()),
        (_, RequestUri::AbsoluteUri(n)) => Err(SSDPErrorKind::InvalidUri(n.into_string()).into()),
    }
}

/// Attempts to construct an `SSDPMessage` from the given response pieces.
fn message_from_response(parts: Incoming<RawStatus>) -> SSDPResult<SSDPMessage> {
    let RawStatus(status_code, _) = parts.subject;
    let headers = parts.headers;

    try!(validate_http_version(parts.version));
    try!(validate_response_code(status_code));

    Ok(SSDPMessage {
        method: MessageType::Response,
        headers: headers,
    })
}

/// Validate the HTTP version for an SSDP message.
fn validate_http_version(version: HttpVersion) -> SSDPResult<()> {
    if version != HttpVersion::Http11 {
        Err(SSDPErrorKind::InvalidHttpVersion.into())
    } else {
        Ok(())
    }
}

/// Validate that the Host header is present.
fn validate_http_host<T>(headers: T) -> SSDPResult<()>
    where T: HeaderRef
{
    // Shouldn't have to do this but hyper doesn't make sure that HTTP/1.1
    // messages contain Host headers so we will assure conformance ourselves.
    if headers.get::<Host>().is_none() {
        Err(SSDPErrorKind::MissingHeader(Host::header_name()).into())
    } else {
        Ok(())
    }
}

/// Validate the response code for an SSDP message.
fn validate_response_code(code: u16) -> SSDPResult<()> {
    if code != VALID_RESPONSE_CODE {
        Err(SSDPErrorKind::ResponseCode(code).into())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod mocks {
    use std::cell::RefCell;
    use std::io::{self, Read, Write, ErrorKind};
    use std::net::SocketAddr;
    use std::time::Duration;
    use std::sync::mpsc::{self, Sender, Receiver};

    use hyper::error;
    use hyper::net::{NetworkConnector, NetworkStream};

    pub struct MockConnector {
        pub receivers: RefCell<Vec<Receiver<Vec<u8>>>>,
    }

    impl MockConnector {
        pub fn new() -> MockConnector {
            MockConnector { receivers: RefCell::new(Vec::new()) }
        }
    }

    impl NetworkConnector for MockConnector {
        type Stream = MockStream;

        fn connect(&self, _: &str, _: u16, _: &str) -> error::Result<Self::Stream> {
            let (send, recv) = mpsc::channel();

            self.receivers.borrow_mut().push(recv);

            Ok(MockStream { sender: send })
        }
    }

    pub struct MockStream {
        sender: Sender<Vec<u8>>,
    }

    impl NetworkStream for MockStream {
        fn peer_addr(&mut self) -> io::Result<SocketAddr> {
            Err(io::Error::new(ErrorKind::AddrNotAvailable, ""))
        }
        fn set_read_timeout(&self, _dur: Option<Duration>) -> io::Result<()> {
            Ok(())
        }
        fn set_write_timeout(&self, _dur: Option<Duration>) -> io::Result<()> {
            Ok(())
        }
    }

    impl Read for MockStream {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::new(ErrorKind::ConnectionAborted, ""))
        }
    }

    impl Write for MockStream {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            // Hyper will generate a request with a /, we need to intercept that.
            let mut buffer = vec![0u8; buf.len()];

            let mut found = false;
            for (src, dst) in buf.iter().zip(buffer.iter_mut()) {
                if *src == b'/' && !found && buf[0] != b'H' {
                    *dst = b'*';
                    found = true;
                } else {
                    *dst = *src;
                }
            }

            self.sender.send(buffer).unwrap();

            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    mod send {
        use std::sync::mpsc::Receiver;

        use super::super::mocks::MockConnector;
        use super::super::SSDPMessage;
        use message::MessageType;

        fn join_buffers(recv_list: &[Receiver<Vec<u8>>]) -> Vec<u8> {
            let mut buffer = Vec::new();

            for recv in recv_list {
                for recv_buf in recv {
                    buffer.extend(&recv_buf[..])
                }
            }

            buffer
        }

        #[test]
        fn positive_search_method_line() {
            let message = SSDPMessage::new(MessageType::Search);
            let mut connector = MockConnector::new();

            message.send(&mut connector, ("127.0.0.1", 0)).unwrap();

            let sent_message = String::from_utf8(join_buffers(&*connector.receivers.borrow())).unwrap();

            assert_eq!(&sent_message[..19], "M-SEARCH * HTTP/1.1");
        }

        #[test]
        fn positive_notify_method_line() {
            let message = SSDPMessage::new(MessageType::Notify);
            let mut connector = MockConnector::new();

            message.send(&mut connector, ("127.0.0.1", 0)).unwrap();

            let sent_message = String::from_utf8(join_buffers(&*connector.receivers.borrow())).unwrap();

            assert_eq!(&sent_message[..17], "NOTIFY * HTTP/1.1");
        }

        #[test]
        fn positive_response_method_line() {
            let message = SSDPMessage::new(MessageType::Response);
            let mut connector = MockConnector::new();

            message.send(&mut connector, ("127.0.0.1", 0)).unwrap();

            let sent_message = String::from_utf8(join_buffers(&*connector.receivers.borrow())).unwrap();

            assert_eq!(&sent_message[..15], "HTTP/1.1 200 OK");
        }

        #[test]
        fn positive_host_header() {
            let message = SSDPMessage::new(MessageType::Search);
            let mut connector = MockConnector::new();

            message.send(&mut connector, ("127.0.0.1", 0)).unwrap();

            let sent_message = String::from_utf8(join_buffers(&*connector.receivers.borrow())).unwrap();

            assert!(sent_message.contains("Host: 127.0.0.1:0"));
        }
    }

    mod parse {
        use super::super::SSDPMessage;
        use header::HeaderRef;
        use receiver::FromRawSSDP;

        #[test]
        fn positive_valid_http() {
            let raw_message = "NOTIFY * HTTP/1.1\r\nHOST: 192.168.1.1\r\n\r\n";

            SSDPMessage::raw_ssdp(raw_message.as_bytes()).unwrap();
        }

        #[test]
        fn positive_intact_header() {
            let raw_message = "NOTIFY * HTTP/1.1\r\nHOST: 192.168.1.1\r\n\r\n";
            let message = SSDPMessage::raw_ssdp(raw_message.as_bytes()).unwrap();

            assert_eq!(&message.get_raw("Host").unwrap()[0][..], &b"192.168.1.1"[..]);
        }

        #[test]
        #[should_panic]
        fn negative_http_version() {
            let raw_message = "NOTIFY * HTTP/2.0\r\nHOST: 192.168.1.1\r\n\r\n";

            SSDPMessage::raw_ssdp(raw_message.as_bytes()).unwrap();
        }

        #[test]
        #[should_panic]
        fn negative_no_host() {
            let raw_message = "NOTIFY * HTTP/1.1\r\n\r\n";

            SSDPMessage::raw_ssdp(raw_message.as_bytes()).unwrap();
        }

        #[test]
        #[should_panic]
        fn negative_path_included() {
            let raw_message = "NOTIFY / HTTP/1.1\r\n\r\n";

            SSDPMessage::raw_ssdp(raw_message.as_bytes()).unwrap();
        }
    }
}
