use poem::{listener::TcpListener, Route, Server, post, get, EndpointExt};
use poem::middleware::Cors;
use poem_openapi::OpenApiService;

use gateway::api::ModelsApi;
use gateway::topology::TopologyApi;
use gateway::analytics::AnalyticsApi;
use gateway::auth_api::AuthApi;
use gateway::user_api::UserApi;
use gateway::organizations::OrganizationsApi;
use gateway::executor::Executor;
use gateway::mcp_api::McpApi;
use mawi_core::auth::middleware::AuthMiddleware;
use mawi_core::license::LicenseProvider;
use gateway::chat_new::ChatApi;
use gateway::images;
use gateway::audio;
use gateway::transcription;
use gateway::speech_to_speech;
use gateway::video;
use gateway::pricing;
use gateway::health;

use std::sync::Arc;

/// Metrics endpoint handler
fn metrics_endpoint(_req: poem::Request) -> String {
    gateway::metrics::gather_metrics()
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Load .env file
    dotenv::dotenv().ok();

    // Initialize logging
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    tracing_subscriber::fmt::init();
    
    // Initialize PostgreSQL database
    let database_url = std::env::var("DATABASE_URL").expect("🔥 DATABASE_URL not set in .env. Please configure it to point to your persistent database.");
    
    println!("📍 Found DATABASE_URL in environment: [REDACTED]");
    let pool = mawi_core::db::init_db(&database_url).await?;
    
    // Shared MCP Manager (must be same instance for API and Executor)
    let mcp_manager = std::sync::Arc::new(tokio::sync::RwLock::new(gateway::mcp_client::McpManager::new()));

    // Auto-connect MCP servers from database (load ALL servers, preserve configs)
    {
        println!("🔌 Loading MCP servers from database...");
        let servers = sqlx::query_as::<_, (String, String, String, String, String, String)>
            ("SELECT id, name, server_type, image_or_command, args, env_vars FROM mcp_servers")
            .fetch_all(&pool)
            .await
            .unwrap_or_default();
            
        let mut manager = mcp_manager.write().await;
        for (id, name, server_type, command, args_str, env_vars_str) in servers {
             println!("🔌 Reconnecting MCP server: {} ({})", name, id);
             let env_map: std::collections::HashMap<String, String> = serde_json::from_str(&env_vars_str).unwrap_or_default();
             let args_list: Vec<String> = serde_json::from_str(&args_str).unwrap_or_default();
             
             // Reconstruct config
             let server_type_enum = match server_type.as_str() {
                 "docker" => gateway::mcp_client::ServerType::Docker,
                 _ => gateway::mcp_client::ServerType::Stdio, 
             };

             let config = gateway::mcp_client::McpServerConfig {
                 id: id.clone(),
                 name: name.clone(),
                 server_type: server_type_enum,
                 image_or_command: command,
                 args: args_list,
                 env_vars: env_map,
             };
             
             match manager.connect(&config).await {
                 Ok(_) => {
                     println!("✅ Successfully reconnected server: {}", name);
                     let _ = sqlx::query("UPDATE mcp_servers SET status = 'connected' WHERE id = $1")
                         .bind(&id)
                         .execute(&pool)
                         .await;
                 }
                 Err(e) => {
                     eprintln!("⚠️ Could not reconnect server '{}': {} (config preserved)", name, e);
                     let _ = sqlx::query("UPDATE mcp_servers SET status = 'disconnected' WHERE id = $1")
                         .bind(&id)
                         .execute(&pool)
                         .await;
                 }
             }
        }
    }

    // Create executor with real provider integration
    let executor = Arc::new(Executor::new(pool.clone(), mcp_manager.clone()));
    
    // Create unified OpenAPI service for Swagger UI
    let api_service = OpenApiService::new(
        (
            ModelsApi { pool: pool.clone() }, 
            TopologyApi { pool: pool.clone() },
            AnalyticsApi { pool: pool.clone() },
            AuthApi { pool: pool.clone() },
            UserApi { pool: pool.clone() },
            OrganizationsApi { pool: pool.clone() },
            ChatApi { executor: executor.clone() },
            McpApi::new(pool.clone(), mcp_manager.clone())
        ), 
        "MaWi API", "1.0")
        .server("http://localhost:8030/v1");
    let ui = api_service.swagger_ui();
    let spec = api_service.spec();
    
    // CORS configuration
    let cors_origins_str = std::env::var("CORS_ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:3001,http://127.0.0.1:3001".to_string());
    
    let cors_origins: Vec<String> = cors_origins_str.split(',')
        .map(|s| s.trim().to_string())
        .collect();

    println!("🌍 CORS Allowed Origins: {:?}", cors_origins);

    let cors = Cors::new()
        .allow_origins(cors_origins)
        .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
        .allow_headers(vec!["Content-Type", "Authorization", "Cookie"])
        .allow_credentials(true);
    
    // Protected Routes (require auth)
    let protected_routes = Route::new()
        .nest("/v1", api_service) // Combined Models, Providers, Services, Topology, Analytics, Auth, User, Chat
        // Image generation endpoint
        .at(
            "/v1/images/generations",
            post(images::image_generations)
                .data(executor.clone())
        )
        // Text-to-speech endpoint
        .at(
            "/v1/audio/speech",
            post(audio::text_to_speech)
                .data(executor.clone())
        )
        // Speech-to-text endpoint
        .at(
            "/v1/audio/transcriptions",
            post(transcription::transcribe_audio)
                .data(executor.clone())
        )
        // Speech-to-speech endpoint
        .at(
            "/v1/audio/speech-to-speech",
            post(speech_to_speech::speech_to_speech_endpoint)
                .data(executor.clone())
        )
        // Video generation endpoint
        .at(
            "/v1/videos/generations",
            post(video::generate_video)
                .data(executor.clone())
        )
        // Video job status polling
        .at(
            "/v1/videos/jobs/:job_id/:model_id",
            get(video::poll_video_job)
                .data(executor.clone())
        )
        // Video content proxy
        .at(
            "/v1/videos/content/:generation_id/:model_id",
            get(video::proxy_video_content)
                .data(executor.clone())
        )
        .with(AuthMiddleware);

    // Build routes
    let mut app = Route::new()
        .nest("/", protected_routes)
        .nest("/swagger-ui", ui)
        .at("/spec", poem::endpoint::make_sync(move |_| spec.clone()))
        .at("/health", get(health::health_check));
    
    // Conditionally add metrics endpoint
    if gateway::metrics::metrics_enabled() {
        println!("📊 Metrics enabled at /metrics");
        app = app.at("/metrics", poem::endpoint::make_sync(metrics_endpoint));
    } else {
        println!("📊 Metrics disabled (set ENABLE_METRICS=true to enable)");
    }
    
    let app = app
        .with(poem::middleware::AddData::new(pool))
        .with(cors);
    
    // License Provider Injection
    #[cfg(feature = "enterprise")]
    let app = {
        let license_manager = std::sync::Arc::new(mawi_enterprise::license::LicenseManager::new());
        // Attempt to reload license on startup (fire and forget result, logs to stdout)
        let _ = license_manager.reload().await; 
        app.with(poem::middleware::AddData::new(license_manager as std::sync::Arc<dyn mawi_core::license::LicenseProvider>))
    };
    
    #[cfg(not(feature = "enterprise"))]
    let app = app.with(poem::middleware::AddData::new(std::sync::Arc::new(mawi_core::license::OssLicenseProvider) as std::sync::Arc<dyn mawi_core::license::LicenseProvider>));

    println!("🚀 MaWi Gateway starting...");
    println!("📊 Swagger UI: http://localhost:8030/swagger-ui");
    println!("💬 Chat API: POST http://localhost:8030/v1/chat/completions");
    println!("🔧 Management APIs: http://localhost:8030/v1/...");

    Server::new(TcpListener::bind("0.0.0.0:8030"))
        .run_with_graceful_shutdown(
            app,
            async move {
                let _ = tokio::signal::ctrl_c().await;
                println!("🛑 Received shutdown signal. Draining requests...");
            },
            Some(std::time::Duration::from_secs(10)),
        )
        .await?;

    Ok(())
}