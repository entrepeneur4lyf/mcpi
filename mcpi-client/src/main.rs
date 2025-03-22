// mcpi-client/src/main.rs
use futures::{SinkExt, StreamExt};
use mcpi_common::{
    DiscoveryResponse, InitializeResult, MCPRequest, MCPResponse, ReadResourceResult, ToolCallResult
};
use serde_json::{json, Value};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use std::error::Error;
use clap::{Parser, Subcommand};
mod discovery;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Domain to discover MCPI services from (uses DNS TXT records)
    #[arg(short, long)]
    domain: Option<String>,
    
    /// Direct URL to MCPI server (bypasses DNS discovery)
    #[arg(short, long)]
    url: Option<String>,
    
    /// Test a specific plugin only
    #[arg(short, long)]
    plugin: Option<String>,
    
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Discover MCPI services for a domain
    Discover { domain: String },
    
    /// Connect to an MCPI server and test it
    Connect { url: String },
    
    /// Test a specific plugin
    Test { plugin: String },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    
    // Determine discovery and WebSocket URLs
    let (discovery_url, ws_url) = if let Some(domain) = cli.domain {
        println!("Performing DNS-based discovery for domain: {}", domain);
        
        match discovery::discover_mcp_services(&domain).await {
            Ok(service_info) => {
                println!("Discovered MCP service:");
                println!("  Version: {}", service_info.version);
                println!("  Endpoint: {}", service_info.endpoint);
                
                let discovery = service_info.endpoint;
                
                // Derive WebSocket URL from discovery URL
                let websocket = if discovery.starts_with("https://") {
                    discovery.replace("https://", "wss://").replace("/mcpi/discover", "/mcpi")
                } else {
                    discovery.replace("http://", "ws://").replace("/mcpi/discover", "/mcpi")
                };
                
                (discovery, websocket)
            },
            Err(e) => {
                println!("DNS discovery failed: {}.", e);
                return Err(format!("DNS discovery failed for domain '{}': {}", domain, e).into());
            }
        }
    } else if let Some(url) = cli.url {
        // If user provides a direct WebSocket URL
        let ws_url = url.clone();
        
        // Try to construct a discovery URL from the WebSocket URL
        let discovery_url = if url.starts_with("wss://") {
            url.replace("wss://", "https://").replace("/mcpi", "/mcpi/discover")
        } else if url.starts_with("ws://") {
            url.replace("ws://", "http://").replace("/mcpi", "/mcpi/discover")
        } else {
            println!("Invalid URL format. URL must start with ws:// or wss://");
            return Err("Invalid URL format".into());
        };
        
        println!("Using provided server URL: {}", ws_url);
        println!("Derived discovery URL: {}", discovery_url);
        
        (discovery_url, ws_url)
    } else {
        // Default to localhost if no domain or URL provided
        let discovery = String::from("http://localhost:3001/mcpi/discover");
        let websocket = String::from("ws://localhost:3001/mcpi");
        
        println!("No domain or URL provided. Using default localhost URLs:");
        println!("Discovery: {}", discovery);
        println!("WebSocket: {}", websocket);
        
        (discovery, websocket)
    };
    
    // First, use the HTTP discovery endpoint to verify the service
    println!("Discovering MCPI service capabilities via HTTP...");
    let discovery_resp = discover_service_http(&discovery_url).await?;
    
    println!("Connected to: {} ({})", discovery_resp.provider.name, discovery_resp.provider.domain);
    println!("Mode: {}", discovery_resp.mode);
    
    println!("\nAvailable capabilities:");
    for cap in &discovery_resp.capabilities {
        println!("  - {} ({}): {}", cap.name, cap.category, cap.description);
        println!("    Operations: {}", cap.operations.join(", "));
    }
    
    println!("\nReferrals:");
    for ref_info in &discovery_resp.referrals {
        println!("  - {} ({}): {}", ref_info.name, ref_info.domain, ref_info.relationship);
    }
    
    // Now connect via WebSocket for MCP protocol
    println!("\nConnecting to MCPI service via WebSocket (MCP protocol)...");
    println!("Connecting to: {}", ws_url);
    let (ws_stream, _) = connect_async(&ws_url).await?;
    println!("WebSocket connection established");
    
    // Split the WebSocket stream
    let (mut write, mut read) = ws_stream.split();
    
    // Initialize the connection
    let init_request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        id: json!(1),
        method: "initialize".to_string(),
        params: Some(json!({
            "clientInfo": {
                "name": "MCPI Test Client",
                "version": "0.1.0"
            },
            "protocolVersion": "0.1.0",
            "capabilities": {
                "sampling": {}
            }
        })),
    };
    
    // Send and handle errors in a simpler way
    write.send(Message::Text(serde_json::to_string(&init_request)?)).await
        .map_err(|_| "WebSocket send error")?;
    
    if let Some(Ok(Message::Text(response))) = read.next().await {
        let parsed: MCPResponse = serde_json::from_str(&response)?;
        if let Some(error) = parsed.error {
            println!("Initialization error: {} (code: {})", error.message, error.code);
            return Err("Failed to initialize MCP connection".into());
        }
        
        if let Some(result) = parsed.result {
            let init_result: InitializeResult = serde_json::from_value(result)?;
            println!("\nMCP connection initialized:");
            println!("  Server: {} v{}", init_result.server_info.name, init_result.server_info.version);
            println!("  Protocol: v{}", init_result.protocol_version);
            if let Some(instructions) = init_result.instructions {
                println!("  Instructions: {}", instructions);
            }
        }
    }
    
    // List resources to show what's available
    let list_resources_request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        id: json!(2),
        method: "resources/list".to_string(),
        params: Some(json!({})),
    };
    
    write.send(Message::Text(serde_json::to_string(&list_resources_request)?)).await
        .map_err(|_| "WebSocket send error")?;
    
    if let Some(Ok(Message::Text(response))) = read.next().await {
        let parsed: MCPResponse = serde_json::from_str(&response)?;
        if let Some(result) = parsed.result {
            println!("\nAvailable MCP resources:");
            if let Some(resources) = result.get("resources").and_then(|r| r.as_array()) {
                for resource in resources {
                    println!("  - {} ({})", 
                        resource.get("name").and_then(|n| n.as_str()).unwrap_or("Unnamed"),
                        resource.get("uri").and_then(|u| u.as_str()).unwrap_or("")
                    );
                    if let Some(description) = resource.get("description").and_then(|d| d.as_str()) {
                        println!("    Description: {}", description);
                    }
                }
            }
        }
    }
    
    // List tools
    let list_tools_request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        id: json!(3),
        method: "tools/list".to_string(),
        params: Some(json!({})),
    };
    
    write.send(Message::Text(serde_json::to_string(&list_tools_request)?)).await
        .map_err(|_| "WebSocket send error")?;
    
    let mut tools = Vec::new();
    let mut tools_info = Vec::new();
    
    if let Some(Ok(Message::Text(response))) = read.next().await {
        let parsed: MCPResponse = serde_json::from_str(&response)?;
        if let Some(result) = parsed.result {
            println!("\nAvailable MCP tools:");
            if let Some(tools_array) = result.get("tools").and_then(|t| t.as_array()) {
                for tool in tools_array {
                    let tool_name = tool.get("name").and_then(|n| n.as_str()).unwrap_or("Unnamed").to_string();
                    tools.push(tool_name.clone());
                    tools_info.push(tool.clone());
                    
                    println!("  - {}", tool_name);
                    if let Some(description) = tool.get("description").and_then(|d| d.as_str()) {
                        println!("    Description: {}", description);
                    }
                    
                    if let Some(schema) = tool.get("inputSchema") {
                        println!("    Input Schema: Operations supported:");
                        if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
                            if let Some(op) = props.get("operation").and_then(|o| o.as_object()) {
                                if let Some(ops) = op.get("enum").and_then(|e| e.as_array()) {
                                    let ops_str: Vec<String> = ops.iter()
                                        .filter_map(|o| o.as_str().map(|s| s.to_string()))
                                        .collect();
                                    println!("      {}", ops_str.join(", "));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Check if user wants to test a specific plugin
    let specific_plugin = cli.plugin;
    
    // If a specific plugin is requested, only test that one
    let tools_to_test = if let Some(plugin) = specific_plugin {
        if tools.contains(&plugin) {
            vec![plugin]
        } else {
            println!("\nRequested plugin '{}' not found. Available plugins: {}", 
                plugin, tools.join(", "));
            return Err("Plugin not found".into());
        }
    } else {
        tools
    };

    // Test each tool with appropriate operations
    for tool_name in tools_to_test {
        println!("\n=== Testing tool: {} ===", tool_name);
        
        // Get this tool's input schema to determine supported operations
        let tool_schema = tools_info.iter()
            .find(|t| t.get("name").and_then(|n| n.as_str()) == Some(&tool_name))
            .and_then(|t| t.get("inputSchema").cloned())
            .unwrap_or_else(|| json!({}));
        
        // Extract supported operations from schema
        let operations = tool_schema
            .get("properties")
            .and_then(|p| p.get("operation"))
            .and_then(|o| o.get("enum"))
            .and_then(|e| e.as_array())
            .map(|ops| {
                ops.iter()
                    .filter_map(|op| op.as_str().map(String::from))
                    .collect::<Vec<String>>()
            })
            .unwrap_or_else(|| vec!["SEARCH".to_string()]);
        
        println!("Supported operations: {:?}", operations);
        
        // Test each supported operation with dynamically generated test arguments
        for operation in operations {
            // Analyze the operation and schema to generate appropriate test arguments
            let arguments = generate_test_arguments(&tool_name, &operation, &tool_schema);
            
            // Call the tool with the generated arguments
            call_tool(&mut write, &mut read, &tool_name, &operation, arguments).await?;
        }
    }
    
    // End the session with a ping
    let ping_request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        id: json!(99),
        method: "ping".to_string(),
        params: None,
    };
    
    write.send(Message::Text(serde_json::to_string(&ping_request)?)).await
        .map_err(|_| "WebSocket send error")?;
    
    if let Some(Ok(Message::Text(response))) = read.next().await {
        let parsed: MCPResponse = serde_json::from_str(&response)?;
        if parsed.result.is_some() {
            println!("\nPing successful, connection healthy");
        } else if let Some(error) = parsed.error {
            println!("\nPing error: {} (code: {})", error.message, error.code);
        }
    }
    
    println!("\nClosing MCP connection");
    
    Ok(())
}

// Generate test arguments dynamically based on the operation and schema
fn generate_test_arguments(tool_name: &str, operation: &str, schema: &Value) -> Value {
    // Create a base arguments object with the operation
    let mut arguments = json!({
        "operation": operation
    });
    
    // Get the properties object from the schema
    if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
        // For each property in the schema (except 'operation' which we already handled)
        for (prop_name, prop_schema) in properties.iter().filter(|(k, _)| *k != "operation") {
            // Get the property description to help us generate an appropriate value
            let description = prop_schema.get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("");
            
            // Get the property type if available
            let prop_type = prop_schema.get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("string");
            
            // Generate a test value based on the property name, description, and operation
            let test_value = match (prop_name.as_str(), prop_type, operation) {
                // ID parameter
                ("id", _, _) if description.to_lowercase().contains("id") => {
                    // Look for clues about the ID format in the operation name
                    if operation.contains("PRODUCT") {
                        json!("product-01")
                    } else if operation.contains("CUSTOMER") {
                        json!("customer-01")
                    } else if operation.contains("ORDER") {
                        json!("order-01")
                    } else if operation.contains("REVIEW") {
                        json!("review-01")
                    } else {
                        json!("sample-id")
                    }
                },
                
                // Query parameter
                ("query", _, _) if description.to_lowercase().contains("query") => {
                    // Generate a simple query string
                    json!("test")
                },
                
                // Context parameter (for HELLO operation)
                ("context", _, "HELLO") => {
                    json!("general")
                },
                
                // Detail level parameter (for HELLO operation)
                ("detail_level", _, "HELLO") => {
                    json!("standard")
                },
                
                // Location parameter (for weather)
                ("location", _, _) if description.to_lowercase().contains("location") => {
                    json!("New York")
                },
                
                // Domain parameter (for referrals)
                ("domain", _, _) if description.to_lowercase().contains("domain") => {
                    json!("example.com")
                },
                
                // Relationship parameter (for referrals)
                ("relationship", _, _) if description.to_lowercase().contains("relationship") => {
                    json!("trusted")
                },
                
                // Type parameter (for filtering content)
                ("type", _, _) => {
                    json!("news")
                },
                
                // Field parameter (for specifying search fields)
                ("field", _, _) => {
                    json!("name")
                },
                
                // Sort parameters
                ("sort_by", _, _) => {
                    json!("date")
                },
                ("order", _, _) => {
                    json!("desc")
                },
                
                // Default for any other string parameters
                (_, "string", _) => {
                    json!("test_value")
                },
                
                // Default for numeric parameters
                (_, "number", _) | (_, "integer", _) => {
                    json!(1)
                },
                
                // Default for boolean parameters
                (_, "boolean", _) => {
                    json!(true)
                },
                
                // Default for any other parameters
                (_, _, _) => {
                    json!(null)
                }
            };
            
            // Add the test value to the arguments if it's not null
            if !test_value.is_null() {
                if let Some(obj) = arguments.as_object_mut() {
                    obj.insert(prop_name.clone(), test_value);
                }
            }
        }
    }
    
    arguments
}

async fn discover_service_http(url: &str) -> Result<DiscoveryResponse, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .send()
        .await?
        .json::<DiscoveryResponse>()
        .await?;
    
    Ok(resp)
}

// Helper function to call a tool
async fn call_tool<S, R>(
    write: &mut S,
    read: &mut R,
    name: &str,
    operation: &str,
    arguments: Value
) -> Result<(), Box<dyn Error>>
where
    S: SinkExt<Message> + Unpin,
    R: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    println!("\nTesting {} with {} operation", name, operation);
    
    let request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        id: json!(99),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": name,
            "arguments": arguments
        })),
    };
    
    println!("Request: {}", serde_json::to_string_pretty(&request.params).unwrap_or_default());
    
    write.send(Message::Text(serde_json::to_string(&request)?)).await
        .map_err(|_| "WebSocket send error")?;
    
    if let Some(Ok(Message::Text(response))) = read.next().await {
        let parsed: MCPResponse = serde_json::from_str(&response)?;
        if let Some(result) = parsed.result {
            let tool_result: ToolCallResult = serde_json::from_value(result)?;
            
            println!("  Result{}", if tool_result.is_error { " (ERROR)" } else { "" });
            for content in tool_result.content {
                if content.content_type == "text" {
                    if let Some(text) = content.text {
                        if tool_result.is_error {
                            println!("  Error: {}", text);
                        } else {
                            // Try to parse as JSON for better display
                            if let Ok(json_result) = serde_json::from_str::<Value>(&text) {
                                println!("  {}", serde_json::to_string_pretty(&json_result).unwrap_or_default());
                            } else {
                                println!("  {}", text);
                            }
                        }
                    }
                }
            }
        }
    }
    
    Ok(())
}