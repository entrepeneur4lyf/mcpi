// mcpi-server/src/message_handler.rs

use crate::traits::MessageHandler;
use crate::plugin_registry::PluginRegistry; // Import PluginRegistry
use mcpi_common::MCPRequest;
use serde_json::{json, Value}; // Value needed for provider_info
use std::future::Future;
use std::ops::Deref;
use std::pin::Pin;
use std::sync::Arc;
use tracing::{error, info};

pub struct McpMessageHandler {
    // Store only the parts needed
    registry: Arc<PluginRegistry>,
    provider_info: Arc<Value>, // Add provider_info state
}

impl McpMessageHandler {
    // Expect Arc<PluginRegistry> and Arc<Value>
    pub fn new(registry: Arc<PluginRegistry>, provider_info: Arc<Value>) -> Self {
        Self { registry, provider_info }
    }

    // Helper function to process a batch of messages
    async fn process_batch(&self, messages: Vec<Value>, client_id: &str) -> Option<String> {
        info!("Processing batch of {} messages from client {}", messages.len(), client_id);
        let mut responses = Vec::new();
        // Clone the necessary state Arc(s) for async usage
        let registry = self.registry.clone(); // Use original variable name
        let provider_info = self.provider_info.clone(); // Clone provider_info

        for message in messages {
            let message_id = message.get("id").cloned().unwrap_or(Value::Null);

            if message_id.is_null() { // Notification
                if let Ok(request) = serde_json::from_value::<MCPRequest>(message.clone()) {
                   match serde_json::to_string(&request) {
                       Ok(request_str) => {
                           // Pass needed state Arcs
                           let _ = crate::process_mcp_message(&request_str, &registry, &provider_info).await; // Pass both
                       },
                       Err(e) => error!("Failed to re-serialize notification: {}", e),
                   }
                } else { error!("Failed to parse potential notification: {:?}", message); }
                continue;
            }

            // Request
            if let Ok(request) = serde_json::from_value::<MCPRequest>(message.clone()) {
                match serde_json::to_string(&request) {
                    Ok(request_str) => {
                        // Pass needed state Arcs
                        if let Some(response_str) = crate::process_mcp_message(&request_str, &registry, &provider_info).await { // Pass both
                            match serde_json::from_str::<Value>(&response_str) {
                                Ok(response_json) => responses.push(response_json),
                                Err(e) => error!("Failed to parse response string: {}", e),
                            }
                        } else {
                            info!("process_mcp_message returned None for request ID: {}", message_id);
                             responses.push(json!({ "jsonrpc": "2.0", "id": message_id, "error": { "code": -32603, "message": "Internal server error" } }));
                        }
                    },
                    Err(e) => {
                        error!("Failed to serialize valid MCPRequest: {}", e);
                         responses.push(json!({ "jsonrpc": "2.0", "id": message_id, "error": { "code": -32603, "message": "Internal server error" } }));
                    }
                }
            } else { // Parse error for request
                responses.push(json!({ "jsonrpc": "2.0", "id": message_id, "error": { "code": -32700, "message": "Parse error: Invalid MCPRequest" } }));
            }
        }

        if responses.is_empty() { return None; }

        match serde_json::to_string(&responses) {
            Ok(batch_response) => Some(batch_response),
            Err(e) => {
                error!("Failed to serialize batch response: {}", e);
                Some(json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32603, "message": "Internal server error" } }).to_string())
            }
        }
    }
}

// Implementation for the struct itself
impl MessageHandler for McpMessageHandler {
    fn handle_message<'a>(&'a self, message: String, client_id: String)
        -> Pin<Box<dyn Future<Output = Option<String>> + Send + 'a>> {

        Box::pin(async move {
            let trimmed_message = message.trim();
            // Clone necessary state Arc(s) for async usage
            let registry = self.registry.clone(); // Use original variable name
            let provider_info = self.provider_info.clone(); // Clone provider_info

            if trimmed_message.starts_with('[') && trimmed_message.ends_with(']') {
                match serde_json::from_str::<Vec<Value>>(&message) {
                    Ok(batch) => self.process_batch(batch, &client_id).await,
                    Err(e) => {
                        error!("Invalid batch request from {}: {}", client_id, e);
                        Some(json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32700, "message": "Parse error: Invalid batch" } }).to_string())
                    }
                }
            } else if trimmed_message.starts_with('{') && trimmed_message.ends_with('}') {
                 info!("Processing single message from client {}", client_id);
                 // Pass needed state Arcs
                 crate::process_mcp_message(&message, &registry, &provider_info).await // Pass both
            } else {
                error!("Invalid message format from {}: {}", client_id, message);
                Some(json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32700, "message": "Parse error: Invalid JSON" } }).to_string())
            }
        })
    }
}

// Implement the MessageHandler trait for Arc<McpMessageHandler>
impl MessageHandler for Arc<McpMessageHandler> {
    fn handle_message<'a>(&'a self, message: String, client_id: String)
        -> Pin<Box<dyn Future<Output = Option<String>> + Send + 'a>> {
        self.deref().handle_message(message, client_id)
    }
}