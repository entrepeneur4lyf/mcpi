use mcpi_common::{McpPlugin, PluginResult, plugin::PluginType};
use mcpi_common::json_plugin::JsonDataCapable;
use serde_json::{json, Value};

pub struct CustomerPlugin {
    name: String,
    description: String,
    data_path: String,
}

impl CustomerPlugin {
    pub fn new(data_base_path: &str) -> Self {
        CustomerPlugin {
            name: "store_customer".to_string(),
            description: "E-commerce customer functionality".to_string(),
            data_path: format!("{}/store/customers/data.json", data_base_path),
        }
    }
}


impl JsonDataCapable for CustomerPlugin {
    fn get_data_path(&self) -> &str {
        &self.data_path
    }
}

impl McpPlugin for CustomerPlugin {
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
        vec!["SEARCH_CUSTOMERS".to_string(), "GET_CUSTOMER".to_string(), "LIST_CUSTOMERS".to_string()]
    }
    
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["SEARCH_CUSTOMERS", "GET_CUSTOMER", "LIST_CUSTOMERS"],
                    "description": "Operation to perform"
                },
                "query": {
                    "type": "string",
                    "description": "Query string for SEARCH_CUSTOMERS operation"
                },
                "id": {
                    "type": "string",
                    "description": "ID for GET_CUSTOMER operation"  
                },
                "field": {
                    "type": "string",
                    "description": "Field to search in for SEARCH_CUSTOMERS operation"
                }
            },
            "required": ["operation"]
        })
    }
    
    // Fix the execute method - it should never actually get called directly
    // if JsonDataPlugin is working correctly, but handle it gracefully just in case
    fn execute(&self, _operation: &str, _params: &Value) -> PluginResult {
        // This is only used if not wrapped with JsonDataPlugin
        Err("Please use JsonDataPlugin wrapper for this plugin".into())
    }
    
    fn get_resources(&self) -> Vec<(String, String, Option<String>)> {
        vec![(
            "customers".to_string(),
            format!("mcpi://provider/resources/store/customers/data.json"),
            Some("Customer data".to_string()),
        )]
    }
}