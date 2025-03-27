// mcpi-common/src/plugin.rs
use serde_json::Value;
use std::error::Error;
// *** UPDATED Import ***
use crate::{ContentItem, ToolAnnotations}; // Use ToolAnnotations based on lib.rs changes

// Plugin type to distinguish between core and extension plugins
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginType {
    Core,    // Built-in core functionality
    Extension, // Add-on functionality
}

/// Trait that defines the interface for MCPI plugins
pub trait McpPlugin: Send + Sync {
    /// Get the unique name of this plugin
    fn name(&self) -> &str;

    /// Get plugin description
    fn description(&self) -> &str;

    /// Get the category this plugin belongs to
    fn category(&self) -> &str;

    /// Get the type of this plugin
    fn plugin_type(&self) -> PluginType {
        PluginType::Extension // Default to extension
    }

    /// Get list of operations this plugin supports
    fn supported_operations(&self) -> Vec<String>;

    /// Get the input schema for this plugin's `execute` method (specifically for tools/call)
    fn input_schema(&self) -> Value;

    /// Execute an operation on this plugin (typically for tools/call)
    fn execute(&self, operation: &str, params: &Value) -> Result<Value, Box<dyn Error + Send + Sync>>;

    /// Get capabilities this plugin provides (legacy or specific use?)
    /// Consider if this is still needed or if `ServerCapabilities` in main is sufficient.
    fn get_capabilities(&self) -> Vec<String> {
        vec![self.name().to_string()]
    }

    /// Get resources this plugin provides (name, uri_suffix, description)
    /// Used for the `resources/list` response.
    fn get_resources(&self) -> Vec<(String, String, Option<String>)> {
        Vec::new() // Default to empty
    }

    // --- NEW METHODS ---

    /// Read a specific resource provided by this plugin.
    fn read_resource(&self, resource_suffix: &str) -> Result<ContentItem, Box<dyn Error + Send + Sync>> {
        Err(format!("Plugin '{}' does not support reading resource '{}'", self.name(), resource_suffix).into())
    }

    /// Get annotations for this plugin when listed as a tool.
    // *** UPDATED Return Type ***
    fn get_tool_annotations(&self) -> Option<ToolAnnotations> { // Use ToolAnnotations
        None // Default: No specific annotations.
    }

    /// Provide completion suggestions for a given method and parameter.
    fn get_completions(&self, param_name: &str, partial_value: &Value, context: &Value) -> Vec<Value> {
        let _ = (param_name, partial_value, context); // Avoid unused warnings
        Vec::new() // Default: No suggestions.
    }
}

/// Simplified result type for plugin operations (used by `execute`)
pub type PluginResult = Result<Value, Box<dyn Error + Send + Sync>>;