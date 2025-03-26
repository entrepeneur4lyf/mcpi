use std::error::Error;
use std::fmt;
use std::str; // For UTF-8 conversion

// Removed regex import
use url::Url;

// Assuming DoH is still the chosen method for lookup
use serde::Deserialize;

// --- Error types (Unchanged) ---
#[derive(Debug)]
pub struct McpDiscoveryError { message: String }
impl McpDiscoveryError { fn new(message: &str) -> Self { McpDiscoveryError { message: message.to_string() } } }
impl fmt::Display for McpDiscoveryError { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "MCP Discovery Error: {}", self.message) } }
impl Error for McpDiscoveryError {}

// --- Service Info (Unchanged) ---
#[derive(Debug, Clone)]
pub struct McpServiceInfo { pub endpoint: String, pub version: String }
impl McpServiceInfo { pub fn new(endpoint: String, version: String) -> Self { McpServiceInfo { endpoint, version } } }

// --- Google DoH Response Structs (If using DoH method) ---
#[derive(Deserialize, Debug)]
struct GoogleDnsAnswer { #[serde(rename = "type")] rr_type: u16, data: String }
#[derive(Deserialize, Debug)]
struct GoogleDnsResponse { #[serde(rename = "Answer")] answer: Option<Vec<GoogleDnsAnswer>>, #[serde(rename = "Status")] status: u32 }

// --- Discovery Function (Using Google DoH - Unchanged from Rev 15) ---
pub async fn discover_mcp_services(domain: &str) -> Result<McpServiceInfo, Box<dyn Error>> {
    println!("Discovering MCP services for domain {} via Google DoH...", domain);
    let mcp_record_name = format!("_mcp.{}", domain);
    let client = reqwest::Client::new();
    let request_url = format!("https://dns.google/resolve?name={}&type=TXT", mcp_record_name);
    println!("Querying Google DoH: {}", request_url);
    let response = client.get(&request_url).header("Accept", "application/dns-json").send().await
        .map_err(|e| Box::new(McpDiscoveryError::new(&format!("HTTP request to Google DoH failed: {}", e))))?;
    if !response.status().is_success() {
        return Err(Box::new(McpDiscoveryError::new(&format!("Google DoH request failed with HTTP status: {}", response.status()))));
    }
    let dns_response: GoogleDnsResponse = response.json().await
        .map_err(|e| Box::new(McpDiscoveryError::new(&format!("Failed to parse JSON response from Google DoH: {}", e))))?;
    println!("Google DoH Response: {:?}", dns_response);
    if dns_response.status != 0 {
         let err_msg = format!("Google DoH reported DNS error status {} for {}", dns_response.status, mcp_record_name);
         return Err(Box::new(McpDiscoveryError::new(&err_msg)));
    }
    if let Some(answers) = dns_response.answer {
        if let Some(txt_answer) = answers.iter().find(|ans| ans.rr_type == 16) {
            let txt_data_unquoted = txt_answer.data.trim_matches('"').to_string();
            println!("Found TXT data (unquoted): \"{}\"", txt_data_unquoted);
            parse_mcp_txt_record(&txt_data_unquoted) // Call the refactored parser
        } else {
            Err(Box::new(McpDiscoveryError::new(&format!("No TXT records found in Google DoH answer for {}", mcp_record_name))))
        }
    } else {
         Err(Box::new(McpDiscoveryError::new(&format!("No MCP TXT records found via Google DoH for {}", mcp_record_name))))
    }
}

// --- Parsing Function (Refactored - No Regex) ---
fn parse_mcp_txt_record(txt_record_content: &str) -> Result<McpServiceInfo, Box<dyn Error>> {
    let txt = txt_record_content.trim();
    println!("Parsing TXT content using whitespace split: \"{}\"", txt);
    let mut version = "mcp1".to_string(); // Default version
    let mut endpoint: Option<String> = None;
    for part in txt.split_whitespace() {
        if let Some((key, value)) = part.split_once('=') {
            match key {
                "v" => { version = value.to_string(); println!("Found version: {}", version); }
                "url" => { endpoint = Some(value.to_string()); println!("Found endpoint: {}", endpoint.as_deref().unwrap_or("")); }
                _ => { println!("Ignoring unknown key: {}", key); }
            }
        } else { println!("Ignoring malformed part: {}", part); }
    }
    let endpoint_str = endpoint.ok_or_else(|| Box::new(McpDiscoveryError::new("No endpoint URL (url=...) found in TXT record")))?;
    let parsed_url = Url::parse(&endpoint_str)?;
    match parsed_url.scheme() {
        "ws" | "wss" | "http" | "https" => Ok(McpServiceInfo::new(endpoint_str, version)),
        invalid_scheme => Err(Box::new(McpDiscoveryError::new(&format!("Invalid endpoint protocol scheme: '{}'. Expected ws, wss, http, or https.", invalid_scheme)))),
    }
}

// --- Tests (Implementations Restored) ---
#[cfg(test)]
mod tests {
     use super::*;

