pub trait FromRawSSDP {
    fn raw_ssdp(bytes: &[u8]) -> SSDPResult<Self>;
}

/// A non-blocking SSDP message reciever for any message that implements FromRawSSDP.
struct SSDPReceiver<T> where T: FromRawSSDP {
    recv: Receiver<Vec<u8>>,
    kill: Arc<AtomicBool>
}

impl<T> SSDPReceiver<T> for T where T: FromRawSSDP {
    fn new(
}