use crate::transport::traits::{McpTransport, MessageHandler, TransportError};
use axum::{
    // Removed unused: State, WebSocketUpgrade
    routing::get, // Removed unused: post (delete uses its own async block)
    Router, // Removed unused: Json
    response::IntoResponse, // Removed unused: Response
    http::{StatusCode, HeaderMap}, // Removed unused: Method, HeaderValue
};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
// Removed unused: warn
use tracing::{error, info};
use uuid::Uuid;
use std::collections::HashMap;
use tokio::sync::RwLock;
use std::net::SocketAddr;

// Structure to store session information
struct SessionInfo {
    last_event_id: Option<String>,
    #[allow(dead_code)] // Added allow(dead_code) as is_active seems unused currently
    is_active: bool,
    #[allow(dead_code)] // Added allow(dead_code) as created_at seems unused currently
    created_at: std::time::Instant,
}

pub struct StreamableHttpTransport {
    port: u16,
    shutdown_signal: Mutex<Option<broadcast::Sender<()>>>,
    sessions: Arc<RwLock<HashMap<String, SessionInfo>>>,
}

impl StreamableHttpTransport {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            shutdown_signal: Mutex::new(None),
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Handle HTTP POST requests (client to server)
    async fn handle_post(
        message_handler: Arc<dyn MessageHandler>,
        sessions: Arc<RwLock<HashMap<String, SessionInfo>>>,
        body: String,
        headers: HeaderMap, // Type already specified here
    ) -> impl IntoResponse {
        // Extract session ID from headers
        let session_id = headers.get("mcp-session-id")
            .and_then(|v| v.to_str().ok()) // Use and_then and ok() for safer parsing
            .map(|s| s.to_string());

        if let Some(ref session_id_str) = session_id {
            // Check if session exists
            let sessions_read = sessions.read().await;
            if !sessions_read.contains_key(session_id_str) {
                // Drop not needed, scope ends anyway
                return (StatusCode::NOT_FOUND, "Session not found").into_response();
            }
        } else {
             // Decide how to handle posts without a session ID - maybe require it?
             // For now, proceeding but logging a warning might be good.
             info!("POST request received without mcp-session-id header.");
             // Or return BadRequest:
             // return (StatusCode::BAD_REQUEST, "mcp-session-id header required for POST").into_response();
        }

        // Generate a client ID for message handling (use session or new UUID)
        let client_id = session_id.unwrap_or_else(|| Uuid::new_v4().to_string());

        // Process the message with the handler
        // Note: check_if_needs_response logic removed as handle_message should always be called
        // The handler itself decides if a response string is generated.
        if let Some(response) = message_handler.handle_message(body, client_id).await {
            (StatusCode::OK, [(axum::http::header::CONTENT_TYPE, "application/json")], response).into_response()
        } else {
             // Handler returned None, meaning no direct response is needed for this message/batch
             // According to JSON-RPC spec for notifications, no content is returned.
             // For batches containing only notifications, an empty response might be expected by some clients,
             // but often 204 No Content is acceptable. Let's use 204.
            (StatusCode::NO_CONTENT, "").into_response()
        }
    }

