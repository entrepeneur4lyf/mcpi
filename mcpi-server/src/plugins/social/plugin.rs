// mcpi-server/src/plugins/social/plugin.rs
use mcpi_common::{McpPlugin, PluginResult, plugin::PluginType};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use tracing::info;
use crate::plugins::social::operations; 

pub struct SocialPlugin {
    name: String,
    description: String,
    data_path: String,
    referrals: Value,
}

impl SocialPlugin {
    pub fn new(data_base_path: &str, referrals: Value) -> Self {
        SocialPlugin {
            name: "social".to_string(),
            description: "Social connections and referrals to other services".to_string(),
            data_path: format!("{}/social/referrals/data.json", data_base_path),
            referrals,
        }
    }
    
    /// Load referrals from file or use the provided ones
    fn load_referrals(&self) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let referrals_path = Path::new(&self.data_path);
        
        if referrals_path.exists() {
            info!("Loading referrals from file: {}", referrals_path.display());
            let data = fs::read_to_string(referrals_path)?;
            Ok(serde_json::from_str(&data)?)
        } else {
            info!("Using provided referrals");
            Ok(self.referrals.clone())
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
        vec!["LIST_REFERRALS".to_string(), "GET_REFERRAL".to_string(), "LIST".to_string()]
    }
    
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["LIST_REFERRALS", "GET_REFERRAL", "LIST"],
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
        // Load referrals
        let referrals = self.load_referrals()?;
        
        // Delegate to operations module
        match operation {
            "LIST_REFERRALS" | "LIST" => {
                let relationship = params.get("relationship").and_then(|r| r.as_str());
                operations::list_referrals(&referrals, relationship)
            },
            "GET_REFERRAL" => {
                let domain = params.get("domain").and_then(|d| d.as_str()).unwrap_or("");
                operations::get_referral(&referrals, domain)
            },
            _ => Err(format!("Unsupported operation: {}", operation).into())
        }
    }
    
    fn get_resources(&self) -> Vec<(String, String, Option<String>)> {
        vec![(
            self.name.clone(),
            format!("mcpi://provider/resources/social/referrals/data.json"),
            Some("Referral relationships".to_string()),
        )]
    }
}