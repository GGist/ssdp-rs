#![feature(core, collections, ip_addr, into_cow, libc, udp)]

extern crate hyper;
extern crate libc;
extern crate time;
extern crate url;

mod error;
mod field;
mod net;
mod receiver;

pub mod header;
pub mod message;

pub use error::{SSDPError, SSDPResult, MsgError};
pub use field::{FieldMap};