    #[test]
    fn test_parse_mcp_txt_record_standard() {
        let txt = "v=mcp1 url=https://mcp.example.com/discover";
        let result = parse_mcp_txt_record(txt).unwrap();
        assert_eq!(result.version, "mcp1");
        assert_eq!(result.endpoint, "https://mcp.example.com/discover");
    }

    #[test]
    fn test_parse_mcp_txt_record_different_order() {
        let txt = "url=wss://secure.mcp.org/path v=mcp2 extra=data";
        let result = parse_mcp_txt_record(txt).unwrap();
        assert_eq!(result.version, "mcp2");
        assert_eq!(result.endpoint, "wss://secure.mcp.org/path");
    }

    #[test]
    fn test_parse_mcp_txt_record_no_version() {
        let txt = "url=ws://local.mcp:8080";
        let result = parse_mcp_txt_record(txt).unwrap();
        assert_eq!(result.version, "mcp1"); // Should default
        assert_eq!(result.endpoint, "ws://local.mcp:8080");
    }

    #[test]
    fn test_parse_mcp_txt_record_no_url() {
        let txt = "v=mcp1 something=else";
        let result = parse_mcp_txt_record(txt);
        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        println!("Got expected error: {}", err_msg); // For test debug output
        assert!(err_msg.contains("No endpoint URL (url=...) found"));
    }

    #[test]
    fn test_parse_mcp_txt_record_invalid_protocol() {
        let txt = "v=mcp1 url=ftp://mcp.example.com/discover";
        let result = parse_mcp_txt_record(txt);
        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        println!("Got expected error: {}", err_msg); // For test debug output
        // The error might come from Url::parse or our custom check, check message contains relevant part
        assert!(err_msg.contains("Invalid endpoint protocol scheme: 'ftp'"));
    }

    #[test]
    fn test_parse_mcp_txt_record_extra_whitespace() {
        // Tests multiple spaces between pairs, leading/trailing handled by initial trim
        let txt = "  v=mcpX   url=http://mcp.test/api  ";
        let result = parse_mcp_txt_record(txt).unwrap();
        assert_eq!(result.version, "mcpX");
        assert_eq!(result.endpoint, "http://mcp.test/api");
    }

    // DoH integration test (still marked ignore)
    #[tokio::test]
    #[ignore] // Ignore network tests by default
    async fn test_discover_google_doh_integration() {
        // Use a domain you know has the record, or one that doesn't for error testing
        let domain = "mcpintegrate.com"; // Or another test domain like "google.com"
        match discover_mcp_services(domain).await {
            Ok(info) => println!("Integration test success: {:?}", info),
            Err(e) => {
                // Depending on the domain, error might be expected
                println!("Integration test got error (may be expected): {}", e);
                // Example: Assert if you expect an error for a domain without the record
                // assert!(e.to_string().contains("No MCP TXT records found"));
            }
        }
    }
}