// mcpi-server/src/main.rs
use axum::extract::ws::{Message, WebSocket};
use axum::{
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use mcpi_common::{
    CapabilityDescription, DiscoveryResponse, MCPRequest, Resource, Tool, MCPI_VERSION,
};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::path::Path;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Instant;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use std::fs;
use tracing::{error, info, warn};

mod admin;
mod plugin_registry;
mod plugins;

use plugin_registry::PluginRegistry;

// Define paths as constants
const CONFIG_FILE_PATH: &str = "data/server/config.json";
const DATA_PATH: &str = "data";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Validate paths
    validate_paths()?;

    // Load configuration
    let config = load_config()?;

    // Extract provider info and referrals
    let provider_info = config.get("provider").cloned().unwrap_or_else(|| json!({}));
    let referrals = config
        .get("referrals")
        .cloned()
        .unwrap_or_else(|| json!([]));

    // Initialize plugin registry
    let registry = Arc::new(PluginRegistry::new());

    // Register all plugins
    registry.register_all_plugins(DATA_PATH, referrals.clone())?;
    info!("Registered {} plugins", registry.get_all_plugins().len());

    // Create app state
    let app_state = Arc::new(AppState {
        registry: registry.clone(),
        provider_info: provider_info.clone(),
        referrals: referrals.clone(),
        active_connections: AtomicUsize::new(0),
        request_count: AtomicUsize::new(0),
        startup_time: Instant::now(),
    });

    // Set up server
    let app = create_app(app_state);

    // Run the server
    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));
    info!("MCPI server listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

// Validate that necessary paths exist
fn validate_paths() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config_file = Path::new(CONFIG_FILE_PATH);
    let data_dir = Path::new(DATA_PATH);

    if !config_file.exists() {
        warn!("Config file not found: {}", CONFIG_FILE_PATH);
        return Err("No configuration file found. Please create config file to continue.".into());
    }

    if !data_dir.exists() {
        warn!("Data directory not found: {}", DATA_PATH);
        return Err("No data directory found. Please create data directory to continue.".into());
    }

    Ok(())
}

// Load configuration from file
fn load_config() -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let config_data = fs::read_to_string(CONFIG_FILE_PATH)?;
    let config: Value = serde_json::from_str(&config_data)?;
    Ok(config)
}

// Create the app with routes
fn create_app(state: Arc<AppState>) -> Router {
    // Build CORS layer
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build application with routes
    Router::new()
        .route("/mcpi", get(ws_handler))
        .route("/mcpi/discover", get(discovery_handler))
        // Add admin routes
        .route("/admin", get(admin::serve_admin_html))
        .route("/api/admin/stats", get(admin::get_stats))
        .route("/api/admin/plugins", get(admin::get_plugins))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}

// Handle WebSocket connections for MCP protocol
async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

// Handle WebSocket connection
async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    // Increment connection counter
    state.active_connections.fetch_add(1, Ordering::SeqCst);

    // Process messages from the client
    while let Some(Ok(message)) = receiver.next().await {
        if let Message::Text(text) = message {
            // Increment request counter
            state.request_count.fetch_add(1, Ordering::SeqCst);

            let response = process_mcp_message(&text, &state).await;

            if let Some(response_text) = response {
                // Send the response back to the client
                if let Err(e) = sender.send(Message::Text(response_text)).await {
                    error!("Error sending message: {}", e);
                    break;
                }
            }
        }
    }

    // Decrement connection counter on disconnect
    state.active_connections.fetch_sub(1, Ordering::SeqCst);

    info!("WebSocket connection closed");
}

// Process an MCP message
async fn process_mcp_message(message: &str, state: &Arc<AppState>) -> Option<String> {
    // Parse the message
    let request: Result<MCPRequest, _> = serde_json::from_str(message);

    match request {
        Ok(request) => match request.method.as_str() {
            "initialize" => Some(handle_initialize(&request, state)),
            "resources/list" => Some(handle_list_resources(&request, state)),
            "resources/read" => Some(handle_read_resource(&request, state)),
            "tools/list" => Some(handle_list_tools(&request, state)),
            "tools/call" => Some(handle_call_tool(&request, state)),
            "ping" => Some(handle_ping(&request)),
            _ => Some(create_error_response(
                request.id,
                -32601,
                format!("Method not found: {}", request.method),
            )),
        },
        Err(e) => Some(create_error_response(
            Value::Null,
            -32700,
            format!("Parse error: {}", e),
        )),
    }
}

