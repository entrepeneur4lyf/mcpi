// mcpi-server/src/plugins/website_plugin.rs

use serde_json::{json, Value};
use std::error::Error;
use std::fs;

/// Website plugin that provides metadata about the website service
pub struct WebsitePlugin {
    config: Value,
}

impl WebsitePlugin {
    /// Create a new website plugin from a config file
    pub fn new(config_path: &str, _data_path: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        // Load configuration
        let config_data = fs::read_to_string(config_path)?;
        let config: Value = serde_json::from_str(&config_data)?;
        
        // Check if configuration has required sections
        if !config.get("provider").is_some() {
            return Err("No provider information found in config".into());
        }
        
        if !config.get("capabilities").is_some() {
            return Err("No capabilities found in config".into());
        }
        
        Ok(WebsitePlugin {
            config,
        })
    }
    
    /// Get the provider information from config
    pub fn get_provider_info(&self) -> Value {
        self.config.get("provider").cloned().unwrap_or_else(|| json!({}))
    }
    
    /// Get referrals information from config
    pub fn get_referrals(&self) -> Value {
        self.config.get("referrals").cloned().unwrap_or_else(|| json!([]))
    }
}