    // Handle HTTP GET requests (server to client, opens SSE stream)
    async fn handle_get(
        _message_handler: Arc<dyn MessageHandler>, // Prefixed with _ as it's unused
        sessions: Arc<RwLock<HashMap<String, SessionInfo>>>,
        headers: HeaderMap, // Type already specified here
    ) -> impl IntoResponse {
        // Extract session ID and Last-Event-ID from headers
        let session_id = headers.get("mcp-session-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let last_event_id = headers.get("last-event-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        if let Some(ref session_id_str) = session_id {
            // Check if session exists and update last_event_id
            let mut sessions_write = sessions.write().await;
            if let Some(session) = sessions_write.get_mut(session_id_str) {
                 // Update session with last event ID if provided
                if let Some(id) = last_event_id {
                     info!("Updating last_event_id for session {}: {}", session_id_str, id);
                     session.last_event_id = Some(id);
                }
                 // Session exists, proceed to setup SSE
            } else {
                 // Session not found
                 // Drop not needed
                 return (StatusCode::NOT_FOUND, "Session not found").into_response();
            }
        } else {
             // Require session ID for GET requests establishing SSE stream
             return (StatusCode::BAD_REQUEST, "mcp-session-id header required for GET").into_response();
        }

        // Placeholder SSE setup
        // TODO: Implement actual Server-Sent Events stream logic here
        // This would involve holding the connection open and sending events
        info!("SSE stream requested for session: {}", session_id.unwrap_or_default());
        (StatusCode::OK, [
            (axum::http::header::CONTENT_TYPE, "text/event-stream"),
            (axum::http::header::CACHE_CONTROL, "no-cache"),
            (axum::http::header::CONNECTION, "keep-alive"),
            // Add CORS headers if needed, e.g.:
            // (axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"),
        ], "data: Connected\n\n").into_response()
    }

    // Method create_session is unused - remove or add #[allow(dead_code)]
    #[allow(dead_code)]
    async fn create_session(&self, session_id: &str) -> Result<(), TransportError> {
        let mut sessions = self.sessions.write().await;
        if sessions.contains_key(session_id) {
            Err(TransportError::ConnectionError(format!("Session {} already exists", session_id)))
        } else {
            info!("Creating new session: {}", session_id);
            sessions.insert(session_id.to_string(), SessionInfo {
                last_event_id: None,
                is_active: true,
                created_at: std::time::Instant::now(),
            });
            Ok(())
        }
    }

    // Method delete_session is unused - remove or add #[allow(dead_code)]
    #[allow(dead_code)]
    async fn delete_session(&self, session_id: &str) -> Result<(), TransportError> {
        let mut sessions = self.sessions.write().await;
        if sessions.remove(session_id).is_some() {
            info!("Deleted session: {}", session_id);
            Ok(())
        } else {
            Err(TransportError::ConnectionError(format!("Session {} not found", session_id)))
        }
    }
}

impl McpTransport for StreamableHttpTransport {
    fn start(&self, message_handler: Arc<dyn MessageHandler>) -> Result<(), TransportError> {
        let port = self.port;
        let (shutdown_tx, mut shutdown_rx) = broadcast::channel(1);

        // Clone Arc for the closures
        let message_handler_for_routes = message_handler.clone();
        let sessions_for_routes = self.sessions.clone();

        // Build application with HTTP routes
        let app = Router::new()
            .route("/mcpi",
                get({
                    let handler = message_handler_for_routes.clone();
                    let sessions = sessions_for_routes.clone();
                    // *** FIX: Added type annotation ***
                    move |headers: HeaderMap| Self::handle_get(handler.clone(), sessions.clone(), headers)
                })
                .post({
                    let handler = message_handler_for_routes.clone();
                    let sessions = sessions_for_routes.clone();
                    // *** FIX: Added type annotation ***
                    move |headers: HeaderMap, body: String| Self::handle_post(handler.clone(), sessions.clone(), body, headers)
                })
                .delete({
                    let sessions = sessions_for_routes.clone();
                     // *** FIX: Added type annotation ***
                    move |headers: HeaderMap| async move {
                        let session_id = headers.get("mcp-session-id")
                            .and_then(|v| v.to_str().ok())
                            .map(|s| s.to_string());

                        if let Some(id) = session_id {
                            let mut sessions_write = sessions.write().await;
                            if sessions_write.remove(&id).is_some() {
                                info!("Session {} terminated via DELETE request", id);
                                (StatusCode::OK, "Session terminated").into_response()
                            } else {
                                info!("DELETE request for non-existent session {}", id);
                                (StatusCode::NOT_FOUND, "Session not found").into_response()
                            }
                        } else {
                            (StatusCode::BAD_REQUEST, "mcp-session-id header required for DELETE").into_response()
                        }
                    }
                })
            );

        // Set up server address
        let addr = SocketAddr::from(([0, 0, 0, 0], port));

        info!("Starting Streamable HTTP transport on {}", addr);

        // Start the server in a background task
        let server_handle = tokio::spawn(async move {
            axum::Server::bind(&addr)
                .serve(app.into_make_service())
                .with_graceful_shutdown(async move {
                    shutdown_rx.recv().await.ok(); // Wait for shutdown signal
                    info!("Graceful shutdown initiated for Streamable HTTP transport");
                })
                .await
                .map_err(|e| error!("Streamable HTTP server error: {}", e))
                .ok(); // Log error but don't panic

            info!("Streamable HTTP transport shut down completely");
        });

        // Store shutdown sender
        match self.shutdown_signal.lock() {
             Ok(mut guard) => {
                *guard = Some(shutdown_tx);
             }
             Err(poisoned) => {
                 error!("Shutdown signal mutex poisoned: {}", poisoned);
                 // Attempt recovery or return error
                 // For simplicity, returning error here
                 // Could potentially try to recover the lock: *poisoned.into_inner() = Some(shutdown_tx);
                 server_handle.abort(); // Stop the server task if we can't store the shutdown handle
                 return Err(TransportError::StartupError("Failed to acquire lock for shutdown signal".to_string()));
             }
         }

        Ok(())
    }

    fn shutdown(&self) -> Result<(), TransportError> {
         match self.shutdown_signal.lock() {
             Ok(guard) => {
                 if let Some(sender) = guard.as_ref() {
                     match sender.send(()) {
                         Ok(_) => info!("Shutdown signal sent to Streamable HTTP transport"),
                         Err(_) => info!("Streamable HTTP transport already shut down or receiver dropped"), // Not necessarily an error
                     }
                 } else {
                     info!("Streamable HTTP transport was not running or already shut down");
                 }
             },
             Err(poisoned) => {
                 error!("Shutdown signal mutex poisoned during shutdown: {}", poisoned);
                 return Err(TransportError::ShutdownError("Failed to acquire lock for shutdown signal".to_string()));
             }
         }
         Ok(())
    }
}

// Removed unused helper function check_if_needs_response