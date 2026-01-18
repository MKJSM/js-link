use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::db::DbPool;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Environment {
    pub id: i64,
    pub name: String,
    pub variables: String, // Stored as JSON
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
}

#[derive(sqlx::FromRow, Clone)]
pub struct EnvironmentDb {
    pub id: i64,
    pub name: String,
    pub variables: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub archived_at: Option<NaiveDateTime>,
}

impl From<EnvironmentDb> for Environment {
    fn from(e: EnvironmentDb) -> Self {
        Self {
            id: e.id,
            name: e.name,
            variables: e.variables,
            created_at: DateTime::from_naive_utc_and_offset(e.created_at, Utc),
            updated_at: DateTime::from_naive_utc_and_offset(e.updated_at, Utc),
            archived_at: e
                .archived_at
                .map(|d| DateTime::from_naive_utc_and_offset(d, Utc)),
        }
    }
}

#[derive(Deserialize)]
pub struct CreateEnvironment {
    name: String,
    variables: String,
}

#[derive(Deserialize)]
pub struct UpdateEnvironment {
    name: String,
    variables: String,
}

#[derive(Deserialize)]
pub struct ListEnvironmentsQuery {
    #[serde(default)]
    include_archived: bool,
}

pub enum EnvironmentError {
    InvalidName,
    EnvironmentNotFound,
    DatabaseError(#[allow(dead_code)] sqlx::Error),
}

impl From<sqlx::Error> for EnvironmentError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => EnvironmentError::EnvironmentNotFound,
            _ => EnvironmentError::DatabaseError(e),
        }
    }
}

impl IntoResponse for EnvironmentError {
    fn into_response(self) -> Response {
        match self {
            EnvironmentError::InvalidName => {
                (StatusCode::BAD_REQUEST, "Invalid environment name").into_response()
            }
            EnvironmentError::EnvironmentNotFound => {
                (StatusCode::NOT_FOUND, "Environment not found").into_response()
            }
            EnvironmentError::DatabaseError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
            }
        }
    }
}

async fn create_environment(
    State(pool): State<DbPool>,
    Json(payload): Json<CreateEnvironment>,
) -> Result<impl IntoResponse, EnvironmentError> {
    log::debug!("Creating environment: name={}", payload.name);

    if payload.name.is_empty() {
        log::warn!("Attempted to create environment with empty name");
        return Err(EnvironmentError::InvalidName);
    }

    let environment_db = sqlx::query_as!(
        EnvironmentDb,
        "INSERT INTO environments (name, variables) VALUES (?, ?) RETURNING id, name, variables, created_at, updated_at, archived_at",
        payload.name,
        payload.variables
    )
    .fetch_one(&pool)
    .await?;

    log::info!(
        "Created environment: id={}, name={}",
        environment_db.id,
        environment_db.name
    );
    Ok((StatusCode::CREATED, Json(Environment::from(environment_db))))
}

async fn list_environments(
    State(pool): State<DbPool>,
    Query(query): Query<ListEnvironmentsQuery>,
) -> Result<impl IntoResponse, EnvironmentError> {
    log::debug!(
        "Listing environments, include_archived={}",
        query.include_archived
    );

    let environments_db = if query.include_archived {
        sqlx::query_as!(
            EnvironmentDb,
            "SELECT id, name, variables, created_at, updated_at, archived_at FROM environments"
        )
        .fetch_all(&pool)
        .await?
    } else {
        sqlx::query_as!(
            EnvironmentDb,
            "SELECT id, name, variables, created_at, updated_at, archived_at FROM environments WHERE archived_at IS NULL"
        )
        .fetch_all(&pool)
        .await?
    };

    let environments: Vec<Environment> =
        environments_db.into_iter().map(Environment::from).collect();
    log::debug!("Found {} environments", environments.len());

    Ok(Json(environments))
}

