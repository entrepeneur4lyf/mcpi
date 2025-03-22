// mcpi-server/src/plugins/website/plugin.rs
use mcpi_common::{McpPlugin, PluginResult, plugin::PluginType};
use mcpi_common::json_plugin::JsonDataCapable;
use serde_json::{json, Value};
use crate::plugins::website::operations;

pub struct WebsitePlugin {
    name: String,
    description: String,
    data_path: String,
}

impl WebsitePlugin {
    pub fn new(data_base_path: &str) -> Self {
        WebsitePlugin {
            name: "website".to_string(),
            description: "Access website content including news, about page, contact info, and more".to_string(),
            data_path: format!("{}/website/content/data.json", data_base_path),
        }
    }
}

impl JsonDataCapable for WebsitePlugin {
    fn get_data_path(&self) -> &str {
        &self.data_path
    }
}

impl McpPlugin for WebsitePlugin {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> &str {
        &self.description
    }
    
    fn category(&self) -> &str {
        "content"
    }
    
    fn plugin_type(&self) -> PluginType {
        PluginType::Core
    }
    
    fn supported_operations(&self) -> Vec<String> {
        vec!["GET".to_string(), "LIST".to_string(), "SEARCH".to_string()]
    }
    
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["GET", "LIST", "SEARCH"],
                    "description": "Operation to perform"
                },
                "query": {
                    "type": "string",
                    "description": "Query string for SEARCH operation"
                },
                "id": {
                    "type": "string",
                    "description": "Content ID for GET operation"  
                },
                "type": {
                    "type": "string",
                    "description": "Content type filter for LIST operation"
                },
                "sort_by": {
                    "type": "string",
                    "description": "Field to sort by for LIST operation"
                },
                "order": {
                    "type": "string",
                    "enum": ["asc", "desc"],
                    "description": "Sort order for LIST operation" 
                }
            },
            "required": ["operation"]
        })
    }
    
    fn execute(&self, operation: &str, params: &Value) -> PluginResult {
        // This will be handled by the JsonDataPlugin, but we need to provide custom
        // handling for the LIST operation with filtering and sorting
        if operation == "LIST" {
            // First load the data
            let data = self.load_data()?;
            
            // Use operations module for custom list handling
            operations::list_with_filters(&data, params)
        } else {
            // For standard operations, we'll let JsonDataPlugin handle it
            Err("Standard operations handled by JsonDataPlugin".into())
        }
    }
    
    fn get_resources(&self) -> Vec<(String, String, Option<String>)> {
        vec![(
            self.name.clone(),
            format!("mcpi://provider/resources/website/content/data.json"),
            Some(self.description.clone()),
        )]
    }
}