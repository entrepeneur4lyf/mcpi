use mcpi_common::{McpPlugin, PluginResult, plugin::PluginType};
use mcpi_common::json_plugin::JsonDataCapable;
use serde_json::{json, Value};

pub struct OrderPlugin {
    name: String,
    description: String,
    data_path: String,
}

impl OrderPlugin {
    pub fn new(data_base_path: &str) -> Self {
        OrderPlugin {
            name: "store_order".to_string(),
            description: "E-commerce order functionality".to_string(),
            data_path: format!("{}/store/orders/data.json", data_base_path),
        }
    }
}

impl JsonDataCapable for OrderPlugin {
    fn get_data_path(&self) -> &str {
        &self.data_path
    }
}

impl McpPlugin for OrderPlugin {
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
        vec!["SEARCH_ORDERS".to_string(), "GET_ORDER".to_string(), "LIST_ORDERS".to_string()]
    }
    
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["SEARCH_ORDERS", "GET_ORDER", "LIST_ORDERS"],
                    "description": "Operation to perform"
                },
                "query": {
                    "type": "string",
                    "description": "Query string for SEARCH_ORDERS operation"
                },
                "id": {
                    "type": "string",
                    "description": "ID for GET_ORDER operation"  
                },
                "field": {
                    "type": "string",
                    "description": "Field to search in for SEARCH_ORDERS operation"
                }
            },
            "required": ["operation"]
        })
    }
    
    fn execute(&self, _operation: &str, _params: &Value) -> PluginResult {
        Err("This method is overridden by JsonDataPlugin".into())
    }
    
    fn get_resources(&self) -> Vec<(String, String, Option<String>)> {
        vec![(
            "orders".to_string(),
            format!("mcpi://provider/resources/store/orders/data.json"),
            Some("Order data".to_string()),
        )]
    }
}