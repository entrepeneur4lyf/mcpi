// mcpi-client/src/main.rs
use futures::{SinkExt, StreamExt, TryStreamExt}; // Added TryStreamExt
use mcpi_common::{
    DiscoveryResponse, InitializeResult, MCPRequest, MCPResponse, CallToolResult,
    MCPI_VERSION, LATEST_MCP_VERSION,
    ContentItem,
    ResourceContentUnion, TextResourceContents, BlobResourceContents,
    Provider, Referral, CapabilityDescription, InitializeParams,
    ListResourcesResult, ListToolsResult,
};
use serde_json::{json, Value};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use std::error::Error;
use std::sync::{Arc, RwLock}; // Use std::sync::RwLock
use clap::{Parser, Subcommand};
use reqwest::{header::{HeaderMap, HeaderValue, CONTENT_TYPE, ACCEPT, HeaderName}, Client as ReqwestClient, Error as ReqwestError};
use rand::Rng;
use tokio::io::AsyncBufReadExt;
use tokio_util::io::StreamReader;
use bytes::Bytes;
use tracing::warn; // Use tracing::warn

mod discovery;

static MCP_SESSION_ID_HEADER: HeaderName = HeaderName::from_static("mcp-session-id");


#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    domain: Option<String>,
    #[arg(short = 'u', long)]
    base_url: Option<String>,
    #[arg(short, long)]
    plugin: Option<String>,
}

#[derive(Subcommand)]
enum Commands { Discover { domain: String } }

type BoxedError = Box<dyn Error + Send + Sync>;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Protocol { McpiWebSocket, McpHttp }

