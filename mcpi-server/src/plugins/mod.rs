// mcpi-server/src/plugins/mod.rs

pub mod website_plugin;
pub mod weather_plugin;

pub use website_plugin::WebsitePlugin;
pub use weather_plugin::WeatherPlugin;