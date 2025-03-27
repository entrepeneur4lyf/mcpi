// mcpi-client/src/main.rs
use futures::{SinkExt, StreamExt};
use mcpi_common::{
    DiscoveryResponse, InitializeResult, MCPRequest, MCPResponse, CallToolResult,
    MCPI_VERSION, LATEST_MCP_VERSION,
    ContentItem,
    ResourceContentUnion, TextResourceContents, BlobResourceContents, // Make sure these are in mcpi-common
    Provider, Referral, CapabilityDescription, InitializeParams,
};
use serde_json::{json, Value};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use std::error::Error;
use clap::{Parser, Subcommand};
use reqwest::{header::{HeaderMap, HeaderValue, CONTENT_TYPE, ACCEPT}, Client as ReqwestClient};
use rand::Rng;

mod discovery;

// Define custom header name
static MCP_SESSION_ID_HEADER: &str = "mcp-session-id";


#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    domain: Option<String>,
    #[arg(short = 'u', long)]
    base_url: Option<String>,
    #[arg(short, long)]
    plugin: Option<String>,
    // #[command(subcommand)]
    // command: Option<Commands>,
}

// Subcommand not currently used
#[derive(Subcommand)]
enum Commands {
    Discover { domain: String },
}

type BoxedError = Box<dyn Error + Send + Sync>;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Protocol {
    McpiWebSocket,
    McpHttp,
}

#[tokio::main]
async fn main() -> Result<(), BoxedError> {
    let cli = Cli::parse();

    let mut protocol = Protocol::McpHttp;
    let mut discovery_url = String::from("http://localhost:3001/mcpi/discover");
    let mut service_base_url = String::from("http://localhost:3001");

    // Determine protocol and base URL
    if let Some(domain) = &cli.domain {
        println!("Performing DNS-based discovery for domain: {}", domain);
        match discovery::discover_mcp_services(domain).await {
            Ok(service_info) => {
                protocol = Protocol::McpiWebSocket; // DNS success implies MCPI/WS
                println!("DNS discovery successful, using MCPI (WebSocket) protocol.");
                discovery_url = service_info.endpoint;
                let base = discovery_url.replace("/mcpi/discover", "");
                service_base_url = base;
                println!("  Discovered Endpoint (for discovery): {}", discovery_url);
                println!("  Derived Base URL: {}", service_base_url);
            },
            Err(e) => { println!("DNS discovery failed: {}. Will try default/base URL (MCP/HTTP).", e); if cli.base_url.is_none() { println!("Using default MCP (HTTP) on localhost."); } }
        }
    }

    if protocol != Protocol::McpiWebSocket {
        if let Some(base_url_arg) = &cli.base_url {
            protocol = Protocol::McpHttp; // Explicitly MCP/HTTP
            println!("Using provided base URL: {}", base_url_arg);
            service_base_url = base_url_arg.trim_end_matches('/').to_string();
            discovery_url = format!("{}/mcpi/discover", service_base_url);
            println!("  Protocol: MCP (HTTP)");
            println!("  Derived Discovery URL: {}", discovery_url);
            println!("  Derived MCP Service Base URL: {}", service_base_url);
        } else if cli.domain.is_none() { // No domain and no base_url -> use localhost defaults
             println!("Using default MCP (HTTP) on localhost.");
             // Defaults already set for MCP/HTTP
        }
    }


    // --- HTTP Discovery ---
    println!("\nDiscovering service capabilities via HTTP discovery endpoint...");
    let discovery_resp = discover_service_http(&discovery_url).await?;
    println!("Provider: {} ({})", discovery_resp.provider.name, discovery_resp.provider.domain);
    println!("Mode: {}", discovery_resp.mode);
    println!("\nAvailable capabilities (from discovery):");
    for cap in &discovery_resp.capabilities { println!("  - {}: {}", cap.name, cap.description); println!("    Ops: {}", cap.operations.join(", ")); }
    println!("\nReferrals (from discovery):");
    for ref_info in &discovery_resp.referrals { println!("  - {}: {}", ref_info.name, ref_info.domain); }


    // --- Branch based on selected protocol ---
    match protocol {
        Protocol::McpiWebSocket => {
            let ws_url = if service_base_url.starts_with("https://") { service_base_url.replace("https://", "wss://") + "/mcpi" } else { service_base_url.replace("http://", "ws://") + "/mcpi" };
            println!("\nConnecting via WebSocket (MCPI) to {}...", ws_url);
            run_mcpi_websocket_client(ws_url, cli.plugin, discovery_resp).await?;
        }
        Protocol::McpHttp => {
            let mcp_url = format!("{}/mcp", service_base_url);
            println!("\nConnecting via Streamable HTTP (MCP) to {}...", mcp_url);
            run_mcp_http_client(mcp_url, cli.plugin, discovery_resp).await?;
        }
    }
    Ok(())
}

