//! Database init + migration runner.
//!
//! Migrations run under a Postgres session-scoped advisory lock so that
//! two gateway instances starting in parallel — common during a rolling
//! deploy — cannot race each other on `CREATE TABLE` / `ALTER TABLE`.
//! See #42.

use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;

/// 64-bit advisory-lock key for the migration runner. Arbitrary but
/// stable: same value across all instances of this codebase, picked to
/// not collide with application-level advisory locks (we don't use any).
/// Hex spells `MAWIMIGR` — recognisable in `pg_locks` if it ever needs
/// to be debugged in production.
const MIGRATION_LOCK_KEY: i64 = 0x4d41_5749_4d49_4752;

/// Read a `u32` env var, falling back to `default` on missing/invalid input.
fn env_u32(name: &str, default: u32) -> u32 {
    std::env::var(name)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

/// Read a `u64` (seconds) env var, falling back to `default` on missing/invalid input.
fn env_secs(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

/// Initialise the Postgres pool with production-tuned defaults.
///
/// Closes #37 — previously used `PgPool::connect(...)` which gives
/// SQLx's defaults (~10 max connections, no acquire timeout). Under
/// modest load the pool starved and requests queued forever.
///
/// All limits are env-overridable so operators can tune for their
/// Postgres tier without rebuilding:
///   * `MG_DB_MAX_CONNECTIONS`    (default 50)
///   * `MG_DB_MIN_CONNECTIONS`    (default 5)
///   * `MG_DB_ACQUIRE_TIMEOUT_S`  (default 5)
///   * `MG_DB_IDLE_TIMEOUT_S`     (default 600 = 10 min)
///   * `MG_DB_MAX_LIFETIME_S`     (default 1800 = 30 min)
pub async fn init_db(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let max_conn = env_u32("MG_DB_MAX_CONNECTIONS", 50);
    let min_conn = env_u32("MG_DB_MIN_CONNECTIONS", 5);
    let acquire_s = env_secs("MG_DB_ACQUIRE_TIMEOUT_S", 5);
    let idle_s = env_secs("MG_DB_IDLE_TIMEOUT_S", 600);
    let lifetime_s = env_secs("MG_DB_MAX_LIFETIME_S", 1800);

    tracing::info!(
        max_connections = max_conn,
        min_connections = min_conn,
        acquire_timeout_s = acquire_s,
        idle_timeout_s = idle_s,
        max_lifetime_s = lifetime_s,
        "configuring Postgres pool"
    );

    let pool = PgPoolOptions::new()
        .max_connections(max_conn)
        .min_connections(min_conn)
        .acquire_timeout(Duration::from_secs(acquire_s))
        .idle_timeout(Some(Duration::from_secs(idle_s)))
        .max_lifetime(Some(Duration::from_secs(lifetime_s)))
        .connect(database_url)
        .await?;
    run_migrations(&pool).await?;
    Ok(pool)
}

/// Acquire a session-scoped advisory lock, run pending migrations, release.
///
/// Concurrency model: instance B starting while instance A is mid-migration
/// blocks on `pg_advisory_lock` until A's session releases (either by
/// explicit unlock at the end of this function, or by the connection
/// dropping if A crashes). Once B acquires the lock, `sqlx::migrate!` is
/// a no-op for already-applied versions, so B proceeds in milliseconds.
async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    let mut conn = pool.acquire().await?;

    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(MIGRATION_LOCK_KEY)
        .execute(&mut *conn)
        .await?;

    let migrate_result = sqlx::migrate!("../migrations").run(&mut *conn).await;

    // Always attempt unlock, even on migration failure. The session-scoped
    // lock would auto-release on connection drop anyway, but explicit
    // release lets the next instance proceed without waiting for the OS
    // to garbage-collect our TCP socket.
    if let Err(e) = sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(MIGRATION_LOCK_KEY)
        .execute(&mut *conn)
        .await
    {
        tracing::warn!(
            error = %e,
            "pg_advisory_unlock failed — lock will release when connection drops"
        );
    }

    migrate_result.map_err(sqlx::Error::from)
}
