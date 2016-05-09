//! Implements the SSDP layer of the `UPnP` standard.
//!
//! This module deals with interface discovery as well as HTTP extensions for
//! accomodating SSDP.

use std::fmt::{Display, Error, Formatter};
use std::result::Result;
use std::borrow::Cow;

/// Separator character for a `FieldMap` and it's value.
pub const PAIR_SEPARATOR: char = ':';

/// Prefix for the "upnp" field key.
const UPNP_PREFIX: &'static str = "upnp";
/// Prefix for the "uuid" field key.
const UUID_PREFIX: &'static str = "uuid";
/// Prefix for the "usn" field key.
const URN_PREFIX: &'static str = "urn";

/// Enumerates key value pairs embedded within SSDP header fields.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum FieldMap {
    /// The "upnp" key with its associated value.
    UPnP(String),
    /// The "uuid" key with its associated value.
    UUID(String),
    /// The "urn" key with its associated value.
    URN(String),
    /// An undefined key, the key and it's value are returned.
    Unknown(String, String),
}

impl FieldMap {
    /// Breaks a field up into a single key and single value which are
    /// separated by a colon and neither of which are empty.
    ///
    /// Separation will occur at the first colon encountered.
    pub fn new<'a, S: Into<Cow<'a, str>>>(value: S) -> Option<Self> {
        FieldMap::parse_bytes(value.into().as_bytes())
    }

    /// Breaks a field up into a single key and single value which are
    /// separated by a colon and neither of which are empty.
    ///
    /// Separation will occur at the first colon encountered.
    pub fn parse_bytes(field: &[u8]) -> Option<Self> {
        let split_index = match field.iter().position(|&b| b == PAIR_SEPARATOR as u8) {
            Some(n) => n,
            None => return None,
        };
        let (key, mut value) = field.split_at(split_index);

        // Ignore Separator Byte
        value = &value[1..];

        // Check Empty Byte Slices
        if key.len() == 0 || value.len() == 0 {
            return None;
        }

        let key = String::from_utf8_lossy(key);
        let value = String::from_utf8_lossy(value).into_owned();

        if matches_uuid_key(key.as_ref()) {
            Some(FieldMap::UUID(value))
        } else if matches_urn_key(key.as_ref()) {
            Some(FieldMap::URN(value))
        } else if matches_upnp_key(key.as_ref()) {
            Some(FieldMap::UPnP(value))
        } else {
            Some(FieldMap::Unknown(key.into_owned(), value))
        }
    }

    pub fn upnp<'a, S: Into<Cow<'a, str>>>(value: S) -> Self {
        FieldMap::UPnP(value.into().into_owned())
    }

    pub fn uuid<'a, S: Into<Cow<'a, str>>>(value: S) -> Self {
        FieldMap::UUID(value.into().into_owned())
    }

    pub fn urn<'a, S: Into<Cow<'a, str>>>(value: S) -> Self {
        FieldMap::URN(value.into().into_owned())
    }

    pub fn unknown<'a, S: Into<Cow<'a, str>>, S2: Into<Cow<'a, str>>>(key: S, value: S2) -> Self {
        FieldMap::Unknown(key.into().into_owned(), value.into().into_owned())
    }
}

impl Display for FieldMap {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let value = match *self {
            FieldMap::UPnP(ref v) => {
                try!(f.write_str(UPNP_PREFIX));
                v
            }
            FieldMap::UUID(ref v) => {
                try!(f.write_str(UUID_PREFIX));
                v
            }
            FieldMap::URN(ref v) => {
                try!(f.write_str(URN_PREFIX));
                v
            }
            FieldMap::Unknown(ref k, ref v) => {
                try!(Display::fmt(k, f));
                v
            }
        };
        try!(f.write_fmt(format_args!("{}", PAIR_SEPARATOR)));
        try!(Display::fmt(value, f));
        Ok(())
    }
}

/// Returns the header field value if the key matches the uuid key, else returns None.
fn matches_uuid_key(key: &str) -> bool {
    UUID_PREFIX == key
}

/// Returns the header field value if the key matches the urn key, else returns None.
fn matches_urn_key(key: &str) -> bool {
    URN_PREFIX == key
}

/// Returns the header field value if the key matches the upnp key, else returns None.
fn matches_upnp_key(key: &str) -> bool {
    UPNP_PREFIX == key
}

#[cfg(test)]
mod tests {
    use super::FieldMap;

    #[test]
    fn positive_non_utf8() {
        let uuid_pair = FieldMap::parse_bytes(&b"uuid:some_value_\x80"[..]).unwrap();
        assert_eq!(uuid_pair, FieldMap::uuid(String::from_utf8_lossy(&b"some_value_\x80".to_vec())));
    }

    #[test]
    fn positive_unknown_non_utf8() {
        let unknown_pair = FieldMap::parse_bytes(&b"some_key\x80:some_value_\x80"[..]).unwrap();
        assert_eq!(unknown_pair,
                   FieldMap::unknown(String::from_utf8_lossy(&b"some_key\x80".to_vec()),
                                     String::from_utf8_lossy(&b"some_value_\x80".to_vec())));
    }

    #[test]
    fn positive_upnp() {
        let upnp_pair = FieldMap::new("upnp:some_value").unwrap();
        assert_eq!(upnp_pair, FieldMap::upnp("some_value"));
    }

    #[test]
    fn positive_uuid() {
        let uuid_pair = FieldMap::new("uuid:some_value").unwrap();
        assert_eq!(uuid_pair, FieldMap::uuid("some_value"));
    }

    #[test]
    fn positive_urn() {
        let urn_pair = FieldMap::new("urn:some_value").unwrap();
        assert_eq!(urn_pair, FieldMap::urn("some_value"));
    }

    #[test]
    fn positive_unknown() {
        let unknown_pair = FieldMap::new("some_key:some_value").unwrap();
        assert_eq!(unknown_pair, FieldMap::unknown("some_key", "some_value"));
    }

    #[test]
    #[should_panic]
    fn negative_no_colon() {
        FieldMap::new("upnpsome_value").unwrap();
    }
}