// --- ========================================= ---
// --- Streamable HTTP Client Implementation (MCP) ---
// --- ========================================= ---
async fn run_mcp_http_client(
    mcp_url: String,
    specific_plugin: Option<String>,
    _discovery_resp: DiscoveryResponse // Mark unused for now
) -> Result<(), BoxedError> {

    let http_client = ReqwestClient::new();
    let mut session_id: Option<String> = None;

    // --- Initial GET /mcp ---
    println!("\nEstablishing SSE connection via GET {}...", mcp_url);
    let get_response = http_client.get(&mcp_url).header(ACCEPT, "text/event-stream").send().await.map_err(|e| format!("GET /mcp request failed: {}", e))?;
    if !get_response.status().is_success() { return Err(format!("GET /mcp failed status: {}", get_response.status()).into()); }
    if let Some(ct) = get_response.headers().get(CONTENT_TYPE) { if !ct.to_str()?.starts_with("text/event-stream") { return Err(format!("Expected text/event-stream, got: {:?}", ct).into()); } } else { return Err("Missing Content-Type on GET /mcp response".into()); }
    if let Some(sid_value) = get_response.headers().get(MCP_SESSION_ID_HEADER) { session_id = Some(sid_value.to_str()?.to_string()); println!("Obtained MCP session ID: {}", session_id.as_ref().unwrap()); } else { println!("Warning: Server did not provide mcp-session-id header."); }
    println!("SSE stream ostensibly connected (event processing not implemented yet).");
    // TODO: Spawn task to handle `get_response.bytes_stream()` for SSE events

    // --- Initialize via POST /mcp ---
    println!("\nSending initialize request via POST {}...", mcp_url);
    let init_params = InitializeParams {
        client_info: mcpi_common::Implementation { name: "MCP Test Client".to_string(), version: "0.1.0".to_string() },
        protocol_version: LATEST_MCP_VERSION.to_string(), // Use MCP version
        capabilities: Default::default(),
    };
    let init_request = MCPRequest { jsonrpc: "2.0".to_string(), id: json!(1), method: "initialize".to_string(), params: Some(serde_json::to_value(init_params)?) };
    let init_req_str = serde_json::to_string(&init_request).map_err(|e| format!("Serialize init err: {}", e))?;
    let mut headers = HeaderMap::new(); headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json")); if let Some(sid) = &session_id { headers.insert(MCP_SESSION_ID_HEADER, HeaderValue::from_str(sid)?); }
    let post_response = http_client.post(&mcp_url).headers(headers.clone()).body(init_req_str).send().await.map_err(|e| format!("POST /mcp initialize failed: {}", e))?;
    if !post_response.status().is_success() { return Err(format!("Initialize POST failed status: {}", post_response.status()).into()); }
    let init_resp_body = post_response.json::<MCPResponse>().await.map_err(|e| format!("Parse initialize POST resp err: {}", e))?;
    if let Some(err) = init_resp_body.error { println!("Init error: {} ({})", err.message, err.code); return Err("Init failed".into()); } if let Some(res) = init_resp_body.result { let init_res: InitializeResult = serde_json::from_value(res).map_err(|e| format!("Parse init result err: {}", e))?; println!("\nMCP initialized: Server: {} v{}, Proto: v{}", init_res.server_info.name, init_res.server_info.version, init_res.protocol_version); if let Some(inst) = init_res.instructions { println!("  Instructions: {}", inst); } } else { return Err("Invalid init response".into()); }

    // --- List Resources via POST /mcp ---
    println!("\nListing Resources via POST {}...", mcp_url);
    let list_res_req = MCPRequest { jsonrpc: "2.0".to_string(), id: json!(2), method: "resources/list".to_string(), params: None };
    let list_res_str = serde_json::to_string(&list_res_req)?;
    let list_res_resp = http_client.post(&mcp_url)
        .headers(headers.clone()) // Reuse headers with session ID
        .body(list_res_str)
        .send()
        .await?
        .json::<MCPResponse>()
        .await?;

    let mut resources_list: Vec<Value> = Vec::new(); // To store resource info if needed later
    if let Some(e) = list_res_resp.error { println!("Err list res: {} ({})", e.message, e.code); }
    else if let Some(r) = list_res_resp.result {
         // Assuming result structure matches ListResourcesResult in common
         if let Some(res_arr) = r.get("resources").and_then(|res| res.as_array()) {
             println!("\nAvailable MCP resources:");
             resources_list = res_arr.clone(); // Store for later use if needed
             for item in res_arr { println!("  - {} ({})", item.get("name").and_then(|n|n.as_str()).unwrap_or("?"), item.get("uri").and_then(|u|u.as_str()).unwrap_or("?")); if let Some(d)=item.get("description").and_then(|d|d.as_str()){println!("    Desc: {}",d);} }
         } else { println!(" (No resources in result)"); }
    } else { println!("Warn: Invalid list res resp (no result/error)"); }


    // --- List Tools via POST /mcp ---
    println!("\nListing Tools via POST {}...", mcp_url);
    let list_tools_req = MCPRequest { jsonrpc: "2.0".to_string(), id: json!(3), method: "tools/list".to_string(), params: None };
    let list_tools_str = serde_json::to_string(&list_tools_req)?;
    let list_tools_resp = http_client.post(&mcp_url)
        .headers(headers.clone()) // Reuse headers with session ID
        .body(list_tools_str)
        .send()
        .await?
        .json::<MCPResponse>()
        .await?;

    let mut tools: Vec<String> = Vec::new();
    let mut tools_info: Vec<Value> = Vec::new(); // Store full tool info
    if let Some(e) = list_tools_resp.error { println!("Err list tools: {} ({})", e.message, e.code); }
    else if let Some(r) = list_tools_resp.result {
        // Assuming result structure matches ListToolsResult
        if let Some(ts)=r.get("tools").and_then(|t|t.as_array()){
            println!("\nAvailable MCP tools:");
            for tool in ts {
                let name=tool.get("name").and_then(|n|n.as_str()).unwrap_or("?").to_string();
                tools.push(name.clone());
                tools_info.push(tool.clone()); // Store full tool info
                println!("  - {}",name);
                if let Some(d)=tool.get("description").and_then(|d|d.as_str()){println!("    Desc: {}",d);}
                if let Some(a)=tool.get("annotations"){println!("    Anno: {}",serde_json::to_string_pretty(a).unwrap_or_default());}
                if let Some(s)=tool.get("inputSchema"){println!("    Ops:");if let Some(ops)=s.get("properties").and_then(|p|p.get("operation")).and_then(|o|o.get("enum")).and_then(|e|e.as_array()){let ops_str:Vec<String>=ops.iter().filter_map(|o|o.as_str().map(String::from)).collect();println!("      {}",ops_str.join(", "));}else{println!("      (N/A)");}}
            }
        } else { println!(" (No tools in result)"); }
    } else { println!("Warn: Invalid list tools resp (no result/error)"); }


    // --- Test Batch via POST /mcp ---
    println!("\nTesting Batch via POST {}...", mcp_url);
    let batch_req_data = json!([{ "jsonrpc": "2.0", "id": 10, "method": "ping", "params": null }, { "jsonrpc": "2.0", "id": 11, "method": "resources/list", "params": null }]);
    let batch_req_str = serde_json::to_string(&batch_req_data).map_err(|e|format!("Serialize batch err: {}",e))?;
    let batch_resp = http_client.post(&mcp_url)
        .headers(headers.clone())
        .body(batch_req_str)
        .send()
        .await?
        .json::<Vec<MCPResponse>>() // Expecting an array for batch response
        .await?;
    println!("Batch response ({} items):", batch_resp.len());
    for (i, r) in batch_resp.iter().enumerate() { println!("  Item {}: ID={}", i+1, r.id); if let Some(err)=&r.error{println!("    Err: {} ({})", err.message, err.code);} }


    // --- Select Tools to Test ---
    let tools_to_test = if let Some(p_name)=specific_plugin{if tools.contains(&p_name){vec![p_name]}else{println!("\nTool '{}' unavailable. Available: {}",p_name,tools.join(", "));return Err("Tool not found".into());}}else{tools};

    // --- Test Tools via POST /mcp ---
    for tool_name in tools_to_test {
        println!("\n=== Testing tool (HTTP): {} ===", tool_name);
        let tool_info = tools_info.iter().find(|t|t.get("name").and_then(|n|n.as_str())==Some(&tool_name)).cloned().unwrap_or_default();
        let default_schema = json!({}); let tool_schema = tool_info.get("inputSchema").unwrap_or(&default_schema);
        let operations = tool_schema.get("properties").and_then(|p|p.get("operation")).and_then(|o|o.get("enum")).and_then(|e|e.as_array()).map(|ops|ops.iter().filter_map(|op|op.as_str().map(String::from)).collect::<Vec<String>>()).unwrap_or_else(||vec!["SEARCH".to_string()]);
        println!("Supported operations: {:?}", operations);

        // TODO: Test completions via POST "completion/complete"

        for operation in operations {
            let args=generate_test_arguments(&tool_name,&operation,tool_schema);
            call_tool_http(&http_client, &mcp_url, session_id.as_deref(), &tool_name, &operation, args).await?;
        }
    }

    // --- Ping via POST /mcp ---
    println!("\nSending ping via POST {}...", mcp_url);
    let ping_req = MCPRequest { jsonrpc: "2.0".to_string(), id: json!(99), method: "ping".to_string(), params: None }; let ping_req_str = serde_json::to_string(&ping_req)?; let ping_resp = http_client.post(&mcp_url).headers(headers).body(ping_req_str).send().await?.json::<MCPResponse>().await?; match ping_resp { MCPResponse{error: Some(err),..} => println!("\nPing error: {} ({})",err.message, err.code), MCPResponse{result: Some(_),..} => println!("\nPing successful."), _ => println!("\nInvalid ping response"), }

    println!("\n(MCP HTTP Client finished - Requires full SSE stream handling & POST->SSE response handling)");
    Ok(())
}


