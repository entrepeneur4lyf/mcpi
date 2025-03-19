// mcpi-server/src/plugin_registry.rs

use mcpi_common::{McpPlugin, PluginResult};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Registry that manages all available plugins
pub struct PluginRegistry {
    plugins: RwLock<HashMap<String, Arc<dyn McpPlugin>>>,
}

impl PluginRegistry {
    /// Create a new plugin registry
    pub fn new() -> Self {
        PluginRegistry {
            plugins: RwLock::new(HashMap::new()),
        }
    }

    /// Register a plugin with the registry
    pub fn register_plugin(&self, plugin: Arc<dyn McpPlugin>) -> Result<(), String> {
        let name = plugin.name().to_string();
        let mut plugins = self.plugins.write().unwrap();
        
        if plugins.contains_key(&name) {
            return Err(format!("Plugin '{}' is already registered", name));
        }
        
        plugins.insert(name, plugin);
        Ok(())
    }

    /// Get a plugin by name
    pub fn get_plugin(&self, name: &str) -> Option<Arc<dyn McpPlugin>> {
        let plugins = self.plugins.read().unwrap();
        plugins.get(name).cloned()
    }

    /// Get all registered plugins
    pub fn get_all_plugins(&self) -> Vec<Arc<dyn McpPlugin>> {
        let plugins = self.plugins.read().unwrap();
        plugins.values().cloned().collect()
    }

    /// Execute a plugin operation
    pub fn execute_plugin(&self, name: &str, operation: &str, params: &Value) -> PluginResult {
        if let Some(plugin) = self.get_plugin(name) {
            plugin.execute(operation, params)
        } else {
            Err(format!("Plugin '{}' not found", name).into())
        }
    }
}