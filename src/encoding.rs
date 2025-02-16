pub const CSI_FINAL_BYTES: &str = r"@[\]^_`{|}~";

pub struct CsiSequence {
    content: Vec<u8>,
}

impl CsiSequence {
    pub fn new(content: Vec<u8>) -> Self {
        CsiSequence { content }
    }
    pub fn content(&self) -> &[u8] {
        &self.content
    }
    pub fn content_as_string(&self) -> String {
        String::from_utf8_lossy(&self.content).to_string()
    }    
}


pub struct OscSequence {
    content: Vec<u8>,
}

impl OscSequence {
    pub fn new(content: Vec<u8>) -> Self {
        OscSequence { content }
    }
    pub fn content(&self) -> &[u8] {
        &self.content
    }
    pub fn content_as_string(&self) -> String {
        String::from_utf8_lossy(&self.content).to_string()
    }
}
