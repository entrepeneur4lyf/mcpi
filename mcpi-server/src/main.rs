use axum::{
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use axum::extract::ws::{Message, WebSocket};
use mcpi_common::{
    CapabilityConfig, CapabilityDescription, Config, DiscoveryResponse, 
    MCPRequest, Resource, Tool, MCPI_VERSION
};
use serde_json::{json, Value};
use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create data directory if it doesn't exist
    let data_dir = Path::new("data");
    if !data_dir.exists() {
        fs::create_dir_all(data_dir)?;
        tracing::warn!("Data directory created. Please place your config.json and data files in the 'data' directory.");
        return Err("No configuration files found. Please add data files to continue.".into());
    }

    // Load configuration
    let config = load_config()?;

    // Create shared state
    let app_state = Arc::new(AppState { config });

    // Build our CORS layer
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build our application with routes
    let app = Router::new()
        .route("/mcpi", get(ws_handler))
        .route("/mcpi/discover", get(discovery_handler))
        .with_state(app_state)
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    // Run our application
    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
    tracing::info!("listening on {}", addr);
    
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
    
    Ok(())
}

// Handle WebSocket connections for MCP protocol
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

// Handle WebSocket connection
async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    
    // Process messages from the client
    while let Some(Ok(message)) = receiver.next().await {
        if let Message::Text(text) = message {
            let response = process_mcp_message(&text, &state).await;
            
            if let Some(response_text) = response {
                // Send the response back to the client
                if let Err(e) = sender.send(Message::Text(response_text)).await {
                    tracing::error!("Error sending message: {}", e);
                    break;
                }
            }
        }
    }
    
    tracing::info!("WebSocket connection closed");
}

// Process an MCP message
async fn process_mcp_message(message: &str, state: &Arc<AppState>) -> Option<String> {
    // Parse the message
    let request: Result<MCPRequest, _> = serde_json::from_str(message);
    
    match request {
        Ok(request) => {
            match request.method.as_str() {
                "initialize" => Some(handle_initialize(&request, state)),
                "resources/list" => Some(handle_list_resources(&request, state)),
                "resources/read" => Some(handle_read_resource(&request)),
                "tools/list" => Some(handle_list_tools(&request, state)),
                "tools/call" => Some(handle_call_tool(&request, state)),
                "ping" => Some(handle_ping(&request)),
                _ => Some(create_error_response(
                    request.id,
                    -32601,
                    format!("Method not found: {}", request.method),
                ))
            }
        },
        Err(e) => Some(create_error_response(
            Value::Null,
            -32700,
            format!("Parse error: {}", e),
        )),
    }
}

// REST discovery endpoint
async fn discovery_handler(
    State(state): State<Arc<AppState>>,
) -> Json<DiscoveryResponse> {
    // Build capability descriptions from config
    let capability_descriptions: Vec<CapabilityDescription> = state
        .config
        .capabilities
        .values()
        .map(|cap| CapabilityDescription {
            name: cap.name.clone(),
            description: cap.description.clone(),
            category: cap.category.clone(),
            operations: cap.operations.clone(),
        })
        .collect();

    // Create response from provider config
    let response = DiscoveryResponse {
        provider: state.config.provider.clone(),
        mode: "active".to_string(),
        capabilities: capability_descriptions,
        referrals: state.config.referrals.clone(),
    };

    Json(response)
}

