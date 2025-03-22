// mcpi-server/src/plugins/mod.rs (updated)
pub mod weather_plugin;
pub mod hello_plugin;
pub mod store_plugin;
pub mod website_plugin;
pub mod social_plugin;

pub use weather_plugin::WeatherPlugin;
pub use hello_plugin::HelloPlugin;
pub use store_plugin::StorePlugin;
pub use website_plugin::WebsitePlugin;
pub use social_plugin::SocialPlugin;