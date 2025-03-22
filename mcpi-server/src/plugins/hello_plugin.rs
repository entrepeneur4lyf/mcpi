// mcpi-server/src/plugins/hello_plugin.rs
use mcpi_common::{McpPlugin, PluginResult};
use mcpi_common::plugin::PluginType;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub struct HelloPlugin {
    name: String,
    description: String,
    data_path: String,
}

impl HelloPlugin {
    pub fn new(data_path: &str) -> Self {
        HelloPlugin {
            name: "hello".to_string(),
            description: "AI agent introduction protocol".to_string(),
            data_path: data_path.to_string(),
        }
    }

    fn load_hello_config(&self) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let config_path = Path::new(&self.data_path).join("hello_config.json");
        if config_path.exists() {
            let data = fs::read_to_string(config_path)?;
            Ok(serde_json::from_str(&data)?)
        } else {
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

    fn generate_hello_response(
        &self,
        config: Value,
        context: &str,
        detail_level: &str
    ) -> PluginResult {
        // Default introduction
        let mut intro_text = config.get("default")
            .and_then(|d| d.get("introduction"))
            .and_then(|i| i.as_str())
            .unwrap_or("Welcome to our website.")
            .to_string();
        
        let mut metadata = config.get("default")
            .and_then(|d| d.get("metadata").cloned())
            .unwrap_or_else(|| json!({}));
        
        // Apply context-specific customization if available
        if !context.is_empty() {
            if let Some(contexts) = config.get("contexts") {
                // Look for exact context match
                if let Some(context_config) = contexts.get(context) {
                    // Override with context-specific introduction if available
                    if let Some(ctx_intro) = context_config.get("introduction").and_then(|i| i.as_str()) {
                        intro_text = ctx_intro.to_string();
                    }
                    
                    // Add context-specific capabilities highlighting
                    if let Some(capabilities) = context_config.get("highlight_capabilities") {
                        metadata["highlight_capabilities"] = capabilities.clone();
                    }
                }
            }
        }
        
        // Adjust detail level
        let result = match detail_level {
            "basic" => {
                // Simplify metadata for basic requests
                let basic_metadata = json!({
                    "provider": metadata.get("provider").cloned().unwrap_or_else(|| json!({}))
                });
                
                json!({
                    "content": [{"type": "text", "text": intro_text}],
                    "metadata": basic_metadata
                })
            },
            "detailed" => {
                // For detailed requests, include everything
                json!({
                    "content": [{"type": "text", "text": intro_text}],
                    "metadata": metadata
                })
            },
            _ => {
                // Standard level is the default
                json!({
                    "content": [{"type": "text", "text": intro_text}],
                    "metadata": {
                        "provider": metadata.get("provider").cloned().unwrap_or_else(|| json!({})),
                        "capabilities": metadata.get("capabilities").cloned().unwrap_or_else(|| json!([])),
                        "topics": metadata.get("primary_focus").cloned().unwrap_or_else(|| json!([]))
                    }
                })
            }
        };
        
        Ok(result)
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
                self.generate_hello_response(hello_config, context, detail_level)
            },
            _ => Err(format!("Unsupported operation: {}", operation).into())
        }
    }
}