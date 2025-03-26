pub mod traits;
mod websocket;

pub use traits::{MessageHandler, McpTransport, TransportError};
pub use websocket::WebSocketTransport;

use std::sync::Arc;

pub struct TransportManager {
    transports: Vec<Box<dyn McpTransport>>,
    message_handler: Arc<dyn MessageHandler>,
}

impl TransportManager {
    pub fn new(message_handler: impl MessageHandler + 'static) -> Self {
        Self {
            transports: Vec::new(),
            message_handler: Arc::new(message_handler),
        }
    }
    
    pub fn register_transport(&mut self, transport: Box<dyn McpTransport>) {
        self.transports.push(transport);
    }
    
    pub async fn start_all(&self) -> Result<(), TransportError> {
        for transport in &self.transports {
            transport.start(self.message_handler.clone())?;
        }
        Ok(())
    }
    
    pub async fn shutdown_all(&self) -> Result<(), TransportError> {
        for transport in &self.transports {
            let _ = transport.shutdown();
        }
        Ok(())
    }
}