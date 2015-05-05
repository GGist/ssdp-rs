struct SearchRequest {
    message: SSDPMessage
}

impl SearchRequest {
    pub fn new() -> SearchRequest {
        SearchRequest{ message: SSDPMessage::new(MessageType::Search }
    }
    
    
}

impl HeaderRef for SearchRequest {
    fn get<H>(&self) -> Option<&H> where H: Header + HeaderFormat {
        self.message.get::<H>(&self.headers)
    }
    
    fn get_raw(&self, name: &str) -> Option<&[Vec<u8>]> {
        self.message.get_raw(&self.headers, name)
    }
}

impl HeaderMut for SearchRequest {
    fn set<H>(&mut self, value: H) where H: Header + HeaderFormat {
        self.message.set(&mut self.headers, value)
    }
    
    fn set_raw<K>(&mut self, name: K, value: Vec<Vec<u8>>) where K: Into<Cow<'static, str>> {
        self.message.set_raw(&mut self.headers, name, value)
    }
}