#[tokio::main]
async fn main() -> Result<(), BoxedError> {
    // tracing_subscriber::fmt::init(); // Initialize if using tracing macros
    let cli = Cli::parse();
    let mut protocol = Protocol::McpHttp;
    let mut discovery_url = String::from("http://localhost:3001/mcpi/discover");
    let mut service_base_url = String::from("http://localhost:3001");

    // Determine protocol and base URL
    if let Some(domain) = &cli.domain {
        println!("Performing DNS-based discovery for domain: {}", domain);
        match discovery::discover_mcp_services(domain).await {
            Ok(service_info) => {
                protocol = Protocol::McpiWebSocket;
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
            protocol = Protocol::McpHttp;
            println!("Using provided base URL: {}", base_url_arg);
            service_base_url = base_url_arg.trim_end_matches('/').to_string();
            discovery_url = format!("{}/mcpi/discover", service_base_url);
            println!("  Protocol: MCP (HTTP)");
            println!("  Derived Discovery URL: {}", discovery_url);
            println!("  Derived MCP Service Base URL: {}", service_base_url);
        } else if cli.domain.is_none() {
             println!("Using default MCP (HTTP) on localhost.");
        }
    }

    println!("\nDiscovering service capabilities via HTTP discovery endpoint...");
    let discovery_resp = discover_service_http(&discovery_url).await?;
    println!("Provider: {} ({})", discovery_resp.provider.name, discovery_resp.provider.domain);
    println!("Mode: {}", discovery_resp.mode);
    println!("\nAvailable capabilities (from discovery):");
    for cap in &discovery_resp.capabilities { println!("  - {}: {}", cap.name, cap.description); println!("    Ops: {}", cap.operations.join(", ")); }
    println!("\nReferrals (from discovery):");
    for ref_info in &discovery_resp.referrals { println!("  - {}: {}", ref_info.name, ref_info.domain); }

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

// --- Streamable HTTP Client Implementation (MCP) ---
async fn run_mcp_http_client(
    mcp_url: String,
    specific_plugin: Option<String>,
    _discovery_resp: DiscoveryResponse
) -> Result<(), BoxedError> {

    let http_client = ReqwestClient::new();
    let mut session_id: Option<String> = None;
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    println!("\nEstablishing SSE connection via GET {}...", mcp_url);
    let get_response = http_client.get(&mcp_url).header(ACCEPT, "text/event-stream").send().await?;

    if !get_response.status().is_success() { return Err(format!("GET /mcp failed status: {}", get_response.status()).into()); }
    if let Some(ct) = get_response.headers().get(CONTENT_TYPE) { if !ct.to_str()?.starts_with("text/event-stream") { return Err(format!("Expected text/event-stream, got: {:?}", ct).into()); } } else { return Err("Missing Content-Type on GET /mcp response".into()); }

    // FIX E0507: Pass static HeaderName by reference to .get()
    if let Some(sid_value) = get_response.headers().get(&MCP_SESSION_ID_HEADER) {
        let sid = sid_value.to_str()?.to_string();
        println!("Obtained MCP session ID: {}", sid);
        session_id = Some(sid.clone());
        // FIX E0507: Pass static HeaderName directly to .insert()
        headers.insert(MCP_SESSION_ID_HEADER.clone(), HeaderValue::from_str(&sid)?);
    } else { println!("Warning: Server did not provide mcp-session-id header."); }
    println!("SSE stream connected.");

    let body_stream = get_response.bytes_stream().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e));
    let stream_reader = StreamReader::new(body_stream);
    let mut lines = stream_reader.lines();

    let last_event_id = Arc::new(RwLock::new(None::<String>));
    let last_event_id_clone = last_event_id.clone();

    tokio::spawn(async move {
        println!("SSE Listener Task Started.");
        let mut current_event_type = String::new();
        let mut current_data = String::new();

        while let Ok(Some(line)) = lines.next_line().await {
            if line.is_empty() {
                if !current_data.is_empty() {
                    let event_type = if current_event_type.is_empty() { "message".to_string() } else { current_event_type.clone() };
                    let data = current_data.trim_end_matches('\n');
                    let id = { last_event_id_clone.read().unwrap().clone() };

                    println!("\n--- SSE Event Received ---");
                    println!("Event: {}", event_type);
                    if let Some(id_val) = &id { println!("ID: {}", id_val); }
                    println!("Data: {}", data);
                    match serde_json::from_str::<Value>(data) { Ok(json_data) => { println!("Parsed Data: {}", serde_json::to_string_pretty(&json_data).unwrap_or_default()); if event_type == "message" { if let Ok(mcp_notif) = serde_json::from_value::<Value>(json_data) { if mcp_notif.get("method").and_then(|m|m.as_str()) == Some("notifications/progress") { println!("  -> Progress Update: {:?}", mcp_notif.get("params")); } else { println!("  -> Other MCP Notification/Request: {:?}", mcp_notif); } } } } Err(e) => { warn!("SSE data was not valid JSON: {}", e); } }
                    println!("--- End SSE Event ---");
                }
                current_data.clear(); current_event_type.clear();
            } else if let Some(data) = line.strip_prefix("data:") { current_data.push_str(data.trim_start()); current_data.push('\n'); }
            else if let Some(event) = line.strip_prefix("event:") { current_event_type = event.trim().to_string(); }
            else if let Some(id) = line.strip_prefix("id:") { let id_str = id.trim().to_string(); if !id_str.is_empty() { *last_event_id_clone.write().unwrap() = Some(id_str); } }
        }
        println!("SSE Listener Task Ended.");
    });

    println!("\nSending initialize request via POST {}...", mcp_url); let init_params = InitializeParams { client_info: mcpi_common::Implementation { name: "MCP Test Client".to_string(), version: "0.1.0".to_string() }, protocol_version: LATEST_MCP_VERSION.to_string(), capabilities: Default::default() }; let init_request = MCPRequest { jsonrpc: "2.0".to_string(), id: json!(1), method: "initialize".to_string(), params: Some(serde_json::to_value(init_params)?) }; let init_req_str = serde_json::to_string(&init_request)?; let init_resp = http_client.post(&mcp_url).headers(headers.clone()).body(init_req_str).send().await?; if !init_resp.status().is_success() { return Err(format!("Initialize POST failed status: {}", init_resp.status()).into()); } let init_resp_body = init_resp.json::<MCPResponse>().await?; if let Some(err) = init_resp_body.error { println!("Init error: {} ({})", err.message, err.code); return Err("Init failed".into()); } if let Some(res) = init_resp_body.result { let init_res: InitializeResult = serde_json::from_value(res)?; println!("\nMCP initialized: Server: {} v{}, Proto: v{}", init_res.server_info.name, init_res.server_info.version, init_res.protocol_version); if let Some(inst) = init_res.instructions { println!("  Instructions: {}", inst); } } else { return Err("Invalid init response".into()); }
    println!("\nListing Resources via POST {}...", mcp_url); let list_res_req = MCPRequest { jsonrpc: "2.0".to_string(), id: json!(2), method: "resources/list".to_string(), params: None }; let list_res_str = serde_json::to_string(&list_res_req)?; let list_res_resp = http_client.post(&mcp_url).headers(headers.clone()).body(list_res_str).send().await?.json::<MCPResponse>().await?; let mut _resources_list: Vec<Value> = Vec::new(); if let Some(e) = list_res_resp.error { println!("Err list res: {} ({})", e.message, e.code); } else if let Some(r) = list_res_resp.result { match serde_json::from_value::<ListResourcesResult>(r) { Ok(list_result) => { println!("\nAvailable MCP resources:"); _resources_list = list_result.resources.iter().map(|res| serde_json::to_value(res).unwrap_or_default()).collect(); if list_result.resources.is_empty() { println!("  (No resources)"); } else { for item in list_result.resources { println!("  - {} ({})", item.name, item.uri); if let Some(d)=item.description{println!("    Desc: {}",d);} } } if list_result.next_cursor.is_some() { println!("  (More resources available...)"); } } Err(e) => println!("Warn: Failed to parse ListResourcesResult: {}", e), } } else { println!("Warn: Invalid list res resp (no result/error)"); }
    println!("\nListing Tools via POST {}...", mcp_url); let list_tools_req = MCPRequest { jsonrpc: "2.0".to_string(), id: json!(3), method: "tools/list".to_string(), params: None }; let list_tools_str = serde_json::to_string(&list_tools_req)?; let list_tools_resp = http_client.post(&mcp_url).headers(headers.clone()).body(list_tools_str).send().await?.json::<MCPResponse>().await?; let mut tools: Vec<String> = Vec::new(); let mut tools_info: Vec<Value> = Vec::new(); if let Some(e) = list_tools_resp.error { println!("Err list tools: {} ({})", e.message, e.code); } else if let Some(r) = list_tools_resp.result { match serde_json::from_value::<ListToolsResult>(r) { Ok(list_result) => { println!("\nAvailable MCP tools:"); if list_result.tools.is_empty() { println!("  (No tools)"); } else { for tool in list_result.tools { tools.push(tool.name.clone()); if let Ok(tool_value) = serde_json::to_value(&tool) { tools_info.push(tool_value); } println!("  - {}", tool.name); if let Some(d)=&tool.description{println!("    Desc: {}",d);} if let Some(a)=&tool.annotations{println!("    Anno: {}",serde_json::to_string_pretty(a).unwrap_or_default());} if let Some(s)=tool.input_schema.get("properties").and_then(|p|p.get("operation")).and_then(|o|o.get("enum")).and_then(|e|e.as_array()){let ops_str:Vec<String>=s.iter().filter_map(|o|o.as_str().map(String::from)).collect();println!("    Ops: {}",ops_str.join(", "));}else{println!("    Ops: (N/A)");} } } if list_result.next_cursor.is_some() { println!("  (More tools available...)"); } } Err(e) => println!("Warn: Failed to parse ListToolsResult: {}", e), } } else { println!("Warn: Invalid list tools resp (no result/error)"); }
    println!("\nTesting Batch via POST {}...", mcp_url); let batch_req_data=json!([{ "jsonrpc": "2.0", "id": 10, "method": "ping", "params": null }, { "jsonrpc": "2.0", "id": 11, "method": "resources/list", "params": null }]); let batch_req_str=serde_json::to_string(&batch_req_data)?; let batch_resp=http_client.post(&mcp_url).headers(headers.clone()).body(batch_req_str).send().await?.json::<Vec<MCPResponse>>().await?; println!("Batch response ({} items):", batch_resp.len()); for (i, r) in batch_resp.iter().enumerate() { println!("  Item {}: ID={}", i+1, r.id); if let Some(err)=&r.error{println!("    Err: {} ({})", err.message, err.code);} }
    let tools_to_test = if let Some(p_name)=specific_plugin{if tools.contains(&p_name){vec![p_name]}else{println!("\nTool '{}' unavailable. Available: {}",p_name,tools.join(", "));return Err("Tool not found".into());}}else{tools};
    for tool_name in tools_to_test { println!("\n=== Testing tool (HTTP): {} ===", tool_name); let tool_info=tools_info.iter().find(|t|t.get("name").and_then(|n|n.as_str())==Some(&tool_name)).cloned().unwrap_or_default(); let default_schema=json!({}); let tool_schema=tool_info.get("inputSchema").unwrap_or(&default_schema); let operations=tool_schema.get("properties").and_then(|p|p.get("operation")).and_then(|o|o.get("enum")).and_then(|e|e.as_array()).map(|ops|ops.iter().filter_map(|op|op.as_str().map(String::from)).collect::<Vec<String>>()).unwrap_or_else(||vec!["SEARCH".to_string()]); println!("Supported operations: {:?}",operations); for operation in operations{let args=generate_test_arguments(&tool_name,&operation,tool_schema); call_tool_http(&http_client, &mcp_url, session_id.as_deref(), &tool_name, &operation, args).await?;}}
    println!("\nSending ping via POST {}...", mcp_url); let ping_req=MCPRequest{jsonrpc:"2.0".to_string(),id:json!(99),method:"ping".to_string(),params:None}; let ping_req_str=serde_json::to_string(&ping_req)?; let ping_resp=http_client.post(&mcp_url).headers(headers).body(ping_req_str).send().await?.json::<MCPResponse>().await?; match ping_resp { MCPResponse{error: Some(err),..} => println!("\nPing error: {} ({})",err.message, err.code), MCPResponse{result: Some(_),..} => println!("\nPing successful."), _ => println!("\nInvalid ping response"), }
    println!("\n(MCP HTTP Client finished - Main task ending, SSE listener may continue briefly)"); tokio::time::sleep(tokio::time::Duration::from_secs(2)).await; Ok(())
}

