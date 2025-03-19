pub fn trace_error(message: impl AsRef<str>, error: &anyhow::Error) {
    tracing::error!("Error({:?}): {:?}, source: {:?}", message.as_ref(), error, error.source());
}