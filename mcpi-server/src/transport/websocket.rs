use crate::transport::traits::{McpTransport, MessageHandler, TransportError};
use axum::{
    extract::WebSocketUpgrade,
    routing::get,
    Router,
};
use futures::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tracing::{error, info};
use uuid::Uuid;

pub struct WebSocketTransport {
    port: u16,
    shutdown_signal: Mutex<Option<broadcast::Sender<()>>>,
}

impl WebSocketTransport {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            shutdown_signal: Mutex::new(None),
        }
    }
}

impl McpTransport for WebSocketTransport {
    fn start(&self, message_handler: Arc<dyn MessageHandler>) -> Result<(), TransportError> {
        let port = self.port;
        let (shutdown_tx, mut shutdown_rx) = broadcast::channel(1);
        
        // Clone for the closure
        let message_handler = message_handler.clone();
        
        // Build application with WebSocket route
        let app = Router::new()
            .route("/mcpi", get(move |ws: WebSocketUpgrade| {
                let handler = message_handler.clone();
                async move {
                    ws.on_upgrade(move |socket| handle_socket(socket, handler))
                }
            }));
            
        // Set up server address
        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        
        info!("Starting WebSocket transport on {}", addr);
        
        // Start the server
        tokio::spawn(async move {
            axum::Server::bind(&addr)
                .serve(app.into_make_service())
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.recv().await;
                })
                .await
                .map_err(|e| error!("WebSocket server error: {}", e))
                .ok();
                
            info!("WebSocket transport shut down");
        });
        
        // Store shutdown sender using proper locking
        if let Ok(mut guard) = self.shutdown_signal.lock() {
            *guard = Some(shutdown_tx);
        } else {
            return Err(TransportError::StartupError("Failed to lock shutdown signal".to_string()));
        }
        
        Ok(())
    }
    
    fn shutdown(&self) -> Result<(), TransportError> {
        if let Ok(guard) = self.shutdown_signal.lock() {
            if let Some(sender) = guard.as_ref() {
                let _ = sender.send(());
                info!("Shutdown signal sent to WebSocket transport");
            }
        } else {
            return Err(TransportError::ShutdownError("Failed to lock shutdown signal".to_string()));
        }
        Ok(())
    }
}

async fn handle_socket(socket: axum::extract::ws::WebSocket, message_handler: Arc<dyn MessageHandler>) {
    let (mut sender, mut receiver) = socket.split();
    
    // Generate client ID
    let client_id = Uuid::new_v4().to_string();
    info!("WebSocket connection established: {}", client_id);
    
    // Process messages
    while let Some(Ok(message)) = receiver.next().await {
        if let axum::extract::ws::Message::Text(text) = message {
            // Use the future returned by handle_message
            if let Some(response) = message_handler.handle_message(text, client_id.clone()).await {
                if let Err(e) = sender.send(axum::extract::ws::Message::Text(response)).await {
                    error!("Error sending message: {}", e);
                    break;
                }
            }
        }
    }
    
    info!("WebSocket connection closed: {}", client_id);
}