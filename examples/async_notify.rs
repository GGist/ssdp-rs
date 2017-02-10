extern crate ssdp;

use std::io::{self, Read};
use std::thread;
use std::time::Duration;

use ssdp::FieldMap;
use ssdp::header::{HeaderMut, NT, NTS, USN};
use ssdp::message::{NotifyListener, NotifyMessage, Listen, Multicast};

fn main() {
    thread::spawn(|| {
        for (msg, src) in NotifyListener::listen().unwrap() {
            println!("Received The Following Message From {}:\n{:?}\n", src, msg);
        }
    });

    // Make Sure Thread Has Started
    thread::sleep(Duration::new(1, 0));

    // Create A Test Message
    let mut message = NotifyMessage::new();

    // Set Some Headers
    message.set(NTS::ByeBye);
    message.set(NT(FieldMap::upnp("rootdevice")));
    message.set(USN(FieldMap::uuid("Hello, This Is Not A UUID!!!"), None));

    message.multicast().unwrap();

    // Wait Until User Is Done Listening For Notify Messages
    println!("Press Enter When You Wish To Exit...\n");
    let input = io::stdin();

    input.bytes().next();
}