// --- WebSocket Client Implementation (MCPI) ---
async fn run_mcpi_websocket_client( ws_url: String, specific_plugin: Option<String>, _discovery_resp: DiscoveryResponse ) -> Result<(), BoxedError> { let (ws_stream, _) = connect_async(&ws_url).await.map_err(|e| format!("WS connection failed: {}", e))?; println!("WebSocket connection established."); let (mut write, mut read) = ws_stream.split(); let init_request = MCPRequest { jsonrpc: "2.0".to_string(), id: json!(1), method: "initialize".to_string(), params: Some(json!({ "clientInfo": { "name": "MCPI Test Client", "version": "0.1.0" }, "protocolVersion": mcpi_common::MCPI_VERSION, "capabilities": {} })) }; let init_req_str = serde_json::to_string(&init_request)?; write.send(Message::Text(init_req_str.into())).await?; if let Some(Ok(Message::Text(resp_str))) = read.next().await { let parsed: MCPResponse = serde_json::from_str(&resp_str)?; if let Some(err) = parsed.error { println!("Init error: {} ({})", err.message, err.code); return Err("Init failed".into()); } if let Some(res) = parsed.result { let init_res: InitializeResult = serde_json::from_value(res)?; println!("\nMCPI initialized: Server: {} v{}, Proto: v{}", init_res.server_info.name, init_res.server_info.version, init_res.protocol_version); if let Some(inst) = init_res.instructions { println!("  Instructions: {}", inst); } } else { return Err("Invalid init response".into()); } } else { return Err("No init response".into()); } let list_res_req = MCPRequest { jsonrpc: "2.0".to_string(), id: json!(2), method: "resources/list".to_string(), params: None }; let list_res_str = serde_json::to_string(&list_res_req)?; write.send(Message::Text(list_res_str.into())).await?; if let Some(Ok(Message::Text(resp_str))) = read.next().await { let parsed: MCPResponse = serde_json::from_str(&resp_str)?; if let Some(e)=parsed.error{println!("Err list res: {} ({})",e.message,e.code);}else if let Some(r)=parsed.result{println!("\nAvailable MCPI resources:");if let Some(res)=r.get("resources").and_then(|r|r.as_array()){for item in res{println!("  - {} ({})", item.get("name").and_then(|n|n.as_str()).unwrap_or("?"), item.get("uri").and_then(|u|u.as_str()).unwrap_or("?")); if let Some(d)=item.get("description").and_then(|d|d.as_str()){println!("    Desc: {}",d);}}}else{println!(" (No resources)");}}else{println!("Warn: Invalid list res resp");}} else { println!("Warn: No list res resp"); } let list_tools_req = MCPRequest { jsonrpc: "2.0".to_string(), id: json!(3), method: "tools/list".to_string(), params: None }; let list_tools_str = serde_json::to_string(&list_tools_req)?; write.send(Message::Text(list_tools_str.into())).await?; let mut tools = Vec::new(); let mut tools_info = Vec::new(); if let Some(Ok(Message::Text(resp_str))) = read.next().await { let parsed: MCPResponse = serde_json::from_str(&resp_str)?; if let Some(e)=parsed.error{println!("Err list tools: {} ({})",e.message,e.code);}else if let Some(r)=parsed.result{println!("\nAvailable MCPI tools:");if let Some(ts)=r.get("tools").and_then(|t|t.as_array()){for tool in ts{let name=tool.get("name").and_then(|n|n.as_str()).unwrap_or("?").to_string();tools.push(name.clone());tools_info.push(tool.clone());println!("  - {}",name);if let Some(d)=tool.get("description").and_then(|d|d.as_str()){println!("    Desc: {}",d);}if let Some(a)=tool.get("annotations"){println!("    Anno: {}",serde_json::to_string_pretty(a).unwrap_or_default());} if let Some(s)=tool.get("inputSchema"){println!("    Ops:");if let Some(ops)=s.get("properties").and_then(|p|p.get("operation")).and_then(|o|o.get("enum")).and_then(|e|e.as_array()){let ops_str:Vec<String>=ops.iter().filter_map(|o|o.as_str().map(String::from)).collect();println!("      {}",ops_str.join(", "));}else{println!("      (N/A)");}}}}else{println!(" (No tools)");}}else{println!("Warn: Invalid list tools resp");}} else {println!("Warn: No list tools resp");} let tools_to_test = if let Some(p_name)=specific_plugin{if tools.contains(&p_name){vec![p_name]}else{println!("\nTool '{}' unavailable. Available: {}",p_name,tools.join(", "));return Err("Tool not found".into());}}else{tools}; println!("\nTesting JSON-RPC batch request..."); let batch_req_data=json!([{ "jsonrpc": "2.0", "id": 10, "method": "ping", "params": null }, { "jsonrpc": "2.0", "id": 11, "method": "resources/list", "params": null }]); let batch_req_str=serde_json::to_string(&batch_req_data)?; write.send(Message::Text(batch_req_str.into())).await?; if let Some(Ok(Message::Text(resp_str)))=read.next().await{if resp_str.trim().starts_with('['){match serde_json::from_str::<Vec<MCPResponse>>(&resp_str){Ok(br)=>{println!("Batch response ({} items):",br.len());for(i,r)in br.iter().enumerate(){println!("  Item {}: ID={}",i+1,r.id);if let Some(err)=&r.error{println!("    Err: {} ({})",err.message,err.code);}}}Err(e)=>println!("Err parsing batch resp: {}",e),}}else{println!("Warn: Expected batch array, got: {}",resp_str);}}else{println!("Warn: No batch resp");} for tool_name in tools_to_test { println!("\n=== Testing tool (WebSocket): {} ===", tool_name); let tool_info=tools_info.iter().find(|t|t.get("name").and_then(|n|n.as_str())==Some(&tool_name)).cloned().unwrap_or_default(); let default_schema=json!({}); let tool_schema=tool_info.get("inputSchema").unwrap_or(&default_schema); let operations=tool_schema.get("properties").and_then(|p|p.get("operation")).and_then(|o|o.get("enum")).and_then(|e|e.as_array()).map(|ops|ops.iter().filter_map(|op|op.as_str().map(String::from)).collect::<Vec<String>>()).unwrap_or_else(||vec!["SEARCH".to_string()]); println!("Supported operations: {:?}",operations); if tool_name=="weather_forecast"&&operations.contains(&"GET_AUDIO".to_string()){/* ... test audio ... */} if tool_name=="weather_forecast"{/* ... test completions ... */} for operation in operations{if tool_name=="weather_forecast"&&operation=="GET_AUDIO"{continue;} let args=generate_test_arguments(&tool_name,&operation,tool_schema); call_tool_ws(&mut write,&mut read,&tool_name,&operation,args).await?;}} let ping_req = MCPRequest { jsonrpc: "2.0".to_string(), id: json!(99), method: "ping".to_string(), params: None }; let ping_req_str=serde_json::to_string(&ping_req)?; write.send(Message::Text(ping_req_str.into())).await?; if let Some(Ok(Message::Text(resp_str)))=read.next().await{match serde_json::from_str::<MCPResponse>(&resp_str){Ok(p)=>{if let Some(err)=p.error{println!("\nPing error: {} ({})",err.message,err.code);}else if p.result.is_some(){println!("\nPing successful.");}else{println!("\nInvalid ping resp");}},Err(e)=>println!("\nErr parsing ping resp: {}",e),}}else{println!("\nNo ping response");} println!("\nClosing MCPI connection"); Ok(()) }

