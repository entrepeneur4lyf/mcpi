use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Define modules
pub mod plugin;
pub mod json_plugin;
pub mod plugin_factory;

// Re-export for convenience
pub use plugin::{McpPlugin, PluginResult};
pub use json_plugin::JsonDataPlugin;
pub use plugin_factory::PluginFactory;

// Protocol version
pub const MCPI_VERSION: &str = "0.1.0";

// Configuration structure
#[derive(Deserialize, Clone)]
pub struct Config {
    pub provider: Provider,
    pub referrals: Vec<Referral>,
    pub capabilities: HashMap<String, CapabilityConfig>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Provider {
    pub name: String,
    pub domain: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branding: Option<BrandingInfo>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct BrandingInfo {
    pub colors: HashMap<String, String>,
    pub logo: LogoInfo,
    pub typography: HashMap<String, String>,
    pub tone: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct LogoInfo {
    pub vector: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Referral {
    pub name: String,
    pub domain: String,
    pub relationship: String,
}

#[derive(Deserialize, Clone)]
pub struct CapabilityConfig {
    pub name: String,
    pub description: String,
    pub category: String,
    pub operations: Vec<String>,
    pub data_file: String,
}

// Discovery endpoint types
#[derive(Serialize, Deserialize)]
pub struct DiscoveryResponse {
    pub provider: Provider,
    pub mode: String,
    pub capabilities: Vec<CapabilityDescription>,
    pub referrals: Vec<Referral>,
}

#[derive(Serialize, Deserialize)]
pub struct CapabilityDescription {
    pub name: String,
    pub description: String,
    pub category: String,
    pub operations: Vec<String>,
}

// MCP Resource types
#[derive(Serialize, Deserialize)]
pub struct Resource {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub uri: String,
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

// MCP Tool types
#[derive(Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

// MCP types for JSON-RPC
#[derive(Deserialize, Serialize, Debug)]  // Added Debug derive here
pub struct MCPRequest {
    pub id: serde_json::Value,
    #[serde(default = "default_jsonrpc")]
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

pub fn default_jsonrpc() -> String {
    "2.0".to_string()
}

#[derive(Deserialize, Serialize)]
pub struct MCPResponse {
    pub id: serde_json::Value,
    pub jsonrpc: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<MCPError>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct MCPError {
    pub code: i32,
    pub message: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

#[derive(Deserialize, Serialize)]
pub struct InitializeResult {
    #[serde(rename = "serverInfo")]
    pub server_info: ServerInfo,
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: serde_json::Value,
    pub instructions: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ResourceContent {
    pub uri: String,
    pub text: String,
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ReadResourceResult {
    pub contents: Vec<ResourceContent>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ToolCallResult {
    pub content: Vec<Content>,
    #[serde(default, rename = "isError")]
    pub is_error: bool,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Content {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: Option<String>,
}