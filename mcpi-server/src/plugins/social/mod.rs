// mcpi-server/src/plugins/social/mod.rs
mod plugin;
mod operations;

pub use plugin::SocialPlugin;
use mcpi_common::McpPlugin;
use serde_json::Value;
use std::{error::Error, sync::Arc};

/// Create a new Social plugin
pub fn create_plugin(data_path: &str, referrals: Value) -> Result<Arc<dyn McpPlugin>, Box<dyn Error + Send + Sync>> {
    Ok(Arc::new(SocialPlugin::new(data_path, referrals)))
}