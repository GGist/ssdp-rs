use std::fmt::{Formatter, Result};

use hyper::error::{self, Error};
use hyper::header::{HeaderFormat, Header};

const SEARCHPORT_HEADER_NAME: &'static str = "SEARCHPORT.UPNP.ORG";

pub const SEARCHPORT_MIN_VALUE: u16 = 49152;

/// Represents a header used to specify a unicast port to send search requests to.
///
/// If a `SearchPort` header is not included in a message then the device must
/// respond to unicast search requests on the standard port of 1900.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct SearchPort(pub u16);

impl Header for SearchPort {
    fn header_name() -> &'static str {
        SEARCHPORT_HEADER_NAME
    }

    fn parse_header(raw: &[Vec<u8>]) -> error::Result<Self> {
        if raw.len() != 1 {
            return Err(Error::Header);
        }

        let cow_str = String::from_utf8_lossy(&raw[0][..]);

        let value = match u16::from_str_radix(&*cow_str, 10) {
            Ok(n) => n,
            Err(_) => return Err(Error::Header),
        };

        if value >= SEARCHPORT_MIN_VALUE {
            Ok(SearchPort(value))
        } else {
            Err(Error::Header)
        }
    }
}

impl HeaderFormat for SearchPort {
    fn fmt_header(&self, fmt: &mut Formatter) -> Result {
        try!(fmt.write_fmt(format_args!("{}", self.0)));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use hyper::header::Header;

    use super::SearchPort;

    #[test]
    fn positive_searchport() {
        let searchport_header_value = &[b"50000"[..].to_vec()];

        SearchPort::parse_header(searchport_header_value).unwrap();
    }

    #[test]
    fn positive_lower_bound() {
        let searchport_header_value = &[b"49152"[..].to_vec()];

        SearchPort::parse_header(searchport_header_value).unwrap();
    }

    #[test]
    fn positive_upper_bound() {
        let searchport_header_value = &[b"65535"[..].to_vec()];

        SearchPort::parse_header(searchport_header_value).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_reserved() {
        let searchport_header_value = &[b"49151"[..].to_vec()];

        SearchPort::parse_header(searchport_header_value).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_nan() {
        let searchport_header_value = &[b"49151a"[..].to_vec()];

        SearchPort::parse_header(searchport_header_value).unwrap();
    }
}
