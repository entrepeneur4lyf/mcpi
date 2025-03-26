use std::fmt;
use std::error::Error;
use std::sync::Arc;
use std::future::Future;
use std::pin::Pin;

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

// Making this trait object-safe by using associated type for future
pub trait MessageHandler: Send + Sync {
    // Return a boxed future instead of using async fn
    fn handle_message<'a>(&'a self, message: String, client_id: String) 
        -> Pin<Box<dyn Future<Output = Option<String>> + Send + 'a>>;
}

pub trait McpTransport: Send + Sync {
    fn start(&self, message_handler: Arc<dyn MessageHandler>) -> Result<(), TransportError>;
    fn shutdown(&self) -> Result<(), TransportError>;
}