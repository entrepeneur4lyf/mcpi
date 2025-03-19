use crate::plugin::{McpPlugin, PluginResult};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

/// A base implementation for plugins that source data from JSON files
pub struct JsonDataPlugin {
    name: String,
    description: String,
    category: String,
    operations: Vec<String>,
    data_file: String,
    data_path: String,
}

impl JsonDataPlugin {
    pub fn new(
        name: &str,
        description: &str,
        category: &str,
        operations: Vec<String>,
        data_file: &str,
        data_path: &str,
    ) -> Self {
        JsonDataPlugin {
            name: name.to_string(),
            description: description.to_string(),
            category: category.to_string(),
            operations,
            data_file: data_file.to_string(),
            data_path: data_path.to_string(),
        }
    }

    /// Load the underlying data file
    pub fn load_data(&self) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let data_path = Path::new(&self.data_path).join(&self.data_file);
        let data = fs::read_to_string(data_path)?;
        let parsed: Value = serde_json::from_str(&data)?;
        Ok(parsed)
    }
}

impl McpPlugin for JsonDataPlugin {
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

    fn input_schema(&self) -> Value {
        json!({
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

    fn execute(&self, operation: &str, params: &Value) -> PluginResult {
        // Check if operation is supported
        if !self.operations.contains(&operation.to_string()) {
            return Err(format!("Operation '{}' not supported for plugin '{}'", operation, self.name).into());
        }

        // Load data
        let data = self.load_data()?;

        // Process based on operation
        match operation {
            "SEARCH" => {
                let query = params.get("query").and_then(|q| q.as_str()).unwrap_or("");
                let field = params.get("field").and_then(|f| f.as_str()).unwrap_or("name");
                
                // Create a longer-lived empty Vec for the unwrap_or case
                let empty_vec = Vec::new();
                let items = data.as_array().unwrap_or(&empty_vec);
                
                // Filter items based on query
                let filtered_items: Vec<Value> = items
                    .iter()
                    .filter(|item| {
                        let field_value = item.get(field).and_then(|f| f.as_str()).unwrap_or("");
                        query.is_empty() || field_value.to_lowercase().contains(&query.to_lowercase())
                    })
                    .cloned()
                    .collect();
                
                Ok(json!({
                    "results": filtered_items,
                    "count": filtered_items.len(),
                    "query": query,
                    "field": field
                }))
            },
            "GET" => {
                let id = params.get("id").and_then(|i| i.as_str()).unwrap_or("");
                
                // Create a longer-lived empty Vec for the unwrap_or case
                let empty_vec = Vec::new();
                let items = data.as_array().unwrap_or(&empty_vec);
                
                // Find item by ID
                let item = items
                    .iter()
                    .find(|i| i.get("id").and_then(|id_val| id_val.as_str()) == Some(id))
                    .cloned();
                
                match item {
                    Some(i) => Ok(i),
                    None => Ok(json!({
                        "error": "Item not found",
                        "id": id
                    }))
                }
            },
            "LIST" => {
                Ok(json!({
                    "results": data,
                    "count": data.as_array().map(|a| a.len()).unwrap_or(0)
                }))
            },
            _ => Err(format!("Unsupported operation '{}'", operation).into())
        }
    }

    fn get_resources(&self) -> Vec<(String, String, Option<String>)> {
        vec![(
            self.name.clone(),
            format!("mcpi://provider/resources/{}", self.data_file),
            Some(self.description.clone()),
        )]
    }
}