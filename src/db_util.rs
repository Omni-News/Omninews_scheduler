use sqlx::pool::PoolOptions;
use sqlx::{mysql::MySql, pool::PoolConnection, MySqlPool};
use std::env;

pub async fn create_pool() -> MySqlPool {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PoolOptions::new()
        .max_connections(10)
        .min_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create MySQL pool")
}

pub async fn get_db(pool: &MySqlPool) -> Result<PoolConnection<MySql>, sqlx::Error> {
    pool.acquire().await.map_err(|e| {
        error!("[Repository] Failed to acquire DB Connection pool: {:?}", e);
        e
    })
}
