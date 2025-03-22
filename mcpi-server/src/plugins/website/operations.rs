// mcpi-server/src/plugins/website/operations.rs
use mcpi_common::PluginResult;
use serde_json::{json, Value};
use tracing::info;

/// Custom LIST operation with filtering and sorting
pub fn list_with_filters(data: &Value, params: &Value) -> PluginResult {
    let content_type = params.get("type").and_then(|t| t.as_str());
    let sort_by = params.get("sort_by").and_then(|s| s.as_str()).unwrap_or("id");
    let sort_order = params.get("order").and_then(|o| o.as_str()).unwrap_or("asc");
    
    let default_items = Vec::new();
    let items = data.as_array().unwrap_or(&default_items);
    
    // Filter by type if specified
    let mut filtered_items: Vec<Value> = if let Some(type_filter) = content_type {
        info!("Filtering website content by type: {}", type_filter);
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
        info!("Sorting website content by date, order: {}", sort_order);
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
    
    info!("List operation completed with filters. Returning {} items.", filtered_items.len());
    
    Ok(json!({
        "results": filtered_items,
        "count": filtered_items.len(),
        "type": content_type,
        "sort_by": sort_by,
        "order": sort_order
    }))
}