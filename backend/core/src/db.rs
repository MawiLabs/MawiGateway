//! Database init + migration runner.
//!
//! Migrations run under a Postgres session-scoped advisory lock so that
//! two gateway instances starting in parallel — common during a rolling
//! deploy — cannot race each other on `CREATE TABLE` / `ALTER TABLE`.
//! See #42.

use sqlx::postgres::PgPool;

/// 64-bit advisory-lock key for the migration runner. Arbitrary but
/// stable: same value across all instances of this codebase, picked to
/// not collide with application-level advisory locks (we don't use any).
/// Hex spells `MAWIMIGR` — recognisable in `pg_locks` if it ever needs
/// to be debugged in production.
const MIGRATION_LOCK_KEY: i64 = 0x4d41_5749_4d49_4752;

pub async fn init_db(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let pool = PgPool::connect(database_url).await?;
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
