extern crate log;
extern crate ssdp;

use log::{Log, LogRecord, LogLevelFilter, LogMetadata};

use ssdp::header::{HeaderMut, Man, MX, ST};
use ssdp::message::{SearchRequest, Multicast};

struct SimpleLogger;

impl Log for SimpleLogger {
    fn enabled(&self, _: &LogMetadata) -> bool {
        true
    }

    fn log(&self, record: &LogRecord) {
        if self.enabled(record.metadata()) {
            println!("{} - {}", record.level(), record.args());
        }
    }
}

fn main() {
    log::set_logger(|max_level| {
        max_level.set(LogLevelFilter::Debug);
        Box::new(SimpleLogger)
    })
        .unwrap();

    // Create Our Search Request
    let mut request = SearchRequest::new();

    // Set Our Desired Headers (Not Verified By The Library)
    request.set(Man);
    request.set(MX(5));
    request.set(ST::All);

    // Collect Our Responses
    request.multicast().unwrap().into_iter().collect::<Vec<_>>();
}
