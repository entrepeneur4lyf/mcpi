// mcpi-common/src/plugin.rs
use serde_json::Value;
use std::error::Error;
use crate::{ContentItem, ToolAnnotation}; // Import necessary types from mcpi-common's lib.rs

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
        // Default implementation might not be very useful.
        // Plugins should likely override this.
        Vec::new()
        /* Example if plugin name relates to a single resource:
        vec![(
            self.name().to_string(), // Resource Name
            format!("{}/default", self.name()), // Resource URI Suffix (e.g., "weather/default")
            Some(self.description().to_string()), // Resource Description
        )]
        */
    }

    // --- NEW METHODS TO FIX E0599 ERRORS ---

    /// Read a specific resource provided by this plugin.
    /// The `resource_suffix` corresponds to the part of the URI after the plugin name.
    /// e.g., if URI is mcpi://domain/resources/store/products/item123, suffix is "products/item123"
    fn read_resource(&self, resource_suffix: &str) -> Result<ContentItem, Box<dyn Error + Send + Sync>> {
        // Default implementation returns an error indicating not supported.
        // Plugins providing resources MUST override this method.
        Err(format!("Plugin '{}' does not support reading resource '{}'", self.name(), resource_suffix).into())
    }

    /// Get annotations for this plugin when listed as a tool.
    /// Used for the `tools/list` response.
    fn get_tool_annotations(&self) -> Option<ToolAnnotation> {
        // Default: No specific annotations.
        // Plugins should override this to provide relevant annotations (read_only, cost, etc.).
        None
    }

    /// Provide completion suggestions for a given method and parameter.
    /// `param_name`: The name of the parameter being completed (e.g., "name", "arguments.operation", "arguments.location").
    /// `partial_value`: The current partial value entered by the user.
    /// `context`: Other parameters provided in the completion request's context (e.g., the tool name if completing arguments).
    fn get_completions(&self, param_name: &str, partial_value: &Value, context: &Value) -> Vec<Value> {
        // Default: No suggestions.
        // Plugins should override this to provide context-aware suggestions.
        let _ = (param_name, partial_value, context); // Avoid unused variable warnings in default impl
        Vec::new()
    }
}

/// Simplified result type for plugin operations (used by `execute`)
pub type PluginResult = Result<Value, Box<dyn Error + Send + Sync>>;