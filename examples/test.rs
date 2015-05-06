extern crate ssdp;
extern crate hyper;

use hyper::{Url};
use hyper::header::{ContentLength};
use ssdp::header::{HeaderMut, Man, MX, ST};
use ssdp::message::search::{SearchRequest};

fn main() {
    let mut request = SearchRequest::new();
    
    request.set(Man);
    request.set(MX(1));
    request.set(ST::All);
    request.set(ContentLength(0));
    
    let response = request.unicast("10.0.1.4:0", "239.255.255.250:1900").unwrap();
    
    for i in response {
        println!("{:?}", i);
    }
}
/*
fn main() {
    // Send A Notify Message
    let notify = NotifyMessage::new();
    notify.multicast();
    notify.unicast("192.168.1.1:1900");
    
    // Listen For Notify Messages
    let notify = NotifyListener::new();
    
    // Send A Search Request, Receive A Response (Could Be A Stream Of Responses)
    let search = SearchRequest::new();
    for i in search.multicast() {
        
    }
    let response_stream = search.multicast();
    let response = search.unicast("192.168.1.1:1900");
}

struct NotifyMessage {
    headers: Headers
}*/