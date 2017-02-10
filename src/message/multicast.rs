use std::net::{SocketAddr, SocketAddrV6};
use std::str::FromStr;

use error::SSDPResult;
use net::connector::UdpConnector;
use message::{self, Config};
use message::ssdp::SSDPMessage;


pub trait Multicast {
    type Item;

    fn multicast(&mut self) -> SSDPResult<Self::Item> {
        self.multicast_with_config(&Default::default())
    }

    fn multicast_with_config(&self, config: &Config) -> SSDPResult<Self::Item>;
}

pub fn send(message: &SSDPMessage, config: &Config) -> SSDPResult<Vec<UdpConnector>> {
    let mut connectors = try!(message::all_local_connectors(Some(config.ttl), &config.mode));

    for conn in &mut connectors {
        match try!(conn.local_addr()) {
            SocketAddr::V4(n) => {
                let mcast_addr = (config.ipv4_addr.as_str(), config.port);
                debug!("Sending ipv4 multicast through {} to {:?}", n, mcast_addr);
                try!(message.send(conn, &mcast_addr));
            }
            SocketAddr::V6(n) => {
                debug!("Sending Ipv6 multicast through {} to {}:{}", n, config.ipv6_addr, config.port);
                //try!(message.send(conn, &mcast_addr));
                try!(message.send(conn,
                                  &SocketAddrV6::new(try!(FromStr::from_str(config.ipv6_addr.as_str())),
                                                     config.port,
                                                     n.flowinfo(),
                                                     n.scope_id())))
            }
        }
    }

    Ok(connectors)
}
