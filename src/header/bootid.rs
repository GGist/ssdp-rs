use std::fmt::{Formatter, Result};

use hyper::error::{self, Error};
use hyper::header::{HeaderFormat, Header};

const BOOTID_HEADER_NAME: &'static str = "BOOTID.UPNP.ORG";

/// Represents a header used to denote the boot instance of a root device.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct BootID(pub u32);

impl Header for BootID {
    fn header_name() -> &'static str {
        BOOTID_HEADER_NAME
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

        // Check if value is negative, then convert to u32
        if value.is_negative() {
            Err(Error::Header)
        } else {
            Ok(BootID(value as u32))
        }
    }
}

impl HeaderFormat for BootID {
    fn fmt_header(&self, fmt: &mut Formatter) -> Result {
        try!(fmt.write_fmt(format_args!("{}", self.0)));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use hyper::header::Header;

    use super::BootID;

    #[test]
    fn positive_bootid() {
        let bootid_header_value = &[b"1216907400"[..].to_vec()];

        BootID::parse_header(bootid_header_value).unwrap();
    }

    #[test]
    fn positive_leading_zeros() {
        let bootid_header_value = &[b"0000001216907400"[..].to_vec()];

        BootID::parse_header(bootid_header_value).unwrap();
    }

    #[test]
    fn positive_lower_bound() {
        let bootid_header_value = &[b"0"[..].to_vec()];

        BootID::parse_header(bootid_header_value).unwrap();
    }

    #[test]
    fn positive_upper_bound() {
        let bootid_header_value = &[b"2147483647"[..].to_vec()];

        BootID::parse_header(bootid_header_value).unwrap();
    }

    #[test]
    fn positive_negative_zero() {
        let bootid_header_value = &[b"-0"[..].to_vec()];

        BootID::parse_header(bootid_header_value).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_overflow() {
        let bootid_header_value = &[b"2290649224"[..].to_vec()];

        BootID::parse_header(bootid_header_value).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_negative_overflow() {
        let bootid_header_value = &[b"-2290649224"[..].to_vec()];

        BootID::parse_header(bootid_header_value).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_nan() {
        let bootid_header_value = &[b"2290wow649224"[..].to_vec()];

        BootID::parse_header(bootid_header_value).unwrap();
    }
}
