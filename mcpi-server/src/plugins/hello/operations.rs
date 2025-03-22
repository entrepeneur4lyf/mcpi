// mcpi-server/src/plugins/hello/operations.rs
use mcpi_common::PluginResult;
use serde_json::{json, Value};
use tracing::info;

/// Generate a response for the HELLO operation
pub fn generate_hello_response(
    config: Value,
    context: &str,
    detail_level: &str
) -> PluginResult {
    info!("Generating Hello response with context: '{}' and detail level: '{}'", context, detail_level);
    
    // Default introduction
    let mut intro_text = config.get("default")
        .and_then(|d| d.get("introduction"))
        .and_then(|i| i.as_str())
        .unwrap_or("Welcome to our website.")
        .to_string();
    
    let mut metadata = config.get("default")
        .and_then(|d| d.get("metadata").cloned())
        .unwrap_or_else(|| json!({}));
    
    // Apply context-specific customization if available
    if !context.is_empty() {
        if let Some(contexts) = config.get("contexts") {
            // Look for exact context match
            if let Some(context_config) = contexts.get(context) {
                // Override with context-specific introduction if available
                if let Some(ctx_intro) = context_config.get("introduction").and_then(|i| i.as_str()) {
                    intro_text = ctx_intro.to_string();
                }
                
                // Add context-specific capabilities highlighting
                if let Some(capabilities) = context_config.get("highlight_capabilities") {
                    metadata["highlight_capabilities"] = capabilities.clone();
                }
            }
        }
    }
    
    // Adjust detail level
    let result = match detail_level {
        "basic" => {
            // Simplify metadata for basic requests
            let basic_metadata = json!({
                "provider": metadata.get("provider").cloned().unwrap_or_else(|| json!({}))
            });
            
            json!({
                "content": [{"type": "text", "text": intro_text}],
                "metadata": basic_metadata
            })
        },
        "detailed" => {
            // For detailed requests, include everything
            json!({
                "content": [{"type": "text", "text": intro_text}],
                "metadata": metadata
            })
        },
        _ => {
            // Standard level is the default
            json!({
                "content": [{"type": "text", "text": intro_text}],
                "metadata": {
                    "provider": metadata.get("provider").cloned().unwrap_or_else(|| json!({})),
                    "capabilities": metadata.get("capabilities").cloned().unwrap_or_else(|| json!([])),
                    "topics": metadata.get("primary_focus").cloned().unwrap_or_else(|| json!([]))
                }
            })
        }
    };
    
    Ok(result)
}