// mcpi-server/src/main.rs

// --- Standard Imports ---
use axum::{
    extract::{ws::{WebSocket, WebSocketUpgrade, Message}, State},
    response::{IntoResponse, Response},
    routing::{get, post, delete}, // Keep post/delete for /mcpi route
    Router, Json,
    // Removed BoxError
    http::{StatusCode, HeaderMap},
};
use mcpi_common::{ // Group common imports
    CapabilityDescription, DiscoveryResponse, MCPRequest, Resource, Tool,
    ServerCapabilities, MCPI_VERSION, ContentItem, ResourcesCapability, ToolsCapability,
    Provider, Referral,
};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    net::SocketAddr,
    path::Path,
    sync::{atomic::{AtomicUsize, Ordering}, Arc},
    time::Instant,
    fs,
    error::Error,
};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
// Removed tower imports related to boxing
// use tower::layer::util::Stack;
// use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
};
use tracing::{error, info, warn};
use url::Url;
use rand::Rng;

// --- Local Modules ---
mod admin;
mod message_handler;
mod plugin_registry;
mod plugins;
mod traits;

use message_handler::McpMessageHandler;
use plugin_registry::PluginRegistry;
use crate::traits::MessageHandler;


// --- Constants ---
const DATA_PATH: &str = "data";
const CONFIG_FILE_PATH: &str = "data/server/data.json";
const SERVER_PORT: u16 = 3001;
const PROTOCOL_VERSION_PLACEHOLDER: &str = "0.1.0-unknown";


// --- Shared Application State ---
pub struct AppState {
    registry: Arc<PluginRegistry>,
    provider_info: Arc<Value>,
    referrals: Arc<Value>,
    message_handler: Arc<McpMessageHandler>,
    http_sessions: Arc<RwLock<HashMap<String, HttpSessionInfo>>>,
    active_ws_connections: AtomicUsize,
    request_count: AtomicUsize,
    startup_time: Instant,
}

// --- Session Info for Streamable HTTP ---
struct HttpSessionInfo {
    last_event_id: Option<String>,
}

// --- Main Function ---
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    tracing_subscriber::fmt::init();
    validate_paths()?;
    let config = load_config()?;

    let provider_info = Arc::new(config.get("provider").cloned().unwrap_or_else(|| json!({})));
    let referrals = Arc::new(config.get("referrals").cloned().unwrap_or_else(|| json!([])));

    let registry = Arc::new(PluginRegistry::new());
    registry.register_all_plugins(DATA_PATH, (*referrals).clone())?;
    info!("Registered {} plugins", registry.get_all_plugins().len());

    // --- Initialize State ---
    // McpMessageHandler takes Arc<PluginRegistry> and Arc<Value>
    let message_handler = Arc::new(McpMessageHandler::new(
        registry.clone(),
        provider_info.clone(),
    ));

    let app_state = Arc::new(AppState {
        registry,
        provider_info,
        referrals,
        message_handler,
        http_sessions: Arc::new(RwLock::new(HashMap::new())),
        active_ws_connections: AtomicUsize::new(0),
        request_count: AtomicUsize::new(0),
        startup_time: Instant::now(),
    });

    // --- Configure CORS ---
    let cors = CorsLayer::permissive();

    // --- Build the SINGLE Router for ALL services (Port 3001) ---
    let app_router = Router::new()
        // --- Routes ---
        .route("/mcp", get(handle_streamable_get).post(handle_streamable_post).delete(handle_streamable_delete))
        .route("/mcpi", get(ws_handler))
        .route("/mcpi/discover", get(discovery_handler))
        .route("/admin", get(admin::serve_admin_html))
        .route("/api/admin/stats", get(admin::get_stats))
        .route("/api/admin/plugins", get(admin::get_plugins))
        // --- Layers ---
        // Apply layers directly to the Router
        .layer(TraceLayer::new_for_http()) // Apply tracing first
        .layer(cors) // Apply CORS layer next
        .with_state(app_state.clone()); // Apply state last

    // --- Start the Single Server ---
    let addr = SocketAddr::from(([0, 0, 0, 0], SERVER_PORT));
    info!("Starting unified server (MCP/MCPI/Admin) on {}", addr);

    let listener = TcpListener::bind(addr).await?;
    info!("Server listening on {}", addr);

    axum::serve(listener, app_router.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Server shut down successfully");
    Ok(())
}

// --- Graceful Shutdown Signal Handler ---
async fn shutdown_signal() {
    tokio::signal::ctrl_c().await.expect("Failed to install Ctrl+C handler");
    info!("Shutdown signal received...");
}


