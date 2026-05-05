pub mod api_error; // OpenAI-shape error envelope for client responses (#84)
pub mod audit; // Append-only audit log emission helpers (#80)
pub mod auth;
pub mod config;
pub mod config_file; // YAML configuration file schema (#yaml-config)
pub mod cost;
pub mod db;
pub mod error; // Typed ProviderError + HTTP status mapping
pub mod http; // Process-wide shared reqwest::Client
pub mod keys;
pub mod license;
pub mod models;
pub mod providers;
pub mod quota;
pub mod retry; // Retry policy honouring ProviderError + Retry-After
pub mod services;
pub mod types;
pub mod unified;

pub mod agentic; // Agentic service types and execution
pub mod plans; // Centralized plan definitions
pub mod pricing; // Model pricing service with fallbacks
pub mod routing; // Intelligent routing strategies
pub mod rtcros;
pub mod scopes; // API key scope predicates (#78)
pub mod security;
pub mod tools; // Tool management types
pub mod utils; // Shared utility functions // Encryption/Decryption utilities

pub fn hello_core() {
    println!("Hello from mawi-core!");
}
