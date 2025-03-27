// mcpi-common/src/lib.rs
use serde::{Deserialize, Serialize};
use serde_json::Value;
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

// --- Protocol Constants ---
// From TS example, assumed associated with the schema
pub const LATEST_MCP_VERSION: &str = "2025-03-26";
// Version for your custom WebSocket protocol (keep separate if structure differs)
pub const MCPI_VERSION: &str = "0.1.0"; // Example - Use your actual MCPI version

// --- JSON-RPC Base Types ---
pub fn default_jsonrpc() -> String { "2.0".to_string() }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MCPRequest {
    #[serde(default = "default_jsonrpc")]
    pub jsonrpc: String,
    pub id: Value, // String or Number
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>, // Keep as Value to handle various param shapes
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MCPResponse {
    #[serde(default = "default_jsonrpc")]
    pub jsonrpc: String,
    pub id: Value, // String, Number, or Null
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>, // Keep as Value to handle various result shapes
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Role {
    User,
    Assistant,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Annotations {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<Vec<Role>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<f64>, // JSON number maps to f64
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
        resource: ResourceContentUnion,
        #[serde(skip_serializing_if = "Option::is_none")]
        annotations: Option<Annotations>
    },
}

// --- Resources ---
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    pub uri: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum ResourceContentUnion {
    Text(TextResourceContents),
    Blob(BlobResourceContents),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TextResourceContents {
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BlobResourceContents {
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    pub blob: String, // Base64
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReadResourceResult {
    pub contents: Vec<ResourceContentUnion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

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
    pub annotations: Option<ToolAnnotations>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CallToolResult {
    pub content: Vec<ContentItem>,
    #[serde(default)]
    pub is_error: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListToolsResult {
     pub tools: Vec<Tool>,
     #[serde(skip_serializing_if = "Option::is_none")]
     pub next_cursor: Option<String>,
     #[serde(skip_serializing_if = "Option::is_none")]
     pub _meta: Option<Value>,
}

// --- Capabilities ---
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
     #[serde(skip_serializing_if = "Option::is_none")] // Added based on schema
     pub completions: Option<CompletionsCapability>,
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
pub struct Implementation { pub name: String, pub version: String }

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    pub protocol_version: String,
    #[serde(default)] pub capabilities: ClientCapabilities,
    pub client_info: Implementation,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    pub protocol_version: String,
    #[serde(default)] pub capabilities: ServerCapabilities,
    pub server_info: Implementation,
    #[serde(skip_serializing_if = "Option::is_none")] pub instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub _meta: Option<Value>,
}

// --- Completions ---
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CompleteRequestParams {
    pub r#ref: ResourceOrPromptRef,
    pub argument: CompletionArgument,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub context: Option<HashMap<String, Value>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ResourceOrPromptRef {
    #[serde(rename = "ref/prompt")] Prompt { name: String },
    #[serde(rename = "ref/resource")] Resource { uri: String },
}

#[derive(Serialize, Deserialize, Debug, Clone)] #[serde(rename_all = "camelCase")] pub struct CompletionArgument { pub name: String, pub value: String }

#[derive(Serialize, Deserialize, Debug, Clone)] #[serde(rename_all = "camelCase")] pub struct CompleteResultCompletion { pub values: Vec<String>, #[serde(skip_serializing_if = "Option::is_none")] pub total: Option<i64>, #[serde(default, skip_serializing_if = "Option::is_none")] pub has_more: Option<bool> }

#[derive(Serialize, Deserialize, Debug, Clone)] #[serde(rename_all = "camelCase")] pub struct CompleteResult { pub completion: CompleteResultCompletion, #[serde(skip_serializing_if = "Option::is_none")] pub _meta: Option<Value> }

// --- Empty Result ---
#[derive(Serialize, Deserialize, Debug, Clone, Default)] #[serde(rename_all = "camelCase")] pub struct EmptyResult { #[serde(skip_serializing_if = "Option::is_none")] pub _meta: Option<Value> }

// --- Other structs (Provider, Referral, Discovery, etc.) ---
// (Kept previous definitions assuming they are still valid or out of scope for MCP spec itself)
#[derive(Deserialize, Serialize, Clone, Debug)] pub struct Provider { pub name: String, pub domain: String, pub description: String, #[serde(skip_serializing_if = "Option::is_none")] pub branding: Option<BrandingInfo> }
#[derive(Deserialize, Serialize, Clone, Debug)] pub struct BrandingInfo { pub colors: HashMap<String, String>, pub logo: LogoInfo, pub typography: HashMap<String, String>, pub tone: String }
#[derive(Deserialize, Serialize, Clone, Debug)] pub struct LogoInfo { pub vector: String }
#[derive(Deserialize, Serialize, Clone, Debug)] pub struct Referral { pub name: String, pub domain: String, pub relationship: String }
#[derive(Serialize, Deserialize, Debug, Clone)] pub struct DiscoveryResponse { pub provider: Provider, pub mode: String, pub capabilities: Vec<CapabilityDescription>, pub referrals: Vec<Referral> }
#[derive(Serialize, Deserialize, Debug, Clone)] pub struct CapabilityDescription { pub name: String, pub description: String, pub category: String, pub operations: Vec<String> }