// mcpi-server/src/plugins/hello/mod.rs
mod plugin;
mod operations;

pub use plugin::HelloPlugin;
use mcpi_common::McpPlugin;
use std::{error::Error, sync::Arc};

/// Create a new Hello plugin
pub fn create_plugin(data_path: &str) -> Result<Arc<dyn McpPlugin>, Box<dyn Error + Send + Sync>> {
    Ok(Arc::new(HelloPlugin::new(data_path)))
}