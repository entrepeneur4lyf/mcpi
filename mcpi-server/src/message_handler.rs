use crate::transport::MessageHandler;
use std::sync::Arc;
use std::future::Future;
use std::pin::Pin;
use tracing::info;

pub struct McpMessageHandler {
    state: Arc<crate::AppState>,
}

impl McpMessageHandler {
    pub fn new(state: Arc<crate::AppState>) -> Self {
        Self { state }
    }
}

impl MessageHandler for McpMessageHandler {
    fn handle_message<'a>(&'a self, message: String, client_id: String)
        -> Pin<Box<dyn Future<Output = Option<String>> + Send + 'a>> {
        info!("Processing message from client {}", client_id);
        
        let state = self.state.clone();
        Box::pin(async move {
            crate::process_mcp_message(&message, &state).await
        })
    }
}