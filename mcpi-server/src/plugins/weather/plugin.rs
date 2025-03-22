// mcpi-server/src/plugins/weather/plugin.rs
use mcpi_common::{McpPlugin, PluginResult, plugin::PluginType};
use serde_json::{json, Value};
use tracing::info;
use crate::plugins::weather::operations; 

pub struct WeatherPlugin {
    name: String,
    description: String,
    locations: Vec<String>,
}

impl WeatherPlugin {
    pub fn new() -> Self {
        WeatherPlugin {
            name: "weather_forecast".to_string(),
            description: "Get weather forecasts for various locations".to_string(),
            locations: vec![
                "New York".to_string(),
                "London".to_string(),
                "Tokyo".to_string(),
                "Sydney".to_string(),
                "Paris".to_string(),
            ],
        }
    }
}

impl McpPlugin for WeatherPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn category(&self) -> &str {
        "weather"
    }
    
    fn plugin_type(&self) -> PluginType {
        PluginType::Extension
    }

    fn supported_operations(&self) -> Vec<String> {
        vec!["GET".to_string(), "LIST".to_string()]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["GET", "LIST"],
                    "description": "Operation to perform"
                },
                "location": {
                    "type": "string",
                    "description": "Location for weather forecast"
                }
            },
            "required": ["operation"]
        })
    }

    fn execute(&self, operation: &str, params: &Value) -> PluginResult {
        match operation {
            "GET" => {
                let location = params.get("location")
                    .and_then(|l| l.as_str())
                    .unwrap_or("New York");
                
                info!("Generating weather forecast for: {}", location);
                
                if self.locations.contains(&location.to_string()) || location == "New York" {
                    operations::generate_forecast(location)
                } else {
                    info!("Location not found: {}", location);
                    Ok(json!({
                        "error": "Location not found",
                        "available_locations": self.locations
                    }))
                }
            },
            "LIST" => {
                info!("Listing forecasts for all available locations");
                operations::list_all_forecasts(&self.locations)
            },
            _ => Err(format!("Unsupported operation: {}", operation).into())
        }
    }

    fn get_resources(&self) -> Vec<(String, String, Option<String>)> {
        vec![(
            self.name.clone(),
            format!("mcpi://provider/resources/weather/locations/data.json"),
            Some("Weather locations and forecasts".to_string()),
        )]
    }
}