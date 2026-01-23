use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    Pool, Sqlite,
};
use std::env;
use std::path::PathBuf;
use std::str::FromStr;

pub type DbPool = Pool<Sqlite>;

/// Returns the application data directory path.
/// Priority: 1. JSLINK_DATA_DIR env var, 2. ~/.js-link/, 3. current directory
pub fn get_app_dir() -> PathBuf {
    // Check for JSLINK_DATA_DIR environment variable first
    if let Ok(dir) = env::var("JSLINK_DATA_DIR") {
        let path = PathBuf::from(&dir);
        if std::fs::create_dir_all(&path).is_ok() {
            log::info!("Using app data directory from JSLINK_DATA_DIR: {}", dir);
            return path;
        }
        log::warn!("Cannot write to JSLINK_DATA_DIR: {}", dir);
    }

    // Use home directory: ~/.js-link/
    if let Some(home_dir) = dirs::home_dir() {
        let app_dir = home_dir.join(".js-link");
        if std::fs::create_dir_all(&app_dir).is_ok() {
            log::info!("Using app data directory: {}", app_dir.display());
            return app_dir;
        }
        log::warn!("Cannot write to home directory");
    }

    // Fallback: current directory
    log::warn!("Using current directory for app data");
    PathBuf::from(".")
}

fn get_db_path() -> String {
    // Check for DATABASE_URL environment variable first (for advanced users)
    if let Ok(url) = env::var("DATABASE_URL") {
        log::info!("Using database path from DATABASE_URL: {}", url);
        return url;
    }

    let app_dir = get_app_dir();
    let db_path = app_dir.join("jslink.db");
    let path = format!("sqlite:{}", db_path.display());
    log::info!("Using database at: {}", path);
    path
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
