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
// You can remove the commented-out code below if they are not used elsewhere.
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
impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransportError::StartupError(msg) => write!(f, "Transport startup error: {}", msg),
            TransportError::ShutdownError(msg) => write!(f, "Transport shutdown error: {}", msg),
            TransportError::ConnectionError(msg) => write!(f, "Transport connection error: {}", msg),
        }
    }
}
impl Error for TransportError {}

pub trait McpTransport: Send + Sync {
    fn start(&self, message_handler: Arc<dyn MessageHandler>) -> Result<(), TransportError>;
    fn shutdown(&self) -> Result<(), TransportError>;
}
*/