// --- Streamable HTTP Handlers ---
async fn handle_streamable_get( State(state): State<Arc<AppState>>, headers: HeaderMap ) -> impl IntoResponse {
    state.request_count.fetch_add(1, Ordering::SeqCst);
    let session_id = headers.get("mcp-session-id").and_then(|v| v.to_str().ok()).map(str::to_string);
    let last_event_id = headers.get("last-event-id").and_then(|v| v.to_str().ok()).map(str::to_string);

    if let Some(ref id_str) = session_id {
        let mut sessions = state.http_sessions.write().await;
        let session = sessions.entry(id_str.clone()).or_insert_with(|| HttpSessionInfo { last_event_id: None });
        if let Some(leid) = last_event_id { info!("Session {}: Updating last_event_id to {}", id_str, leid); session.last_event_id = Some(leid); }
        info!("SSE stream requested for session: {}", id_str);
        (StatusCode::OK, [ ("content-type", "text/event-stream"), ("cache-control", "no-cache"), ("connection", "keep-alive"), ], "data: Connected (SSE Placeholder)\n\n").into_response()
    } else {
        warn!("GET /mcp missing mcp-session-id");
        (StatusCode::BAD_REQUEST, "mcp-session-id header required").into_response()
    }
}

async fn handle_streamable_post( State(state): State<Arc<AppState>>, headers: HeaderMap, body: String ) -> impl IntoResponse {
    state.request_count.fetch_add(1, Ordering::SeqCst);
    let session_id = headers.get("mcp-session-id").and_then(|v| v.to_str().ok()).map(str::to_string);
    let client_id = session_id.clone().unwrap_or_else(|| format!("http-{}", rand::thread_rng().gen::<u32>()));

    if let Some(ref id_str) = session_id { if !state.http_sessions.read().await.contains_key(id_str) { warn!("POST /mcp for non-existent session: {}", id_str); } else { info!("POST /mcp for session: {}", id_str); } }
    else { info!("POST /mcp without session ID (client_id: {})", client_id); }

    if let Some(response_body) = state.message_handler.handle_message(body, client_id).await { (StatusCode::OK, [("content-type", "application/json")], response_body).into_response() }
    else { (StatusCode::NO_CONTENT, "").into_response() }
}

async fn handle_streamable_delete( State(state): State<Arc<AppState>>, headers: HeaderMap ) -> impl IntoResponse {
    state.request_count.fetch_add(1, Ordering::SeqCst);
    if let Some(session_id) = headers.get("mcp-session-id").and_then(|v| v.to_str().ok()) {
        if state.http_sessions.write().await.remove(session_id).is_some() { info!("Session {} terminated via DELETE /mcp", session_id); (StatusCode::OK, "Session terminated").into_response() }
        else { warn!("DELETE /mcp for non-existent session: {}", session_id); (StatusCode::NOT_FOUND, "Session not found").into_response() }
    } else { warn!("DELETE /mcp missing mcp-session-id"); (StatusCode::BAD_REQUEST, "mcp-session-id header required").into_response() }
}

// --- WebSocket Handlers ---
async fn ws_handler( ws: WebSocketUpgrade, State(state): State<Arc<AppState>>, _headers: HeaderMap ) -> Response {
    let client_id = format!("ws-{}", rand::thread_rng().gen::<u32>());
    info!("WebSocket upgrade request (/mcpi) from client: {}", client_id);
    ws.on_upgrade(move |socket| handle_socket(socket, state, client_id))
}
async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>, client_id: String) {
    info!("WebSocket client connected: {}", client_id); state.active_ws_connections.fetch_add(1, Ordering::SeqCst); loop { tokio::select! { msg_result = socket.recv() => { match msg_result { Some(Ok(msg)) => { if !process_ws_message(msg, &mut socket, &state, &client_id).await { break; } } Some(Err(e)) => { warn!("WS recv error from {}: {}", client_id, e); break; } None => { info!("WS client {} disconnected (recv None)", client_id); break; } } } } } info!("WebSocket client disconnected: {}", client_id); state.active_ws_connections.fetch_sub(1, Ordering::SeqCst);
}
async fn process_ws_message( msg: Message, socket: &mut WebSocket, state: &Arc<AppState>, client_id: &str, ) -> bool {
    match msg { Message::Text(text) => { info!("Received text from WS {}: {}", client_id, text.chars().take(100).collect::<String>()); if let Some(response) = state.message_handler.handle_message(text, client_id.to_string()).await { if socket.send(Message::Text(response)).await.is_err() { return false; } } } Message::Binary(_) => warn!("Unexpected binary msg from WS {}", client_id), Message::Ping(data) => if socket.send(Message::Pong(data)).await.is_err() { return false; }, Message::Pong(_) => info!("Received Pong from WS {}", client_id), Message::Close(_) => { info!("WS client {} sent close frame", client_id); return false; } } true
}

