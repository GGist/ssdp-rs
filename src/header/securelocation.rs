use std::fmt::{Formatter, Result};

use hyper::header::{HeaderFormat, Header};

const SECURELOCATION_HEADER_NAME: &'static str = "SECURELOCATION.UPNP.ORG";

/// Represents a UPnP SecureLocation header which is a url provided by a device
/// which allows control points to retrieve device and service descriptions over
/// HTTPS.
///
/// Can be used instead of the Location header field.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct SecureLocation(pub String);

unsafe impl Sync for SecureLocation { }

unsafe impl Send for SecureLocation { }

impl Header for SecureLocation {
    fn header_name() -> &'static str {
        SECURELOCATION_HEADER_NAME
    }
    
    fn parse_header(raw: &[Vec<u8>]) -> Option<Self> {
        if raw.len() != 1 || raw[0].is_empty() {
            return None
        }
        
        let owned_bytes = raw[0].clone();
        
        match String::from_utf8(owned_bytes) {
            Ok(n)  => Some(SecureLocation(n)),
            Err(_) => None
        }
    }
}

impl HeaderFormat for SecureLocation {
    fn fmt_header(&self, fmt: &mut Formatter) -> Result {
        try!(fmt.write_str(&self.0));
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use hyper::header::{Header};
    
    use super::{SecureLocation};
    
    #[test]
    fn positive_securelocation() {
        let securelocation_header_value = &[b"https://192.168.1.1/"[..].to_vec()];
        
        SecureLocation::parse_header(securelocation_header_value).unwrap();
    }
    
    #[test]
    fn positive_invalid_url() {
        let securelocation_header_value = &[b"just some text"[..].to_vec()];
        
        SecureLocation::parse_header(securelocation_header_value).unwrap();
    }
    
    #[test]
    #[should_panic]
    fn negative_empty() {
        let securelocation_header_value = &[b""[..].to_vec()];
        
        SecureLocation::parse_header(securelocation_header_value).unwrap();
    }
    
    #[test]
    #[should_panic]
    fn negative_invalid_utf8() {
        let securelocation_header_value = &[b"https://192.168.1.1/\x80"[..].to_vec()];
        
        SecureLocation::parse_header(securelocation_header_value).unwrap();
    }
}