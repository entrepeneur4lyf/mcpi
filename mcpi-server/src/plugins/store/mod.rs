mod product;
mod customer;
mod order;
mod review;
mod operations;

// Do NOT use "customer::" - just use the actual exports from the files
pub use product::ProductPlugin;
pub use customer::CustomerPlugin;
pub use order::OrderPlugin;
pub use review::ReviewPlugin;

use mcpi_common::{JsonDataPlugin, McpPlugin};
use std::error::Error;
use std::sync::Arc;

/// Create all store plugins
pub fn create_plugins(data_path: &str) -> Result<Vec<Arc<dyn McpPlugin>>, Box<dyn Error + Send + Sync>> {
    // Create instances of each plugin and wrap them with JsonDataPlugin
    Ok(vec![
        Arc::new(JsonDataPlugin::new(ProductPlugin::new(data_path))),
        Arc::new(JsonDataPlugin::new(CustomerPlugin::new(data_path))),
        Arc::new(JsonDataPlugin::new(OrderPlugin::new(data_path))),
        Arc::new(JsonDataPlugin::new(ReviewPlugin::new(data_path))),
    ])
}