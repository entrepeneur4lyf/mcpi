// mcpi-common/src/lib.rs
use serde::{Deserialize, Serialize};
use serde_json::Value; // Keep Value for dynamic schemas/params
use std::collections::HashMap; // Keep HashMap if used

// Define modules
pub mod plugin;
pub mod json_plugin;
pub mod plugin_factory;

// Re-export for convenience
pub use plugin::{McpPlugin, PluginResult};
pub use json_plugin::JsonDataPlugin;
pub use plugin_factory::PluginFactory;
pub use plugin::PluginType;

// --- Protocol Constants ---
pub const LATEST_MCP_VERSION: &str = "2025-03-26"; // Version for Streamable HTTP standard
pub const MCPI_VERSION: &str = "2025-03-26"; // Aligning MCPI version for structure compatibility
                                          // Change if your MCPI is truly different version structure

// --- JSON-RPC Base Types ---
pub fn default_jsonrpc() -> String { "2.0".to_string() }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MCPRequest {
    #[serde(default = "default_jsonrpc")]
    pub jsonrpc: String,
    pub id: Value, // String or Number
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MCPResponse {
    #[serde(default = "default_jsonrpc")]
    pub jsonrpc: String,
    pub id: Value, // String, Number, or Null
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<MCPError>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MCPError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

// --- Annotations ---
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Annotations {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<Vec<Role>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<f64>, // JSON number maps to f64
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Role {
    User,
    Assistant,
}

// --- Content Items ---
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ContentItem {
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        annotations: Option<Annotations>
    },
    Image {
        data: String, // Base64
        mime_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        annotations: Option<Annotations>
    },
    Audio {
        data: String, // Base64
        mime_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        annotations: Option<Annotations>
    },
    Resource {
        resource: ResourceContentUnion, // Use an enum for different content types
        #[serde(skip_serializing_if = "Option::is_none")]
        annotations: Option<Annotations>
    },
}

// --- Resources ---
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    pub uri: String, // URI Format
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
}

// Union for different resource content types
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)] // Use untagged for Resource { resource: ... }
pub enum ResourceContentUnion {
    Text(TextResourceContents),
    Blob(BlobResourceContents),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TextResourceContents {
    pub uri: String, // URI Format
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BlobResourceContents {
    pub uri: String, // URI Format
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    pub blob: String, // Base64
}

// Result for resources/read
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReadResourceResult {
    pub contents: Vec<ResourceContentUnion>, // Use the union
}

// Result for resources/list
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListResourcesResult {
     pub resources: Vec<Resource>,
     #[serde(skip_serializing_if = "Option::is_none")]
     pub next_cursor: Option<String>, // Assuming Cursor is String
     #[serde(skip_serializing_if = "Option::is_none")]
     pub _meta: Option<Value>,
}


// --- Tools ---

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ToolAnnotations {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_only_hint: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destructive_hint: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotent_hint: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_world_hint: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub input_schema: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ToolAnnotations>, // Use updated struct
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CallToolResult {
    pub content: Vec<ContentItem>,
    #[serde(default)] // Field is optional, defaults to false if missing
    pub is_error: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListToolsResult {
     pub tools: Vec<Tool>,
     #[serde(skip_serializing_if = "Option::is_none")]
     pub next_cursor: Option<String>, // Assuming Cursor is String
     #[serde(skip_serializing_if = "Option::is_none")]
     pub _meta: Option<Value>,
}


// --- Capabilities ---

// Define capability structs based on schema (empty or with bool flags)
#[derive(Serialize, Deserialize, Debug, Clone, Default)] #[serde(rename_all = "camelCase")] pub struct RootsCapability { #[serde(default)] pub list_changed: bool, }
#[derive(Serialize, Deserialize, Debug, Clone, Default)] #[serde(rename_all = "camelCase")] pub struct SamplingCapability {}
#[derive(Serialize, Deserialize, Debug, Clone, Default)] #[serde(rename_all = "camelCase")] pub struct CompletionsCapability {}
#[derive(Serialize, Deserialize, Debug, Clone, Default)] #[serde(rename_all = "camelCase")] pub struct LoggingCapability {}
#[derive(Serialize, Deserialize, Debug, Clone, Default)] #[serde(rename_all = "camelCase")] pub struct PromptsCapability { #[serde(default)] pub list_changed: bool, }
#[derive(Serialize, Deserialize, Debug, Clone, Default)] #[serde(rename_all = "camelCase")] pub struct ResourcesCapability { #[serde(default)] pub subscribe: bool, #[serde(default)] pub list_changed: bool, }
#[derive(Serialize, Deserialize, Debug, Clone, Default)] #[serde(rename_all = "camelCase")] pub struct ToolsCapability { #[serde(default)] pub list_changed: bool, }

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ClientCapabilities {
     #[serde(skip_serializing_if = "Option::is_none")]
     pub experimental: Option<Value>,
     #[serde(skip_serializing_if = "Option::is_none")]
     pub roots: Option<RootsCapability>,
     #[serde(skip_serializing_if = "Option::is_none")]
     pub sampling: Option<SamplingCapability>,
     #[serde(skip_serializing_if = "Option::is_none")]
     pub completions: Option<CompletionsCapability>, // Added completions
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<LoggingCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completions: Option<CompletionsCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptsCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourcesCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,
}


// --- Initialization ---

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Implementation {
    pub name: String,
    pub version: String,
}

// Params for InitializeRequest
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    pub protocol_version: String,
    #[serde(default)] // Capabilities optional in request? Check schema/usage
    pub capabilities: ClientCapabilities,
    pub client_info: Implementation,
}

// Result for initialize
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    pub protocol_version: String,
    #[serde(default)]
    pub capabilities: ServerCapabilities,
    pub server_info: Implementation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

// --- Completions (Matching Schema) ---
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CompleteRequestParams {
    pub r#ref: ResourceOrPromptRef, // Use enum for ref type
    pub argument: CompletionArgument,
    #[serde(flatten, skip_serializing_if = "Option::is_none")] // Allow extra context fields
    pub context: Option<HashMap<String, Value>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ResourceOrPromptRef {
    #[serde(rename = "ref/prompt")]
    Prompt { name: String },
    #[serde(rename = "ref/resource")]
    Resource { uri: String },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CompletionArgument {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CompleteResultCompletion {
    pub values: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] // If None means false
    pub has_more: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CompleteResult {
    pub completion: CompleteResultCompletion,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

// --- Other common structs (Provider, Referral, etc.) ---
// (Keep your existing definitions for these if they haven't changed in the spec)

#[derive(Deserialize, Serialize, Clone, Debug)] // Added Debug
pub struct Provider {
    pub name: String,
    pub domain: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branding: Option<BrandingInfo>,
}

#[derive(Deserialize, Serialize, Clone, Debug)] pub struct BrandingInfo { /* ... */ pub colors: HashMap<String, String>, pub logo: LogoInfo, pub typography: HashMap<String, String>, pub tone: String } // Keep if needed
#[derive(Deserialize, Serialize, Clone, Debug)] pub struct LogoInfo { /* ... */ pub vector: String } // Keep if needed
#[derive(Deserialize, Serialize, Clone, Debug)] pub struct Referral { pub name: String, pub domain: String, pub relationship: String, }


#[derive(Serialize, Deserialize, Debug, Clone)] // Added Debug + Clone
pub struct DiscoveryResponse {
    pub provider: Provider,
    pub mode: String,
    pub capabilities: Vec<CapabilityDescription>,
    pub referrals: Vec<Referral>,
}
#[derive(Serialize, Deserialize, Debug, Clone)] // Added Debug + Clone
pub struct CapabilityDescription {
    pub name: String,
    pub description: String,
    pub category: String,
    pub operations: Vec<String>,
}

// --- Empty Result ---
// For requests that return success with no data (like ping, subscribe, etc.)
// Can often just use serde_json::Value::Null or an empty struct
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct EmptyResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}