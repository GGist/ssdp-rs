use std::fmt::{Formatter, Result};

use hyper::error::{self, Error};
use hyper::header::{HeaderFormat, Header};

const NTS_HEADER_NAME: &'static str = "NTS";

const ALIVE_HEADER: &'static str = "ssdp:alive";
const UPDATE_HEADER: &'static str = "ssdp:update";
const BYEBYE_HEADER: &'static str = "ssdp:byebye";

/// Represents a header which specifies a notification sub type.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum NTS {
    /// An entity is announcing itself to the network.
    Alive,
    /// An entity is updating its presence on the network. Introduced in UPnP 1.0.
    ///
    /// Contrary to it's name, an update message will only appear when some UPnP
    /// enabled interface is added to an already existing UPnP device on a network.
    Update,
    /// An entity is removing itself from the network.
    ByeBye,
}

impl Header for NTS {
    fn header_name() -> &'static str {
        NTS_HEADER_NAME
    }

    fn parse_header(raw: &[Vec<u8>]) -> error::Result<Self> {
        if raw.len() != 1 {
            return Err(Error::Header);
        }

        if &raw[0][..] == ALIVE_HEADER.as_bytes() {
            Ok(NTS::Alive)
        } else if &raw[0][..] == UPDATE_HEADER.as_bytes() {
            Ok(NTS::Update)
        } else if &raw[0][..] == BYEBYE_HEADER.as_bytes() {
            Ok(NTS::ByeBye)
        } else {
            Err(Error::Header)
        }
    }
}

impl HeaderFormat for NTS {
    fn fmt_header(&self, fmt: &mut Formatter) -> Result {
        match *self {
            NTS::Alive => try!(fmt.write_str(ALIVE_HEADER)),
            NTS::Update => try!(fmt.write_str(UPDATE_HEADER)),
            NTS::ByeBye => try!(fmt.write_str(BYEBYE_HEADER)),
        };

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use hyper::header::Header;

    use super::NTS;

    #[test]
    fn positive_alive() {
        let alive_header = &[b"ssdp:alive"[..].to_vec()];

        match NTS::parse_header(alive_header) {
            Ok(NTS::Alive) => (),
            _ => panic!("Didn't Match With NTS::Alive"),
        };
    }

    #[test]
    fn positive_update() {
        let update_header = &[b"ssdp:update"[..].to_vec()];

        match NTS::parse_header(update_header) {
            Ok(NTS::Update) => (),
            _ => panic!("Didn't Match With NTS::Update"),
        };
    }

    #[test]
    fn positive_byebye() {
        let byebye_header = &[b"ssdp:byebye"[..].to_vec()];

        match NTS::parse_header(byebye_header) {
            Ok(NTS::ByeBye) => (),
            _ => panic!("Didn't Match With NTS::ByeBye"),
        };
    }

    #[test]
    #[should_panic]
    fn negative_alive_extra() {
        let alive_extra_header = &[b"ssdp:alive_someotherbytes"[..].to_vec()];

        NTS::parse_header(alive_extra_header).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_unknown() {
        let unknown_header = &[b"ssdp:somestring"[..].to_vec()];

        NTS::parse_header(unknown_header).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_empty() {
        let empty_header = &[b""[..].to_vec()];

        NTS::parse_header(empty_header).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_no_value() {
        let no_value_header = &[b"ssdp:"[..].to_vec()];

        NTS::parse_header(no_value_header).unwrap();
    }
}
