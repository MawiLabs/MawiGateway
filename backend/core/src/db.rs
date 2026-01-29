use sqlx::postgres::PgPool;

pub async fn init_db(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let pool = PgPool::connect(database_url).await?;
    
    // Run migrations
    sqlx::migrate!("../migrations")
        .run(&pool)
        .await?;
    
    Ok(pool)
}
