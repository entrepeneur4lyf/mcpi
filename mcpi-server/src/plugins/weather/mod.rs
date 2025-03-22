// mcpi-server/src/plugins/weather/mod.rs
mod plugin;
mod operations;

pub use plugin::WeatherPlugin;
use mcpi_common::McpPlugin;
use std::{error::Error, sync::Arc};

/// Create a new Weather plugin
pub fn create_plugin() -> Result<Arc<dyn McpPlugin>, Box<dyn Error + Send + Sync>> {
    Ok(Arc::new(WeatherPlugin::new()))
}