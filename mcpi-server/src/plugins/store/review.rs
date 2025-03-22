use mcpi_common::{McpPlugin, PluginResult, plugin::PluginType};
use mcpi_common::json_plugin::JsonDataCapable;
use serde_json::{json, Value};

pub struct ReviewPlugin {
    name: String,
    description: String,
    data_path: String,
}

impl ReviewPlugin {
    pub fn new(data_base_path: &str) -> Self {
        ReviewPlugin {
            name: "store_review".to_string(),
            description: "E-commerce review functionality".to_string(),
            data_path: format!("{}/store/reviews/data.json", data_base_path),
        }
    }
}

impl JsonDataCapable for ReviewPlugin {
    fn get_data_path(&self) -> &str {
        &self.data_path
    }
}

impl McpPlugin for ReviewPlugin {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> &str {
        &self.description
    }
    
    fn category(&self) -> &str {
        "commerce"
    }
    
    fn plugin_type(&self) -> PluginType {
        PluginType::Core
    }
    
    fn supported_operations(&self) -> Vec<String> {
        vec!["SEARCH_REVIEWS".to_string(), "GET_REVIEW".to_string(), "LIST_REVIEWS".to_string()]
    }
    
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["SEARCH_REVIEWS", "GET_REVIEW", "LIST_REVIEWS"],
                    "description": "Operation to perform"
                },
                "query": {
                    "type": "string",
                    "description": "Query string for SEARCH_REVIEWS operation"
                },
                "id": {
                    "type": "string",
                    "description": "ID for GET_REVIEW operation"  
                },
                "field": {
                    "type": "string",
                    "description": "Field to search in for SEARCH_REVIEWS operation"
                }
            },
            "required": ["operation"]
        })
    }
    
    fn execute(&self, operation: &str, params: &Value) -> PluginResult {
        Err("This method is overridden by JsonDataPlugin".into())
    }
    
    fn get_resources(&self) -> Vec<(String, String, Option<String>)> {
        vec![(
            "reviews".to_string(),
            format!("mcpi://provider/resources/store/reviews/data.json"),
            Some("Review data".to_string()),
        )]
    }
}