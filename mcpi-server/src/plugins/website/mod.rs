// mcpi-server/src/plugins/website/mod.rs
mod plugin;
mod operations;

pub use plugin::WebsitePlugin;
use mcpi_common::McpPlugin;
use std::{error::Error, sync::Arc};

/// Create a new Website plugin
pub fn create_plugin(data_path: &str) -> Result<Arc<dyn McpPlugin>, Box<dyn Error + Send + Sync>> {
    Ok(Arc::new(WebsitePlugin::new(data_path)))
}