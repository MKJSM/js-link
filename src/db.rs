use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    Pool, Sqlite,
};
use std::env;
use std::str::FromStr;

pub type DbPool = Pool<Sqlite>;

fn get_db_path() -> String {
    // Check for DATABASE_URL environment variable first
    if let Ok(url) = env::var("DATABASE_URL") {
        log::info!("Using database path from DATABASE_URL: {}", url);
        return url;
    }

    // Use home directory: ~/.js-link/jslink.db
    if let Some(home_dir) = dirs::home_dir() {
        let app_dir = home_dir.join(".js-link");
        if std::fs::create_dir_all(&app_dir).is_ok() {
            let db_path = app_dir.join("jslink.db");
            let path = format!("sqlite:{}", db_path.display());
            log::info!("Using database at: {}", path);
            return path;
        }
        log::warn!("Cannot write to home directory, using current directory");
    }

    // Fallback: current directory
    log::warn!("Using database in current directory");
    "sqlite:jslink.db".to_string()
}

pub async fn create_pool() -> Result<DbPool, sqlx::Error> {
    let db_url = get_db_path();
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
