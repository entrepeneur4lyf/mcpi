// mcpi-client/src/main.rs
use futures::{SinkExt, StreamExt};
use mcpi_common::{
    DiscoveryResponse, InitializeResult, MCPRequest, MCPResponse, ToolCallResult,
    MCPI_VERSION, // Import MCPI_VERSION
    ContentItem, // Needed for call_tool result parsing
    // Removed unused imports
};
use serde_json::{json, Value};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use std::error::Error;
use clap::{Parser, Subcommand};
use reqwest; // Import reqwest
use rand::Rng; // Import Rng trait

mod discovery;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Domain to discover MCPI services from (uses DNS TXT records)
    #[arg(short, long)]
    domain: Option<String>,

    /// Direct base URL to server (e.g., http://localhost:3001), paths will be added
    #[arg(short, long)]
    base_url: Option<String>,

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
    // Removed Connect subcommand as we now derive URLs
}

// Define a consistent error type
type BoxedError = Box<dyn Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<(), BoxedError> {
    let cli = Cli::parse();

    // Determine discovery and WebSocket URLs
    let (discovery_url, ws_url) = if let Some(domain) = cli.domain {
        println!("Performing DNS-based discovery for domain: {}", domain);
        match discovery::discover_mcp_services(&domain).await {
            Ok(service_info) => {
                println!("Discovered MCP service:");
                println!("  Version: {}", service_info.version);
                println!("  Endpoint: {}", service_info.endpoint); // Discovery endpoint
                let discovery_ep = service_info.endpoint;
                let base = discovery_ep.replace("/mcpi/discover", "");
                let ws_derived = if base.starts_with("https://") {
                    base.replace("https://", "wss://") + "/mcpi" // Use /mcpi for WS
                } else {
                    base.replace("http://", "ws://") + "/mcpi" // Use /mcpi for WS
                };
                println!("Derived WebSocket URL: {}", ws_derived);
                (discovery_ep, ws_derived)
            },
            Err(e) => {
                println!("DNS discovery failed: {}.", e);
                return Err(format!("DNS discovery failed for domain '{}': {}", domain, e).into());
            }
        }
    } else if let Some(base_url) = cli.base_url {
        let base = base_url.trim_end_matches('/');
        let discovery = format!("{}/mcpi/discover", base);
        let websocket = if base.starts_with("https://") {
            format!("wss://{}/mcpi", base.trim_start_matches("https://")) // Use /mcpi for WS
        } else {
            format!("ws://{}/mcpi", base.trim_start_matches("http://")) // Use /mcpi for WS
        };
        println!("Using provided base URL: {}", base_url);
        println!("Derived Discovery URL: {}", discovery);
        println!("Derived WebSocket URL: {}", websocket);
        (discovery, websocket)
    } else {
        let discovery = String::from("http://localhost:3001/mcpi/discover");
        let websocket = String::from("ws://localhost:3001/mcpi"); // Default WS path /mcpi
        println!("No domain or base URL provided. Using default localhost URLs:");
        println!("Discovery: {}", discovery);
        println!("WebSocket: {}", websocket);
        (discovery, websocket)
    };

    // --- HTTP Discovery ---
    println!("\nDiscovering service capabilities via HTTP...");
    let discovery_resp = discover_service_http(&discovery_url).await?;
    println!("Connected to Provider: {} ({})", discovery_resp.provider.name, discovery_resp.provider.domain);
    println!("Mode: {}", discovery_resp.mode);
    println!("\nAvailable capabilities (from discovery):");
    for cap in &discovery_resp.capabilities { println!("  - {}: {}", cap.name, cap.description); println!("    Ops: {}", cap.operations.join(", ")); }
    println!("\nReferrals (from discovery):");
    for ref_info in &discovery_resp.referrals { println!("  - {}: {}", ref_info.name, ref_info.domain); }

    // --- WebSocket Connection (MCPI) ---
    println!("\nConnecting via WebSocket (MCPI)...");
    println!("Connecting to: {}", ws_url);
    let (ws_stream, _) = connect_async(&ws_url).await.map_err(|e| format!("WS connection failed: {}", e))?;
    println!("WebSocket connection established.");
    let (mut write, mut read) = ws_stream.split();

    // --- Initialize (MCPI) ---
    let init_request = MCPRequest {
        jsonrpc: "2.0".to_string(), id: json!(1), method: "initialize".to_string(),
        params: Some(json!({
            "clientInfo": { "name": "MCPI Test Client", "version": "0.1.0" },
            "protocolVersion": mcpi_common::MCPI_VERSION, // Correct version for MCPI/WS
            "capabilities": { /* Client caps */ }
        })),
    };
    let init_req_str = serde_json::to_string(&init_request).map_err(|e| format!("Serialize init err: {}", e))?;
    write.send(Message::Text(init_req_str.into())).await.map_err(|e| format!("WS send err (init): {}", e))?;

    if let Some(Ok(Message::Text(resp_str))) = read.next().await {
        let parsed: MCPResponse = serde_json::from_str(&resp_str).map_err(|e| format!("Parse init resp err: {}", e))?;
        if let Some(err) = parsed.error { println!("Init error: {} ({})", err.message, err.code); return Err("Init failed".into()); }
        if let Some(res) = parsed.result {
            let init_res: InitializeResult = serde_json::from_value(res).map_err(|e| format!("Parse init result err: {}", e))?;
            println!("\nMCPI connection initialized: Server: {} v{}, Protocol: v{}", init_res.server_info.name, init_res.server_info.version, init_res.protocol_version);
            if let Some(inst) = init_res.instructions { println!("  Instructions: {}", inst); }
        } else { return Err("Invalid init response".into()); }
    } else { return Err("No init response".into()); }

    // --- List Resources (MCPI) ---
    let list_res_req = MCPRequest { jsonrpc: "2.0".to_string(), id: json!(2), method: "resources/list".to_string(), params: None };
    let list_res_str = serde_json::to_string(&list_res_req).map_err(|e| format!("Serialize list res err: {}", e))?;
    write.send(Message::Text(list_res_str.into())).await.map_err(|e| format!("WS send err (list res): {}", e))?;

    if let Some(Ok(Message::Text(resp_str))) = read.next().await {
        let parsed: MCPResponse = serde_json::from_str(&resp_str).map_err(|e| format!("Parse list res resp err: {}", e))?;
        if let Some(e) = parsed.error { println!("Err list res: {} ({})", e.message, e.code); }
        else if let Some(r) = parsed.result { println!("\nAvailable MCPI resources:"); if let Some(res)=r.get("resources").and_then(|r|r.as_array()){for item in res{println!("  - {} ({})", item.get("name").and_then(|n|n.as_str()).unwrap_or("?"), item.get("uri").and_then(|u|u.as_str()).unwrap_or("?")); if let Some(d)=item.get("description").and_then(|d|d.as_str()){println!("    Desc: {}",d);}}}else{println!(" (No resources)");}}
        else { println!("Warn: Invalid list res resp"); }
    } else { println!("Warn: No list res resp"); }

    // --- List Tools (MCPI) ---
    let list_tools_req = MCPRequest { jsonrpc: "2.0".to_string(), id: json!(3), method: "tools/list".to_string(), params: None };
    let list_tools_str = serde_json::to_string(&list_tools_req).map_err(|e| format!("Serialize list tools err: {}", e))?;
    write.send(Message::Text(list_tools_str.into())).await.map_err(|e| format!("WS send err (list tools): {}", e))?;

    let mut tools = Vec::new(); let mut tools_info = Vec::new();
    if let Some(Ok(Message::Text(resp_str))) = read.next().await {
        let parsed: MCPResponse = serde_json::from_str(&resp_str).map_err(|e| format!("Parse list tools resp err: {}", e))?;
        if let Some(e)=parsed.error{println!("Err list tools: {} ({})",e.message,e.code);}
        else if let Some(r)=parsed.result{println!("\nAvailable MCPI tools:");if let Some(ts)=r.get("tools").and_then(|t|t.as_array()){for tool in ts{let name=tool.get("name").and_then(|n|n.as_str()).unwrap_or("?").to_string();tools.push(name.clone());tools_info.push(tool.clone());println!("  - {}",name);if let Some(d)=tool.get("description").and_then(|d|d.as_str()){println!("    Desc: {}",d);}if let Some(a)=tool.get("annotations"){println!("    Anno: {}",serde_json::to_string_pretty(a).unwrap_or_default());} if let Some(s)=tool.get("inputSchema"){println!("    Ops:");if let Some(ops)=s.get("properties").and_then(|p|p.get("operation")).and_then(|o|o.get("enum")).and_then(|e|e.as_array()){let ops_str:Vec<String>=ops.iter().filter_map(|o|o.as_str().map(String::from)).collect();println!("      {}",ops_str.join(", "));}else{println!("      (N/A)");}}}}else{println!(" (No tools)");}}
        else{println!("Warn: Invalid list tools resp");}
    } else {println!("Warn: No list tools resp");}

    // --- Select Tools to Test ---
    let tools_to_test = if let Some(p_name)=cli.plugin{if tools.contains(&p_name){vec![p_name]}else{println!("\nTool '{}' unavailable. Available: {}",p_name,tools.join(", "));return Err("Tool not found".into());}}else{tools};

    // --- Test Batch (MCPI) ---
    println!("\nTesting JSON-RPC batch request...");
    let batch_req_data = json!([{ "jsonrpc": "2.0", "id": 10, "method": "ping", "params": null }, { "jsonrpc": "2.0", "id": 11, "method": "resources/list", "params": null }]);
    let batch_req_str = serde_json::to_string(&batch_req_data).map_err(|e| format!("Serialize batch err: {}", e))?;
    write.send(Message::Text(batch_req_str.into())).await.map_err(|e| format!("WS send err (batch): {}", e))?;
    if let Some(Ok(Message::Text(resp_str))) = read.next().await { if resp_str.trim().starts_with('[') { match serde_json::from_str::<Vec<MCPResponse>>(&resp_str) { Ok(br) => { println!("Batch response ({} items):", br.len()); for (i, r) in br.iter().enumerate() { println!("  Item {}: ID={}", i+1, r.id); if let Some(err)=&r.error{println!("    Err: {} ({})", err.message, err.code);} } } Err(e) => println!("Err parsing batch resp: {}", e), }} else { println!("Warn: Expected batch array, got: {}", resp_str); } } else { println!("Warn: No batch resp"); }

    // --- Test Tools (MCPI) ---
    for tool_name in tools_to_test {
        println!("\n=== Testing tool: {} ===", tool_name);
        let tool_info = tools_info.iter().find(|t|t.get("name").and_then(|n|n.as_str())==Some(&tool_name)).cloned().unwrap_or_default();
        let default_schema = json!({}); let tool_schema = tool_info.get("inputSchema").unwrap_or(&default_schema);
        let operations = tool_schema.get("properties").and_then(|p|p.get("operation")).and_then(|o|o.get("enum")).and_then(|e|e.as_array()).map(|ops|ops.iter().filter_map(|op|op.as_str().map(String::from)).collect::<Vec<String>>()).unwrap_or_else(||vec!["SEARCH".to_string()]);
        println!("Supported operations: {:?}", operations);

        if tool_name == "weather_forecast" && operations.contains(&"GET_AUDIO".to_string()) {
            println!("\nTesting GET_AUDIO for weather...");
            let audio_req = MCPRequest { jsonrpc: "2.0".to_string(), id: json!(50), method: "tools/call".to_string(), params: Some(json!({"name":"weather_forecast", "arguments":{"operation":"GET_AUDIO", "location":"London"}})) };
            let audio_req_str = serde_json::to_string(&audio_req).map_err(|e| format!("Serialize audio req err: {}", e))?;
            write.send(Message::Text(audio_req_str.into())).await.map_err(|e| format!("WS send err (audio): {}", e))?;
            if let Some(Ok(Message::Text(resp_str))) = read.next().await { match serde_json::from_str::<MCPResponse>(&resp_str) { Ok(p)=>if let Some(r)=p.result{println!("Audio resp result: {}", serde_json::to_string_pretty(&r).unwrap_or_default());}else if let Some(e)=p.error{println!("Audio req err: {} ({})", e.message, e.code);}, Err(e)=>println!("Err parsing audio resp: {}", e),}} else { println!("Warn: No audio resp"); }
        }

        if tool_name == "weather_forecast" {
             println!("\nTesting completions for location..."); let comps=get_completions(&mut write,&mut read,"tools/call","arguments.location","L",Some(json!({"name":"weather_forecast"}))).await; match comps{Ok(s)=>println!("  Loc suggestions for 'L': {:?}",s),Err(e)=>println!("  Err completions: {}",e),}
        }

        for operation in operations {
            if tool_name=="weather_forecast"&&operation=="GET_AUDIO"{continue;} let args=generate_test_arguments(&tool_name,&operation,tool_schema); call_tool(&mut write,&mut read,&tool_name,&operation,args).await?;
        }
    }

    // --- Ping (MCPI) ---
    let ping_req = MCPRequest { jsonrpc: "2.0".to_string(), id: json!(99), method: "ping".to_string(), params: None };
    let ping_req_str = serde_json::to_string(&ping_req).map_err(|e| format!("Serialize ping err: {}", e))?;
    write.send(Message::Text(ping_req_str.into())).await.map_err(|e| format!("WS send err (ping): {}", e))?;
    if let Some(Ok(Message::Text(resp_str))) = read.next().await { match serde_json::from_str::<MCPResponse>(&resp_str) {
        Ok(p)=>{if let Some(err)=p.error{println!("\nPing error: {} ({})",err.message,err.code);}else if p.result.is_some(){println!("\nPing successful.");}else{println!("\nInvalid ping resp");}}, Err(e)=>println!("\nErr parsing ping resp: {}",e),
    }} else { println!("\nNo ping response"); }

    println!("\nClosing MCPI connection");
    Ok(())
}