// REST discovery endpoint
async fn discovery_handler(State(state): State<Arc<AppState>>) -> Json<DiscoveryResponse> {
    // Increment request counter
    state.request_count.fetch_add(1, Ordering::SeqCst);

    // Extract provider name and domain
    let provider_name = state
        .provider_info
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("MCPI Provider")
        .to_string();

    let provider_domain = state
        .provider_info
        .get("domain")
        .and_then(|d| d.as_str())
        .unwrap_or("example.com")
        .to_string();

    let provider_description = state
        .provider_info
        .get("description")
        .and_then(|d| d.as_str())
        .unwrap_or("MCPI Provider")
        .to_string();

    // Build provider from extracted info
    let provider = mcpi_common::Provider {
        name: provider_name,
        domain: provider_domain,
        description: provider_description,
        branding: None,
    };

    // Extract referrals from state
    let referrals = if let Some(refs_array) = state.referrals.as_array() {
        refs_array
            .iter()
            .filter_map(|r| {
                let name = r.get("name").and_then(|n| n.as_str())?;
                let domain = r.get("domain").and_then(|d| d.as_str())?;
                let relationship = r.get("relationship").and_then(|rel| rel.as_str())?;

                Some(mcpi_common::Referral {
                    name: name.to_string(),
                    domain: domain.to_string(),
                    relationship: relationship.to_string(),
                })
            })
            .collect()
    } else {
        Vec::new()
    };

    // Build capability descriptions from plugins
    let capability_descriptions: Vec<CapabilityDescription> = state
        .registry
        .get_all_plugins()
        .iter()
        .map(|plugin| CapabilityDescription {
            name: plugin.name().to_string(),
            description: plugin.description().to_string(),
            category: plugin.category().to_string(),
            operations: plugin.supported_operations(),
        })
        .collect();

    // Create response
    let response = DiscoveryResponse {
        provider,
        mode: "active".to_string(),
        capabilities: capability_descriptions,
        referrals,
    };

    Json(response)
}

// Handle MCP initialize request
fn handle_initialize(request: &MCPRequest, state: &Arc<AppState>) -> String {
    // Collect plugin names
    let capability_names: Vec<String> = state
        .registry
        .get_all_plugins()
        .iter()
        .map(|plugin| plugin.name().to_string())
        .collect();

    // Extract provider name
    let provider_name = state
        .provider_info
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("MCPI Provider")
        .to_string();

    // Extract provider description
    let provider_description = state
        .provider_info
        .get("description")
        .and_then(|d| d.as_str())
        .unwrap_or("MCPI Provider")
        .to_string();

    let response = json!({
        "jsonrpc": "2.0",
        "id": request.id,
        "result": {
            "serverInfo": {
                "name": provider_name,
                "version": MCPI_VERSION
            },
            "protocolVersion": "0.1.0",
            "capabilities": {
                "resources": {
                    "listChanged": true,
                    "subscribe": true
                },
                "tools": {
                    "listChanged": true
                }
            },
            "instructions": format!("This is an MCPI server for {}. You can access plugins like: {}.",
                provider_description,
                capability_names.join(", ")
            )
        }
    });

    response.to_string()
}

// Handle MCP resources/list request
fn handle_list_resources(request: &MCPRequest, state: &Arc<AppState>) -> String {
    // Extract provider domain
    let provider_domain = state
        .provider_info
        .get("domain")
        .and_then(|d| d.as_str())
        .unwrap_or("example.com")
        .to_string();

    // Collect resources from all plugins
    let resources: Vec<Resource> = state
        .registry
        .get_all_plugins()
        .iter()
        .flat_map(|plugin| {
            plugin
                .get_resources()
                .into_iter()
                .map(|(name, uri, description)| Resource {
                    name,
                    description,
                    uri: uri.replace("provider", &provider_domain),
                    mime_type: Some("application/json".to_string()),
                    size: None,
                })
        })
        .collect();

    let response = json!({
        "jsonrpc": "2.0",
        "id": request.id,
        "result": {
            "resources": resources
        }
    });

    response.to_string()
}

