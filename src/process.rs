


pub struct ProcessData {
    pub stdin: Box<dyn tokio::io::AsyncWrite + Unpin + Send + Sync>,
    pub stdout: Box<dyn tokio::io::AsyncRead + Unpin + Send + Sync>,
    pub dyn_data: Box<dyn ProcessDataDyn>,
}

pub trait ProcessDataDyn {
    fn release(&mut self) -> Result<(), Box<dyn std::error::Error>>;
}
