//! Database init.
//!
//! The pool defaults are tuned for production: ~50 connections, modest
//! acquire timeout, and explicit idle/lifetime caps so we don't hold
//! stale conns through transient network glitches. All values are
//! overridable via env so SREs can adapt without recompiling.

use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;

pub async fn init_db(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let max_connections: u32 = parse_env_u32("DATABASE_MAX_CONNECTIONS", 50);
    let min_connections: u32 = parse_env_u32("DATABASE_MIN_CONNECTIONS", 5);
    let acquire_timeout_secs: u64 = parse_env_u64("DATABASE_ACQUIRE_TIMEOUT_SECS", 5);
    let idle_timeout_secs: u64 = parse_env_u64("DATABASE_IDLE_TIMEOUT_SECS", 600);
    let max_lifetime_secs: u64 = parse_env_u64("DATABASE_MAX_LIFETIME_SECS", 1800);

    tracing::info!(
        max_connections,
        min_connections,
        acquire_timeout_secs,
        idle_timeout_secs,
        max_lifetime_secs,
        "initialising Postgres pool"
    );

    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .min_connections(min_connections)
        .acquire_timeout(Duration::from_secs(acquire_timeout_secs))
        .idle_timeout(Duration::from_secs(idle_timeout_secs))
        .max_lifetime(Duration::from_secs(max_lifetime_secs))
        // Tests connections before handing them out — catches dead conns
        // after a failover without surfacing them to the request handler.
        .test_before_acquire(true)
        .connect(database_url)
        .await?;

    sqlx::migrate!("../migrations").run(&pool).await?;
    Ok(pool)
}

fn parse_env_u32(key: &str, default: u32) -> u32 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn parse_env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}