// --- Other Handlers (Discovery, MCP Processing Logic) ---
async fn discovery_handler(State(state): State<Arc<AppState>>) -> Json<DiscoveryResponse> {
    state.request_count.fetch_add(1, Ordering::SeqCst); info!("Handling /mcpi/discover request");
    let provider = Provider { name: state.provider_info.get("name").and_then(|n|n.as_str()).unwrap_or("").to_string(), domain: state.provider_info.get("domain").and_then(|d|d.as_str()).unwrap_or("").to_string(), description: state.provider_info.get("description").and_then(|d|d.as_str()).unwrap_or("").to_string(), branding: None };
    let referrals = if let Some(refs) = state.referrals.as_array() { refs.iter().filter_map(|r| Some(Referral{name: r.get("name")?.as_str()?.to_string(), domain: r.get("domain")?.as_str()?.to_string(), relationship: r.get("relationship")?.as_str()?.to_string() })).collect() } else { vec![] };
    let caps = state.registry.get_all_plugins().iter().map(|p| CapabilityDescription{name: p.name().to_string(), description: p.description().to_string(), category: p.category().to_string(), operations: p.supported_operations()}).collect();
    Json(DiscoveryResponse { provider, mode: "active".to_string(), capabilities: caps, referrals })
}

// --- MCP Message Processing Logic ---
pub async fn process_mcp_message( message: &str, registry: &Arc<PluginRegistry>, provider_info: &Arc<Value>, ) -> Option<String> {
    match serde_json::from_str::<MCPRequest>(message) {
        Ok(req) => { let span=tracing::info_span!("process_mcp_req",id=%req.id,method=%req.method); let _e=span.enter(); info!("Processing"); match req.method.as_str() {
            "initialize" => Some(handle_initialize(&req, registry, provider_info)), "resources/list" => Some(handle_list_resources(&req, registry, provider_info)), "resources/read" => Some(handle_read_resource(&req, registry)), "tools/list" => Some(handle_list_tools(&req, registry)), "tools/call" => Some(handle_call_tool(&req, registry)), "completions" => Some(handle_completions(&req, registry)), "ping" => Some(handle_ping(&req)),
            _ => { warn!("Method not found"); Some(create_error_response(req.id, -32601, format!("Method not found: {}", req.method))) }
        }}
        Err(e) => { error!("Parse error: {}", e); Some(create_error_response(Value::Null, -32700, format!("Parse error: {}", e))) }
    }
}