// --- ========================================== ---
// --- WebSocket Client Implementation (MCPI) ---
// --- ========================================== ---
async fn run_mcpi_websocket_client( ws_url: String, specific_plugin: Option<String>, _discovery_resp: DiscoveryResponse ) -> Result<(), BoxedError> {
    // ... (WebSocket logic remains largely the same as before, using call_tool_ws) ...
    let (ws_stream, _) = connect_async(&ws_url).await.map_err(|e| format!("WS connection failed: {}", e))?;
    println!("WebSocket connection established.");
    let (mut write, mut read) = ws_stream.split();

    // Init
    let init_request = MCPRequest { jsonrpc: "2.0".to_string(), id: json!(1), method: "initialize".to_string(), params: Some(json!({ "clientInfo": { "name": "MCPI Test Client", "version": "0.1.0" }, "protocolVersion": mcpi_common::MCPI_VERSION, "capabilities": {} })) };
    let init_req_str = serde_json::to_string(&init_request).map_err(|e| format!("Serialize init err: {}", e))?;
    write.send(Message::Text(init_req_str.into())).await.map_err(|e| format!("WS send err (init): {}", e))?;
    if let Some(Ok(Message::Text(resp_str))) = read.next().await { let parsed: MCPResponse = serde_json::from_str(&resp_str).map_err(|e| format!("Parse init resp err: {}", e))?; if let Some(err) = parsed.error { println!("Init error: {} ({})", err.message, err.code); return Err("Init failed".into()); } if let Some(res) = parsed.result { let init_res: InitializeResult = serde_json::from_value(res).map_err(|e| format!("Parse init result err: {}", e))?; println!("\nMCPI initialized: Server: {} v{}, Proto: v{}", init_res.server_info.name, init_res.server_info.version, init_res.protocol_version); if let Some(inst) = init_res.instructions { println!("  Instructions: {}", inst); } } else { return Err("Invalid init response".into()); } } else { return Err("No init response".into()); }

    // List Resources
    let list_res_req = MCPRequest { jsonrpc: "2.0".to_string(), id: json!(2), method: "resources/list".to_string(), params: None }; let list_res_str = serde_json::to_string(&list_res_req).map_err(|e| format!("Serialize list res err: {}", e))?; write.send(Message::Text(list_res_str.into())).await.map_err(|e| format!("WS send err (list res): {}", e))?; if let Some(Ok(Message::Text(resp_str))) = read.next().await { let parsed: MCPResponse = serde_json::from_str(&resp_str).map_err(|e| format!("Parse list res resp err: {}", e))?; if let Some(e)=parsed.error{println!("Err list res: {} ({})",e.message,e.code);}else if let Some(r)=parsed.result{println!("\nAvailable MCPI resources:");if let Some(res)=r.get("resources").and_then(|r|r.as_array()){for item in res{println!("  - {} ({})", item.get("name").and_then(|n|n.as_str()).unwrap_or("?"), item.get("uri").and_then(|u|u.as_str()).unwrap_or("?")); if let Some(d)=item.get("description").and_then(|d|d.as_str()){println!("    Desc: {}",d);}}}else{println!(" (No resources)");}}else{println!("Warn: Invalid list res resp");}} else { println!("Warn: No list res resp"); }

    // List Tools
    let list_tools_req = MCPRequest { jsonrpc: "2.0".to_string(), id: json!(3), method: "tools/list".to_string(), params: None }; let list_tools_str = serde_json::to_string(&list_tools_req).map_err(|e| format!("Serialize list tools err: {}", e))?; write.send(Message::Text(list_tools_str.into())).await.map_err(|e| format!("WS send err (list tools): {}", e))?; let mut tools = Vec::new(); let mut tools_info = Vec::new(); if let Some(Ok(Message::Text(resp_str))) = read.next().await { let parsed: MCPResponse = serde_json::from_str(&resp_str).map_err(|e| format!("Parse list tools resp err: {}", e))?; if let Some(e)=parsed.error{println!("Err list tools: {} ({})",e.message,e.code);}else if let Some(r)=parsed.result{println!("\nAvailable MCPI tools:");if let Some(ts)=r.get("tools").and_then(|t|t.as_array()){for tool in ts{let name=tool.get("name").and_then(|n|n.as_str()).unwrap_or("?").to_string();tools.push(name.clone());tools_info.push(tool.clone());println!("  - {}",name);if let Some(d)=tool.get("description").and_then(|d|d.as_str()){println!("    Desc: {}",d);}if let Some(a)=tool.get("annotations"){println!("    Anno: {}",serde_json::to_string_pretty(a).unwrap_or_default());} if let Some(s)=tool.get("inputSchema"){println!("    Ops:");if let Some(ops)=s.get("properties").and_then(|p|p.get("operation")).and_then(|o|o.get("enum")).and_then(|e|e.as_array()){let ops_str:Vec<String>=ops.iter().filter_map(|o|o.as_str().map(String::from)).collect();println!("      {}",ops_str.join(", "));}else{println!("      (N/A)");}}}}else{println!(" (No tools)");}}else{println!("Warn: Invalid list tools resp");}} else {println!("Warn: No list tools resp");}

    // Select Tools
    let tools_to_test = if let Some(p_name)=specific_plugin{if tools.contains(&p_name){vec![p_name]}else{println!("\nTool '{}' unavailable. Available: {}",p_name,tools.join(", "));return Err("Tool not found".into());}}else{tools};

    // Test Batch
    println!("\nTesting JSON-RPC batch request..."); let batch_req_data=json!([{ "jsonrpc": "2.0", "id": 10, "method": "ping", "params": null }, { "jsonrpc": "2.0", "id": 11, "method": "resources/list", "params": null }]); let batch_req_str=serde_json::to_string(&batch_req_data).map_err(|e|format!("Serialize batch err: {}",e))?; write.send(Message::Text(batch_req_str.into())).await.map_err(|e|format!("WS send err (batch): {}",e))?; if let Some(Ok(Message::Text(resp_str)))=read.next().await{if resp_str.trim().starts_with('['){match serde_json::from_str::<Vec<MCPResponse>>(&resp_str){Ok(br)=>{println!("Batch response ({} items):",br.len());for(i,r)in br.iter().enumerate(){println!("  Item {}: ID={}",i+1,r.id);if let Some(err)=&r.error{println!("    Err: {} ({})",err.message,err.code);}}}Err(e)=>println!("Err parsing batch resp: {}",e),}}else{println!("Warn: Expected batch array, got: {}",resp_str);}}else{println!("Warn: No batch resp");}

    // Test Tools
    for tool_name in tools_to_test { println!("\n=== Testing tool (WebSocket): {} ===", tool_name); let tool_info=tools_info.iter().find(|t|t.get("name").and_then(|n|n.as_str())==Some(&tool_name)).cloned().unwrap_or_default(); let default_schema=json!({}); let tool_schema=tool_info.get("inputSchema").unwrap_or(&default_schema); let operations=tool_schema.get("properties").and_then(|p|p.get("operation")).and_then(|o|o.get("enum")).and_then(|e|e.as_array()).map(|ops|ops.iter().filter_map(|op|op.as_str().map(String::from)).collect::<Vec<String>>()).unwrap_or_else(||vec!["SEARCH".to_string()]); println!("Supported operations: {:?}",operations); if tool_name=="weather_forecast"&&operations.contains(&"GET_AUDIO".to_string()){/* ... test audio ... */} if tool_name=="weather_forecast"{/* ... test completions ... */} for operation in operations{if tool_name=="weather_forecast"&&operation=="GET_AUDIO"{continue;} let args=generate_test_arguments(&tool_name,&operation,tool_schema); call_tool_ws(&mut write,&mut read,&tool_name,&operation,args).await?;}} // Use WebSocket call_tool

    // Ping
    let ping_req = MCPRequest { jsonrpc: "2.0".to_string(), id: json!(99), method: "ping".to_string(), params: None }; let ping_req_str=serde_json::to_string(&ping_req).map_err(|e|format!("Serialize ping err: {}",e))?; write.send(Message::Text(ping_req_str.into())).await.map_err(|e|format!("WS send err (ping): {}",e))?; if let Some(Ok(Message::Text(resp_str)))=read.next().await{match serde_json::from_str::<MCPResponse>(&resp_str){Ok(p)=>{if let Some(err)=p.error{println!("\nPing error: {} ({})",err.message,err.code);}else if p.result.is_some(){println!("\nPing successful.");}else{println!("\nInvalid ping resp");}},Err(e)=>println!("\nErr parsing ping resp: {}",e),}}else{println!("\nNo ping response");}

    println!("\nClosing MCPI connection"); Ok(())
}


