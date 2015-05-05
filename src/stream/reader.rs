use std::sync::mpsc::{Receiver, Sender};
use std::io::{BufReader, Read, Write, ErrorKind, Error, Result};
use std::net::{SocketAddr};

use hyper::net::{NetworkStream};
use hyper::server::request::{Request};

use super::message::{SSDPMessage};

/// Receives packets sent from various entities and uses them to build 
pub fn read_requests(recv: Receiver<(Vec<u8>, SocketAddr)>, send: Sender<SSDPMessage>) {
    loop {
        // Let Thread Panic If Sender Hung Up
        let (data, remote) = recv.recv().unwrap();
        
        let mut reader = SSDPReader::new(data, remote);
        let mut buf_reader = BufReader::new(&mut reader as &mut NetworkStream);
        
        // TODO: Add Logging For Failed Requests
        let request = Request::new(&mut buf_reader, remote).unwrap();
        
        // Let Thread Panic If Receiver Hung Up
        send.send(SSDPMessage::new(request)).unwrap();
    }
}

pub fn read_responses(recv: Receiver<(Vec<u8>, SocketAddr)>, send: Sender<SSDPMessage>) {
    loop {
        // Let Thread Panic If Sender Hung Up
        let (data, remote) = recv.recv().unwrap();
        
        let mut reader = SSDPReader::new(data, remote);
        let mut buf_reader = BufReader::new(&mut reader as &mut NetworkStream);
        
        // TODO: Add Logging For Failed Requests
        let request = Request::new(&mut buf_reader, remote).unwrap();
        
        // Let Thread Panic If Receiver Hung Up
        send.send(SSDPMessage::new(request)).unwrap();
    }
}
/*
/// Provides an interface for Hyper to read SSDP messages from a data buffer.
#[derive(Clone)]
struct SSDPReader {
    data_buffer: Vec<u8>,
    remote_addr: SocketAddr,
    read_pos: usize
}

impl SSDPReader {
    /// Creates a new SSDPReader.
    fn new(data: Vec<u8>, addr: SocketAddr) -> SSDPReader {
        SSDPReader{ 
            data_buffer: data, 
            remote_addr: addr, 
            read_pos: 0 
        }
    }
}

impl Read for SSDPReader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        // Check If All Data Has Been Read, Simulate Terminated Connection
        if self.read_pos > self.data_buffer.len() {
            return Err(Error::new(ErrorKind::ConnectionAborted, 
                                  "All Bytes Read From SSDPReader", None)) 
        }
        
        // Write As Many Bytes As Possible
        let mut bytes_written = 0;
        for (src, dst) in self.data_buffer.iter().skip(self.read_pos).zip(buf.iter_mut()) {
            *dst = *src;
            bytes_written += 1;
        }
        
        // Record Bytes Written
        self.read_pos += bytes_written;
        
        Ok(bytes_written)
    }
}

impl Write for SSDPReader {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        Ok(0)
    }
    
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl NetworkStream for SSDPReader {
    fn peer_addr(&mut self) -> Result<SocketAddr> {
        Ok(self.remote_addr)
    }
}*/