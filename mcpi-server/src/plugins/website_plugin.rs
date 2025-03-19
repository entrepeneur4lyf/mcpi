// mcpi-server/src/plugins/website_plugin.rs

use mcpi_common::{JsonDataPlugin, McpPlugin};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::Path;

/// Website plugin that provides product search, customer lookup, order history, and reviews
pub struct WebsitePlugin {
    config: Value,
    plugins: HashMap<String, JsonDataPlugin>,
}

impl WebsitePlugin {
    /// Create a new website plugin from a config file
    pub fn new(config_path: &str, base_data_path: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        // Load configuration
        let config_data = fs::read_to_string(config_path)?;
        let config: Value = serde_json::from_str(&config_data)?;
        
        // Initialize plugins map
        let mut plugins = HashMap::new();
        
        // Determine data directory for JSON files
        let data_path = if Path::new(base_data_path).join("mock").exists() {
            Path::new(base_data_path).join("mock")
        } else {
            Path::new(base_data_path).to_path_buf()
        };
        
        // Check for capabilities in config
        if let Some(capabilities) = config.get("capabilities").and_then(|c| c.as_object()) {
            for (name, cap_config) in capabilities {
                let name_str = name.clone();
                let description = cap_config.get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("No description");
                
                let category = cap_config.get("category")
                    .and_then(|c| c.as_str())
                    .unwrap_or("misc");
                
                let operations = cap_config.get("operations")
                    .and_then(|o| o.as_array())
                    .and_then(|arr| {
                        let mut ops = Vec::new();
                        for op in arr {
                            if let Some(op_str) = op.as_str() {
                                ops.push(op_str.to_string());
                            }
                        }
                        Some(ops)
                    })
                    .unwrap_or_else(|| vec!["SEARCH".to_string(), "GET".to_string(), "LIST".to_string()]);
                
                // Use the data_file from config, or default to name.json
                let data_file = cap_config.get("data_file")
                    .and_then(|d| d.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("{}.json", name_str));
                
                // Construct full path to data file
                let full_path = data_path.join(&data_file);
                
                // Validate that the data file exists
                if !full_path.exists() {
                    return Err(format!("Data file for capability '{}' not found: {}", 
                        name, full_path.display()).into());
                }
                
                // Create the plugin 
                let plugin = JsonDataPlugin::new(
                    &name_str,
                    description,
                    category,
                    operations,
                    &data_file,
                    &data_path.to_string_lossy(),
                );
                
                plugins.insert(name_str, plugin);
            }
        } else {
            return Err("No capabilities found in config".into());
        }
        
        Ok(WebsitePlugin {
            config,
            plugins,
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
    
    /// Get all underlying plugins as dynamic trait objects
    pub fn get_plugins(&self) -> Vec<&dyn McpPlugin> {
        self.plugins.values()
            .map(|plugin| plugin as &dyn McpPlugin)
            .collect()
    }
}