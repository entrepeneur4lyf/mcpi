// mcpi-common/src/json_plugin.rs
use crate::plugin::{McpPlugin, PluginResult};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use tracing::{info, warn};

/// A trait that specifies JSON data capabilities
pub trait JsonDataCapable: Send + Sync {
    /// Get the path to the data file
    fn get_data_path(&self) -> &str;
    
    /// Load JSON data from the file
    fn load_data(&self) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let data_path = Path::new(self.get_data_path());
        info!("Loading data from file: {}", data_path.display());
        
        if !data_path.exists() {
            return Err(format!("Data file does not exist: {}", data_path.display()).into());
        }
        
        let data = fs::read_to_string(data_path)?;
        let parsed: Value = serde_json::from_str(&data)?;
        Ok(parsed)
    }
    
    /// Search for items in data matching a query
    fn search_items(&self, data: &Value, query: &str, field: &str) -> PluginResult {
        let default_items = Vec::new();
        let items = data.as_array().unwrap_or(&default_items);
        
        let filtered_items: Vec<Value> = items
            .iter()
            .filter(|item| {
                let field_value = item.get(field).and_then(|f| f.as_str()).unwrap_or("");
                query.is_empty() || field_value.to_lowercase().contains(&query.to_lowercase())
            })
            .cloned()
            .collect();
        
        info!("Search operation completed. Found {} items.", filtered_items.len());
        
        Ok(json!({
            "results": filtered_items,
            "count": filtered_items.len(),
            "query": query,
            "field": field
        }))
    }
    
    /// Get a specific item by ID
    fn get_item(&self, data: &Value, id: &str) -> PluginResult {
        let default_items = Vec::new();
        let items = data.as_array().unwrap_or(&default_items);
        
        let item = items
            .iter()
            .find(|i| i.get("id").and_then(|id_val| id_val.as_str()) == Some(id))
            .cloned();
        
        match item {
            Some(i) => {
                info!("Get operation completed. Found item with ID: {}", id);
                Ok(i)
            },
            None => {
                warn!("Item not found with ID: {}", id);
                Ok(json!({
                    "error": "Item not found",
                    "id": id
                }))
            }
        }
    }
    
    /// List all items
    fn list_items(&self, data: &Value) -> PluginResult {
        let count = data.as_array().map(|a| a.len()).unwrap_or(0);
        info!("List operation completed. Returning {} items.", count);
        
        Ok(json!({
            "results": data,
            "count": count
        }))
    }
}

/// A plugin that handles JSON data
pub struct JsonDataPlugin<T: JsonDataCapable + Send + Sync> {
    provider: T,
}

impl<T: JsonDataCapable + Send + Sync> JsonDataPlugin<T> {
    pub fn new(provider: T) -> Self {
        JsonDataPlugin { provider }
    }
}

impl<T: JsonDataCapable + McpPlugin + Send + Sync> McpPlugin for JsonDataPlugin<T> {
    fn name(&self) -> &str {
        self.provider.name()
    }
    
    fn description(&self) -> &str {
        self.provider.description()
    }
    
    fn category(&self) -> &str {
        self.provider.category()
    }
    
    fn plugin_type(&self) -> crate::plugin::PluginType {
        self.provider.plugin_type()
    }
    
    fn supported_operations(&self) -> Vec<String> {
        self.provider.supported_operations()
    }
    
    fn input_schema(&self) -> Value {
        self.provider.input_schema()
    }
    
    fn execute(&self, operation: &str, params: &Value) -> PluginResult {
        // First load the data
        let data = match self.provider.load_data() {
            Ok(data) => data,
            Err(e) => return Err(format!("Failed to load data: {}", e).into()),
        };
        
        // Process based on operation type
        match operation {
            op if op.contains("SEARCH") => {
                let query = params.get("query").and_then(|q| q.as_str()).unwrap_or("");
                let field = params.get("field").and_then(|f| f.as_str()).unwrap_or("name");
                self.provider.search_items(&data, query, field)
            },
            op if op.contains("GET") => {
                let id = params.get("id").and_then(|i| i.as_str()).unwrap_or("");
                self.provider.get_item(&data, id)
            },
            op if op.contains("LIST") => {
                self.provider.list_items(&data)
            },
            _ => {
                // For any other operations, delegate to the provider
                // But most plugins won't have custom operations so they'll just return errors
                self.provider.execute(operation, params)
            }
        }
    }
    
    fn get_resources(&self) -> Vec<(String, String, Option<String>)> {
        self.provider.get_resources()
    }
    
    fn get_capabilities(&self) -> Vec<String> {
        self.provider.get_capabilities()
    }
}