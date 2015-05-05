
extern crate url;

use url::{Url};

fn main() {
    let x = Url::parse("udp://192.168.1.1:1900").unwrap();
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