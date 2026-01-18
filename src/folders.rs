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
pub struct Folder {
    id: i64,
    name: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    archived_at: Option<DateTime<Utc>>,
}

#[derive(sqlx::FromRow, Clone)]
struct FolderDb {
    id: i64,
    name: String,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    archived_at: Option<NaiveDateTime>,
}

impl From<FolderDb> for Folder {
    fn from(f: FolderDb) -> Self {
        Self {
            id: f.id,
            name: f.name,
            created_at: DateTime::from_naive_utc_and_offset(f.created_at, Utc),
            updated_at: DateTime::from_naive_utc_and_offset(f.updated_at, Utc),
            archived_at: f
                .archived_at
                .map(|d| DateTime::from_naive_utc_and_offset(d, Utc)),
        }
    }
}

#[derive(Deserialize)]
pub struct CreateFolder {
    name: String,
}

#[derive(Deserialize)]
pub struct UpdateFolder {
    name: String,
}

#[derive(Deserialize)]
pub struct ListFoldersQuery {
    #[serde(default)]
    include_archived: bool,
}

pub enum FolderError {
    InvalidName,
    FolderNotFound,
    DatabaseError(#[allow(dead_code)] sqlx::Error),
}

impl From<sqlx::Error> for FolderError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => FolderError::FolderNotFound,
            _ => FolderError::DatabaseError(e),
        }
    }
}

impl IntoResponse for FolderError {
    fn into_response(self) -> Response {
        match self {
            FolderError::InvalidName => {
                (StatusCode::BAD_REQUEST, "Invalid folder name").into_response()
            }
            FolderError::FolderNotFound => {
                (StatusCode::NOT_FOUND, "Folder not found").into_response()
            }
            FolderError::DatabaseError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
            }
        }
    }
}

async fn create_folder(
    State(pool): State<DbPool>,
    Json(payload): Json<CreateFolder>,
) -> Result<impl IntoResponse, FolderError> {
    log::debug!("Creating folder with name: {}", payload.name);

    if payload.name.is_empty() {
        log::warn!("Attempted to create folder with empty name");
        return Err(FolderError::InvalidName);
    }

    let folder_db = sqlx::query_as!(
        FolderDb,
        "INSERT INTO folders (name) VALUES (?) RETURNING id, name, created_at, updated_at, archived_at",
        payload.name
    )
    .fetch_one(&pool)
    .await?;

    log::info!(
        "Created folder: id={}, name={}",
        folder_db.id,
        folder_db.name
    );
    Ok((StatusCode::CREATED, Json(Folder::from(folder_db))))
}

async fn list_folders(
    State(pool): State<DbPool>,
    Query(query): Query<ListFoldersQuery>,
) -> Result<impl IntoResponse, FolderError> {
    log::debug!(
        "Listing folders, include_archived={}",
        query.include_archived
    );

    let folders_db = if query.include_archived {
        sqlx::query_as!(
            FolderDb,
            "SELECT id, name, created_at, updated_at, archived_at FROM folders"
        )
        .fetch_all(&pool)
        .await?
    } else {
        sqlx::query_as!(
            FolderDb,
            "SELECT id, name, created_at, updated_at, archived_at FROM folders WHERE archived_at IS NULL"
        )
        .fetch_all(&pool)
        .await?
    };

    let folders: Vec<Folder> = folders_db.into_iter().map(Folder::from).collect();
    log::debug!("Found {} folders", folders.len());

    Ok(Json(folders))
}

