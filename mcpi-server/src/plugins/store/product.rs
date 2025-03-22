// mcpi-server/src/plugins/store/product.rs
use mcpi_common::{McpPlugin, PluginResult, plugin::PluginType};
use mcpi_common::json_plugin::JsonDataCapable;
use serde_json::{json, Value};

pub struct ProductPlugin {
    name: String,
    description: String,
    data_path: String,
}

impl ProductPlugin {
    pub fn new(data_base_path: &str) -> Self {
        ProductPlugin {
            name: "store_product".to_string(),
            description: "E-commerce product functionality".to_string(),
            data_path: format!("{}/store/products/data.json", data_base_path),
        }
    }
}

impl JsonDataCapable for ProductPlugin {
    fn get_data_path(&self) -> &str {
        &self.data_path
    }
}

impl McpPlugin for ProductPlugin {
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
        vec!["SEARCH_PRODUCTS".to_string(), "GET_PRODUCT".to_string(), "LIST_PRODUCTS".to_string()]
    }
    
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["SEARCH_PRODUCTS", "GET_PRODUCT", "LIST_PRODUCTS"],
                    "description": "Operation to perform"
                },
                "query": {
                    "type": "string",
                    "description": "Query string for SEARCH_PRODUCTS operation"
                },
                "id": {
                    "type": "string",
                    "description": "ID for GET_PRODUCT operation"  
                },
                "field": {
                    "type": "string",
                    "description": "Field to search in for SEARCH_PRODUCTS operation"
                }
            },
            "required": ["operation"]
        })
    }
    
    // This is a default implementation that will be overridden by JsonDataPlugin
    fn execute(&self, operation: &str, params: &Value) -> PluginResult {
        Err("This method is overridden by JsonDataPlugin".into())
    }
    
    fn get_resources(&self) -> Vec<(String, String, Option<String>)> {
        vec![(
            "products".to_string(),
            format!("mcpi://provider/resources/store/products/data.json"),
            Some("Product catalog data".to_string()),
        )]
    }
}