// mcpi-server/src/plugins/social/operations.rs
use mcpi_common::PluginResult;
use serde_json::{json, Value};
use tracing::{info, warn};

/// List referrals, optionally filtered by relationship type
pub fn list_referrals(referrals: &Value, relationship: Option<&str>) -> PluginResult {
    // Extract referrals array
    let empty_vec = Vec::new();
    let referrals_array = referrals.as_array().unwrap_or(&empty_vec);
    
    // Filter by relationship if specified
    let filtered_referrals = if let Some(rel) = relationship {
        info!("Filtering referrals by relationship: {}", rel);
        referrals_array.iter()
            .filter(|r| {
                r.get("relationship").and_then(|rr| rr.as_str()) == Some(rel)
            })
            .cloned()
            .collect::<Vec<_>>()
    } else {
        referrals_array.clone()
    };
    
    info!("List referrals operation completed. Found {} referrals.", filtered_referrals.len());
    
    Ok(json!({
        "referrals": filtered_referrals,
        "count": filtered_referrals.len()
    }))
}

/// Get a specific referral by domain
pub fn get_referral(referrals: &Value, domain: &str) -> PluginResult {
    // Extract referrals array
    let empty_vec = Vec::new();
    let referrals_array = referrals.as_array().unwrap_or(&empty_vec);
    
    // Find referral by domain
    let referral = referrals_array.iter()
        .find(|r| r.get("domain").and_then(|d| d.as_str()) == Some(domain))
        .cloned();
    
    match referral {
        Some(r) => {
            info!("Found referral for domain: {}", domain);
            Ok(r)
        },
        None => {
            warn!("Referral not found for domain: {}", domain);
            Ok(json!({
                "error": "Referral not found",
                "domain": domain
            }))
        }
    }
}