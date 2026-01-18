use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::db::DbPool;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct NetworkSettings {
    pub id: i64,
    pub auto_proxy: bool,
    pub http_proxy: Option<String>,
    pub https_proxy: Option<String>,
    pub no_proxy: Option<String>,
}

#[derive(sqlx::FromRow, Clone)]
struct NetworkSettingsDb {
    id: i64,
    auto_proxy: bool,
    http_proxy: Option<String>,
    https_proxy: Option<String>,
    no_proxy: Option<String>,
}

impl From<NetworkSettingsDb> for NetworkSettings {
    fn from(s: NetworkSettingsDb) -> Self {
        Self {
            id: s.id,
            auto_proxy: s.auto_proxy,
            http_proxy: s.http_proxy,
            https_proxy: s.https_proxy,
            no_proxy: s.no_proxy,
        }
    }
}

#[derive(Deserialize)]
pub struct UpdateNetworkSettings {
    auto_proxy: bool,
    http_proxy: Option<String>,
    https_proxy: Option<String>,
    no_proxy: Option<String>,
}

pub enum NetworkSettingsError {
    SettingsNotFound,
    DatabaseError(#[allow(dead_code)] sqlx::Error),
}

impl From<sqlx::Error> for NetworkSettingsError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => NetworkSettingsError::SettingsNotFound,
            _ => NetworkSettingsError::DatabaseError(e),
        }
    }
}

impl IntoResponse for NetworkSettingsError {
    fn into_response(self) -> Response {
        match self {
            NetworkSettingsError::SettingsNotFound => {
                (StatusCode::NOT_FOUND, "Network settings not found").into_response()
            }
            NetworkSettingsError::DatabaseError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
            }
        }
    }
}

async fn get_network_settings(
    State(pool): State<DbPool>,
) -> Result<impl IntoResponse, NetworkSettingsError> {
    log::debug!("Getting network settings");

    let settings_db = sqlx::query_as!(
        NetworkSettingsDb,
        "SELECT id, auto_proxy, http_proxy, https_proxy, no_proxy FROM network_settings WHERE id = 1"
    )
    .fetch_one(&pool)
    .await?;

    let settings = NetworkSettings::from(settings_db);
    log::debug!(
        "Network settings: auto_proxy={}, http_proxy={:?}, https_proxy={:?}",
        settings.auto_proxy,
        settings.http_proxy,
        settings.https_proxy
    );

    Ok(Json(settings))
}

async fn update_network_settings(
    State(pool): State<DbPool>,
    Json(payload): Json<UpdateNetworkSettings>,
) -> Result<impl IntoResponse, NetworkSettingsError> {
    log::info!("Updating network settings: auto_proxy={}, http_proxy={:?}, https_proxy={:?}, no_proxy={:?}", 
        payload.auto_proxy, payload.http_proxy, payload.https_proxy, payload.no_proxy);

    let settings_db = sqlx::query_as!(
        NetworkSettingsDb,
        "UPDATE network_settings SET auto_proxy = ?, http_proxy = ?, https_proxy = ?, no_proxy = ? WHERE id = 1 RETURNING id, auto_proxy, http_proxy, https_proxy, no_proxy",
        payload.auto_proxy,
        payload.http_proxy,
        payload.https_proxy,
        payload.no_proxy,
    )
    .fetch_one(&pool)
    .await?;

    let settings = NetworkSettings::from(settings_db);
    log::info!("Network settings updated successfully");

    Ok(Json(settings))
}

pub fn routes(pool: DbPool) -> Router {
    Router::new()
        .route(
            "/settings/network",
            get(get_network_settings).put(update_network_settings),
        )
        .with_state(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use axum_test::TestServer;
    use serde_json::json;

    // Helper to ensure default settings are present for tests that need it
    async fn ensure_default_network_settings(pool: &DbPool) {
        // Attempt to insert, ignore if it already exists due to migration
        let _ = sqlx::query!(
            "INSERT OR IGNORE INTO network_settings (id, auto_proxy, http_proxy, https_proxy, no_proxy) VALUES (1, TRUE, NULL, NULL, NULL)"
        )
        .execute(pool)
        .await;
    }

    #[tokio::test]
    async fn test_get_network_settings_success() {
        let pool = db::create_test_pool().await;
        ensure_default_network_settings(&pool).await; // Ensure default is there
        let server = TestServer::new(routes(pool)).unwrap();

        let response = server.get("/settings/network").await;

        response.assert_status(StatusCode::OK);
        let settings: NetworkSettings = response.json();
        assert!(settings.auto_proxy);
    }

    #[tokio::test]
    async fn test_update_network_settings_success() {
        let pool = db::create_test_pool().await;
        ensure_default_network_settings(&pool).await; // Ensure default is there
        let server = TestServer::new(routes(pool)).unwrap();

        let response = server
            .put("/settings/network")
            .json(&json!({
                "auto_proxy": false,
                "http_proxy": "http://localhost:8080",
                "https_proxy": null,
                "no_proxy": "localhost"
            }))
            .await;

        response.assert_status(StatusCode::OK);
        let settings: NetworkSettings = response.json();
        assert!(!settings.auto_proxy);
        assert_eq!(
            settings.http_proxy,
            Some("http://localhost:8080".to_string())
        );
        assert_eq!(settings.https_proxy, None);
        assert_eq!(settings.no_proxy, Some("localhost".to_string()));
    }
}
