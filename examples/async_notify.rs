extern crate ssdp;

use std::io::{self, Read};
use std::thread;

use ssdp::{FieldMap};
use ssdp::header::{HeaderMut, NT, NTS, USN};
use ssdp::message::{NotifyListener, NotifyMessage};

fn main() {
    thread::spawn(|| {
        for message in NotifyListener::listen().unwrap() {
            println!("{:?}\n", message);
        }
    });
    
    // Make Sure Thread Has Started
    thread::sleep_ms(1000);
    
    // Send A Test Message
    let mut message = NotifyMessage::new();
    
    // Set Some Headers
    message.set(NTS::ByeBye);
    message.set(NT(FieldMap::UPnP(b"rootdevice".to_vec())));
    message.set(USN(FieldMap::UUID(b"Not A Real UUID...Hello!!!".to_vec()), None));
    
    message.multicast().unwrap();
    
    // Let Thread Print Initial Message
    thread::sleep_ms(1000);
    
    // Wait Until User Is Done Listening For Notify Messages
    println!("Press Enter When You Wish To Exit...");
    let input = io::stdin();
    
    input.bytes().next();
}