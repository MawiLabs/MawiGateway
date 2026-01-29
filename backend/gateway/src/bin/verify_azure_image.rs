use dotenv::dotenv;
use reqwest::Client;
use serde_json::json;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    println!("🧪 Testing Azure Image Generation...");

    let api_key = match env::var("AZURE_OPENAI_API_KEY").or_else(|_| env::var("AZURE_API_KEY")) {
        Ok(val) => val,
        Err(_) => {
            eprintln!("❌ ERROR: AZURE_OPENAI_API_KEY (or AZURE_API_KEY) not found in env.");
            return Ok(());
        }
    };
        
    let base_url = match env::var("AZURE_OPENAI_ENDPOINT").or_else(|_| env::var("AZURE_BASE_URL")) {
        Ok(val) => val,
        Err(_) => {
            eprintln!("❌ ERROR: AZURE_OPENAI_ENDPOINT (or AZURE_BASE_URL) not found in env.");
            return Ok(());
        }
    };
    
    println!("✅ Auth loaded.");
    println!("  Endpoint: {}", base_url);
        
    let deployment = "acad-solimg-prod-swc-001"; // Hardcoded from user report
    let api_version = "2024-02-15-preview"; 
    
    let url = format!(
        "{}/openai/deployments/{}/images/generations?api-version={}",
        base_url.trim_end_matches('/'),
        deployment,
        api_version
    );
    
    println!("📍 URL: {}", url);
    
    let client = Client::new();
    let response = client
        .post(&url)
        .header("api-key", api_key)
        .json(&json!({
            "prompt": "A test image of a futuristic computer",
            "n": 1,
            "size": "1024x1024"
        }))
        .send()
        .await?;
        
    println!("📥 Status: {}", response.status());
    let body = response.text().await?;
    println!("📦 Body: {}", body);
    
    Ok(())
}