// --- Helper Functions ---
async fn discover_service_http(url: &str) -> Result<DiscoveryResponse, BoxedError> { let client=reqwest::Client::new(); Ok(client.get(url).send().await?.json::<DiscoveryResponse>().await?) }
fn generate_test_arguments(_tool_name: &str, operation: &str, schema: &Value) -> Value { let mut args=json!({"operation": operation}); if let Some(props)=schema.get("properties").and_then(|p|p.as_object()){for(p_name,p_schema)in props.iter().filter(|(k,_)|*k!="operation"){let desc=p_schema.get("description").and_then(|d|d.as_str()).unwrap_or("");let p_type=p_schema.get("type").and_then(|t|t.as_str()).unwrap_or("string");let t_val=match(p_name.as_str(),p_type,operation){("id",_,_)if desc.contains("ID")=>json!("test-id-123"),("query",_,_)=>json!("search query"),("location",_,_)=>json!("London"),("domain",_,_)=>json!("target.example.com"),("relationship",_,_)=>json!("affiliate"),(_,"string",_)=>json!("default str"),(_,"number",_)|(_,"integer",_)=>json!(42),(_,"boolean",_)=>json!(false),_=>Value::Null,};if !t_val.is_null(){if let Some(obj)=args.as_object_mut(){obj.insert(p_name.clone(),t_val);}}}} args }
async fn get_completions<S, R>( write: &mut S, read: &mut R, method: &str, param_name: &str, partial_val: &str, context: Option<Value> ) -> Result<Vec<String>, BoxedError> where S: SinkExt<Message, Error=tokio_tungstenite::tungstenite::Error>+Unpin, R: StreamExt<Item=Result<Message, tokio_tungstenite::tungstenite::Error>>+Unpin { let mut params=json!({"method":method,"parameterName":param_name,"partialValue":partial_val}); if let Some(ctx_obj)=context.as_ref().and_then(|c|c.as_object()){if let Some(p_obj)=params.as_object_mut(){for(k,v)in ctx_obj{p_obj.insert(format!("context.{}",k),v.clone());}}}else if let Some(ctx_val)=context{if let Some(p_obj)=params.as_object_mut(){p_obj.insert("context".to_string(),ctx_val);}} let req = MCPRequest { jsonrpc:"2.0".to_string(), id:json!(98), method:"completions".to_string(), params:Some(params) }; let req_str = serde_json::to_string(&req).map_err(|e|format!("Serialize completions err: {}",e))?; write.send(Message::Text(req_str.into())).await.map_err(|e|format!("WS send err (completions): {}",e))?; if let Some(Ok(Message::Text(resp_str)))=read.next().await{match serde_json::from_str::<MCPResponse>(&resp_str){Ok(p)=>{if let Some(e)=p.error{println!("Completions error: {} ({})",e.message,e.code);Ok(vec![])}else if let Some(r)=p.result{Ok(r.get("suggestions").and_then(|s|s.as_array()).map(|a|a.iter().filter_map(|v|v.as_str().map(String::from)).collect()).unwrap_or_default())}else{println!("Invalid completions resp");Ok(vec![])}}Err(e)=>Err(format!("Parse completions resp err: {}",e).into()),}}else{Err("No completions response".into())} }

