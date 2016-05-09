use std::fmt::{Formatter, Result};

use hyper::error::{self, Error};
use hyper::header::{HeaderFormat, Header};

const CONFIGID_HEADER_NAME: &'static str = "CONFIGID.UPNP.ORG";

/// Represents a header used to denote the configuration of a device's DDD.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct ConfigID(pub u32);

impl Header for ConfigID {
    fn header_name() -> &'static str {
        CONFIGID_HEADER_NAME
    }

    fn parse_header(raw: &[Vec<u8>]) -> error::Result<Self> {
        if raw.len() != 1 {
            return Err(Error::Header);
        }

        let cow_str = String::from_utf8_lossy(&raw[0][..]);

        // Value needs to be a 31 bit non-negative integer, so convert to i32
        let value = match i32::from_str_radix(&*cow_str, 10) {
            Ok(n) => n,
            Err(_) => return Err(Error::Header),
        };

        // UPnP 1.1 spec says higher numbers are reserved for future use by the
        // technical committee. Devices should use numbers in the range 0 to
        // 16777215 (2^24-1) but I am not sure where the reserved numbers will
        // appear so we will ignore checking that the range is satisfied here.

        // Check if value is negative, then convert to u32
        if value.is_negative() {
            Err(Error::Header)
        } else {
            Ok(ConfigID(value as u32))
        }
    }
}

impl HeaderFormat for ConfigID {
    fn fmt_header(&self, fmt: &mut Formatter) -> Result {
        try!(fmt.write_fmt(format_args!("{}", self.0)));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use hyper::header::Header;

    use super::ConfigID;

    #[test]
    fn positive_configid() {
        let configid_header_value = &[b"1777215"[..].to_vec()];

        ConfigID::parse_header(configid_header_value).unwrap();
    }

    #[test]
    fn positive_reserved() {
        let configid_header_value = &[b"20720000"[..].to_vec()];

        ConfigID::parse_header(configid_header_value).unwrap();
    }

    #[test]
    fn positive_lower_bound() {
        let configid_header_value = &[b"0"[..].to_vec()];

        ConfigID::parse_header(configid_header_value).unwrap();
    }

    #[test]
    fn positive_upper_bound() {
        let configid_header_value = &[b"2147483647"[..].to_vec()];

        ConfigID::parse_header(configid_header_value).unwrap();
    }

    #[test]
    fn positive_negative_zero() {
        let configid_header_value = &[b"-0"[..].to_vec()];

        ConfigID::parse_header(configid_header_value).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_overflow() {
        let configid_header_value = &[b"2290649224"[..].to_vec()];

        ConfigID::parse_header(configid_header_value).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_negative_overflow() {
        let configid_header_value = &[b"-2290649224"[..].to_vec()];

        ConfigID::parse_header(configid_header_value).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_nan() {
        let configid_header_value = &[b"2290wow649224"[..].to_vec()];

        ConfigID::parse_header(configid_header_value).unwrap();
    }
}
