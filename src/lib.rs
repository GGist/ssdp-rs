#![allow(unused_features)]
#![feature(ip)]
#![recursion_limit = "1024"]

//! An asynchronous abstraction for discovering devices and services on a network.
//!
//! SSDP stands for Simple Service Discovery Protocol and it is a protocol that uses
//! HTTPMU to distribute messages across a local network for devices and services to
//! discover each other. SSDP can most commonly be found in devices that implement
//! `UPnP` as it is used as the discovery mechanism for that standard.

extern crate hyper;
#[macro_use]
extern crate log;
extern crate time;
extern crate get_if_addrs;
extern crate net2;
#[macro_use]
extern crate error_chain;

mod error;
mod field;
mod net;
mod receiver;

pub mod header;
pub mod message;

pub use error::{SSDPError, SSDPErrorKind, SSDPResultExt, SSDPResult};
pub use field::FieldMap;
pub use receiver::{SSDPReceiver, SSDPIter};
pub use net::IpVersionMode;
