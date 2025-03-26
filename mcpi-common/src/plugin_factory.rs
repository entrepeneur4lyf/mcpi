// mcpi-common/src/plugin_factory.rs
use crate::json_plugin::{JsonDataCapable, JsonDataPlugin};
use crate::McpPlugin;
use std::sync::Arc;

// Define a basic JsonData implementation that can be used by the factory
struct BasicJsonData {
    name: String,
    description: String,
    category: String,
    operations: Vec<String>,
    data_file: String,
    data_path: String,
}

impl BasicJsonData {
    fn new(
        name: &str,
        description: &str,
        category: &str,
        operations: Vec<String>,
        data_file: &str,
        data_path: &str,
    ) -> Self {
        BasicJsonData {
            name: name.to_string(),
            description: description.to_string(),
            category: category.to_string(),
            operations,
            data_file: data_file.to_string(),
            data_path: data_path.to_string(),
        }
    }
}

impl JsonDataCapable for BasicJsonData {
    fn get_data_path(&self) -> &str {
        &self.data_path
    }
}

impl McpPlugin for BasicJsonData {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> &str {
        &self.description
    }
    
    fn category(&self) -> &str {
        &self.category
    }
    
    fn supported_operations(&self) -> Vec<String> {
        self.operations.clone()
    }
    
    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": self.operations,
                    "description": "Operation to perform"
                },
                "query": {
                    "type": "string",
                    "description": "Query string for SEARCH operation"
                },
                "id": {
                    "type": "string",
                    "description": "ID for GET operation"  
                },
                "field": {
                    "type": "string",
                    "description": "Field to search in for SEARCH operation"
                }
            },
            "required": ["operation"]
        })
    }
    
    fn execute(&self, operation: &str, params: &serde_json::Value) -> crate::PluginResult {
        // This will be handled by JsonDataPlugin
        Err("This method is handled by JsonDataPlugin".into())
    }
}

pub struct PluginFactory;

impl PluginFactory {
    pub fn create_plugin_from_config(
        name: &str,
        description: &str,
        category: &str,
        operations: Vec<String>,
        data_file: &str,
        data_path: &str,
    ) -> Arc<dyn McpPlugin> {
        // Create the basic data provider
        let data_provider = BasicJsonData::new(
            name,
            description,
            category,
            operations,
            data_file,
            data_path,
        );
        
        // Wrap it in the JsonDataPlugin
        Arc::new(JsonDataPlugin::new(data_provider))
    }
}