// --- Helper Functions ---
fn generate_test_arguments(_tool_name: &str, operation: &str, schema: &Value) -> Value {
    let mut args=json!({"operation": operation}); if let Some(props)=schema.get("properties").and_then(|p|p.as_object()){for(p_name,p_schema)in props.iter().filter(|(k,_)|*k!="operation"){let desc=p_schema.get("description").and_then(|d|d.as_str()).unwrap_or("");let p_type=p_schema.get("type").and_then(|t|t.as_str()).unwrap_or("string");let t_val=match(p_name.as_str(),p_type,operation){("id",_,_)if desc.contains("ID")=>json!("test-id-123"),("query",_,_)=>json!("search query"),("location",_,_)=>json!("London"),("domain",_,_)=>json!("target.example.com"),("relationship",_,_)=>json!("affiliate"),(_,"string",_)=>json!("default str"),(_,"number",_)|(_,"integer",_)=>json!(42),(_,"boolean",_)=>json!(false),_=>Value::Null,};if !t_val.is_null(){if let Some(obj)=args.as_object_mut(){obj.insert(p_name.clone(),t_val);}}}} args
}

async fn get_completions<S, R>( write: &mut S, read: &mut R, method: &str, param_name: &str, partial_val: &str, context: Option<Value> ) -> Result<Vec<String>, BoxedError> where S: SinkExt<Message, Error=tokio_tungstenite::tungstenite::Error>+Unpin, R: StreamExt<Item=Result<Message, tokio_tungstenite::tungstenite::Error>>+Unpin {
    let mut params=json!({"method":method,"parameterName":param_name,"partialValue":partial_val}); if let Some(ctx_obj)=context.as_ref().and_then(|c|c.as_object()){if let Some(p_obj)=params.as_object_mut(){for(k,v)in ctx_obj{p_obj.insert(format!("context.{}",k),v.clone());}}}else if let Some(ctx_val)=context{if let Some(p_obj)=params.as_object_mut(){p_obj.insert("context".to_string(),ctx_val);}}
    let req = MCPRequest { jsonrpc:"2.0".to_string(), id:json!(98), method:"completions".to_string(), params:Some(params) };
    let req_str = serde_json::to_string(&req).map_err(|e|format!("Serialize completions err: {}",e))?;
    write.send(Message::Text(req_str.into())).await.map_err(|e|format!("WS send err (completions): {}",e))?;
    if let Some(Ok(Message::Text(resp_str)))=read.next().await{match serde_json::from_str::<MCPResponse>(&resp_str){Ok(p)=>{if let Some(e)=p.error{println!("Completions error: {} ({})",e.message,e.code);Ok(vec![])}else if let Some(r)=p.result{Ok(r.get("suggestions").and_then(|s|s.as_array()).map(|a|a.iter().filter_map(|v|v.as_str().map(String::from)).collect()).unwrap_or_default())}else{println!("Invalid completions resp");Ok(vec![])}}Err(e)=>Err(format!("Parse completions resp err: {}",e).into()),}}else{Err("No completions response".into())}
}

