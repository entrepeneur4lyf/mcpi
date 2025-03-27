// mcpi-common/src/lib.rs
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
pub use plugin::PluginType;

// Protocol version
pub const MCPI_VERSION: &str = "2025-03-26";

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

// Content types
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum ContentItem {
    #[serde(rename = "text")]
    Text { text: String },
    
    #[serde(rename = "image")]
    Image { 
        data: String, 
        #[serde(rename = "mimeType")]
        mime_type: String 
    },
    
    // New audio content type
    #[serde(rename = "audio")]
    Audio { 
        data: String, 
        #[serde(rename = "mimeType")]
        mime_type: String 
    },
    
    #[serde(rename = "resource")]
    Resource {
        resource: ResourceContent
    }
}

// Tool annotations support
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolAnnotation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_only: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destructive: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_confirmation: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limited: Option<bool>,
}

// MCP Tool types
#[derive(Serialize, Deserialize, Debug)]
pub struct Tool {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ToolAnnotation>,
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

// Client capabilities with completions support
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ClientCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<SamplingCapability>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roots: Option<RootsCapability>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completions: Option<CompletionsCapability>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct SamplingCapability {}

#[derive(Serialize, Deserialize, Debug)]
pub struct RootsCapability {
    #[serde(rename = "listChanged")]
    pub list_changed: bool,
}

// New completions capability
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct CompletionsCapability {}

// Update server capabilities
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourcesCapability>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptsCapability>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<LoggingCapability>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResourcesCapability {
    pub subscribe: bool,
    #[serde(rename = "listChanged")]
    pub list_changed: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ToolsCapability {
    #[serde(rename = "listChanged")]
    pub list_changed: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PromptsCapability {
    #[serde(rename = "listChanged")]
    pub list_changed: bool,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct LoggingCapability {}

// Add progress notification with message field
#[derive(Serialize, Deserialize, Debug)]
pub struct ProgressNotification {
    pub id: serde_json::Value,
    pub percentage: Option<u8>,
    pub message: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct InitializeParams {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
    #[serde(rename = "clientInfo")]
    pub client_info: ClientInfo,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

#[derive(Deserialize, Serialize)]
pub struct InitializeResult {
    #[serde(rename = "serverInfo")]
    pub server_info: ServerInfo,
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    pub instructions: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]  // Added Clone trait here
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
    pub content: Vec<ContentItem>,
    #[serde(default, rename = "isError")]
    pub is_error: bool,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Content {
    #[serde(flatten)]
    pub content: ContentItem
}