use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    Pool, Sqlite,
};
use std::env;
use std::str::FromStr;

pub type DbPool = Pool<Sqlite>;

pub async fn create_pool() -> Result<DbPool, sqlx::Error> {
    let db_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:jslink.db".to_string());
    log::debug!("Connecting to database at: {}", db_url);

    let connection_options = SqliteConnectOptions::from_str(&db_url)?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connection_options)
        .await?;

    log::info!("Database pool created successfully");

    // Run migrations
    log::info!("Running pending migrations...");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");
    log::info!("Migrations ran successfully");

    Ok(pool)
}

#[cfg(test)]
pub async fn create_test_pool() -> DbPool {
    log::debug!("Creating test database pool (in-memory)");

    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database pool");

    log::debug!("Running migrations on test database");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations on in-memory database");

    log::debug!("Test database pool created successfully");
    pool
}
