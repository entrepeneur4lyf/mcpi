// mcpi-server/src/traits.rs
use std::future::Future;
use std::pin::Pin;

// MessageHandler trait definition
// Place this where both main.rs and message_handler.rs can access it.
pub trait MessageHandler: Send + Sync {
    fn handle_message<'a>(&'a self, message: String, client_id: String)
        -> Pin<Box<dyn Future<Output = Option<String>> + Send + 'a>>;
}

// NOTE: McpTransport and TransportError are likely no longer needed
// if using the single-server model directly in main.rs.
// Keep them if you have other uses, otherwise they can be removed.
/*
use std::fmt;
use std::error::Error;
use std::sync::Arc;

#[derive(Debug)]
pub enum TransportError {
    StartupError(String),
    ShutdownError(String),
    ConnectionError(String),
}
impl fmt::Display for TransportError { ... }
impl Error for TransportError {}

pub trait McpTransport: Send + Sync {
    fn start(&self, message_handler: Arc<dyn MessageHandler>) -> Result<(), TransportError>;
    fn shutdown(&self) -> Result<(), TransportError>;
}
*/