// mcpi-server/src/admin.rs

use axum::{extract::State, response::Html, Json}; // Ensure Html is imported
use serde_json::{json, Value};
use std::sync::{atomic::Ordering, Arc};
use std::time::Instant;

use crate::AppState; // Import shared AppState

// Handler for GET /admin
pub async fn serve_admin_html() -> Html<&'static str> {
    // Embed the content of the admin.html file directly into the binary at compile time.
    // This path assumes your `static` directory is directly inside the `mcpi-server` project folder,
    // at the same level as the `src` directory.
    // Adjust the path if your directory structure is different (e.g., `src/static/admin.html`).
    Html(include_str!("../static/admin.html"))
}

// Handler for GET /api/admin/stats
pub async fn get_stats(State(state): State<Arc<AppState>>) -> Json<Value> {
    // Calculate uptime
    let uptime = Instant::now().duration_since(state.startup_time).as_secs();

    // Access stats from AppState
    let active_ws_connections = state.active_ws_connections.load(Ordering::SeqCst);
    let request_count = state.request_count.load(Ordering::SeqCst);
    // Use read().await because http_sessions is behind a RwLock
    let http_sessions_count = state.http_sessions.read().await.len();

    Json(json!({
        "uptime_seconds": uptime,
        "active_websocket_connections": active_ws_connections,
        "active_http_sessions": http_sessions_count,
        "total_requests_processed": request_count,
    }))
}

// Handler for GET /api/admin/plugins
pub async fn get_plugins(State(state): State<Arc<AppState>>) -> Json<Value> {
    let plugins_info: Vec<Value> = state
        .registry
        .get_all_plugins()
        .iter()
        .map(|plugin| {
            json!({
                "name": plugin.name(),
                "description": plugin.description(),
                "category": plugin.category(),
                "type": format!("{:?}", plugin.plugin_type()), // Show plugin type enum variant name
                "operations": plugin.supported_operations(),
            })
        })
        .collect();

    Json(json!({ "plugins": plugins_info }))
}