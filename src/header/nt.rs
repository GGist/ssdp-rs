use std::fmt::{Formatter, Display, Result};

use hyper::error::{self, Error};
use hyper::header::{HeaderFormat, Header};

use FieldMap;

const NT_HEADER_NAME: &'static str = "NT";

/// Represents a header used to specify a notification type.
///
/// Any double colons will not be processed as separate `FieldMap`'s.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct NT(pub FieldMap);

impl NT {
    pub fn new(field: FieldMap) -> NT {
        NT(field)
    }
}

impl Header for NT {
    fn header_name() -> &'static str {
        NT_HEADER_NAME
    }

    fn parse_header(raw: &[Vec<u8>]) -> error::Result<Self> {
        if raw.len() != 1 {
            return Err(Error::Header);
        }

        match FieldMap::parse_bytes(&raw[0][..]) {
            Some(n) => Ok(NT(n)),
            None => Err(Error::Header),
        }
    }
}

impl HeaderFormat for NT {
    fn fmt_header(&self, fmt: &mut Formatter) -> Result {
        try!(Display::fmt(&self.0, fmt));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use hyper::header::Header;

    use super::NT;
    use FieldMap::{UPnP, UUID, URN, Unknown};

    #[test]
    fn positive_uuid() {
        let header = "uuid:a984bc8c-aaf0-5dff-b980-00d098bda247";

        let data = match NT::parse_header(&[header.to_string().into_bytes()]) {
            Ok(NT(UUID(n))) => n,
            _ => panic!("uuid Token Not Parsed"),
        };

        assert!(header.chars().skip(5).zip(data.chars()).all(|(a, b)| a == b));
    }

    #[test]
    fn positive_upnp() {
        let header = "upnp:rootdevice";

        let data = match NT::parse_header(&[header.to_string().into_bytes()]) {
            Ok(NT(UPnP(n))) => n,
            _ => panic!("upnp Token Not Parsed"),
        };

        assert!(header.chars().skip(5).zip(data.chars()).all(|(a, b)| a == b));
    }

    #[test]
    fn positive_urn() {
        let header = "urn:schemas-upnp-org:device:printer:1";

        let data = match NT::parse_header(&[header.to_string().into_bytes()]) {
            Ok(NT(URN(n))) => n,
            _ => panic!("urn Token Not Parsed"),
        };

        assert!(header.chars().skip(4).zip(data.chars()).all(|(a, b)| a == b));
    }

    #[test]
    fn positive_unknown() {
        let header = "max-age:1500::upnp:rootdevice";

        let (k, v) = match NT::parse_header(&[header.to_string().into_bytes()]) {
            Ok(NT(Unknown(k, v))) => (k, v),
            _ => panic!("Unknown Token Not Parsed"),
        };

        let sep_iter = ":".chars();
        let mut original_iter = header.chars();
        let mut result_iter = k.chars().chain(sep_iter).chain(v.chars());

        assert!(original_iter.by_ref().zip(result_iter.by_ref()).all(|(a, b)| a == b));
        assert!(result_iter.next().is_none() && original_iter.next().is_none());
    }

    #[test]
    fn positive_short_field() {
        let header = "a:a";

        let (k, v) = match NT::parse_header(&[header.to_string().into_bytes()]) {
            Ok(NT(Unknown(k, v))) => (k, v),
            _ => panic!("Unknown Short Token Not Parsed"),
        };

        let sep_iter = ":".chars();
        let mut original_iter = header.chars();
        let mut result_iter = k.chars().chain(sep_iter).chain(v.chars());

        assert!(original_iter.by_ref().zip(result_iter.by_ref()).all(|(a, b)| a == b));
        assert!(result_iter.next().is_none() && original_iter.next().is_none());
    }

    #[test]
    fn positive_leading_double_colon() {
        let leading_double_colon_header = &["uuid::a984bc8c-aaf0-5dff-b980-00d098bda247"
                                                .to_string()
                                                .into_bytes()];

        let result = match NT::parse_header(leading_double_colon_header).unwrap() {
            NT(UUID(n)) => n,
            _ => panic!("NT Double Colon Failed To Parse"),
        };

        assert_eq!(result, ":a984bc8c-aaf0-5dff-b980-00d098bda247");
    }

    #[test]
    #[should_panic]
    fn negative_double_colon() {
        let double_colon_header = &["::".to_string().into_bytes()];

        NT::parse_header(double_colon_header).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_single_colon() {
        let single_colon_header = &[":".to_string().into_bytes()];

        NT::parse_header(single_colon_header).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_empty_field() {
        let empty_header = &["".to_string().into_bytes()];

        NT::parse_header(empty_header).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_no_colon() {
        let no_colon_header = &["some_key-some_value".to_string().into_bytes()];

        NT::parse_header(no_colon_header).unwrap();
    }
}