async fn get_folder(
    State(pool): State<DbPool>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, FolderError> {
    log::debug!("Getting folder with id: {}", id);

    let folder_db = sqlx::query_as!(
        FolderDb,
        "SELECT id, name, created_at, updated_at, archived_at FROM folders WHERE id = ?",
        id
    )
    .fetch_one(&pool)
    .await?;

    log::debug!("Found folder: id={}, name={}", folder_db.id, folder_db.name);
    Ok(Json(Folder::from(folder_db)))
}

async fn update_folder(
    State(pool): State<DbPool>,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateFolder>,
) -> Result<impl IntoResponse, FolderError> {
    log::debug!("Updating folder id={} with name: {}", id, payload.name);

    if payload.name.is_empty() {
        log::warn!("Attempted to update folder {} with empty name", id);
        return Err(FolderError::InvalidName);
    }

    let folder_db = sqlx::query_as!(
        FolderDb,
        "UPDATE folders SET name = ? WHERE id = ? RETURNING id, name, created_at, updated_at, archived_at",
        payload.name,
        id
    )
    .fetch_one(&pool)
    .await?;

    log::info!(
        "Updated folder: id={}, name={}",
        folder_db.id,
        folder_db.name
    );
    Ok(Json(Folder::from(folder_db)))
}

async fn archive_folder(
    State(pool): State<DbPool>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, FolderError> {
    log::debug!("Archiving folder id: {}", id);

    let now = Utc::now().naive_utc();
    let result = sqlx::query("UPDATE folders SET archived_at = ? WHERE id = ?")
        .bind(now)
        .bind(id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        log::warn!("Folder not found for archiving: id={}", id);
        return Err(FolderError::FolderNotFound);
    }

    log::info!("Archived folder: id={}", id);
    Ok(StatusCode::OK)
}

async fn unarchive_folder(
    State(pool): State<DbPool>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, FolderError> {
    log::debug!("Unarchiving folder id: {}", id);

    let result = sqlx::query("UPDATE folders SET archived_at = NULL WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        log::warn!("Folder not found for unarchiving: id={}", id);
        return Err(FolderError::FolderNotFound);
    }

    log::info!("Unarchived folder: id={}", id);
    Ok(StatusCode::OK)
}

async fn delete_folder(
    State(pool): State<DbPool>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, FolderError> {
    log::debug!("Deleting folder id: {}", id);

    let result = sqlx::query("DELETE FROM folders WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        log::warn!("Folder not found for deletion: id={}", id);
        return Err(FolderError::FolderNotFound);
    }

    log::info!("Deleted folder: id={}", id);
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes(pool: DbPool) -> Router {
    Router::new()
        .route("/folders", post(create_folder).get(list_folders))
        .route(
            "/folders/:id",
            get(get_folder).put(update_folder).delete(delete_folder),
        )
        .route("/folders/:id/archive", put(archive_folder))
        .route("/folders/:id/unarchive", put(unarchive_folder))
        .with_state(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use axum_test::TestServer;
    use chrono::Utc;
    use serde_json::json;

    async fn create_test_folder(pool: &DbPool, name: &str) -> FolderDb {
        sqlx::query_as!(
            FolderDb,
            "INSERT INTO folders (name) VALUES (?) RETURNING id, name, created_at, updated_at, archived_at",
            name
        )
        .fetch_one(pool)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn test_create_folder_success() {
        let pool = db::create_test_pool().await;
        let server = TestServer::new(routes(pool)).unwrap();

        let response = server
            .post("/folders")
            .json(&json!({ "name": "New Folder" }))
            .await;

        response.assert_status(StatusCode::CREATED);
        let folder: Folder = response.json();
        assert_eq!(folder.name, "New Folder");
    }

    #[tokio::test]
    async fn test_create_folder_bad_request_empty_name() {
        let pool = db::create_test_pool().await;
        let server = TestServer::new(routes(pool)).unwrap();

        let response = server.post("/folders").json(&json!({ "name": "" })).await;

        response.assert_status(StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_create_folder_bad_request_missing_name() {
        let pool = db::create_test_pool().await;
        let server = TestServer::new(routes(pool)).unwrap();

        let response = server.post("/folders").json(&json!({})).await;

        response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn test_list_folders() {
        let pool = db::create_test_pool().await;
        create_test_folder(&pool, "folder1").await;
        create_test_folder(&pool, "folder2").await;

        let server = TestServer::new(routes(pool.clone())).unwrap();

        let response = server.get("/folders").await;

        response.assert_status(StatusCode::OK);
        let folders: Vec<Folder> = response.json();
        assert_eq!(folders.len(), 2);
    }

    #[tokio::test]
    async fn test_list_folders_include_archived() {
        let pool = db::create_test_pool().await;
        create_test_folder(&pool, "folder1").await;
        let folder2 = create_test_folder(&pool, "folder2").await;
        sqlx::query("UPDATE folders SET archived_at = ? WHERE id = ?")
            .bind(Utc::now().naive_utc())
            .bind(folder2.id)
            .execute(&pool)
            .await
            .unwrap();

        let server = TestServer::new(routes(pool.clone())).unwrap();

        let response = server.get("/folders?include_archived=true").await;

        response.assert_status(StatusCode::OK);
        let folders: Vec<Folder> = response.json();
        assert_eq!(folders.len(), 2);
    }

    #[tokio::test]
    async fn test_list_folders_empty() {
        let pool = db::create_test_pool().await;
        let server = TestServer::new(routes(pool)).unwrap();

        let response = server.get("/folders").await;

        response.assert_status(StatusCode::OK);
        let folders: Vec<Folder> = response.json();
        assert_eq!(folders.len(), 0);
    }

    #[tokio::test]
    async fn test_get_folder_success() {
        let pool = db::create_test_pool().await;
        let folder = create_test_folder(&pool, "folder1").await;
        let server = TestServer::new(routes(pool.clone())).unwrap();

        let response = server.get(&format!("/folders/{}", folder.id)).await;

        response.assert_status(StatusCode::OK);
        let fetched_folder: Folder = response.json();
        assert_eq!(fetched_folder.id, folder.id);
    }

    #[tokio::test]
    async fn test_get_folder_not_found() {
        let pool = db::create_test_pool().await;
        let server = TestServer::new(routes(pool)).unwrap();

        let response = server.get("/folders/999").await;

        response.assert_status(StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_update_folder_success() {
        let pool = db::create_test_pool().await;
        let folder = create_test_folder(&pool, "folder1").await;
        let server = TestServer::new(routes(pool.clone())).unwrap();

        let response = server
            .put(&format!("/folders/{}", folder.id))
            .json(&json!({ "name": "updated name" }))
            .await;

        response.assert_status(StatusCode::OK);
        let updated_folder: Folder = response.json();
        assert_eq!(updated_folder.name, "updated name");
    }

    #[tokio::test]
    async fn test_update_folder_not_found() {
        let pool = db::create_test_pool().await;
        let server = TestServer::new(routes(pool)).unwrap();

        let response = server
            .put("/folders/999")
            .json(&json!({ "name": "updated name" }))
            .await;

        response.assert_status(StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_update_folder_bad_request() {
        let pool = db::create_test_pool().await;
        let folder = create_test_folder(&pool, "folder1").await;
        let server = TestServer::new(routes(pool.clone())).unwrap();

        let response = server
            .put(&format!("/folders/{}", folder.id))
            .json(&json!({ "name": "" }))
            .await;

        response.assert_status(StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_archive_folder_success() {
        let pool = db::create_test_pool().await;
        let folder = create_test_folder(&pool, "folder1").await;
        let server = TestServer::new(routes(pool.clone())).unwrap();

        let response = server.put(&format!("/folders/{}/archive", folder.id)).await;

        response.assert_status(StatusCode::OK);

        // Verify that the folder is archived
        let folders: Vec<Folder> = server.get("/folders").await.json();
        assert_eq!(folders.len(), 0);
    }

    #[tokio::test]
    async fn test_archive_folder_not_found() {
        let pool = db::create_test_pool().await;
        let server = TestServer::new(routes(pool)).unwrap();

        let response = server.put("/folders/999/archive").await;

        response.assert_status(StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_unarchive_folder_success() {
        let pool = db::create_test_pool().await;
        let folder = create_test_folder(&pool, "folder1").await;
        sqlx::query("UPDATE folders SET archived_at = ? WHERE id = ?")
            .bind(Utc::now().naive_utc())
            .bind(folder.id)
            .execute(&pool)
            .await
            .unwrap();
        let server = TestServer::new(routes(pool.clone())).unwrap();

        let response = server
            .put(&format!("/folders/{}/unarchive", folder.id))
            .await;

        response.assert_status(StatusCode::OK);

        // Verify that the folder is unarchived
        let folders: Vec<Folder> = server.get("/folders").await.json();
        assert_eq!(folders.len(), 1);
    }

    #[tokio::test]
    async fn test_unarchive_folder_not_found() {
        let pool = db::create_test_pool().await;
        let server = TestServer::new(routes(pool)).unwrap();

        let response = server.put("/folders/999/unarchive").await;

        response.assert_status(StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_delete_folder_success() {
        let pool = db::create_test_pool().await;
        let folder = create_test_folder(&pool, "folder1").await;
        let server = TestServer::new(routes(pool.clone())).unwrap();

        let response = server.delete(&format!("/folders/{}", folder.id)).await;

        response.assert_status(StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_delete_folder_not_found() {
        let pool = db::create_test_pool().await;
        let server = TestServer::new(routes(pool)).unwrap();

        let response = server.delete("/folders/999").await;

        response.assert_status(StatusCode::NOT_FOUND);
    }
}
