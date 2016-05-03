use std::io::{self, ErrorKind, Read, Write};
use std::net::{UdpSocket, SocketAddr};
use std::time::Duration;
use hyper::net::NetworkStream;

/// A type that wraps a `UdpSocket` and a `SocketAddr` and implements the `NetworkStream`
/// trait.
///
/// Note that reading from this stream will generate an error, this object is
/// used for intercepting Http messages from Hyper and sending them out via Udp.
/// The response(s) from client(s) are to be handled by some other object that
/// has a cloned handle to our internal `UdpSocket` handle.
pub struct UdpSender {
    udp: UdpSocket,
    dst: SocketAddr,
    buf: Vec<u8>,
}

impl UdpSender {
    /// Creates a new UdpSender object.
    pub fn new(udp: UdpSocket, dst: SocketAddr) -> UdpSender {
        UdpSender {
            udp: udp,
            dst: dst,
            buf: Vec::new(),
        }
    }
}

impl NetworkStream for UdpSender {
    fn peer_addr(&mut self) -> io::Result<SocketAddr> {
        Ok(self.dst)
    }
    fn set_read_timeout(&self, _dur: Option<Duration>) -> io::Result<()> {
        Ok(())
    }
    fn set_write_timeout(&self, _dur: Option<Duration>) -> io::Result<()> {
        Ok(())
    }
}

impl Read for UdpSender {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        // Simulate Some Network Error So Our Process Doesnt Hang
        Err(io::Error::new(ErrorKind::ConnectionAborted, "UdpSender Can Not Be Read From"))
    }
}

impl Write for UdpSender {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Hyper will generate a request with a /, we need to intercept that.
        let mut buffer = vec![0u8; buf.len()];

        let mut found = false;
        for (src, dst) in buf.iter().zip(buffer.iter_mut()) {
            if *src == b'/' && !found && buf[0] != b'H' {
                *dst = b'*';
                found = true;
            } else {
                *dst = *src;
            }
        }

        self.buf.append(&mut buffer);

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        debug!("Sent HTTP Request:\n{}", String::from_utf8_lossy(&self.buf[..]));

        let result = self.udp.send_to(&self.buf[..], self.dst);
        self.buf.clear();

        result.map(|_| ())
    }
}

impl Clone for UdpSender {
    fn clone(&self) -> UdpSender {
        let udp_clone = self.udp.try_clone().unwrap();

        UdpSender {
            udp: udp_clone,
            dst: self.dst,
            buf: self.buf.clone(),
        }
    }

    fn clone_from(&mut self, source: &UdpSender) {
        let udp_clone = source.udp.try_clone().unwrap();

        self.udp = udp_clone;
        self.dst = source.dst;
    }
}
