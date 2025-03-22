// mcpi-server/src/plugins/store_plugin.rs
use mcpi_common::{McpPlugin, PluginResult};
use mcpi_common::plugin::PluginType;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub struct StorePlugin {
    name: String,
    description: String,
    data_path: String,
    product_data_file: String,
    customer_data_file: String,
    order_data_file: String,
    review_data_file: String,
}

impl StorePlugin {
    pub fn new(data_path: &str) -> Self {
        StorePlugin {
            name: "store".to_string(),
            description: "E-commerce store functionality".to_string(),
            data_path: data_path.to_string(),
            product_data_file: "products.json".to_string(),
            customer_data_file: "customers.json".to_string(),
            order_data_file: "orders.json".to_string(),
            review_data_file: "reviews.json".to_string(),
        }
    }

    fn load_data(&self, data_file: &str) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let data_path = Path::new(&self.data_path).join(data_file);
        let data = fs::read_to_string(data_path)?;
        let parsed: Value = serde_json::from_str(&data)?;
        Ok(parsed)
    }

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
        
        Ok(json!({
            "results": filtered_items,
            "count": filtered_items.len(),
            "query": query,
            "field": field
        }))
    }

    fn get_item(&self, data: &Value, id: &str) -> PluginResult {
        let default_items = Vec::new();
        let items = data.as_array().unwrap_or(&default_items);
        
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
    }

    fn list_items(&self, data: &Value) -> PluginResult {
        Ok(json!({
            "results": data,
            "count": data.as_array().map(|a| a.len()).unwrap_or(0)
        }))
    }
}

impl McpPlugin for StorePlugin {
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
        vec!["SEARCH_PRODUCTS".to_string(), "GET_PRODUCT".to_string(), "LIST_PRODUCTS".to_string(),
             "SEARCH_CUSTOMERS".to_string(), "GET_CUSTOMER".to_string(), "LIST_CUSTOMERS".to_string(),
             "SEARCH_ORDERS".to_string(), "GET_ORDER".to_string(), "LIST_ORDERS".to_string(),
             "SEARCH_REVIEWS".to_string(), "GET_REVIEW".to_string(), "LIST_REVIEWS".to_string()]
    }
    
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["SEARCH_PRODUCTS", "GET_PRODUCT", "LIST_PRODUCTS",
                             "SEARCH_CUSTOMERS", "GET_CUSTOMER", "LIST_CUSTOMERS",
                             "SEARCH_ORDERS", "GET_ORDER", "LIST_ORDERS",
                             "SEARCH_REVIEWS", "GET_REVIEW", "LIST_REVIEWS"],
                    "description": "Operation to perform"
                },
                "query": {
                    "type": "string",
                    "description": "Query string for SEARCH operations"
                },
                "id": {
                    "type": "string",
                    "description": "ID for GET operations"  
                },
                "field": {
                    "type": "string",
                    "description": "Field to search in for SEARCH operations"
                }
            },
            "required": ["operation"]
        })
    }
    
    fn execute(&self, operation: &str, params: &Value) -> PluginResult {
        // Determine which data file to use based on operation
        let data_file = match operation {
            op if op.contains("PRODUCT") => &self.product_data_file,
            op if op.contains("CUSTOMER") => &self.customer_data_file,
            op if op.contains("ORDER") => &self.order_data_file,
            op if op.contains("REVIEW") => &self.review_data_file,
            _ => return Err(format!("Unsupported operation: {}", operation).into())
        };
        
        // Load appropriate data
        let data = self.load_data(data_file)?;
        
        // Process based on operation type
        if operation.starts_with("SEARCH") {
            let query = params.get("query").and_then(|q| q.as_str()).unwrap_or("");
            let field = params.get("field").and_then(|f| f.as_str()).unwrap_or("name");
            self.search_items(&data, query, field)
        } else if operation.starts_with("GET") {
            let id = params.get("id").and_then(|i| i.as_str()).unwrap_or("");
            self.get_item(&data, id)
        } else if operation.starts_with("LIST") {
            self.list_items(&data)
        } else {
            Err(format!("Unsupported operation: {}", operation).into())
        }
    }
    
    fn get_resources(&self) -> Vec<(String, String, Option<String>)> {
        vec![
            (
                "products".to_string(),
                format!("mcpi://provider/resources/{}", self.product_data_file),
                Some("Product catalog data".to_string()),
            ),
            (
                "customers".to_string(),
                format!("mcpi://provider/resources/{}", self.customer_data_file),
                Some("Customer information".to_string()),
            ),
            (
                "orders".to_string(),
                format!("mcpi://provider/resources/{}", self.order_data_file),
                Some("Order history data".to_string()),
            ),
            (
                "reviews".to_string(),
                format!("mcpi://provider/resources/{}", self.review_data_file),
                Some("Product reviews data".to_string()),
            ),
        ]
    }
}