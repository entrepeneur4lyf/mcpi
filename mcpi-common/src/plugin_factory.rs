// mcpi-common/src/plugin_factory.rs

use crate::json_plugin::JsonDataPlugin;
use crate::McpPlugin;
use std::sync::Arc;

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
        // Create a JSON data plugin by default
        Arc::new(JsonDataPlugin::new(
            name,
            description,
            category,
            operations,
            data_file,
            data_path,
        ))
    }
}