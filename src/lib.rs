#![feature(core, collections, into_cow, ip_addr, libc, lookup_host, udp)]

extern crate hyper;
extern crate libc;
extern crate time;

mod error;
mod field;
mod net;
mod receiver;

pub mod header;
pub mod message;

pub use error::{SSDPError, SSDPResult};
pub use field::{FieldMap};