async fn discover_service_http(url: &str) -> Result<DiscoveryResponse, BoxedError> {
    let client=reqwest::Client::new(); Ok(client.get(url).send().await?.json::<DiscoveryResponse>().await?)
}

async fn call_tool<S, R>( write: &mut S, read: &mut R, name: &str, operation: &str, arguments: Value ) -> Result<(), BoxedError> where S: SinkExt<Message, Error=tokio_tungstenite::tungstenite::Error>+Unpin, R: StreamExt<Item=Result<Message, tokio_tungstenite::tungstenite::Error>>+Unpin {
    println!("\nTesting {} with {} operation",name,operation);
    let req=MCPRequest{jsonrpc:"2.0".to_string(),id:json!(format!("{}-{}-{}",name,operation,rand::thread_rng().gen::<u16>())),method:"tools/call".to_string(),params:Some(json!({"name":name,"arguments":arguments}))};
    println!("Request Params: {}", serde_json::to_string_pretty(req.params.as_ref().unwrap()).unwrap_or_default());
    let req_str=serde_json::to_string(&req).map_err(|e|format!("Serialize tool call err: {}",e))?;
    write.send(Message::Text(req_str.into())).await.map_err(|e|format!("WS send err (tool call): {}",e))?;
    if let Some(Ok(Message::Text(resp_str)))=read.next().await{match serde_json::from_str::<MCPResponse>(&resp_str){Ok(p)=>{if let Some(e)=p.error{println!("  Tool call error: {} ({})",e.message,e.code);}else if let Some(r)=p.result{match serde_json::from_value::<ToolCallResult>(r.clone()){Ok(tr)=>{println!("  Result{}",if tr.is_error{" (ERROR)"}else{""});for c in tr.content{match c{mcpi_common::ContentItem::Text{text}=>{if let Ok(j)=serde_json::from_str::<Value>(&text){println!("  {}",serde_json::to_string_pretty(&j).unwrap_or(text));}else{println!("  {}",text);}},mcpi_common::ContentItem::Audio{data,mime_type}=>println!("  Audio: {}b, {}",data.len(),mime_type),mcpi_common::ContentItem::Image{data,mime_type}=>println!("  Image: {}b, {}",data.len(),mime_type),mcpi_common::ContentItem::Resource{resource}=>println!("  Resource: {}",resource.uri),}}}Err(e)=>println!("  Err parsing ToolCallResult: {}\nRaw: {}",e,serde_json::to_string_pretty(&r).unwrap_or_default()),}}else{println!("  Invalid tool call response");}}Err(e)=>println!("Err parsing tool call resp: {}",e),}}else{println!("Warn: No tool call resp");} Ok(())
}