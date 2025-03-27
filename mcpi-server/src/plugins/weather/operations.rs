// mcpi-server/src/plugins/weather/operations.rs
use mcpi_common::PluginResult;
use serde_json::{json, Value};
use tracing::info;

/// Generate a random forecast for a given location
pub fn generate_forecast(location: &str) -> PluginResult {
    // For simulation purposes, we're using deterministic "random" values
    let conditions = ["Sunny", "Cloudy", "Rainy", "Snowy", "Windy", "Foggy"];
    
    // Use location to deterministically select a condition
    let condition_index = match location {
        "New York" => 0,
        "London" => 3,
        "Tokyo" => 1,
        "Sydney" => 0,
        "Paris" => 2,
        _ => 0,
    };
    
    let condition = conditions[condition_index % conditions.len()];
    
    // Base temperature based on condition
    let temp_base = match condition {
        "Sunny" => 75,
        "Cloudy" => 65,
        "Rainy" => 55,
        "Snowy" => 30,
        "Windy" => 60,
        "Foggy" => 55,
        _ => 70,
    };
    
    // Deterministic variations based on location
    let location_modifier = match location {
        "New York" => 0,
        "London" => -5,
        "Tokyo" => 5,
        "Sydney" => 10,
        "Paris" => -2,
        _ => 0,
    };
    
    let temp_min = temp_base - 5 + location_modifier;
    let temp_max = temp_base + 5 + location_modifier;
    let temp_current = (temp_min + temp_max) / 2;
    
    let humidity = 60 + condition_index * 5;
    let wind_speed = if condition == "Windy" { 20 } else { 5 + condition_index };
    
    info!("Generated forecast for {}: {}, {}°F", location, condition, temp_current);
    
    Ok(json!({
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
                "precipitation": humidity - 30,
            },
            {
                "day": "Tomorrow",
                "condition": conditions[(condition_index + 1) % conditions.len()],
                "temp_min": temp_min - 2,
                "temp_max": temp_max - 2,
                "precipitation": (humidity - 30 + 10) % 100,
            },
            {
                "day": "Day after tomorrow",
                "condition": conditions[(condition_index + 2) % conditions.len()],
                "temp_min": temp_min - 4,
                "temp_max": temp_max - 4,
                "precipitation": (humidity - 30 + 20) % 100,
            }
        ]
    }))
}

/// Generate an audio forecast for a location
pub fn generate_audio_forecast(location: &str) -> PluginResult {
    // In a real implementation, this would generate real audio data
    // For this example, we're using a dummy base64-encoded audio snippet
    let dummy_audio_data = "UklGRiQAAABXQVZFZm10IBAAAAABAAEARKwAAIhYAQACABAAZGF0YQAAAAA=";
    
    info!("Generated audio forecast for {}", location);
    
    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": format!("Audio weather forecast for {}", location)
            },
            {
                "type": "audio",
                "data": dummy_audio_data,
                "mimeType": "audio/wav"
            }
        ]
    }))
}

/// List forecasts for all available locations
pub fn list_all_forecasts(locations: &[String]) -> PluginResult {
    info!("Generating forecasts for {} locations", locations.len());
    
    let forecasts = locations.iter()
        .map(|location| generate_forecast(location))
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    
    Ok(json!({
        "results": forecasts,
        "count": forecasts.len(),
        "available_locations": locations
    }))
}