async fn get_environment(
    State(pool): State<DbPool>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, EnvironmentError> {
    log::debug!("Getting environment with id: {}", id);

    let environment_db = sqlx::query_as!(
        EnvironmentDb,
        "SELECT id, name, variables, created_at, updated_at, archived_at FROM environments WHERE id = ?",
        id
    )
    .fetch_one(&pool)
    .await?;

    log::debug!(
        "Found environment: id={}, name={}",
        environment_db.id,
        environment_db.name
    );
    Ok(Json(Environment::from(environment_db)))
}

async fn update_environment(
    State(pool): State<DbPool>,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateEnvironment>,
) -> Result<impl IntoResponse, EnvironmentError> {
    log::debug!("Updating environment id={} with name: {}", id, payload.name);

    if payload.name.is_empty() {
        log::warn!("Attempted to update environment {} with empty name", id);
        return Err(EnvironmentError::InvalidName);
    }

    let environment_db = sqlx::query_as!(
        EnvironmentDb,
        "UPDATE environments SET name = ?, variables = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? RETURNING id, name, variables, created_at, updated_at, archived_at",
        payload.name,
        payload.variables,
        id
    )
    .fetch_one(&pool)
    .await?;

    log::info!(
        "Updated environment: id={}, name={}",
        environment_db.id,
        environment_db.name
    );
    Ok(Json(Environment::from(environment_db)))
}

async fn archive_environment(
    State(pool): State<DbPool>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, EnvironmentError> {
    log::debug!("Archiving environment id: {}", id);

    let now = Utc::now().naive_utc();
    let result = sqlx::query("UPDATE environments SET archived_at = ? WHERE id = ?")
        .bind(now)
        .bind(id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        log::warn!("Environment not found for archiving: id={}", id);
        return Err(EnvironmentError::EnvironmentNotFound);
    }

    log::info!("Archived environment: id={}", id);
    Ok(StatusCode::OK)
}

async fn unarchive_environment(
    State(pool): State<DbPool>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, EnvironmentError> {
    log::debug!("Unarchiving environment id: {}", id);

    let result = sqlx::query("UPDATE environments SET archived_at = NULL WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        log::warn!("Environment not found for unarchiving: id={}", id);
        return Err(EnvironmentError::EnvironmentNotFound);
    }

    log::info!("Unarchived environment: id={}", id);
    Ok(StatusCode::OK)
}

async fn delete_environment(
    State(pool): State<DbPool>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, EnvironmentError> {
    log::debug!("Deleting environment id: {}", id);

    let result = sqlx::query("DELETE FROM environments WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        log::warn!("Environment not found for deletion: id={}", id);
        return Err(EnvironmentError::EnvironmentNotFound);
    }

    log::info!("Deleted environment: id={}", id);
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes(pool: DbPool) -> Router {
    Router::new()
        .route(
            "/environments",
            post(create_environment).get(list_environments),
        )
        .route(
            "/environments/:id",
            get(get_environment)
                .put(update_environment)
                .delete(delete_environment),
        )
        .route("/environments/:id/archive", put(archive_environment))
        .route("/environments/:id/unarchive", put(unarchive_environment))
        .with_state(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use axum_test::TestServer;
    use chrono::Utc;
    use serde_json::json;

    async fn create_test_environment(pool: &DbPool, name: &str, variables: &str) -> EnvironmentDb {
        sqlx::query_as!(
            EnvironmentDb,
            "INSERT INTO environments (name, variables) VALUES (?, ?) RETURNING id, name, variables, created_at, updated_at, archived_at",
            name,
            variables
        )
        .fetch_one(pool)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn test_create_environment_success() {
        let pool = db::create_test_pool().await;
        let server = TestServer::new(routes(pool)).unwrap();

        let response = server
            .post("/environments")
            .json(&json!({ "name": "New Env", "variables": "{}" }))
            .await;

        response.assert_status(StatusCode::CREATED);
        let environment: Environment = response.json();
        assert_eq!(environment.name, "New Env");
    }

    #[tokio::test]
    async fn test_create_environment_bad_request_empty_name() {
        let pool = db::create_test_pool().await;
        let server = TestServer::new(routes(pool)).unwrap();

        let response = server
            .post("/environments")
            .json(&json!({ "name": "", "variables": "{}" }))
            .await;

        response.assert_status(StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_list_environments() {
        let pool = db::create_test_pool().await;
        create_test_environment(&pool, "env1", "{}").await;
        create_test_environment(&pool, "env2", "{}").await;

        let server = TestServer::new(routes(pool.clone())).unwrap();

        let response = server.get("/environments").await;

        response.assert_status(StatusCode::OK);
        let environments: Vec<Environment> = response.json();
        assert_eq!(environments.len(), 2);
    }

    #[tokio::test]
    async fn test_list_environments_include_archived() {
        let pool = db::create_test_pool().await;
        create_test_environment(&pool, "env1", "{}").await;
        let env2 = create_test_environment(&pool, "env2", "{}").await;
        sqlx::query("UPDATE environments SET archived_at = ? WHERE id = ?")
            .bind(Utc::now().naive_utc())
            .bind(env2.id)
            .execute(&pool)
            .await
            .unwrap();

        let server = TestServer::new(routes(pool.clone())).unwrap();

        let response = server.get("/environments?include_archived=true").await;

        response.assert_status(StatusCode::OK);
        let environments: Vec<Environment> = response.json();
        assert_eq!(environments.len(), 2);
    }

    #[tokio::test]
    async fn test_list_environments_empty() {
        let pool = db::create_test_pool().await;
        let server = TestServer::new(routes(pool)).unwrap();

        let response = server.get("/environments").await;

        response.assert_status(StatusCode::OK);
        let environments: Vec<Environment> = response.json();
        assert_eq!(environments.len(), 0);
    }

    #[tokio::test]
    async fn test_get_environment_success() {
        let pool = db::create_test_pool().await;
        let environment = create_test_environment(&pool, "env1", "{}").await;
        let server = TestServer::new(routes(pool.clone())).unwrap();

        let response = server
            .get(&format!("/environments/{}", environment.id))
            .await;

        response.assert_status(StatusCode::OK);
        let fetched_environment: Environment = response.json();
        assert_eq!(fetched_environment.id, environment.id);
    }

    #[tokio::test]
    async fn test_get_environment_not_found() {
        let pool = db::create_test_pool().await;
        let server = TestServer::new(routes(pool)).unwrap();

        let response = server.get("/environments/999").await;

        response.assert_status(StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_update_environment_success() {
        let pool = db::create_test_pool().await;
        let environment = create_test_environment(&pool, "env1", "{}").await;
        let server = TestServer::new(routes(pool.clone())).unwrap();

        let response = server
            .put(&format!("/environments/{}", environment.id))
            .json(&json!({ "name": "updated name", "variables": "{{\"key\": \"value\"}}" }))
            .await;

        response.assert_status(StatusCode::OK);
        let updated_environment: Environment = response.json();
        assert_eq!(updated_environment.name, "updated name");
        assert_eq!(updated_environment.variables, "{{\"key\": \"value\"}}");
    }

    #[tokio::test]
    async fn test_update_environment_not_found() {
        let pool = db::create_test_pool().await;
        let server = TestServer::new(routes(pool)).unwrap();

        let response = server
            .put("/environments/999")
            .json(&json!({ "name": "updated name", "variables": "{}" }))
            .await;

        response.assert_status(StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_update_environment_bad_request() {
        let pool = db::create_test_pool().await;
        let environment = create_test_environment(&pool, "env1", "{}").await;
        let server = TestServer::new(routes(pool.clone())).unwrap();

        let response = server
            .put(&format!("/environments/{}", environment.id))
            .json(&json!({ "name": "", "variables": "{}" }))
            .await;

        response.assert_status(StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_archive_environment_success() {
        let pool = db::create_test_pool().await;
        let environment = create_test_environment(&pool, "env1", "{}").await;
        let server = TestServer::new(routes(pool.clone())).unwrap();

        let response = server
            .put(&format!("/environments/{}/archive", environment.id))
            .await;

        response.assert_status(StatusCode::OK);

        // Verify that the environment is archived
        let environments: Vec<Environment> = server.get("/environments").await.json();
        assert_eq!(environments.len(), 0);
    }

    #[tokio::test]
    async fn test_archive_environment_not_found() {
        let pool = db::create_test_pool().await;
        let server = TestServer::new(routes(pool)).unwrap();

        let response = server.put("/environments/999/archive").await;

        response.assert_status(StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_unarchive_environment_success() {
        let pool = db::create_test_pool().await;
        let environment = create_test_environment(&pool, "env1", "{}").await;
        sqlx::query("UPDATE environments SET archived_at = ? WHERE id = ?")
            .bind(Utc::now().naive_utc())
            .bind(environment.id)
            .execute(&pool)
            .await
            .unwrap();
        let server = TestServer::new(routes(pool.clone())).unwrap();

        let response = server
            .put(&format!("/environments/{}/unarchive", environment.id))
            .await;

        response.assert_status(StatusCode::OK);

        // Verify that the environment is unarchived
        let environments: Vec<Environment> = server.get("/environments").await.json();
        assert_eq!(environments.len(), 1);
    }

    #[tokio::test]
    async fn test_unarchive_environment_not_found() {
        let pool = db::create_test_pool().await;
        let server = TestServer::new(routes(pool)).unwrap();

        let response = server.put("/environments/999/unarchive").await;

        response.assert_status(StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_delete_environment_success() {
        let pool = db::create_test_pool().await;
        let environment = create_test_environment(&pool, "env1", "{}").await;
        let server = TestServer::new(routes(pool.clone())).unwrap();

        let response = server
            .delete(&format!("/environments/{}", environment.id))
            .await;

        response.assert_status(StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_delete_environment_not_found() {
        let pool = db::create_test_pool().await;
        let server = TestServer::new(routes(pool)).unwrap();

        let response = server.delete("/environments/999").await;

        response.assert_status(StatusCode::NOT_FOUND);
    }
}
