// mcpi-server/src/plugins/hello/plugin.rs
use mcpi_common::{McpPlugin, PluginResult, plugin::PluginType};
use crate::plugins::hello::operations;
use serde_json::{json, Value};
use std::path::Path;
use std::fs;
use tracing::{info, error};

pub struct HelloPlugin {
    name: String,
    description: String,
    data_path: String,
}

impl HelloPlugin {
    pub fn new(data_base_path: &str) -> Self {
        HelloPlugin {
            name: "hello".to_string(),
            description: "AI agent introduction protocol".to_string(),
            data_path: format!("{}/hello/config/data.json", data_base_path),
        }
    }

    fn load_hello_config(&self) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let config_path = Path::new(&self.data_path);
        if config_path.exists() {
            info!("Loading Hello config from: {}", config_path.display());
            let data = fs::read_to_string(config_path)?;
            Ok(serde_json::from_str(&data)?)
        } else {
            info!("No Hello config found, using defaults");
            // Fall back to generating a basic hello response
            Ok(json!({
                "default": {
                    "introduction": "Hello! I'm the AI assistant for this website. How can I assist you today?",
                    "metadata": {
                        "provider": {
                            "name": "MCPI Service",
                            "domain": "mcpintegrate.com"
                        }
                    }
                }
            }))
        }
    }
}

impl McpPlugin for HelloPlugin {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> &str {
        &self.description
    }
    
    fn category(&self) -> &str {
        "agent"
    }
    
    fn plugin_type(&self) -> PluginType {
        PluginType::Core
    }
    
    fn supported_operations(&self) -> Vec<String> {
        vec!["HELLO".to_string()]
    }
    
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["HELLO"],
                    "description": "Get an introduction from the website's AI assistant"
                },
                "context": {
                    "type": "string",
                    "description": "Optional context about requester's intent (e.g., shopping, support)"
                },
                "detail_level": {
                    "type": "string",
                    "enum": ["basic", "standard", "detailed"],
                    "description": "Amount of detail to include in the response"
                }
            },
            "required": ["operation"]
        })
    }
    
    fn execute(&self, operation: &str, params: &Value) -> PluginResult {
        match operation {
            "HELLO" => {
                // Extract optional parameters
                let context = params.get("context").and_then(|c| c.as_str()).unwrap_or("");
                let detail_level = params.get("detail_level").and_then(|d| d.as_str()).unwrap_or("standard");
                
                // Get hello configuration
                let hello_config = self.load_hello_config()?;
                
                // Generate appropriate response based on context and detail level
                operations::generate_hello_response(hello_config, context, detail_level)
            },
            _ => Err(format!("Unsupported operation: {}", operation).into())
        }
    }
    
    fn get_resources(&self) -> Vec<(String, String, Option<String>)> {
        vec![(
            self.name.clone(),
            format!("mcpi://provider/resources/hello_config.json"),
            Some("Hello protocol configuration".to_string()),
        )]
    }
}