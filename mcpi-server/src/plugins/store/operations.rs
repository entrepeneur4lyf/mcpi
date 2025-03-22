// mcpi-server/src/plugins/store/operations.rs
use mcpi_common::PluginResult;
use serde_json::{json, Value};
use tracing::info;

// Common operations for store plugins can be defined here if needed.
// Currently, most operations are handled by the JsonDataPlugin.

// Example of a custom operation that could be added in the future
pub fn calculate_product_stats(products: &Value) -> PluginResult {
    // Create Vec before using it to avoid temporary value issues
    let empty_vec = Vec::new();
    let products_array = products.as_array().unwrap_or(&empty_vec);
    
    let total_products = products_array.len();
    let in_stock_count = products_array.iter()
        .filter(|p| p.get("inStock").and_then(|v| v.as_bool()).unwrap_or(false))
        .count();
    
    let avg_price = if total_products > 0 {
        let total_price: f64 = products_array.iter()
            .filter_map(|p| p.get("price").and_then(|v| v.as_f64()))
            .sum();
        total_price / total_products as f64
    } else {
        0.0
    };
    
    info!("Calculated product stats: {} total, {} in stock, ${:.2} avg price", 
          total_products, in_stock_count, avg_price);
    
    Ok(json!({
        "total_products": total_products,
        "in_stock_count": in_stock_count,
        "out_of_stock_count": total_products - in_stock_count,
        "average_price": avg_price
    }))
}