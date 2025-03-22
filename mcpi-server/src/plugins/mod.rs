// mcpi-server/src/plugins/mod.rs
pub mod hello;
pub mod store;
pub mod website;
pub mod social;
pub mod weather;

// Re-export plugin factory functions for convenience
pub use hello::create_plugin as create_hello_plugin;
pub use store::create_plugins as create_store_plugins;
pub use website::create_plugin as create_website_plugin;
pub use social::create_plugin as create_social_plugin;
pub use weather::create_plugin as create_weather_plugin;