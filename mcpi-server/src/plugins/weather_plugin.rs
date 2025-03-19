// mcpi-server/src/plugins/weather_plugin.rs

use mcpi_common::{McpPlugin, PluginResult};
use serde_json::{json, Value};

/// Weather API plugin that provides simulated weather forecasts
pub struct WeatherPlugin {
    name: String,
    description: String,
    locations: Vec<String>,
}

impl WeatherPlugin {
    /// Create a new weather plugin
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
    
    /// Generate a random forecast for a given location
    fn generate_forecast(&self, location: &str) -> Value {
        let conditions = ["Sunny", "Cloudy", "Rainy", "Snowy", "Windy", "Foggy"];
        let condition = conditions[fastrand::usize(0..conditions.len())];
        
        let temp_base = match condition {
            "Sunny" => 75,
            "Cloudy" => 65,
            "Rainy" => 55,
            "Snowy" => 30,
            "Windy" => 60,
            "Foggy" => 55,
            _ => 70,
        };
        
        let temp_range = 10;
        let temp_min = temp_base - fastrand::u8(0..temp_range) as i32;
        let temp_max = temp_base + fastrand::u8(0..temp_range) as i32;
        let temp_current = temp_min + fastrand::u8(0..(temp_max - temp_min) as u8) as i32;
        
        let humidity = fastrand::u8(30..90);
        let wind_speed = match condition {
            "Windy" => fastrand::u8(15..30),
            _ => fastrand::u8(2..15),
        };
        
        json!({
            "location": location,
            "condition": condition,
            "temperature": {
                "current": temp_current,
                "min": temp_min,
                "max": temp_max,
            },
            "humidity": humidity,
            "wind_speed": wind_speed,
            "updated": chrono::Utc::now().to_rfc3339(),
            "forecast": [
                {
                    "day": "Today",
                    "condition": condition,
                    "temp_min": temp_min,
                    "temp_max": temp_max,
                    "precipitation": fastrand::u8(0..100),
                },
                {
                    "day": "Tomorrow",
                    "condition": conditions[fastrand::usize(0..conditions.len())],
                    "temp_min": temp_min - 2 + fastrand::i8(-5..5) as i32,
                    "temp_max": temp_max - 2 + fastrand::i8(-5..5) as i32,
                    "precipitation": fastrand::u8(0..100),
                },
                {
                    "day": "Day after tomorrow",
                    "condition": conditions[fastrand::usize(0..conditions.len())],
                    "temp_min": temp_min - 4 + fastrand::i8(-5..5) as i32,
                    "temp_max": temp_max - 4 + fastrand::i8(-5..5) as i32,
                    "precipitation": fastrand::u8(0..100),
                }
            ]
        })
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
                // Get forecast for a specific location
                let location = params.get("location")
                    .and_then(|l| l.as_str())
                    .unwrap_or("New York");
                
                if self.locations.contains(&location.to_string()) || location == "New York" {
                    Ok(self.generate_forecast(location))
                } else {
                    Ok(json!({
                        "error": "Location not found",
                        "available_locations": self.locations
                    }))
                }
            },
            "LIST" => {
                // List all available locations with a sample forecast
                let forecasts = self.locations.iter()
                    .map(|location| self.generate_forecast(location))
                    .collect::<Vec<_>>();
                
                Ok(json!({
                    "results": forecasts,
                    "count": forecasts.len(),
                    "available_locations": self.locations
                }))
            },
            _ => Err(format!("Unsupported operation: {}", operation).into())
        }
    }

    fn get_resources(&self) -> Vec<(String, String, Option<String>)> {
        vec![(
            self.name.clone(),
            format!("mcpi://provider/resources/weather.json"),
            Some(self.description.clone()),
        )]
    }
}