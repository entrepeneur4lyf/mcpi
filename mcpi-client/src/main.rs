use futures::{SinkExt, StreamExt};
use mcpi_common::{
    DiscoveryResponse, InitializeResult, MCPRequest, MCPResponse, ReadResourceResult, ToolCallResult
};
use serde_json::{json, Value};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use std::error::Error;
use clap::{Arg, Command};
mod discovery;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Parse command line arguments
    let matches = Command::new("MCPI Client")
        .version("0.1.0")
        .author("MCPI Team")
        .about("Model Context Protocol Integration Client")
        .arg(Arg::new("domain")
            .short('d')
            .long("domain")
            .value_name("DOMAIN")
            .help("Domain to discover MCPI services from (uses DNS TXT records)")
            .required(false))
        .arg(Arg::new("url")
            .short('u')
            .long("url")
            .value_name("URL")
            .help("Direct URL to MCPI server (bypasses DNS discovery)")
            .required(false))
        .arg(Arg::new("plugin")
            .short('p')
            .long("plugin")
            .value_name("PLUGIN")
            .help("Test a specific plugin only")
            .required(false))
        .get_matches();

    // Determine discovery and WebSocket URLs
    let (discovery_url, ws_url) = if let Some(domain) = matches.get_one::<String>("domain") {
        println!("Performing DNS-based discovery for domain: {}", domain);
        
        match discovery::discover_mcp_services(domain).await {
            Ok(service_info) => {
                println!("Discovered MCP service:");
                println!("  Version: {}", service_info.version);
                println!("  Endpoint: {}", service_info.endpoint);
                
                let discovery = service_info.endpoint;
                
                // We'll derive the WebSocket URL after making the discovery request
                println!("Using discovery endpoint: {}", discovery);
                
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
    } else if let Some(url) = matches.get_one::<String>("url") {
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
    
    // List resources
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
    
    // Read a resource
    let read_resource_request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        id: json!(3),
        method: "resources/read".to_string(),
        params: Some(json!({
            "uri": format!("mcpi://{}/resources/products.json", discovery_resp.provider.domain)
        })),
    };
    
    write.send(Message::Text(serde_json::to_string(&read_resource_request)?)).await
        .map_err(|_| "WebSocket send error")?;
    
    if let Some(Ok(Message::Text(response))) = read.next().await {
        let parsed: MCPResponse = serde_json::from_str(&response)?;
        if let Some(result) = parsed.result {
            let read_result: ReadResourceResult = serde_json::from_value(result)?;
            println!("\nResource content sample (first 150 chars):");
            for content in read_result.contents {
                println!("  URI: {}", content.uri);
                println!("  Type: {}", content.mime_type.unwrap_or_else(|| "unknown".to_string()));
                println!("  Content: {}", content.text.chars().take(150).collect::<String>() + "...");
            }
        } else if let Some(error) = parsed.error {
            println!("Error reading resource: {} (code: {})", error.message, error.code);
        }
    }
    
    // List tools
    let list_tools_request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        id: json!(4),
        method: "tools/list".to_string(),
        params: Some(json!({})),
    };
    
    write.send(Message::Text(serde_json::to_string(&list_tools_request)?)).await
        .map_err(|_| "WebSocket send error")?;
    
    let mut tools = Vec::new();
    if let Some(Ok(Message::Text(response))) = read.next().await {
        let parsed: MCPResponse = serde_json::from_str(&response)?;
        if let Some(result) = parsed.result {
            println!("\nAvailable MCP tools:");
            if let Some(tools_array) = result.get("tools").and_then(|t| t.as_array()) {
                for tool in tools_array {
                    let tool_name = tool.get("name").and_then(|n| n.as_str()).unwrap_or("Unnamed").to_string();
                    tools.push(tool_name.clone());
                    
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
    let specific_plugin = matches.get_one::<String>("plugin").map(|s| s.to_string());
    
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
        
        match tool_name.as_str() {
            "product_search" => {
                // Test SEARCH operation for products
                println!("\nCalling product_search tool with SEARCH operation...");
                let call_tool_request = MCPRequest {
                    jsonrpc: "2.0".to_string(),
                    id: json!(5),
                    method: "tools/call".to_string(),
                    params: Some(json!({
                        "name": "product_search",
                        "arguments": {
                            "operation": "SEARCH",
                            "query": "bamboo"
                        }
                    })),
                };
                
                write.send(Message::Text(serde_json::to_string(&call_tool_request)?)).await
                    .map_err(|_| "WebSocket send error")?;
                
                if let Some(Ok(Message::Text(response))) = read.next().await {
                    let parsed: MCPResponse = serde_json::from_str(&response)?;
                    if let Some(result) = parsed.result {
                        let tool_result: ToolCallResult = serde_json::from_value(result)?;
                        
                        println!("\nTool call result{}:", if tool_result.is_error { " (ERROR)" } else { "" });
                        for content in tool_result.content {
                            if content.content_type == "text" {
                                if let Some(text) = content.text {
                                    if tool_result.is_error {
                                        println!("  Error: {}", text);
                                    } else {
                                        // Parse the result as JSON for better display
                                        if let Ok(json_result) = serde_json::from_str::<Value>(&text) {
                                            if let Some(results) = json_result.get("results").and_then(|r| r.as_array()) {
                                                println!("  Found {} products matching 'bamboo':", results.len());
                                                for product in results {
                                                    println!("  - {} ({})", 
                                                        product.get("name").and_then(|n| n.as_str()).unwrap_or("Unnamed"),
                                                        product.get("id").and_then(|i| i.as_str()).unwrap_or("")
                                                    );
                                                    println!("    Price: ${}", product.get("price").and_then(|p| p.as_f64()).unwrap_or(0.0));
                                                    println!("    Description: {}", product.get("description").and_then(|d| d.as_str()).unwrap_or(""));
                                                }
                                            } else {
                                                println!("  {}", text);
                                            }
                                        } else {
                                            println!("  {}", text);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                
                // Test GET operation for products
                call_tool(&mut write, &mut read, "product_search", "GET", 
                    json!({"id": "eco-1001"})).await?; // Bamboo Water Bottle
                
                // Test LIST operation for products
                call_tool(&mut write, &mut read, "product_search", "LIST", 
                    json!({})).await?;
            },
            
            "customer_lookup" => {
                // Test GET operation for customer
                call_tool(&mut write, &mut read, "customer_lookup", "GET", 
                    json!({"id": "cust-1001"})).await?;
                
                // Test LIST operation for customers
                call_tool(&mut write, &mut read, "customer_lookup", "LIST", 
                    json!({})).await?;
            },
            
            "order_history" => {
                // Test GET operation for orders
                call_tool(&mut write, &mut read, "order_history", "GET", 
                    json!({"id": "order-5001"})).await?;
                
                // Test SEARCH operation for orders by customer
                call_tool(&mut write, &mut read, "order_history", "SEARCH", 
                    json!({"field": "customer_id", "query": "cust-1001"})).await?;
                
                // Test LIST operation for orders
                call_tool(&mut write, &mut read, "order_history", "LIST", 
                    json!({})).await?;
            },
            
            "product_reviews" => {
                // Test GET operation for reviews
                call_tool(&mut write, &mut read, "product_reviews", "GET", 
                    json!({"id": "rev-2001"})).await?;
                
                // Test SEARCH operation for reviews by product
                call_tool(&mut write, &mut read, "product_reviews", "SEARCH", 
                    json!({"field": "product_id", "query": "eco-1001"})).await?;
                
                // Test LIST operation for reviews
                call_tool(&mut write, &mut read, "product_reviews", "LIST", 
                    json!({})).await?;
            },
            
            "website_content" => {
                // Test GET operation for about page
                call_tool(&mut write, &mut read, "website_content", "GET", 
                    json!({"id": "about"})).await?;
                
                // Test LIST operation for news content
                call_tool(&mut write, &mut read, "website_content", "LIST", 
                    json!({"type": "news", "sort_by": "date", "order": "desc"})).await?;
                
                // Test SEARCH operation across all content
                call_tool(&mut write, &mut read, "website_content", "SEARCH", 
                    json!({"query": "sustainability"})).await?;
            },
            
            "weather_forecast" => {
                // Test GET operation for weather
                call_tool(&mut write, &mut read, "weather_forecast", "GET", 
                    json!({"location": "London"})).await?;
                
                // Test LIST operation for weather
                call_tool(&mut write, &mut read, "weather_forecast", "LIST", 
                    json!({})).await?;
            },
            
            _ => {
                println!("Skipping unknown tool: {}", tool_name);
            }
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
    
    // Create a new arguments object with the operation included
    let mut args = serde_json::Map::new();
    args.insert("operation".to_string(), json!(operation));
    
    // Add all other arguments
    if let Some(arg_obj) = arguments.as_object() {
        for (key, value) in arg_obj {
            args.insert(key.clone(), value.clone());
        }
    }
    
    let request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        id: json!(99),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": name,
            "arguments": args
        })),
    };
    
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
                                if let Some(results) = json_result.get("results").and_then(|r| r.as_array()) {
                                    println!("  Found {} results", results.len());
                                    let sample_size = std::cmp::min(results.len(), 2);
                                    for i in 0..sample_size {
                                        println!("  Sample result {}: {}", i+1, 
                                            serde_json::to_string_pretty(&results[i]).unwrap_or_default());
                                    }
                                    if results.len() > sample_size {
                                        println!("  ... and {} more results", results.len() - sample_size);
                                    }
                                } else {
                                    println!("  {}", serde_json::to_string_pretty(&json_result).unwrap_or_default());
                                }
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