#![feature(core, collections, into_cow, ip_addr, lookup_host, udp)]

//! An asyncronous abstraction for discovering devices and services on a network.
//!
//! SSDP stands for Simple Service Discovery Protocol and it is a protocol that uses
//! HTTPMU to distribute messages across a local network for devices and services to
//! discover each other. SSDP can most commonly be found in devices that implement
//! UPnP as it is used as the discovery mechanism for that standard.
//!
//! ## Search Example
//! ```
//! extern crate ssdp;
//! 
//! use ssdp::header::{HeaderMut, Man, MX, ST};
//! use ssdp::message::{SearchRequest};
//! 
//! fn main() {
//!     // Create Our Search Request
//!     let mut request = SearchRequest::new();
//!     
//!     // Set Our Desired Headers (Not Verified By The Library)
//!     request.set(Man); request.set(MX(5)); request.set(ST::All);
//!     
//!     // Iterate Over Streaming Responses
//!     for response in request.multicast().unwrap() {
//!         println!("{:?}\n\n", response);
//!     }
//! }
//! ```

extern crate hyper;
extern crate libc;
extern crate log;
extern crate time;

mod error;
mod field;
mod net;
mod receiver;

pub mod header;
pub mod message;

pub use error::{SSDPError, SSDPResult};
pub use field::{FieldMap};