// Renamed original call_tool to call_tool_ws
async fn call_tool_ws<S, R>( write: &mut S, read: &mut R, name: &str, operation: &str, arguments: Value ) -> Result<(), BoxedError> where S: SinkExt<Message, Error=tokio_tungstenite::tungstenite::Error>+Unpin, R: StreamExt<Item=Result<Message, tokio_tungstenite::tungstenite::Error>>+Unpin {
    println!("\nTesting {} with {} operation (WebSocket)",name,operation);
    let req=MCPRequest{jsonrpc:"2.0".to_string(),id:json!(format!("{}-{}-{}",name,operation,rand::thread_rng().gen::<u16>())),method:"tools/call".to_string(),params:Some(json!({"name":name,"arguments":arguments}))};
    println!("Request Params: {}", serde_json::to_string_pretty(req.params.as_ref().unwrap()).unwrap_or_default());
    let req_str=serde_json::to_string(&req).map_err(|e|format!("Serialize tool call err: {}",e))?;
    write.send(Message::Text(req_str.into())).await.map_err(|e|format!("WS send err (tool call): {}",e))?;
    if let Some(Ok(Message::Text(resp_str)))=read.next().await{match serde_json::from_str::<MCPResponse>(&resp_str){Ok(p)=>{if let Some(e)=p.error{println!("  Tool call error: {} ({})",e.message,e.code);}else if let Some(r)=p.result{match serde_json::from_value::<CallToolResult>(r.clone()){Ok(tr)=>{println!("  Result{}",if tr.is_error{" (ERROR)"}else{""});for c in tr.content{match c{ ContentItem::Text{text,..}=>{if let Ok(j)=serde_json::from_str::<Value>(&text){println!("  {}",serde_json::to_string_pretty(&j).unwrap_or(text));}else{println!("  {}",text);}}, ContentItem::Audio{data,mime_type,..}=>println!("  Audio: {}b, {}",data.len(),mime_type), ContentItem::Image{data,mime_type,..}=>println!("  Image: {}b, {}",data.len(),mime_type), ContentItem::Resource{resource,..}=>{match resource{ResourceContentUnion::Text(tc)=>println!("  Resource (Text): {}",tc.uri),ResourceContentUnion::Blob(bc)=>println!("  Resource (Blob): {}",bc.uri),}},}}}Err(e)=>println!("  Err parsing ToolCallResult: {}\nRaw: {}",e,serde_json::to_string_pretty(&r).unwrap_or_default()),}}else{println!("  Invalid tool call response");}}Err(e)=>println!("Err parsing tool call resp: {}",e),}}else{println!("Warn: No tool call resp");} Ok(())
}

