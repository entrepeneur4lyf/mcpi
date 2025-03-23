// mcpi-server/src/plugins/website/mod.rs
mod plugin;
mod operations;

pub use plugin::WebsitePlugin;
use mcpi_common::{McpPlugin, JsonDataPlugin};
use std::{error::Error, sync::Arc};

/// Create a new Website plugin
pub fn create_plugin(data_path: &str) -> Result<Arc<dyn McpPlugin>, Box<dyn Error + Send + Sync>> {
    let website_plugin = WebsitePlugin::new(data_path);
    Ok(Arc::new(JsonDataPlugin::new(website_plugin)))
}