


pub struct ProcessData {
    pub stdin: Box<dyn tokio::io::AsyncWrite + Unpin>,
    pub stdout: Box<dyn tokio::io::AsyncRead + Unpin>,
    pub dyn_data: Box<dyn ProcessDataDyn>,
}

pub trait ProcessDataDyn {
    fn release(&mut self) -> Result<(), Box<dyn std::error::Error>>;
}
