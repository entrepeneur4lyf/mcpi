use std::error::Error;
use std::fmt;
use std::process::Command;
use regex::Regex;
use url::Url;

#[derive(Debug)]
pub struct McpDiscoveryError {
    message: String,
}

impl McpDiscoveryError {
    fn new(message: &str) -> Self {
        McpDiscoveryError {
            message: message.to_string(),
        }
    }
}

impl fmt::Display for McpDiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MCP Discovery Error: {}", self.message)
    }
}

impl Error for McpDiscoveryError {}

#[derive(Debug, Clone)]
pub struct McpServiceInfo {
    pub endpoint: String,
    pub version: String,
}

impl McpServiceInfo {
    pub fn new(endpoint: String, version: String) -> Self {
        McpServiceInfo {
            endpoint,
            version,
        }
    }
}

/// Discovers MCP services for a given domain by querying DNS TXT records
pub async fn discover_mcp_services(domain: &str) -> Result<McpServiceInfo, Box<dyn Error>> {
    println!("Discovering MCP services for domain: {}", domain);
    
    // Use dig to query TXT records (cross-platform approach would use a DNS library)
    let mcp_record = format!("_mcp.{}", domain);
    let output = Command::new("dig")
        .args(["+short", "TXT", &mcp_record])
        .output()?;
    
    if !output.status.success() {
        return Err(Box::new(McpDiscoveryError::new(&format!(
            "Failed to query DNS TXT records for {}", mcp_record
        ))));
    }
    
    let output_str = String::from_utf8(output.stdout)?;
    if output_str.trim().is_empty() {
        return Err(Box::new(McpDiscoveryError::new(&format!(
            "No MCP TXT record found for {}", mcp_record
        ))));
    }
    
    // Parse the TXT record
    println!("Found TXT record: {}", output_str.trim());
    parse_mcp_txt_record(&output_str.trim())
}

/// Parses an MCP TXT record and extracts endpoint and version
fn parse_mcp_txt_record(txt_record: &str) -> Result<McpServiceInfo, Box<dyn Error>> {
    // Remove surrounding quotes if present
    let txt = txt_record.trim_matches('"');
    
    // Extract key-value pairs using regex
    let v_regex = Regex::new(r"v=([^\s]+)")?;
    let endpoint_regex = Regex::new(r"url=([^\s]+)")?;
    
    // Extract version
    let version = match v_regex.captures(txt) {
        Some(caps) => caps.get(1).unwrap().as_str().to_string(),
        None => "mcp1".to_string(), // Default to version 1 if not specified
    };
    
    // Extract endpoint
    let endpoint = match endpoint_regex.captures(txt) {
        Some(caps) => caps.get(1).unwrap().as_str().to_string(),
        None => return Err(Box::new(McpDiscoveryError::new("No endpoint URL found in TXT record"))),
    };
    
    // Validate endpoint
    let url = Url::parse(&endpoint)?;
    if url.scheme() != "wss" && url.scheme() != "ws" && url.scheme() != "https" && url.scheme() != "http" {
        return Err(Box::new(McpDiscoveryError::new(&format!(
            "Invalid endpoint protocol: {}. Expected ws://, wss://, http:// or https://", endpoint
        ))));
    }
    
    Ok(McpServiceInfo::new(endpoint, version))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_mcp_txt_record() {
        let txt = "\"v=mcp1 url=https://mcp.example.com/discover\"";
        let result = parse_mcp_txt_record(txt).unwrap();
        
        assert_eq!(result.version, "mcp1");
        assert_eq!(result.endpoint, "https://mcp.example.com/discover");
    }
}