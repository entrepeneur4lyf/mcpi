// mcpi-server/src/plugins/social_plugin.rs
use mcpi_common::{McpPlugin, PluginResult};
use mcpi_common::plugin::PluginType;
use serde_json::{json, Value};

pub struct SocialPlugin {
    name: String,
    description: String,
    referrals: Value,
}

impl SocialPlugin {
    pub fn new(referrals: Value) -> Self {
        SocialPlugin {
            name: "social".to_string(),
            description: "Social connections and referrals to other services".to_string(),
            referrals,
        }
    }
}

impl McpPlugin for SocialPlugin {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> &str {
        &self.description
    }
    
    fn category(&self) -> &str {
        "social"
    }
    
    fn plugin_type(&self) -> PluginType {
        PluginType::Core
    }
    
    fn supported_operations(&self) -> Vec<String> {
        vec!["LIST_REFERRALS".to_string(), "GET_REFERRAL".to_string()]
    }
    
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["LIST_REFERRALS", "GET_REFERRAL"],
                    "description": "Operation to perform"
                },
                "domain": {
                    "type": "string",
                    "description": "Domain name for GET_REFERRAL operation"
                },
                "relationship": {
                    "type": "string",
                    "description": "Filter referrals by relationship type"
                }
            },
            "required": ["operation"]
        })
    }
    
    fn execute(&self, operation: &str, params: &Value) -> PluginResult {
        let empty_vec = Vec::new();
        let referrals = self.referrals.as_array().unwrap_or(&empty_vec);
                
        match operation {
            "LIST_REFERRALS" => {
                // Filter by relationship if specified
                let relationship = params.get("relationship").and_then(|r| r.as_str());
                
                let filtered_referrals = if let Some(rel) = relationship {
                    referrals.iter()
                        .filter(|r| {
                            r.get("relationship").and_then(|rr| rr.as_str()) == Some(rel)
                        })
                        .cloned()
                        .collect::<Vec<_>>()
                } else {
                    referrals.clone()
                };
                
                Ok(json!({
                    "referrals": filtered_referrals,
                    "count": filtered_referrals.len()
                }))
            },
            "GET_REFERRAL" => {
                let domain = params.get("domain").and_then(|d| d.as_str()).unwrap_or("");
                
                let referral = referrals.iter()
                    .find(|r| r.get("domain").and_then(|d| d.as_str()) == Some(domain))
                    .cloned();
                
                match referral {
                    Some(r) => Ok(r),
                    None => Ok(json!({
                        "error": "Referral not found",
                        "domain": domain
                    }))
                }
            },
            _ => Err(format!("Unsupported operation: {}", operation).into())
        }
    }
}