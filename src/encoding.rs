pub const CSI_FINAL_BYTES: &str = r"@[\]^_`{|}~";

pub struct CsiSequence {
    content: Vec<u8>,
}

impl CsiSequence {
    pub fn new(content: impl Into<Vec<u8>>) -> Self {
        CsiSequence { content: content.into() }
    }
    pub fn content(&self) -> &[u8] {
        &self.content
    }
    pub fn content_as_string(&self) -> String {
        String::from_utf8_lossy(&self.content).to_string()
    }    
}

impl <T: Into<String>> From<T> for CsiSequence {
    fn from(value: T) -> Self {
        CsiSequence { content: value.into().as_bytes().to_vec() }
    }
}


pub struct OscSequence {
    content: Vec<u8>,
}

impl OscSequence {
    pub fn new(content: impl Into<Vec<u8>>) -> Self {
        OscSequence { content: content.into() }
    }
    pub fn content(&self) -> &[u8] {
        &self.content
    }
    pub fn content_as_string(&self) -> String {
        String::from_utf8_lossy(&self.content).to_string()
    }
}

impl <T: Into<String>> From<T> for OscSequence {
    fn from(value: T) -> Self {
        OscSequence { content: value.into().as_bytes().to_vec() }
    }
}