// --- NEW: Helper function to call a tool via HTTP POST ---
async fn call_tool_http(
    client: &ReqwestClient, // Use shared client
    mcp_url: &str,
    session_id: Option<&str>, // Borrow session ID string
    name: &str,
    operation: &str,
    arguments: Value
) -> Result<(), BoxedError> {
    println!("\nTesting {} with {} operation (HTTP)", name, operation);

    let request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        id: json!(format!("{}-{}-{}", name, operation, rand::thread_rng().gen::<u16>())),
        method: "tools/call".to_string(),
        params: Some(json!({ "name": name, "arguments": arguments })),
    };
    println!("Request Params: {}", serde_json::to_string_pretty(request.params.as_ref().unwrap()).unwrap_or_default());

    let request_str = serde_json::to_string(&request).map_err(|e| format!("Serialize tool call req err: {}", e))?;

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    if let Some(sid) = session_id { headers.insert(MCP_SESSION_ID_HEADER, HeaderValue::from_str(sid)?); }

    let response = client.post(mcp_url).headers(headers).body(request_str).send().await.map_err(|e| format!("POST /mcp tool call failed: {}", e))?;

    if !response.status().is_success() { return Err(format!("Tool call POST failed status: {}", response.status()).into()); }

    // TODO: Handle potential SSE stream response from POST
    let content_type = response.headers().get(CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or("");
    if content_type.starts_with("application/json") {
        let response_body = response.json::<MCPResponse>().await.map_err(|e| format!("Parse tool call resp err: {}", e))?;
         match response_body {
             MCPResponse { error: Some(e), .. } => println!("  Tool call error: {} ({})", e.message, e.code),
             MCPResponse { result: Some(r), .. } => { match serde_json::from_value::<CallToolResult>(r.clone()) { Ok(tr) => { println!("  Result{}", if tr.is_error { " (ERROR)" } else { "" }); for c in tr.content { match c { ContentItem::Text{text,..}=>{if let Ok(j)=serde_json::from_str::<Value>(&text){println!("  {}",serde_json::to_string_pretty(&j).unwrap_or(text));}else{println!("  {}",text);}}, ContentItem::Audio{data,mime_type,..}=>println!("  Audio: {}b, {}",data.len(),mime_type), ContentItem::Image{data,mime_type,..}=>println!("  Image: {}b, {}",data.len(),mime_type), ContentItem::Resource{resource,..}=>{match resource{ResourceContentUnion::Text(tc)=>println!("  Resource (Text): {}",tc.uri),ResourceContentUnion::Blob(bc)=>println!("  Resource (Blob): {}",bc.uri),}}, } } } Err(e) => println!("  Err parsing ToolCallResult: {}\nRaw: {}", e, serde_json::to_string_pretty(&r).unwrap_or_default()), } }
             _ => println!("  Invalid tool call response format"),
         }
    } else if content_type.starts_with("text/event-stream") { println!("  Received SSE stream response from POST (Handling not implemented)."); }
    else { println!("  Received unexpected Content-Type from POST: {}", content_type); }

    Ok(())
}