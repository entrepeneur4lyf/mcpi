use axum::{
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use axum::extract::ws::{Message, WebSocket};
use mcpi_common::{
    CapabilityDescription, DiscoveryResponse, McpPlugin, 
    MCPRequest, Resource, Tool, MCPI_VERSION
};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use std::fs;
use tracing::{error, info, warn};

mod plugin_registry;
mod plugins;

use plugin_registry::PluginRegistry;
use plugins::{WebsitePlugin, WeatherPlugin};


// Define paths as constants
const CONFIG_FILE_PATH: &str = "data/config.json";
const DATA_PATH: &str = "data/mock";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create config and data directories if they don't exist
    let config_file = Path::new(CONFIG_FILE_PATH);
    let data_dir = Path::new(DATA_PATH);
    
    if !config_file.exists() {
        tracing::warn!("Config file not found. Please create '{}'.", CONFIG_FILE_PATH);
        return Err("No configuration file found. Please create config file to continue.".into());
    }
    
    if !data_dir.exists() {
        tracing::warn!("Data directory not found. Please create '{}'.", DATA_PATH);
        return Err("No data files found. Please add data files to continue.".into());
    }

    // Initialize plugin registry
    let registry = Arc::new(PluginRegistry::new());
    
    // Load configuration to get capability details
    let config_data = fs::read_to_string(config_file)?;
    let config: Value = serde_json::from_str(&config_data)?;
    
    // Extract capabilities from config for future reference
    let capabilities = match config.get("capabilities") {
        Some(caps) => caps.clone(),
        None => {
            tracing::error!("No capabilities found in configuration file");
            return Err("No capabilities defined in configuration".into());
        }
    };
    
    // Load and register website plugin
    let website_plugin = match WebsitePlugin::new(CONFIG_FILE_PATH, DATA_PATH) {
        Ok(plugin) => plugin,
        Err(e) => {
            tracing::error!("Failed to initialize website plugin: {}", e);
            return Err(e);
        }
    };
    
    // Get provider info for later use
    let provider_info = website_plugin.get_provider_info();
    let referrals = website_plugin.get_referrals();
    
    // Register all capabilities from the configuration as individual plugins
    if let Some(caps_obj) = capabilities.as_object() {
        for (cap_name, cap_config) in caps_obj {
            let name = match cap_config.get("name").and_then(|n| n.as_str()) {
                Some(n) => n.to_string(),
                None => cap_name.clone(),
            };
            
            let description = cap_config.get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("No description available")
                .to_string();
            
            let category = cap_config.get("category")
                .and_then(|c| c.as_str())
                .unwrap_or("misc")
                .to_string();
            
            let operations = cap_config.get("operations")
                .and_then(|o| o.as_array())
                .map(|ops_array| {
                    ops_array.iter()
                        .filter_map(|op| op.as_str().map(|s| s.to_string()))
                        .collect::<Vec<String>>()
                })
                .unwrap_or_else(|| vec!["SEARCH".to_string(), "GET".to_string(), "LIST".to_string()]);
            
            let data_file = cap_config.get("data_file")
                .and_then(|f| f.as_str())
                .unwrap_or(&format!("{}.json", name))
                .to_string();
            
            // Validate that the data file exists
            let full_data_path = Path::new(DATA_PATH).join(&data_file);
            if !full_data_path.exists() {
                tracing::error!("Data file not found for capability '{}': {}", 
                    name, full_data_path.display());
                return Err(format!("Data file not found: {}", full_data_path.display()).into());
            }
            
            // Create and register this plugin
            let plugin: Arc<dyn McpPlugin> = Arc::new(
                mcpi_common::JsonDataPlugin::new(
                    &name,
                    &description,
                    &category,
                    operations,
                    &data_file,
                    DATA_PATH,
                )
            );
            
            if let Err(e) = registry.register_plugin(plugin) {
                tracing::error!("Failed to register plugin '{}': {}", name, e);
                return Err(e.into());
            }
            
            tracing::info!("Registered plugin: {}", name);
        }
    }
    
    // Register weather plugin
    let weather_plugin = Arc::new(WeatherPlugin::new());
    if let Err(e) = registry.register_plugin(weather_plugin) {
        tracing::error!("Failed to register weather plugin: {}", e);
        return Err(e.into());
    }
    tracing::info!("Registered plugin: weather_forecast");

    // Create shared state
    let app_state = Arc::new(AppState { 
        registry: registry.clone(),
        provider_info: provider_info.clone(),
        referrals: referrals.clone(),
    });

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
                "resources/read" => Some(handle_read_resource(&request, state)),
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
    // Extract provider name and domain
    let provider_name = state.provider_info.get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("MCPI Provider")
        .to_string();
    
    let provider_domain = state.provider_info.get("domain")
        .and_then(|d| d.as_str())
        .unwrap_or("example.com")
        .to_string();
    
    let provider_description = state.provider_info.get("description")
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
        refs_array.iter()
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
    let capability_descriptions: Vec<CapabilityDescription> = state.registry
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
    let capability_names: Vec<String> = state.registry
        .get_all_plugins()
        .iter()
        .map(|plugin| plugin.name().to_string())
        .collect();
    
    // Extract provider name
    let provider_name = state.provider_info.get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("MCPI Provider")
        .to_string();
    
    // Extract provider description
    let provider_description = state.provider_info.get("description")
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
    let provider_domain = state.provider_info.get("domain")
        .and_then(|d| d.as_str())
        .unwrap_or("example.com")
        .to_string();
    
    // Collect resources from all plugins
    let resources: Vec<Resource> = state.registry
        .get_all_plugins()
        .iter()
        .flat_map(|plugin| {
            plugin.get_resources().into_iter().map(|(name, uri, description)| {
                Resource {
                    name,
                    description,
                    uri: uri.replace("provider", &provider_domain),
                    mime_type: Some("application/json".to_string()),
                    size: None,
                }
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
            tracing::info!("Resource URI requested: {}", uri);

            // Extract filename from URI
            // Format: mcpi://domain/resources/filename.json
            let parts: Vec<&str> = uri.split('/').collect();
            
            if parts.len() >= 4 {
                let filename = parts.last().unwrap();
                
                // Construct the full path to the JSON file
                let data_path = Path::new(DATA_PATH).join(filename);
                
                // Log the file path
                tracing::info!("Looking for file: {}", data_path.display());
                
                // Check if the file exists
                if data_path.exists() {
                    tracing::info!("File exists: {}", data_path.display());
                    
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
                        },
                        Err(e) => {
                            tracing::error!("Error reading file: {}", e);
                            return create_error_response(
                                request.id.clone(),
                                100,
                                format!("Error reading resource file: {}", e),
                            );
                        }
                    }
                } else {
                    tracing::warn!("File does not exist: {}", data_path.display());
                    return create_error_response(
                        request.id.clone(),
                        100,
                        format!("Resource file not found: {}", filename),
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
    let tools: Vec<Tool> = state.registry
        .get_all_plugins()
        .iter()
        .map(|plugin| {
            Tool {
                name: plugin.name().to_string(),
                description: Some(plugin.description().to_string()),
                input_schema: plugin.input_schema(),
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

fn handle_call_tool(request: &MCPRequest, state: &Arc<AppState>) -> String {
    info!("Handling tools/call request");

    if let Some(params) = &request.params {
        if let Some(tool_name) = params.get("name").and_then(|n| n.as_str()) {
            info!("Calling tool: {}", tool_name);

            if let Some(arguments) = params.get("arguments").and_then(|a| a.as_object()) {
                // Extract operation
                let operation = arguments.get("operation")
                    .and_then(|o| o.as_str())
                    .unwrap_or("SEARCH");

                info!("Operation: {}", operation);

                // Execute the plugin operation
                match state.registry.execute_plugin(tool_name, operation, &json!(arguments)) {
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
                    },
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
struct AppState {
    registry: Arc<PluginRegistry>,
    provider_info: Value,
    referrals: Value,
}