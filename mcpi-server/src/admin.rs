// mcpi-server/src/admin.rs
use axum::{
    extract::State,
    response::{Html, IntoResponse},
    Json,
};
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use crate::AppState;

// Serve the admin HTML page
pub async fn serve_admin_html() -> impl IntoResponse {
    Html(r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>MCPI Admin</title>
            <style>
                body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; max-width: 1000px; margin: 0 auto; padding: 20px; line-height: 1.6; }
                .card { border: 1px solid #ddd; border-radius: 8px; padding: 20px; margin-bottom: 20px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
                h1, h2 { color: #3498db; }
                h1 { border-bottom: 2px solid #eee; padding-bottom: 10px; }
                table { width: 100%; border-collapse: collapse; margin-top: 10px; }
                table, th, td { border: 1px solid #ddd; }
                th, td { padding: 12px; text-align: left; }
                th { background-color: #f8f9fa; }
                tr:nth-child(even) { background-color: #f2f2f2; }
                .stat-value { font-size: 24px; font-weight: bold; margin: 10px 0; color: #2c3e50; }
                .stats-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 15px; }
                .stat-card { background-color: #f8f9fa; padding: 15px; border-radius: 8px; text-align: center; }
                .stat-card h3 { margin-top: 0; color: #7f8c8d; font-size: 16px; }
                .loading { color: #7f8c8d; font-style: italic; }
                .refresh-button { background-color: #3498db; color: white; border: none; padding: 8px 16px; border-radius: 4px; cursor: pointer; font-weight: bold; }
                .refresh-button:hover { background-color: #2980b9; }
                .header-with-actions { display: flex; justify-content: space-between; align-items: center; }
                .note { font-size: 12px; color: #7f8c8d; margin-top: 8px; font-style: italic; }
            </style>
        </head>
        <body>
            <h1>MCPI Admin Panel</h1>
            
            <div class="card">
                <div class="header-with-actions">
                    <h2>Server Statistics</h2>
                    <button id="refresh-stats" class="refresh-button">Refresh</button>
                </div>
                <div class="stats-grid" id="stats">
                    <div class="stat-card">
                        <h3>Loading...</h3>
                        <div class="stat-value">-</div>
                    </div>
                </div>
                <p class="note">Stats auto-refresh every 5 seconds</p>
            </div>
            
            <div class="card">
                <div class="header-with-actions">
                    <h2>Plugins</h2>
                    <button id="refresh-plugins" class="refresh-button">Refresh</button>
                </div>
                <div id="plugins">
                    <p class="loading">Loading plugin information...</p>
                </div>
            </div>
            
            <script>
                // Refresh stats every 5 seconds
                function refreshStats() {
                    fetch('/api/admin/stats')
                        .then(res => res.json())
                        .then(data => {
                            const statsHtml = `
                                <div class="stat-card">
                                    <h3>Uptime</h3>
                                    <div class="stat-value">${data.uptime_formatted || '0'}</div>
                                </div>
                                <div class="stat-card">
                                    <h3>Total Requests</h3>
                                    <div class="stat-value">${data.request_count || '0'}</div>
                                </div>
                                <div class="stat-card">
                                    <h3>Active Connections</h3>
                                    <div class="stat-value">${data.active_connections || '0'}</div>
                                </div>
                                <div class="stat-card">
                                    <h3>Plugins</h3>
                                    <div class="stat-value">${data.plugin_count || '0'}</div>
                                </div>
                            `;
                            
                            document.getElementById('stats').innerHTML = statsHtml;
                        })
                        .catch(err => {
                            console.error('Error fetching stats:', err);
                            document.getElementById('stats').innerHTML = `
                                <div class="stat-card">
                                    <h3>Error</h3>
                                    <div class="stat-value">Failed to load stats</div>
                                </div>
                            `;
                        });
                }
                
                // Fetch plugins once
                function fetchPlugins() {
                    document.getElementById('plugins').innerHTML = '<p class="loading">Loading plugin information...</p>';
                    
                    fetch('/api/admin/plugins')
                        .then(res => res.json())
                        .then(data => {
                            const rows = data.plugins.map(p => `
                                <tr>
                                    <td>${p.name}</td>
                                    <td>${p.description}</td>
                                    <td>${p.category}</td>
                                    <td>${p.operations.join(', ')}</td>
                                </tr>
                            `).join('');
                            
                            document.getElementById('plugins').innerHTML = `
                                <table>
                                    <thead>
                                        <tr>
                                            <th>Name</th>
                                            <th>Description</th>
                                            <th>Category</th>
                                            <th>Operations</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        ${rows}
                                    </tbody>
                                </table>
                            `;
                        })
                        .catch(err => {
                            console.error('Error fetching plugins:', err);
                            document.getElementById('plugins').innerHTML = 
                                '<p>Error loading plugins information</p>';
                        });
                }
                
                // Initial load
                refreshStats();
                fetchPlugins();
                
                // Set up refresh interval for stats
                setInterval(refreshStats, 5000);
                
                // Add event listeners for manual refresh
                document.getElementById('refresh-stats').addEventListener('click', refreshStats);
                document.getElementById('refresh-plugins').addEventListener('click', fetchPlugins);
            </script>
        </body>
        </html>
    "#)
}

// Get server stats
pub async fn get_stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Calculate uptime
    let uptime = state.startup_time.elapsed();
    let uptime_secs = uptime.as_secs();
    
    // Format uptime in a human-readable way
    let uptime_formatted = format!(
        "{}d {}h {}m {}s",
        uptime_secs / 86400,
        (uptime_secs % 86400) / 3600,
        (uptime_secs % 3600) / 60,
        uptime_secs % 60
    );
    
    // Get current connection count
    let active_connections = state.active_connections.load(Ordering::SeqCst);
    
    // Get total request count
    let request_count = state.request_count.load(Ordering::SeqCst);
    
    // Get plugin count
    let plugin_count = state.registry.get_all_plugins().len();
    
    // Build stats response
    let stats = json!({
        "uptime_secs": uptime_secs,
        "uptime_formatted": uptime_formatted,
        "request_count": request_count,
        "active_connections": active_connections,
        "plugin_count": plugin_count
    });
    
    Json(stats)
}

// Get plugins list
pub async fn get_plugins(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let plugins = state.registry.get_all_plugins();
    let plugin_info: Vec<serde_json::Value> = plugins.iter().map(|plugin| {
        json!({
            "name": plugin.name(),
            "description": plugin.description(),
            "category": plugin.category(),
            "type": format!("{:?}", plugin.plugin_type()),
            "operations": plugin.supported_operations()
        })
    }).collect();
    
    Json(json!({
        "plugins": plugin_info
    }))
}