// mcpi-server/src/plugin_registry.rs
use mcpi_common::{McpPlugin, PluginResult};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::error::Error;
use tracing::info;

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
    pub fn register_plugin(&self, plugin: Arc<dyn McpPlugin>) -> Result<(), Box<dyn Error + Send + Sync>> {
        let name = plugin.name().to_string();
        let mut plugins = self.plugins.write().unwrap();
        
        if plugins.contains_key(&name) {
            return Err(format!("Plugin '{}' is already registered", name).into());
        }
        
        plugins.insert(name.clone(), plugin);
        info!("Registered plugin: {}", name);
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
    
    /// Register all plugins
    pub fn register_all_plugins(&self, data_path: &str, referrals: Value) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Register all core plugins
        self.register_core_plugins(data_path, referrals.clone())?;
        
        // Register all extension plugins
        self.register_extension_plugins()?;
        
        Ok(())
    }
    
    /// Register core plugins
    fn register_core_plugins(&self, data_path: &str, referrals: Value) -> Result<(), Box<dyn Error + Send + Sync>> {
        use crate::plugins::{
            hello, store, website, social
        };
        
        // Register hello plugin
        let hello_plugin = hello::create_plugin(data_path)?;
        self.register_plugin(hello_plugin)?;
        
        // Register website plugin
        let website_plugin = website::create_plugin(data_path)?;
        self.register_plugin(website_plugin)?;
        
        // Register store plugins - use the vector returned from create_plugins
        let store_plugins = store::create_plugins(data_path)?;
        for plugin in store_plugins {
            self.register_plugin(plugin)?;
        }
        
        // Register social plugin
        let social_plugin = social::create_plugin(data_path, referrals)?;
        self.register_plugin(social_plugin)?;
        
        Ok(())
    }
    
    /// Register extension plugins
    fn register_extension_plugins(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        use crate::plugins::weather;
        
        // Register weather plugin
        let weather_plugin = weather::create_plugin()?;
        self.register_plugin(weather_plugin)?;
        
        Ok(())
    }
}