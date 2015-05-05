use std::net::{UdpSocket, SocketAddr};
use std::io::{Read, Result};
use std::thread::{self};
use std::sync::{Arc};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError, Sender};

use super::reader::{self};
use super::message::{SSDPMessage};

// Should Be Enough To Hold All SSDP Packets.
const DEFAULT_MSG_LEN: usize = 600;

/// Provides a non-blocking interface for retrieving SSDPMessages off of some 
/// UdpSocket that is receiving messages from one or more entities.
pub struct SSDPReceiver {
    udp_sock: UdpSocket,
    udp_addr: SocketAddr,
    msg_recv: Receiver<SSDPMessage>,
    kill_flag: Arc<AtomicBool>
}

impl SSDPReceiver {
    /// Spawns a worker thread that listens for SSDPMessages and forwards them
    /// back to the SSDPReceiver object that is returned from this method.
    pub fn spawn(udp: UdpSocket) -> Result<SSDPReceiver> {
        let local_addr = try!(udp.local_addr());
        let kill_flag = Arc::new(AtomicBool::new(false));
        let (msg_send, msg_recv) = mpsc::channel();
        
        let udp_clone = try!(udp.try_clone());
        let kill_clone = kill_flag.clone();
        thread::spawn(move || {
            receive_messages(udp_clone, msg_send, kill_clone);
        });
        
        Ok(SSDPReceiver{ udp_sock: udp, udp_addr: local_addr, msg_recv: msg_recv, kill_flag: kill_flag })
    }
    
    pub fn recv(&mut self) -> SSDPMessage {
        self.msg_recv.recv().unwrap()
    }
}

impl Drop for SSDPReceiver {
    fn drop(&mut self) {
        // TODO: Add Logging For If Kill Flag Is Already Set -> Packet Receiver Failed
    
        // SeqCst, Keep Write To UdpSocket Below Us
        self.kill_flag.store(true, Ordering::SeqCst);
        
        // Don't Care About Return Value, If It Fails Nothing We Can Do...
        self.udp_sock.send_to(&[0], self.udp_addr);
    }
}

// TODO: Add Logging
/// Listens for packets coming off of the given UdpSocket and sends each packet
/// to a worker thread that builds the SSDPMessage which is sent back to the
/// SSDPReceiver.
fn receive_messages(udp: UdpSocket, msg_send: Sender<SSDPMessage>, kill: Arc<AtomicBool>) {
    let (pckt_send, pckt_recv) = mpsc::channel();
    
    // Spawn Message Reader Thread
    thread::spawn(move || {
        reader::read_messages(pckt_recv, msg_send);
    });

    // Receive Packets On The UdpSocket
    loop {
        let mut pckt_buf = vec![0u8; DEFAULT_MSG_LEN];
        
        // Receive A Packet From The UdpSocket
        let mut result = udp.recv_from(&mut pckt_buf[..]);
        while result.is_err() {
            result = udp.recv_from(&mut pckt_buf[..]);
        }
        
        // Check If We Received A Kill Order
        if kill.load(Ordering::SeqCst) {
            break;
        }
        let (pckt_len, pckt_src) = result.unwrap();
        
        // Check The Length Returned By The UdpSocket
        if pckt_len > pckt_buf.len() {
            kill.store(true, Ordering::SeqCst);
            
            panic!("Something Is Wrong With UdpSocket Recv Length")
        }
        unsafe{ pckt_buf.set_len(pckt_len) };
        
        // Send The Packet To The Reader Thread
        pckt_send.send((pckt_buf, pckt_src)).unwrap();
    }
}

