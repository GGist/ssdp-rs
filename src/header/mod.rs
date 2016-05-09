//! Headers and primitives for parsing headers within SSDP requests.
//!
//! This module combines abstractions at both the HTTPU/HTTPMU layer and SSDP
//! layer in order to provide a cleaner interface for extending the underlying
//! HTTP parsing library.

use std::borrow::Cow;
use std::fmt::Debug;

use hyper::header::{Headers, Header, HeaderFormat};

mod bootid;
mod configid;
mod man;
mod mx;
mod nt;
mod nts;
mod searchport;
mod securelocation;
mod st;
mod usn;

pub use self::bootid::BootID;
pub use self::configid::ConfigID;
pub use self::man::Man;
pub use self::mx::MX;
pub use self::nt::NT;
pub use self::nts::NTS;
pub use self::searchport::SearchPort;
pub use self::securelocation::SecureLocation;
pub use self::st::ST;
pub use self::usn::USN;

// Re-exports
pub use hyper::header::{Location, Server, CacheControl, CacheDirective};

/// Trait for viewing the contents of a header structure.
pub trait HeaderRef: Debug {
    /// View a reference to a header field if it exists.
    fn get<H>(&self) -> Option<&H> where H: Header + HeaderFormat;

    /// View a reference to the raw bytes of a header field if it exists.
    fn get_raw(&self, name: &str) -> Option<&[Vec<u8>]>;
}

impl<'a, T: ?Sized> HeaderRef for &'a T
    where T: HeaderRef
{
    fn get<H>(&self) -> Option<&H>
        where H: Header + HeaderFormat
    {
        HeaderRef::get::<H>(*self)
    }

    fn get_raw(&self, name: &str) -> Option<&[Vec<u8>]> {
        HeaderRef::get_raw(*self, name)
    }
}

impl<'a, T: ?Sized> HeaderRef for &'a mut T
    where T: HeaderRef
{
    fn get<H>(&self) -> Option<&H>
        where H: Header + HeaderFormat
    {
        HeaderRef::get::<H>(*self)
    }

    fn get_raw(&self, name: &str) -> Option<&[Vec<u8>]> {
        HeaderRef::get_raw(*self, name)
    }
}

impl HeaderRef for Headers {
    fn get<H>(&self) -> Option<&H>
        where H: Header + HeaderFormat
    {
        Headers::get::<H>(self)
    }

    fn get_raw(&self, name: &str) -> Option<&[Vec<u8>]> {
        Headers::get_raw(self, name)
    }
}

/// Trait for manipulating the contents of a header structure.
pub trait HeaderMut: Debug {
    /// Set a header to the given value.
    fn set<H>(&mut self, value: H) where H: Header + HeaderFormat;

    /// Set a header to the given raw bytes.
    fn set_raw<K>(&mut self, name: K, value: Vec<Vec<u8>>) where K: Into<Cow<'static, str>> + Debug;
}

impl<'a, T: ?Sized> HeaderMut for &'a mut T
    where T: HeaderMut
{
    fn set<H>(&mut self, value: H)
        where H: Header + HeaderFormat
    {
        HeaderMut::set(*self, value)
    }

    fn set_raw<K>(&mut self, name: K, value: Vec<Vec<u8>>)
        where K: Into<Cow<'static, str>> + Debug
    {
        HeaderMut::set_raw(*self, name, value)
    }
}

impl HeaderMut for Headers {
    fn set<H>(&mut self, value: H)
        where H: Header + HeaderFormat
    {
        Headers::set(self, value)
    }

    fn set_raw<K>(&mut self, name: K, value: Vec<Vec<u8>>)
        where K: Into<Cow<'static, str>> + Debug
    {
        Headers::set_raw(self, name, value)
    }
}

// #[cfg(test)]
// pub mod mock {
// use std::any::{Any};
// use std::borrow::{ToOwned};
// use std::clone::{Clone};
// use std::collections::{HashMap};
//
// use hyper::header::{Header, HeaderFormat};
//
// use ssdp::header::{HeaderView};
//
// #[derive(Debug)]
// pub struct MockHeaderView {
// map: HashMap<&'static str, (Box<Any>, [Vec<u8>; 1])>
// }
//
// impl MockHeaderView {
// pub fn new() -> MockHeaderView {
// MockHeaderView{ map: HashMap::new() }
// }
//
// pub fn insert<H>(&mut self, value: &str) where H: Header + HeaderFormat {
// let header_bytes = [value.to_owned().into_bytes()];
//
// let header = match H::parse_header(&header_bytes[..]) {
// Some(n) => n,
// None    => panic!("Failed To Parse value As Header!!!")
// };
//
// self.map.insert(H::header_name(), (Box::new(header), header_bytes));
// }
// }
//
// impl Clone for MockHeaderView {
// fn clone(&self) -> MockHeaderView {
// panic!("Can Not Clone A MockHeaderView")
// }
// }
//
// impl HeaderView for MockHeaderView {
// fn view<H>(&self) -> Option<&H> where H: Header + HeaderFormat {
// match self.map.get(H::header_name()) {
// Some(&(ref header, _)) => header.downcast_ref::<H>(),
// None => None
// }
// }
//
// fn view_raw(&self, name: &str) -> Option<&[Vec<u8>]> {
// match self.map.get(name) {
// Some(&(_, ref header_bytes)) => Some(header_bytes),
// None => None
// }
// }
// }
// }