// --- Helper Functions ---
async fn discover_service_http(url: &str) -> Result<DiscoveryResponse, BoxedError> { let client=reqwest::Client::new(); Ok(client.get(url).send().await?.json::<DiscoveryResponse>().await?) }
fn generate_test_arguments(_tool_name: &str, operation: &str, schema: &Value) -> Value { let mut args=json!({"operation": operation}); if let Some(props)=schema.get("properties").and_then(|p|p.as_object()){for(p_name,p_schema)in props.iter().filter(|(k,_)|*k!="operation"){let desc=p_schema.get("description").and_then(|d|d.as_str()).unwrap_or("");let p_type=p_schema.get("type").and_then(|t|t.as_str()).unwrap_or("string");let t_val=match(p_name.as_str(),p_type,operation){("id",_,_)if desc.contains("ID")=>json!("test-id-123"),("query",_,_)=>json!("search query"),("location",_,_)=>json!("London"),("domain",_,_)=>json!("target.example.com"),("relationship",_,_)=>json!("affiliate"),(_,"string",_)=>json!("default str"),(_,"number",_)|(_,"integer",_)=>json!(42),(_,"boolean",_)=>json!(false),_=>Value::Null,};if !t_val.is_null(){if let Some(obj)=args.as_object_mut(){obj.insert(p_name.clone(),t_val);}}}} args }
async fn get_completions<S, R>( write: &mut S, read: &mut R, method: &str, param_name: &str, partial_val: &str, context: Option<Value> ) -> Result<Vec<String>, BoxedError> where S: SinkExt<Message, Error=tokio_tungstenite::tungstenite::Error>+Unpin, R: StreamExt<Item=Result<Message, tokio_tungstenite::tungstenite::Error>>+Unpin { let mut params=json!({"method":method,"parameterName":param_name,"partialValue":partial_val}); if let Some(ctx_obj)=context.as_ref().and_then(|c|c.as_object()){if let Some(p_obj)=params.as_object_mut(){for(k,v)in ctx_obj{p_obj.insert(format!("context.{}",k),v.clone());}}}else if let Some(ctx_val)=context{if let Some(p_obj)=params.as_object_mut(){p_obj.insert("context".to_string(),ctx_val);}} let req = MCPRequest { jsonrpc:"2.0".to_string(), id:json!(98), method:"completions".to_string(), params:Some(params) }; let req_str = serde_json::to_string(&req)?; write.send(Message::Text(req_str.into())).await?; if let Some(Ok(Message::Text(resp_str)))=read.next().await{match serde_json::from_str::<MCPResponse>(&resp_str){Ok(p)=>{if let Some(e)=p.error{println!("Completions error: {} ({})",e.message,e.code);Ok(vec![])}else if let Some(r)=p.result{Ok(r.get("suggestions").and_then(|s|s.as_array()).map(|a|a.iter().filter_map(|v|v.as_str().map(String::from)).collect()).unwrap_or_default())}else{println!("Invalid completions resp");Ok(vec![])}}Err(e)=>Err(format!("Parse completions resp err: {}",e).into()),}}else{Err("No completions response".into())} }
async fn call_tool_ws<S, R>( write: &mut S, read: &mut R, name: &str, operation: &str, arguments: Value ) -> Result<(), BoxedError> where S: SinkExt<Message, Error=tokio_tungstenite::tungstenite::Error>+Unpin, R: StreamExt<Item=Result<Message, tokio_tungstenite::tungstenite::Error>>+Unpin { println!("\nTesting {} with {} operation (WebSocket)",name,operation); let req=MCPRequest{jsonrpc:"2.0".to_string(),id:json!(format!("{}-{}-{}",name,operation,rand::thread_rng().gen::<u16>())),method:"tools/call".to_string(),params:Some(json!({"name":name,"arguments":arguments}))}; println!("Request Params: {}", serde_json::to_string_pretty(req.params.as_ref().unwrap()).unwrap_or_default()); let req_str=serde_json::to_string(&req)?; write.send(Message::Text(req_str.into())).await?; if let Some(Ok(Message::Text(resp_str)))=read.next().await{match serde_json::from_str::<MCPResponse>(&resp_str){Ok(p)=>{if let Some(e)=p.error{println!("  Tool call error: {} ({})",e.message,e.code);}else if let Some(r)=p.result{match serde_json::from_value::<CallToolResult>(r.clone()){Ok(tr)=>{println!("  Result{}",if tr.is_error{" (ERROR)"}else{""});for c in tr.content{match c{ ContentItem::Text{text,..}=>{if let Ok(j)=serde_json::from_str::<Value>(&text){println!("  {}",serde_json::to_string_pretty(&j).unwrap_or(text));}else{println!("  {}",text);}}, ContentItem::Audio{data,mime_type,..}=>println!("  Audio: {}b, {}",data.len(),mime_type), ContentItem::Image{data,mime_type,..}=>println!("  Image: {}b, {}",data.len(),mime_type), ContentItem::Resource{resource,..}=>{match resource{ResourceContentUnion::Text(tc)=>println!("  Resource (Text): {}",tc.uri),ResourceContentUnion::Blob(bc)=>println!("  Resource (Blob): {}",bc.uri),}},}}}Err(e)=>println!("  Err parsing ToolCallResult: {}\nRaw: {}",e,serde_json::to_string_pretty(&r).unwrap_or_default()),}}else{println!("  Invalid tool call response");}}Err(e)=>println!("Err parsing tool call resp: {}",e),}}else{println!("Warn: No tool call resp");} Ok(()) }
async fn call_tool_http( client: &ReqwestClient, mcp_url: &str, session_id: Option<&str>, name: &str, operation: &str, arguments: Value ) -> Result<(), BoxedError> { println!("\nTesting {} with {} operation (HTTP)", name, operation); let req=MCPRequest{jsonrpc:"2.0".to_string(),id:json!(format!("{}-{}-{}",name,operation,rand::thread_rng().gen::<u16>())),method:"tools/call".to_string(),params:Some(json!({"name":name,"arguments":arguments}))}; println!("Request Params: {}", serde_json::to_string_pretty(req.params.as_ref().unwrap()).unwrap_or_default()); let req_str=serde_json::to_string(&req)?; let mut headers = HeaderMap::new(); headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json")); if let Some(sid)=session_id{headers.insert(MCP_SESSION_ID_HEADER.clone(), HeaderValue::from_str(sid)?);} let response = client.post(mcp_url).headers(headers).body(req_str).send().await?; if !response.status().is_success() { return Err(format!("Tool call POST failed status: {}", response.status()).into()); } let content_type = response.headers().get(CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or(""); if content_type.starts_with("application/json") { let resp_body = response.json::<MCPResponse>().await?; match resp_body { MCPResponse { error: Some(e), .. } => println!("  Tool call error: {} ({})", e.message, e.code), MCPResponse { result: Some(r), .. } => { match serde_json::from_value::<CallToolResult>(r.clone()) { Ok(tr) => { println!("  Result{}", if tr.is_error { " (ERROR)" } else { "" }); for c in tr.content { match c { ContentItem::Text{text,..}=>{if let Ok(j)=serde_json::from_str::<Value>(&text){println!("  {}",serde_json::to_string_pretty(&j).unwrap_or(text));}else{println!("  {}",text);}}, ContentItem::Audio{data,mime_type,..}=>println!("  Audio: {}b, {}",data.len(),mime_type), ContentItem::Image{data,mime_type,..}=>println!("  Image: {}b, {}",data.len(),mime_type), ContentItem::Resource{resource,..}=>{match resource{ResourceContentUnion::Text(tc)=>println!("  Resource (Text): {}",tc.uri),ResourceContentUnion::Blob(bc)=>println!("  Resource (Blob): {}",bc.uri),}}, } } } Err(e) => println!("  Err parsing ToolCallResult: {}\nRaw: {}", e, serde_json::to_string_pretty(&r).unwrap_or_default()), } } _ => println!("  Invalid tool call response format"), } } else if content_type.starts_with("text/event-stream") { println!("  Received SSE stream response from POST (Handling not implemented)."); } else { println!("  Received unexpected Content-Type from POST: {}", content_type); } Ok(()) }