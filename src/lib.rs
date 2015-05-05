#![feature(collections)]

use std::error::{Error};
use std::fmt::{self, Display, Formatter};

extern crate hyper;

mod field;

pub mod header;
pub mod message;

pub use field::{FieldMap};

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

/// Construct Self via an attempted conversion.
trait MaybeFrom<T> {
    /// Attempt to convert a T.
    fn maybe_from(T) -> Option<Self>;
}