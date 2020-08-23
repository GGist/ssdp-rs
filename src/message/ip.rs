//! This module contains a copy of the implementation from the nightly feature
//! "ip" required to use the functionality of `IpAddr::is_global`.
//! These APIs should be removed and their new equivalents replaced when the ip
//! feature is stabilized. See this issue for tracking: 
//!    https://github.com/rust-lang/rust/issues/27709

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

pub fn is_global(addr: &IpAddr) -> bool {
    match addr {
        IpAddr::V4(addr) => {
            // check if this address is 192.0.0.9 or 192.0.0.10. These addresses are the only two
            // globally routable addresses in the 192.0.0.0/24 range.
            if u32::from(addr.clone()) == 0xc0000009 || u32::from(addr.clone()) == 0xc000000a {
                return true;
            }
            !addr.is_private()
            && !addr.is_loopback()
            && !addr.is_link_local()
            && !addr.is_broadcast()
            && !addr.is_documentation()
            && !is_shared(addr)
            && !is_ietf_protocol_assignment(addr)
            && !is_reserved(addr)
            && !is_benchmarking(addr)
            // Make sure the address is not in 0.0.0.0/8
            && addr.octets()[0] != 0
        } 
        IpAddr::V6(addr) => {
            match multicast_scope(&addr) {
                Some(Ipv6MulticastScope::Global) => true,
                None => is_unicast_global(&addr),
                _ => false,
            }
        },
    }
}

fn is_shared(addr: &Ipv4Addr) -> bool {
    addr.octets()[0] == 100 && (addr.octets()[1] & 0b1100_0000 == 0b0100_0000)
}

fn is_ietf_protocol_assignment(addr: &Ipv4Addr) -> bool {
    addr.octets()[0] == 192 && addr.octets()[1] == 0 && addr.octets()[2] == 0
}

fn is_reserved(addr: &Ipv4Addr) -> bool {
    addr.octets()[0] & 240 == 240 && !addr.is_broadcast()
}

fn is_benchmarking(addr: &Ipv4Addr) -> bool {
    addr.octets()[0] == 198 && (addr.octets()[1] & 0xfe) == 18
}

enum Ipv6MulticastScope {
    InterfaceLocal,
    LinkLocal,
    RealmLocal,
    AdminLocal,
    SiteLocal,
    OrganizationLocal,
    Global,
}

fn multicast_scope(addr: &Ipv6Addr) -> Option<Ipv6MulticastScope> {
    if addr.is_multicast() {
        match addr.segments()[0] & 0x000f {
            1 => Some(Ipv6MulticastScope::InterfaceLocal),
            2 => Some(Ipv6MulticastScope::LinkLocal),
            3 => Some(Ipv6MulticastScope::RealmLocal),
            4 => Some(Ipv6MulticastScope::AdminLocal),
            5 => Some(Ipv6MulticastScope::SiteLocal),
            8 => Some(Ipv6MulticastScope::OrganizationLocal),
            14 => Some(Ipv6MulticastScope::Global),
            _ => None,
        }
    } else {
        None
    }
}

fn is_unicast_global(addr: &Ipv6Addr) -> bool {
    !addr.is_multicast()
        && !addr.is_loopback()
        && !is_unicast_link_local(addr)
        && !is_unique_local(addr)
        && !addr.is_unspecified()
        && !is_documentation(addr)
}

fn is_unicast_link_local(addr: &Ipv6Addr) -> bool {
    (addr.segments()[0] & 0xffc0) == 0xfe80
}

fn is_unique_local(addr: &Ipv6Addr) -> bool {
    (addr.segments()[0] & 0xfe00) == 0xfc00
}

fn is_documentation(addr: &Ipv6Addr) -> bool {
    (addr.segments()[0] == 0x2001) && (addr.segments()[1] == 0xdb8)
}