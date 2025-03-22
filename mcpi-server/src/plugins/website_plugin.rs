// mcpi-server/src/plugins/website_plugin.rs
use mcpi_common::{McpPlugin, PluginResult};
use mcpi_common::plugin::PluginType;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub struct WebsitePlugin {
    name: String,
    description: String,
    data_path: String,
    content_file: String,
}

impl WebsitePlugin {
    pub fn new(data_path: &str) -> Self {
        WebsitePlugin {
            name: "website".to_string(),
            description: "Access website content including news, about, contact, etc.".to_string(),
            data_path: data_path.to_string(),
            content_file: "website_content.json".to_string(),
        }
    }

    fn load_data(&self) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let data_path = Path::new(&self.data_path).join(&self.content_file);
        let data = fs::read_to_string(data_path)?;
        let parsed: Value = serde_json::from_str(&data)?;
        Ok(parsed)
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
        // Load website content data
        let data = self.load_data()?;
        let default_items = Vec::new();
        let items = data.as_array().unwrap_or(&default_items);
        
        match operation {
            "SEARCH" => {
                let query = params.get("query").and_then(|q| q.as_str()).unwrap_or("");
                
                // Search across all content
                let filtered_items: Vec<Value> = items
                    .iter()
                    .filter(|item| {
                        let content = item.get("content").and_then(|c| c.as_str()).unwrap_or("");
                        let title = item.get("title").and_then(|t| t.as_str()).unwrap_or("");
                        
                        content.to_lowercase().contains(&query.to_lowercase()) || 
                        title.to_lowercase().contains(&query.to_lowercase())
                    })
                    .cloned()
                    .collect();
                
                Ok(json!({
                    "results": filtered_items,
                    "count": filtered_items.len(),
                    "query": query
                }))
            },
            "GET" => {
                let id = params.get("id").and_then(|i| i.as_str()).unwrap_or("");
                
                let item = items
                    .iter()
                    .find(|i| i.get("id").and_then(|id_val| id_val.as_str()) == Some(id))
                    .cloned();
                
                match item {
                    Some(i) => Ok(i),
                    None => Ok(json!({
                        "error": "Content not found",
                        "id": id
                    }))
                }
            },
            "LIST" => {
                let content_type = params.get("type").and_then(|t| t.as_str());
                let sort_by = params.get("sort_by").and_then(|s| s.as_str()).unwrap_or("id");
                let sort_order = params.get("order").and_then(|o| o.as_str()).unwrap_or("asc");
                
                // Filter by type if specified
                let mut filtered_items: Vec<Value> = if let Some(type_filter) = content_type {
                    items
                        .iter()
                        .filter(|item| {
                            item.get("page_type").and_then(|pt| pt.as_str()) == Some(type_filter)
                        })
                        .cloned()
                        .collect()
                } else {
                    items.clone()
                };
                
                // Sort items if needed
                if sort_by == "date" {
                    filtered_items.sort_by(|a, b| {
                        let a_date = a.get("date").and_then(|d| d.as_str()).unwrap_or("")
                            .cmp(b.get("date").and_then(|d| d.as_str()).unwrap_or(""));
                        
                        if sort_order == "desc" {
                            a_date.reverse()
                        } else {
                            a_date
                        }
                    });
                }
                
                Ok(json!({
                    "results": filtered_items,
                    "count": filtered_items.len(),
                    "type": content_type,
                    "sort_by": sort_by,
                    "order": sort_order
                }))
            },
            _ => Err(format!("Unsupported operation: {}", operation).into())
        }
    }
    
    fn get_resources(&self) -> Vec<(String, String, Option<String>)> {
        vec![(
            self.name.clone(),
            format!("mcpi://provider/resources/{}", self.content_file),
            Some(self.description.clone()),
        )]
    }
}