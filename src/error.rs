use std::convert::{From};
use std::error::{Error};
use std::fmt::{self, Display, Formatter};
use std::marker::{Reflect};

/// Result that can return a T or an SSDPError.
pub type SSDPResult<T> = Result<T, SSDPError>;

/// Enumerates all errors that can occur when dealing with an SSDP message.
#[derive(Debug)]
pub enum SSDPError {
    /// Message is not valid HTTP.
    ///
    /// Message is supplied as a list of bytes.
    InvalidHttp(Vec<u8>),
    /// Message did not specify HTTP/1.1 as version.
    InvalidHttpVersion,
    /// Message consists of an error code.
    ///
    /// Error code is supplied.
    ResponseCode(u16),
    /// Method supplied is not a valid SSDP method.
    ///
    /// Method received is supplied.
    InvalidMethod(String),
    /// Uri supplied is not a valid SSDP uri.
    ///
    /// URI received is supplied.
    InvalidUri(String),
    /// Header is missing from the message.
    ///
    /// Expected header is supplied.
    MissingHeader(&'static str),
    /// Header has an invalid value.
    ///
    /// Header name with error message are supplied.
    InvalidHeader(&'static str, &'static str),
    /// Some other error occurred.
    Other(Box<Error>)
}

impl Display for SSDPError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match *self {
            SSDPError::InvalidHttp(ref n) => {
                let http_str = String::from_utf8_lossy(n);
                
                f.write_fmt(format_args!("Invalid Http: {}", http_str))
            },
            SSDPError::InvalidHttpVersion => {
                f.write_str("Invalid Http Version")
            },
            SSDPError::ResponseCode(n) => {
                f.write_fmt(format_args!("Response Code: {}", n))
            },
            SSDPError::InvalidMethod(ref n) => {
                f.write_fmt(format_args!("Invalid Method: {}", n))
            },
            SSDPError::InvalidUri(ref n) => {
                f.write_fmt(format_args!("Invalid URI: {}", n))
            },
            SSDPError::MissingHeader(n) => {
                f.write_fmt(format_args!("Missing Header: {}", n))
            },
            SSDPError::InvalidHeader(name, value) => {
                f.write_fmt(format_args!("Invalid Header: {}: {}", name, value))
            },
            SSDPError::Other(ref n) => {
                f.write_fmt(format_args!("Other: {}", n.description()))
            }
        }
    }
}

impl<T> From<T> for SSDPError where T: Error + 'static {
    fn from(err: T) -> SSDPError {
        SSDPError::Other(Box::new(err) as Box<Error>)
    }
}

/// Basic type implementing the Error trait.
#[derive(Debug)]
pub struct MsgError {
    desc: &'static str
}

impl MsgError {
    pub fn new(desc: &'static str) -> MsgError {
        MsgError{ desc: desc }
    }
}

impl Reflect for MsgError { }

impl Error for MsgError {
    fn description(&self) -> &str {
        self.desc
    }
}

impl Display for MsgError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        f.write_str(self.desc)
    }
}