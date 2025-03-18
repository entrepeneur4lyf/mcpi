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
        .get_matches();

    let mut discovery_url = String::from("http://localhost:3001/mcpi/discover");
    let mut ws_url = String::from("ws://localhost:3001/mcpi");
    
    // Check if we should use DNS discovery
    if let Some(domain) = matches.get_one::<String>("domain") {
        println!("Performing DNS-based discovery for domain: {}", domain);
        
        match discovery::discover_mcp_services(domain).await {
            Ok(service_info) => {
                println!("Discovered MCP service:");
                println!("  Version: {}", service_info.version);
                println!("  Endpoint: {}", service_info.endpoint);
                
                // The TXT record gives us the discovery URL, which we use to get more details
                discovery_url = service_info.endpoint;
                
                // We'll derive the WebSocket URL after making the discovery request
                println!("Using discovery endpoint: {}", discovery_url);
            },
            Err(e) => {
                println!("DNS discovery failed: {}. Falling back to default URL.", e);
            }
        }
    } else if let Some(url) = matches.get_one::<String>("url") {
        // If user provides a direct WebSocket URL
        ws_url = url.clone();
        
        // Try to construct a discovery URL from the WebSocket URL
        if ws_url.starts_with("wss://") {
            discovery_url = ws_url.replace("wss://", "https://").replace("/mcpi", "/mcpi/discover");
        } else if ws_url.starts_with("ws://") {
            discovery_url = ws_url.replace("ws://", "http://").replace("/mcpi", "/mcpi/discover");
        }
        
        println!("Using provided server URL: {}", ws_url);
        println!("Derived discovery URL: {}", discovery_url);
    }
    
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
    
    // If we're using DNS discovery, derive the WebSocket URL from the discovery URL
    if matches.contains_id("domain") && !matches.contains_id("url") {
        // We don't need to use the domain directly, just derive the WebSocket URL
        if discovery_url.starts_with("https://") {
            ws_url = discovery_url.replace("https://", "wss://").replace("/mcpi/discover", "/mcpi");
        } else if discovery_url.starts_with("http://") {
            ws_url = discovery_url.replace("http://", "ws://").replace("/mcpi/discover", "/mcpi");
        }
        
        println!("\nDerived WebSocket URL for connection: {}", ws_url);
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
    
    write.send(Message::Text(serde_json::to_string(&init_request)?)).await?;
    
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
    
    write.send(Message::Text(serde_json::to_string(&list_resources_request)?)).await?;
    
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
    
    write.send(Message::Text(serde_json::to_string(&read_resource_request)?)).await?;
    
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
    
    write.send(Message::Text(serde_json::to_string(&list_tools_request)?)).await?;
    
    if let Some(Ok(Message::Text(response))) = read.next().await {
        let parsed: MCPResponse = serde_json::from_str(&response)?;
        if let Some(result) = parsed.result {
            println!("\nAvailable MCP tools:");
            if let Some(tools) = result.get("tools").and_then(|t| t.as_array()) {
                for tool in tools {
                    println!("  - {}", tool.get("name").and_then(|n| n.as_str()).unwrap_or("Unnamed"));
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
    
    // Call a tool
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
    
    write.send(Message::Text(serde_json::to_string(&call_tool_request)?)).await?;
    
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
    
    // Call another tool with GET operation
    println!("\nCalling customer_lookup tool with GET operation...");
    let call_tool_request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        id: json!(6),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "customer_lookup",
            "arguments": {
                "operation": "GET",
                "id": "cust-1001"
            }
        })),
    };
    
    write.send(Message::Text(serde_json::to_string(&call_tool_request)?)).await?;
    
    if let Some(Ok(Message::Text(response))) = read.next().await {
        let parsed: MCPResponse = serde_json::from_str(&response)?;
        if let Some(result) = parsed.result {
            let tool_result: ToolCallResult = serde_json::from_value(result)?;
            
            println!("\nCustomer lookup result:");
            for content in tool_result.content {
                if content.content_type == "text" {
                    if let Some(text) = content.text {
                        if let Ok(json_result) = serde_json::from_str::<Value>(&text) {
                            println!("  Customer: {} ({})", 
                                json_result.get("name").and_then(|n| n.as_str()).unwrap_or("Unknown"),
                                json_result.get("id").and_then(|i| i.as_str()).unwrap_or("")
                            );
                            println!("  Email: {}", json_result.get("email").and_then(|e| e.as_str()).unwrap_or(""));
                            println!("  Tier: {}", json_result.get("tier").and_then(|t| t.as_str()).unwrap_or(""));
                            println!("  Since: {}", json_result.get("since").and_then(|s| s.as_str()).unwrap_or(""));
                            
                            if let Some(preferences) = json_result.get("preferences").and_then(|p| p.as_object()) {
                                println!("  Preferences:");
                                for (key, value) in preferences {
                                    println!("    {}: {}", key, value);
                                }
                            }
                        } else {
                            println!("  {}", text);
                        }
                    }
                }
            }
        }
    }
    
    // End the session with a ping
    let ping_request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        id: json!(7),
        method: "ping".to_string(),
        params: None,
    };
    
    write.send(Message::Text(serde_json::to_string(&ping_request)?)).await?;
    
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