// Handle MCP resources/read request
fn handle_read_resource(request: &MCPRequest, _state: &Arc<AppState>) -> String {
    if let Some(params) = &request.params {
        if let Some(uri) = params.get("uri").and_then(|u| u.as_str()) {
            info!("Resource URI requested: {}", uri);

            // Parse the URI to extract the resource path
            // Format: mcpi://domain/resources/plugin-type/resource-name/data.json
            // Example: mcpi://example.com/resources/store/products/data.json

            let parts: Vec<&str> = uri.split('/').collect();
            if parts.len() >= 6 {
                // Extract the resource path starting from the DATA_PATH
                // Construct a path like: data/store/products/data.json
                let resource_path = format!(
                    "{}/{}/{}",
                    DATA_PATH,
                    parts[parts.len() - 3], // plugin type (e.g., store)
                    parts[parts.len() - 2]  // resource name (e.g., products)
                );

                let filename = parts.last().unwrap();
                let full_path = format!("{}/{}", resource_path, filename);

                let data_path = Path::new(&full_path);

                info!("Looking for file: {}", data_path.display());

                // Check if the file exists
                if data_path.exists() {
                    info!("File exists: {}", data_path.display());

                    // Read the JSON file
                    match fs::read_to_string(data_path) {
                        Ok(content) => {
                            let response = json!({
                                "jsonrpc": "2.0",
                                "id": request.id,
                                "result": {
                                    "contents": [
                                        {
                                            "uri": uri,
                                            "text": content,
                                            "mimeType": "application/json"
                                        }
                                    ]
                                }
                            });

                            return response.to_string();
                        }
                        Err(e) => {
                            error!("Error reading file: {}", e);
                            return create_error_response(
                                request.id.clone(),
                                100,
                                format!("Error reading resource file: {}", e),
                            );
                        }
                    }
                } else {
                    warn!("File does not exist: {}", data_path.display());
                    return create_error_response(
                        request.id.clone(),
                        100,
                        format!("Resource file not found: {}", full_path),
                    );
                }
            }
        }
    }

    create_error_response(
        request.id.clone(),
        -32602,
        "Invalid params for resources/read".to_string(),
    )
}

// Handle MCP tools/list request
fn handle_list_tools(request: &MCPRequest, state: &Arc<AppState>) -> String {
    // Convert plugins to tools
    let tools: Vec<Tool> = state
        .registry
        .get_all_plugins()
        .iter()
        .map(|plugin| Tool {
            name: plugin.name().to_string(),
            description: Some(plugin.description().to_string()),
            input_schema: plugin.input_schema(),
        })
        .collect();

    let response = json!({
        "jsonrpc": "2.0",
        "id": request.id,
        "result": {
            "tools": tools
        }
    });

    response.to_string()
}

// Handle MCP tools/call request
fn handle_call_tool(request: &MCPRequest, state: &Arc<AppState>) -> String {
    info!("Handling tools/call request");

    if let Some(params) = &request.params {
        if let Some(tool_name) = params.get("name").and_then(|n| n.as_str()) {
            info!("Calling tool: {}", tool_name);

            if let Some(arguments) = params.get("arguments").and_then(|a| a.as_object()) {
                // Extract operation
                let operation = arguments
                    .get("operation")
                    .and_then(|o| o.as_str())
                    .unwrap_or("SEARCH");

                info!("Operation: {}", operation);

                // Execute the plugin operation
                match state
                    .registry
                    .execute_plugin(tool_name, operation, &json!(arguments))
                {
                    Ok(result) => {
                        info!("Tool execution successful");
                        let response = json!({
                            "jsonrpc": "2.0",
                            "id": request.id,
                            "result": {
                                "content": [
                                    {
                                        "type": "text",
                                        "text": serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
                                    }
                                ]
                            }
                        });

                        return response.to_string();
                    }
                    Err(e) => {
                        error!("Tool execution error: {}", e);
                        let response = json!({
                            "jsonrpc": "2.0",
                            "id": request.id,
                            "result": {
                                "content": [
                                    {
                                        "type": "text",
                                        "text": format!("Error: {}", e)
                                    }
                                ],
                                "isError": true
                            }
                        });

                        return response.to_string();
                    }
                }
            } else {
                warn!("Missing 'arguments' in tools/call request");
                return create_error_response(
                    request.id.clone(),
                    -32602,
                    "Invalid params for tools/call".to_string(),
                );
            }
        } else {
            warn!("Missing 'name' in tools/call request");
            return create_error_response(
                request.id.clone(),
                -32602,
                "Invalid params for tools/call".to_string(),
            );
        }
    } else {
        warn!("Missing 'params' in tools/call request");
        return create_error_response(
            request.id.clone(),
            -32602,
            "Invalid params for tools/call".to_string(),
        );
    }
}

// Handle MCP ping request
fn handle_ping(request: &MCPRequest) -> String {
    let response = json!({
        "jsonrpc": "2.0",
        "id": request.id,
        "result": {}
    });

    response.to_string()
}

// Create an error response
fn create_error_response(id: Value, code: i32, message: String) -> String {
    let response = json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    });

    response.to_string()
}

// Shared application state
pub struct AppState {
    registry: Arc<PluginRegistry>,
    provider_info: Value,
    referrals: Value,
    active_connections: AtomicUsize,
    request_count: AtomicUsize,
    startup_time: Instant,
}