// Handle MCP initialize request
fn handle_initialize(request: &MCPRequest, state: &Arc<AppState>) -> String {
    // Collect keys and convert to strings for joining
    let capability_keys: Vec<String> = state.config.capabilities.keys()
        .map(|k| k.to_string())
        .collect();
    
    let response = json!({
        "jsonrpc": "2.0",
        "id": request.id,
        "result": {
            "serverInfo": {
                "name": state.config.provider.name.clone(),
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
            "instructions": format!("This is an MCPI server for {}. You can access capabilities like: {}.",
                state.config.provider.description,
                capability_keys.join(", ")
            )
        }
    });
    
    response.to_string()
}

// Handle MCP resources/list request
fn handle_list_resources(request: &MCPRequest, state: &Arc<AppState>) -> String {
    // Convert capabilities to resources
    let resources: Vec<Resource> = state.config.capabilities.values()
        .map(|cap| Resource {
            name: cap.name.clone(),
            description: Some(cap.description.clone()),
            uri: format!("mcpi://{}/resources/{}", state.config.provider.domain, cap.data_file),
            mime_type: Some("application/json".to_string()),
            size: None,
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
fn handle_read_resource(request: &MCPRequest) -> String {
    if let Some(params) = &request.params {
        if let Some(uri) = params.get("uri").and_then(|u| u.as_str()) {
            // Extract filename from URI
            // Format: mcpi://domain/resources/filename.json
            let parts: Vec<&str> = uri.split('/').collect();
            if parts.len() >= 4 {
                let filename = parts.last().unwrap();
                
                // Load resource data
                let data_path = Path::new("data").join(filename);
                if data_path.exists() {
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
                        },
                        Err(e) => {
                            return create_error_response(
                                request.id.clone(),
                                100,
                                format!("Error reading resource: {}", e),
                            );
                        }
                    }
                } else {
                    return create_error_response(
                        request.id.clone(),
                        100,
                        format!("Resource not found: {}", uri),
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
    // Convert capabilities to tools
    let tools: Vec<Tool> = state.config.capabilities.values()
        .map(|cap| {
            // Create input schema based on operations
            let input_schema = json!({
                "type": "object",
                "properties": {
                    "operation": {
                        "type": "string",
                        "enum": cap.operations,
                        "description": "Operation to perform"
                    },
                    "query": {
                        "type": "string",
                        "description": "Query string for SEARCH operation"
                    },
                    "id": {
                        "type": "string",
                        "description": "ID for GET operation"
                    }
                },
                "required": ["operation"]
            });
            
            Tool {
                name: cap.name.clone(),
                description: Some(cap.description.clone()),
                input_schema,
            }
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
    if let Some(params) = &request.params {
        if let Some(tool_name) = params.get("name").and_then(|n| n.as_str()) {
            if let Some(arguments) = params.get("arguments").and_then(|a| a.as_object()) {
                if let Some(capability) = state.config.capabilities.get(tool_name) {
                    // Extract operation and parameters
                    let operation = arguments.get("operation").and_then(|o| o.as_str())
                        .unwrap_or("SEARCH");
                    
                    if !capability.operations.contains(&operation.to_string()) {
                        return create_error_response(
                            request.id.clone(),
                            100,
                            format!("Operation '{}' not supported for tool '{}'", operation, tool_name),
                        );
                    }
                    
                    // Execute capability
                    match execute_capability(&capability, operation, &json!(arguments)) {
                        Ok(result) => {
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
                        },
                        Err(e) => {
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
                    return create_error_response(
                        request.id.clone(),
                        100,
                        format!("Tool not found: {}", tool_name),
                    );
                }
            }
        }
    }
    
    create_error_response(
        request.id.clone(),
        -32602,
        "Invalid params for tools/call".to_string(),
    )
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

// Generic capability execution
fn execute_capability(
    capability: &CapabilityConfig,
    operation: &str,
    params: &Value,
) -> Result<Value, Box<dyn std::error::Error>> {
    // Load data from the configured data file
    let data_path = Path::new("data").join(&capability.data_file);
    let data = fs::read_to_string(data_path)?;
    let data: Value = serde_json::from_str(&data)?;
    
    // Process based on capability and operation
    match operation {
        "SEARCH" => {
            let query = params.get("query").and_then(|q| q.as_str()).unwrap_or("");
            let field = params.get("field").and_then(|f| f.as_str()).unwrap_or("name");
            
            // Create a longer-lived empty Vec for the unwrap_or case
            let empty_vec = Vec::new();
            let items = data.as_array().unwrap_or(&empty_vec);
            
            // Filter items based on query
            let filtered_items: Vec<Value> = items
                .iter()
                .filter(|item| {
                    let field_value = item.get(field).and_then(|f| f.as_str()).unwrap_or("");
                    query.is_empty() || field_value.to_lowercase().contains(&query.to_lowercase())
                })
                .cloned()
                .collect();
            
            Ok(json!({
                "results": filtered_items,
                "count": filtered_items.len(),
                "query": query,
                "field": field
            }))
        },
        "GET" => {
            let id = params.get("id").and_then(|i| i.as_str()).unwrap_or("");
            
            // Create a longer-lived empty Vec for the unwrap_or case
            let empty_vec = Vec::new();
            let items = data.as_array().unwrap_or(&empty_vec);
            
            // Find item by ID
            let item = items
                .iter()
                .find(|i| i.get("id").and_then(|id_val| id_val.as_str()) == Some(id))
                .cloned();
            
            match item {
                Some(i) => Ok(i),
                None => Ok(json!({
                    "error": "Item not found",
                    "id": id
                }))
            }
        },
        "LIST" => {
            Ok(json!({
                "results": data,
                "count": data.as_array().map(|a| a.len()).unwrap_or(0)
            }))
        },
        _ => Err(format!("Unsupported operation '{}'", operation).into())
    }
}

// Helper function to load configuration
fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    let config_path = Path::new("data/config.json");
    
    if !config_path.exists() {
        return Err("Config file not found. Please create data/config.json".into());
    }
    
    let config_data = fs::read_to_string(config_path)?;
    let config: Config = serde_json::from_str(&config_data)?;
    
    // Validate that all referenced data files exist
    for (capability_name, capability) in &config.capabilities {
        let data_path = Path::new("data").join(&capability.data_file);
        println!("Checking capability: {}, Data file path: {:?}", capability_name, data_path);
        if !data_path.exists() {
            return Err(format!(
                "Data file '{}' for capability '{}' not found. Please create data/{}", 
                capability.data_file, capability_name, capability.data_file
            ).into());
        }
    }
    
    Ok(config)
}

// Shared application state
struct AppState {
    config: Config,
}