// --- MCP Request Handler Implementations ---
fn handle_initialize(_request: &MCPRequest, registry: &Arc<PluginRegistry>, provider_info: &Arc<Value>) -> String {
    let caps=ServerCapabilities{resources:Some(ResourcesCapability{list_changed:true,subscribe:true}),tools:Some(ToolsCapability{list_changed:true}),prompts:None,logging:None}; let name=provider_info.get("name").and_then(|v|v.as_str()).unwrap_or("").to_string(); let desc=provider_info.get("description").and_then(|v|v.as_str()).unwrap_or("").to_string(); let _names=registry.get_all_plugins().iter().map(|p|p.name()).collect::<Vec<_>>(); json!({"jsonrpc":"2.0","id":_request.id,"result":{"serverInfo":{"name":name,"version":MCPI_VERSION},"protocolVersion":PROTOCOL_VERSION_PLACEHOLDER,"capabilities":caps,"instructions":format!("Provider: {}",desc)}}).to_string()
}
fn handle_list_resources(_request: &MCPRequest, registry: &Arc<PluginRegistry>, provider_info: &Arc<Value>) -> String {
    let domain=provider_info.get("domain").and_then(|d|d.as_str()).unwrap_or("example.com"); let resources=registry.get_all_plugins().iter().flat_map(|p|p.get_resources().into_iter().map(|(n,s,d)|Resource{name:n,description:d,uri:format!("mcpi://{}/resources/{}/{}",domain,p.name(),s),mime_type:Some("application/json".into()),size:None})).collect::<Vec<_>>(); json!({"jsonrpc":"2.0","id":_request.id,"result":{"resources":resources}}).to_string()
}
fn handle_read_resource(request: &MCPRequest, registry: &Arc<PluginRegistry>) -> String {
     if let Some(u)=request.params.as_ref().and_then(|p|p.get("uri")?.as_str()){if let Ok(uri)=Url::parse(u){if uri.scheme()=="mcpi"{let path:Vec<&str>=uri.path_segments().map(|i|i.collect()).unwrap_or_default();if path.len()>=3&&path[0]=="resources"{let(p_name,r_suffix)=(path[1],path[2..].join("/"));if let Some(p)=registry.get_plugin(p_name){match p.read_resource(&r_suffix){Ok(c)=>return json!({"jsonrpc":"2.0","id":request.id,"result":{"contents":[c]}}).to_string(),Err(e)=>{warn!("Read err: {}",e);return create_error_response(request.id.clone(),100,format!("Read err: {}",e));}}}else{warn!("Plugin not found: {}",p_name);}}else{warn!("Invalid path: {}",uri.path());}}else{warn!("Invalid scheme: {}",uri.scheme());}}else{warn!("Invalid URI: {}",u);}}else{warn!("Missing URI");} create_error_response(request.id.clone(),-32602,"Invalid params".into())
}
fn handle_list_tools(_request: &MCPRequest, registry: &Arc<PluginRegistry>) -> String {
    let tools=registry.get_all_plugins().iter().map(|p|Tool{name:p.name().into(),description:Some(p.description().into()),input_schema:p.input_schema(),annotations:p.get_tool_annotations()}).collect::<Vec<_>>(); json!({"jsonrpc":"2.0","id":_request.id,"result":{"tools":tools}}).to_string()
}
fn handle_call_tool(request: &MCPRequest, registry: &Arc<PluginRegistry>) -> String {
     if let Some(p)=request.params.as_ref().and_then(|p|p.as_object()){if let(Some(name),Some(args))=(p.get("name").and_then(|n|n.as_str()),p.get("arguments")){let op=args.get("operation").and_then(|o|o.as_str()).unwrap_or("DEFAULT");match registry.execute_plugin(name,op,args){Ok(res)=>{let c=match res{Value::String(s)=>vec![ContentItem::Text{text:s}],Value::Null=>vec![],_=>vec![ContentItem::Text{text:res.to_string()}]};return json!({"jsonrpc":"2.0","id":request.id,"result":{"content":c}}).to_string();},Err(e)=>{let ec=ContentItem::Text{text:format!("Exec err: {}",e)};return json!({"jsonrpc":"2.0","id":request.id,"result":{"content":[ec],"isError":true}}).to_string();}}}} create_error_response(request.id.clone(),-32602,"Invalid params".into())
}
fn handle_completions(_request: &MCPRequest, registry: &Arc<PluginRegistry>) -> String {
     let suggestions:Vec<Value>=vec![]; if let Some(p)=_request.params.as_ref().and_then(|p|p.as_object()){if let(Some(m),Some(pn),Some(pv))=(p.get("method").and_then(|m|m.as_str()),p.get("parameterName").and_then(|p|p.as_str()),p.get("partialValue")){if m=="tools/call"{let ctx=p.get("context").cloned().unwrap_or(Value::Null);if let Some(t_name)=ctx.get("name").and_then(|n|n.as_str()){if let Some(plugin)=registry.get_plugin(t_name){let sugg=plugin.get_completions(pn,pv,&ctx);return json!({"jsonrpc":"2.0","id":_request.id,"result":{"suggestions":sugg}}).to_string();}}else if pn=="name"{let names:Vec<String>=registry.get_all_plugins().iter().map(|p|p.name().to_string()).filter(|n|n.starts_with(pv.as_str().unwrap_or(""))).collect();return json!({"jsonrpc":"2.0","id":_request.id,"result":{"suggestions":names}}).to_string();}}}} json!({"jsonrpc":"2.0","id":_request.id,"result":{"suggestions":suggestions}}).to_string()
}
fn handle_ping(_request: &MCPRequest) -> String { json!({"jsonrpc":"2.0","id":_request.id,"result":{}}).to_string() }
fn create_error_response(id: Value, code: i32, message: String) -> String { json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}}).to_string() }

// --- Utility Functions ---
fn validate_paths() -> Result<(), Box<dyn Error + Send + Sync>> { let c=Path::new(CONFIG_FILE_PATH); let d=Path::new(DATA_PATH); if !c.exists(){return Err(format!("Config file missing: {}",CONFIG_FILE_PATH).into());} if !d.exists(){return Err(format!("Data dir missing: {}",DATA_PATH).into());} Ok(()) }
fn load_config() -> Result<Value, Box<dyn Error + Send + Sync>> { let d=fs::read_to_string(CONFIG_FILE_PATH)?; serde_json::from_str(&d).map_err(|e|e.into()) }