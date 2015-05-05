//! Implements the SSDP layer of the UPnP standard.
//! 
//! This module deals with interface discovery as well as HTTP extensions for
//! accomodating SSDP.

use std::fmt::{Display, Error, Formatter};
use std::result::{Result};

/// Separator character for a FieldMap and it's value.
pub const PAIR_SEPARATOR: u8 = b':';

/// Prefix for the "upnp" field key.
const UPNP_PREFIX: &'static str = "upnp";
/// Prefix for the "uuid" field key.
const UUID_PREFIX: &'static str = "uuid";
/// Prefix for the "usn" field key.
const URN_PREFIX:  &'static str = "urn";

/// Key value pairs embedded within SSDP header fields.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum FieldMap {
    /// The "upnp" key with its associated value.
    UPnP(Vec<u8>),
    /// The "uuid" key with its associated value.
    UUID(Vec<u8>),
    /// The "urn" key with its associated value.
    URN(Vec<u8>),
    /// An undefined key, the key and it's value are returned.
    Unknown(Vec<u8>, Vec<u8>)
}

impl FieldMap {
    /// Breaks a field up into a single key and single value which are
    /// separated by a colon and neither of which are empty.
    ///
    /// Separation will occur at the first colon encountered.
    pub fn new(field: &[u8]) -> Option<FieldMap> {
        let split_index = match field.position_elem(&PAIR_SEPARATOR) {
            Some(n) => n,
            None    => return None
        };
        let (key, mut value) = field.split_at(split_index);
        
        // Ignore Separator Byte
        value = &value[1..];
        
        // Check Empty Byte Slices
        if key.len() == 0 || value.len() == 0 {
            return None
        }
        
        if matches_uuid_key(key) {
            Some(FieldMap::UUID(value.to_vec()))
        } else if matches_urn_key(key) {
            Some(FieldMap::URN(value.to_vec()))
        } else if matches_upnp_key(key) {
            Some(FieldMap::UPnP(value.to_vec()))
        } else {
            Some(FieldMap::Unknown(key.to_vec(), value.to_vec()))
        }
    }
}

impl Display for FieldMap {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let value = match *self {
            FieldMap::UPnP(ref v) => { try!(f.write_str(UPNP_PREFIX)); v },
            FieldMap::UUID(ref v) => { try!(f.write_str(UUID_PREFIX)); v },
            FieldMap::URN(ref v)  => { try!(f.write_str(URN_PREFIX)); v },
            FieldMap::Unknown(ref k, ref v) => {
                let key = String::from_utf8_lossy(k);
                try!(Display::fmt(&key, f));
                
                v
            }
        };
        try!(f.write_fmt(format_args!("{}", PAIR_SEPARATOR as char)));
        
        let cow_value = String::from_utf8_lossy(value);
        try!(Display::fmt(&cow_value, f));
        
        Ok(())
    }
}

/// Returns the header field value if the key matches the uuid key, else returns None.
fn matches_uuid_key(key: &[u8]) -> bool {
    UUID_PREFIX.as_bytes() == key
}

/// Returns the header field value if the key matches the urn key, else returns None.
fn matches_urn_key(key: &[u8]) -> bool {
    URN_PREFIX.as_bytes() == key
}

/// Returns the header field value if the key matches the upnp key, else returns None.
fn matches_upnp_key(key: &[u8]) -> bool {
    UPNP_PREFIX.as_bytes() == key
}

#[cfg(test)]
mod tests {
    use super::{FieldMap};
    
    #[test]
    fn positive_upnp() {
        let upnp_pair = FieldMap::new(&b"upnp:some_value_\x80"[..]).unwrap();
        
        assert_eq!(upnp_pair, FieldMap::UPnP(b"some_value_\x80".to_vec()));
    }
    
    #[test]
    fn positive_uuid() {
        let uuid_pair = FieldMap::new(&b"uuid:some_value_\x80"[..]).unwrap();
        
        assert_eq!(uuid_pair, FieldMap::UUID(b"some_value_\x80".to_vec()));
    }
    
    #[test]
    fn positive_urn() {
        let urn_pair = FieldMap::new(&b"urn:some_value_\x80"[..]).unwrap();
        
        assert_eq!(urn_pair, FieldMap::URN(b"some_value_\x80".to_vec()));
    }
    
    #[test]
    fn positive_unknown() {
        let unknown_pair = FieldMap::new(&b"some_key\x80:some_value_\x80"[..]).unwrap();
        
        assert_eq!(unknown_pair, FieldMap::Unknown(b"some_key\x80".to_vec(), b"some_value_\x80".to_vec()));
    }
    
    #[test]
    #[should_panic]
    fn negative_no_colon() {
        FieldMap::new(&b"upnpsome_value_\x80"[..]).unwrap();
    }
}