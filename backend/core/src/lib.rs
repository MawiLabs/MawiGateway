pub mod config;
pub mod cost;
pub mod db;
pub mod models;
pub mod services;
pub mod types;
pub mod providers;
pub mod unified;
pub mod keys;
pub mod auth;
pub mod quota;
pub mod license;

pub mod routing; // Intelligent routing strategies
pub mod pricing; // Model pricing service with fallbacks
pub mod rtcros;
pub mod agentic; // Agentic service types and execution
pub mod tools;   // Tool management types
pub mod plans;   // Centralized plan definitions
pub mod utils;   // Shared utility functions
pub mod security; // Encryption/Decryption utilities

pub fn hello_core() {
    println!("Hello from mawi-core!");
}

