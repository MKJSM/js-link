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
pub struct Request {
    pub id: i64,
    pub name: String,
    pub method: String,
    pub url: String,
    pub body: Option<String>,
    pub headers: Option<String>,
    pub folder_id: Option<i64>,
    pub request_type: String, // 'api' or 'ws'
    pub body_type: String,    // 'none', 'json', 'xml', 'text', 'form', 'multipart', 'binary'
    pub body_content: Option<String>,
    pub auth_type: String, // 'none', 'bearer', 'basic'
    pub auth_token: Option<String>,
    pub auth_username: Option<String>,
    pub auth_password: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
}

#[derive(sqlx::FromRow, Clone)]
pub struct RequestDb {
    pub id: i64,
    pub name: String,
    pub method: String,
    pub url: String,
    pub body: Option<String>,
    pub headers: Option<String>,
    pub folder_id: Option<i64>,
    pub request_type: String, // 'api' or 'ws'
    pub body_type: String,
    pub body_content: Option<String>,
    pub auth_type: String,
    pub auth_token: Option<String>,
    pub auth_username: Option<String>,
    pub auth_password: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub archived_at: Option<NaiveDateTime>,
}

impl From<RequestDb> for Request {
    fn from(r: RequestDb) -> Self {
        Self {
            id: r.id,
            name: r.name,
            method: r.method,
            url: r.url,
            body: r.body,
            headers: r.headers,
            folder_id: r.folder_id,
            request_type: r.request_type,
            body_type: r.body_type,
            body_content: r.body_content,
            auth_type: r.auth_type,
            auth_token: r.auth_token,
            auth_username: r.auth_username,
            auth_password: r.auth_password,
            created_at: DateTime::from_naive_utc_and_offset(r.created_at, Utc),
            updated_at: DateTime::from_naive_utc_and_offset(r.updated_at, Utc),
            archived_at: r
                .archived_at
                .map(|d| DateTime::from_naive_utc_and_offset(d, Utc)),
        }
    }
}

#[derive(Deserialize, Clone)]
pub struct CreateRequest {
    pub name: String,
    pub method: String,
    pub url: String,
    pub body: Option<String>,
    pub headers: Option<String>,
    pub folder_id: Option<i64>,
    #[serde(default = "default_request_type")]
    pub request_type: String, // 'api' or 'ws'
    #[serde(default = "default_body_type")]
    pub body_type: String,
    pub body_content: Option<String>,
    #[serde(default = "default_auth_type")]
    pub auth_type: String,
    pub auth_token: Option<String>,
    pub auth_username: Option<String>,
    pub auth_password: Option<String>,
}

fn default_request_type() -> String {
    "api".to_string()
}

fn default_body_type() -> String {
    "none".to_string()
}

fn default_auth_type() -> String {
    "none".to_string()
}

#[derive(Deserialize, Clone)]
pub struct UpdateRequest {
    name: String,
    method: String,
    url: String,
    body: Option<String>,
    headers: Option<String>,
    folder_id: Option<i64>,
    #[serde(default = "default_request_type")]
    request_type: String, // 'api' or 'ws'
    #[serde(default = "default_body_type")]
    body_type: String,
    body_content: Option<String>,
    #[serde(default = "default_auth_type")]
    auth_type: String,
    auth_token: Option<String>,
    auth_username: Option<String>,
    auth_password: Option<String>,
}

#[derive(Deserialize)]
pub struct ListRequestsQuery {
    #[serde(default)]
    include_archived: bool,
    #[serde(default)]
    folder_id: Option<i64>,
}

pub enum RequestError {
    InvalidName,
    InvalidMethod,
    RequestNotFound,
    DatabaseError(#[allow(dead_code)] sqlx::Error),
}

impl From<sqlx::Error> for RequestError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => RequestError::RequestNotFound,
            _ => RequestError::DatabaseError(e),
        }
    }
}

impl IntoResponse for RequestError {
    fn into_response(self) -> Response {
        match self {
            RequestError::InvalidName => {
                (StatusCode::BAD_REQUEST, "Invalid request name").into_response()
            }
            RequestError::InvalidMethod => {
                (StatusCode::BAD_REQUEST, "Invalid HTTP method").into_response()
            }
            RequestError::RequestNotFound => {
                (StatusCode::NOT_FOUND, "Request not found").into_response()
            }
            RequestError::DatabaseError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
            }
        }
    }
}

async fn create_request(
    State(pool): State<DbPool>,
    Json(payload): Json<CreateRequest>,
) -> Result<impl IntoResponse, RequestError> {
    log::debug!(
        "Creating request: name={}, method={}, url={}, folder_id={:?}, request_type={}",
        payload.name,
        payload.method,
        payload.url,
        payload.folder_id,
        payload.request_type
    );

    if payload.name.is_empty() {
        log::warn!("Attempted to create request with empty name");
        return Err(RequestError::InvalidName);
    }

    // Only validate HTTP method for API requests, not for WebSocket
    if payload.request_type != "ws" {
        match payload.method.to_uppercase().as_str() {
            "GET" | "POST" | "PUT" | "DELETE" | "PATCH" | "HEAD" | "OPTIONS" => (),
            _ => {
                log::warn!("Invalid HTTP method: {}", payload.method);
                return Err(RequestError::InvalidMethod);
            }
        }
    }

    let request_db = sqlx::query_as!(
        RequestDb,
        "INSERT INTO requests (name, method, url, body, headers, folder_id, request_type, body_type, body_content, auth_type, auth_token, auth_username, auth_password) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id, name, method, url, body, headers, folder_id, request_type, body_type, body_content, auth_type, auth_token, auth_username, auth_password, created_at, updated_at, archived_at",
        payload.name,
        payload.method,
        payload.url,
        payload.body,
        payload.headers,
        payload.folder_id,
        payload.request_type,
        payload.body_type,
        payload.body_content,
        payload.auth_type,
        payload.auth_token,
        payload.auth_username,
        payload.auth_password
    )
    .fetch_one(&pool)
    .await?;

    log::info!(
        "Created request: id={}, name={}, method={}",
        request_db.id,
        request_db.name,
        request_db.method
    );
    Ok((StatusCode::CREATED, Json(Request::from(request_db))))
}

async fn list_requests(
    State(pool): State<DbPool>,
    Query(query): Query<ListRequestsQuery>,
) -> Result<impl IntoResponse, RequestError> {
    log::debug!(
        "Listing requests: include_archived={}, folder_id={:?}",
        query.include_archived,
        query.folder_id
    );

    let requests_db = match (query.include_archived, query.folder_id) {
        (false, None) => {
            sqlx::query_as!(
                RequestDb,
                "SELECT id, name, method, url, body, headers, folder_id, request_type, body_type, body_content, auth_type, auth_token, auth_username, auth_password, created_at, updated_at, archived_at FROM requests WHERE archived_at IS NULL"
            )
            .fetch_all(&pool)
            .await?
        }
        (true, None) => {
            sqlx::query_as!(
                RequestDb,
                "SELECT id, name, method, url, body, headers, folder_id, request_type, body_type, body_content, auth_type, auth_token, auth_username, auth_password, created_at, updated_at, archived_at FROM requests"
            )
            .fetch_all(&pool)
            .await?
        }
        (false, Some(folder_id)) => {
            sqlx::query_as!(
                RequestDb,
                "SELECT id, name, method, url, body, headers, folder_id, request_type, body_type, body_content, auth_type, auth_token, auth_username, auth_password, created_at, updated_at, archived_at FROM requests WHERE archived_at IS NULL AND folder_id = ?",
                folder_id
            )
            .fetch_all(&pool)
            .await?
        }
        (true, Some(folder_id)) => {
            sqlx::query_as!(
                RequestDb,
                "SELECT id, name, method, url, body, headers, folder_id, request_type, body_type, body_content, auth_type, auth_token, auth_username, auth_password, created_at, updated_at, archived_at FROM requests WHERE folder_id = ?",
                folder_id
            )
            .fetch_all(&pool)
            .await?
        }
    };

    let requests: Vec<Request> = requests_db.into_iter().map(Request::from).collect();
    log::debug!("Found {} requests", requests.len());

    Ok(Json(requests))
}

async fn get_request(
    State(pool): State<DbPool>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, RequestError> {
    log::debug!("Getting request with id: {}", id);

    let request_db = sqlx::query_as!(
        RequestDb,
        "SELECT id, name, method, url, body, headers, folder_id, request_type, body_type, body_content, auth_type, auth_token, auth_username, auth_password, created_at, updated_at, archived_at FROM requests WHERE id = ?",
        id
    )
    .fetch_one(&pool)
    .await?;

    log::debug!(
        "Found request: id={}, name={}, method={}",
        request_db.id,
        request_db.name,
        request_db.method
    );
    Ok(Json(Request::from(request_db)))
}

async fn update_request(
    State(pool): State<DbPool>,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateRequest>,
) -> Result<impl IntoResponse, RequestError> {
    log::debug!(
        "Updating request id={}: name={}, method={}, url={}, request_type={}",
        id,
        payload.name,
        payload.method,
        payload.url,
        payload.request_type
    );

    if payload.name.is_empty() {
        log::warn!("Attempted to update request {} with empty name", id);
        return Err(RequestError::InvalidName);
    }

    // Only validate HTTP method for API requests, not for WebSocket
    if payload.request_type != "ws" {
        match payload.method.to_uppercase().as_str() {
            "GET" | "POST" | "PUT" | "DELETE" | "PATCH" | "HEAD" | "OPTIONS" => (),
            _ => {
                log::warn!("Invalid HTTP method for request {}: {}", id, payload.method);
                return Err(RequestError::InvalidMethod);
            }
        }
    }

    let request_db = sqlx::query_as!(
        RequestDb,
        "UPDATE requests SET name = ?, method = ?, url = ?, body = ?, headers = ?, folder_id = ?, request_type = ?, body_type = ?, body_content = ?, auth_type = ?, auth_token = ?, auth_username = ?, auth_password = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? RETURNING id, name, method, url, body, headers, folder_id, request_type, body_type, body_content, auth_type, auth_token, auth_username, auth_password, created_at, updated_at, archived_at",
        payload.name,
        payload.method,
        payload.url,
        payload.body,
        payload.headers,
        payload.folder_id,
        payload.request_type,
        payload.body_type,
        payload.body_content,
        payload.auth_type,
        payload.auth_token,
        payload.auth_username,
        payload.auth_password,
        id
    )
    .fetch_one(&pool)
    .await?;

    log::info!(
        "Updated request: id={}, name={}, method={}",
        request_db.id,
        request_db.name,
        request_db.method
    );
    Ok(Json(Request::from(request_db)))
}

async fn archive_request(
    State(pool): State<DbPool>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, RequestError> {
    log::debug!("Archiving request id: {}", id);

    let now = Utc::now().naive_utc();
    let result = sqlx::query("UPDATE requests SET archived_at = ? WHERE id = ?")
        .bind(now)
        .bind(id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        log::warn!("Request not found for archiving: id={}", id);
        return Err(RequestError::RequestNotFound);
    }

    log::info!("Archived request: id={}", id);
    Ok(StatusCode::OK)
}

async fn unarchive_request(
    State(pool): State<DbPool>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, RequestError> {
    log::debug!("Unarchiving request id: {}", id);

    let result = sqlx::query("UPDATE requests SET archived_at = NULL WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        log::warn!("Request not found for unarchiving: id={}", id);
        return Err(RequestError::RequestNotFound);
    }

    log::info!("Unarchived request: id={}", id);
    Ok(StatusCode::OK)
}

async fn delete_request(
    State(pool): State<DbPool>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, RequestError> {
    log::debug!("Deleting request id: {}", id);

    let result = sqlx::query("DELETE FROM requests WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        log::warn!("Request not found for deletion: id={}", id);
        return Err(RequestError::RequestNotFound);
    }

    log::info!("Deleted request: id={}", id);
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes(pool: DbPool) -> Router {
    Router::new()
        .route("/requests", post(create_request).get(list_requests))
        .route(
            "/requests/:id",
            get(get_request).put(update_request).delete(delete_request),
        )
        .route("/requests/:id/archive", put(archive_request))
        .route("/requests/:id/unarchive", put(unarchive_request))
        .with_state(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use axum_test::TestServer;
    use chrono::Utc;
    use serde_json::json;

    async fn create_test_request(pool: &DbPool, req: &CreateRequest) -> RequestDb {
        sqlx::query_as!(
            RequestDb,
            "INSERT INTO requests (name, method, url, body, headers, folder_id, request_type, body_type, body_content, auth_type, auth_token, auth_username, auth_password) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id, name, method, url, body, headers, folder_id, request_type, body_type, body_content, auth_type, auth_token, auth_username, auth_password, created_at, updated_at, archived_at",
            req.name,
            req.method,
            req.url,
            req.body,
            req.headers,
            req.folder_id,
            req.request_type,
            req.body_type,
            req.body_content,
            req.auth_type,
            req.auth_token,
            req.auth_username,
            req.auth_password
        )
        .fetch_one(pool)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn test_create_request_success() {
        let pool = db::create_test_pool().await;
        let server = TestServer::new(routes(pool)).unwrap();

        let response = server
            .post("/requests")
            .json(&json!({
                "name": "New Request",
                "method": "GET",
                "url": "http://example.com",
                "body": null,
                "headers": null,
                "folder_id": null
            }))
            .await;

        response.assert_status(StatusCode::CREATED);
    }

    #[tokio::test]
    async fn test_create_request_invalid_method() {
        let pool = db::create_test_pool().await;
        let server = TestServer::new(routes(pool)).unwrap();

        let response = server
            .post("/requests")
            .json(&json!({
                "name": "New Request",
                "method": "INVALID",
                "url": "http://example.com",
                "body": null,
                "headers": null,
                "folder_id": null
            }))
            .await;

        response.assert_status(StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_create_request_empty_name() {
        let pool = db::create_test_pool().await;
        let server = TestServer::new(routes(pool)).unwrap();

        let response = server
            .post("/requests")
            .json(&json!({
                "name": "",
                "method": "GET",
                "url": "http://example.com",
                "body": null,
                "headers": null,
                "folder_id": null
            }))
            .await;

        response.assert_status(StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_list_requests() {
        let pool = db::create_test_pool().await;
        let req1 = CreateRequest {
            name: "req1".to_string(),
            method: "GET".to_string(),
            url: "http://example.com".to_string(),
            body: None,
            headers: None,
            folder_id: None,
            request_type: "api".to_string(),
            body_type: "none".to_string(),
            body_content: None,
            auth_type: "none".to_string(),
            auth_token: None,
            auth_username: None,
            auth_password: None,
        };
        create_test_request(&pool, &req1).await;
        let server = TestServer::new(routes(pool.clone())).unwrap();

        let response = server.get("/requests").await;

        response.assert_status(StatusCode::OK);
        let requests: Vec<Request> = response.json();
        assert_eq!(requests.len(), 1);
    }

    #[tokio::test]
    async fn test_list_requests_include_archived() {
        let pool = db::create_test_pool().await;
        let req1 = CreateRequest {
            name: "req1".to_string(),
            method: "GET".to_string(),
            url: "http://example.com".to_string(),
            body: None,
            headers: None,
            folder_id: None,
            request_type: "api".to_string(),
            body_type: "none".to_string(),
            body_content: None,
            auth_type: "none".to_string(),
            auth_token: None,
            auth_username: None,
            auth_password: None,
        };
        let req2 = create_test_request(&pool, &req1).await;
        sqlx::query("UPDATE requests SET archived_at = ? WHERE id = ?")
            .bind(Utc::now().naive_utc())
            .bind(req2.id)
            .execute(&pool)
            .await
            .unwrap();
        let server = TestServer::new(routes(pool.clone())).unwrap();

        let response = server.get("/requests?include_archived=true").await;

        response.assert_status(StatusCode::OK);
        let requests: Vec<Request> = response.json();
        assert_eq!(requests.len(), 1);
    }

    #[tokio::test]
    async fn test_list_requests_empty() {
        let pool = db::create_test_pool().await;
        let server = TestServer::new(routes(pool.clone())).unwrap();

        let response = server.get("/requests").await;

        response.assert_status(StatusCode::OK);
        let requests: Vec<Request> = response.json();
        assert_eq!(requests.len(), 0);
    }

    #[tokio::test]
    async fn test_get_request_success() {
        let pool = db::create_test_pool().await;
        let req1 = CreateRequest {
            name: "req1".to_string(),
            method: "GET".to_string(),
            url: "http://example.com".to_string(),
            body: None,
            headers: None,
            folder_id: None,
            request_type: "api".to_string(),
            body_type: "none".to_string(),
            body_content: None,
            auth_type: "none".to_string(),
            auth_token: None,
            auth_username: None,
            auth_password: None,
        };
        let request = create_test_request(&pool, &req1).await;
        let server = TestServer::new(routes(pool.clone())).unwrap();

        let response = server.get(&format!("/requests/{}", request.id)).await;

        response.assert_status(StatusCode::OK);
        let fetched_request: Request = response.json();
        assert_eq!(fetched_request.id, request.id);
    }

    #[tokio::test]
    async fn test_get_request_not_found() {
        let pool = db::create_test_pool().await;
        let server = TestServer::new(routes(pool)).unwrap();

        let response = server.get("/requests/999").await;

        response.assert_status(StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_update_request_success() {
        let pool = db::create_test_pool().await;
        // Create a folder with id 1 for the foreign key constraint
        sqlx::query!(
            "INSERT INTO folders (id, name) VALUES (?, ?)",
            1,
            "Test Folder"
        )
        .execute(&pool)
        .await
        .unwrap();

        let req = CreateRequest {
            name: "old name".to_string(),
            method: "GET".to_string(),
            url: "http://example.com".to_string(),
            body: None,
            headers: None,
            folder_id: None,
            request_type: "api".to_string(),
            body_type: "none".to_string(),
            body_content: None,
            auth_type: "none".to_string(),
            auth_token: None,
            auth_username: None,
            auth_password: None,
        };
        let request_db = create_test_request(&pool, &req).await;
        let server = TestServer::new(routes(pool.clone())).unwrap();

        let response = server
            .put(&format!("/requests/{}", request_db.id))
            .json(&json!({
                "name": "new name",
                "method": "POST",
                "url": "http://new.com",
                "body": "new body",
                "headers": "new headers",
                "folder_id": 1
            }))
            .await;

        response.assert_status(StatusCode::OK);
        let updated_request: Request = response.json();
        assert_eq!(updated_request.name, "new name");
        assert_eq!(updated_request.method, "POST");
        assert_eq!(updated_request.url, "http://new.com");
        assert_eq!(updated_request.body, Some("new body".to_string()));
        assert_eq!(updated_request.headers, Some("new headers".to_string()));
        assert_eq!(updated_request.folder_id, Some(1));
    }

    #[tokio::test]
    async fn test_update_request_not_found() {
        let pool = db::create_test_pool().await;
        let server = TestServer::new(routes(pool)).unwrap();

        let response = server
            .put("/requests/999")
            .json(&json!({
                "name": "new name",
                "method": "POST",
                "url": "http://new.com",
                "body": "new body",
                "headers": "new headers",
                "folder_id": 1
            }))
            .await;

        response.assert_status(StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_update_request_bad_request_empty_name() {
        let pool = db::create_test_pool().await;
        let req = CreateRequest {
            name: "old name".to_string(),
            method: "GET".to_string(),
            url: "http://example.com".to_string(),
            body: None,
            headers: None,
            folder_id: None,
            request_type: "api".to_string(),
            body_type: "none".to_string(),
            body_content: None,
            auth_type: "none".to_string(),
            auth_token: None,
            auth_username: None,
            auth_password: None,
        };
        let request_db = create_test_request(&pool, &req).await;
        let server = TestServer::new(routes(pool.clone())).unwrap();

        let response = server
            .put(&format!("/requests/{}", request_db.id))
            .json(&json!({
                "name": "",
                "method": "POST",
                "url": "http://new.com",
                "body": "new body",
                "headers": "new headers",
                "folder_id": 1
            }))
            .await;

        response.assert_status(StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_update_request_bad_request_invalid_method() {
        let pool = db::create_test_pool().await;
        let req = CreateRequest {
            name: "old name".to_string(),
            method: "GET".to_string(),
            url: "http://example.com".to_string(),
            body: None,
            headers: None,
            folder_id: None,
            request_type: "api".to_string(),
            body_type: "none".to_string(),
            body_content: None,
            auth_type: "none".to_string(),
            auth_token: None,
            auth_username: None,
            auth_password: None,
        };
        let request_db = create_test_request(&pool, &req).await;
        let server = TestServer::new(routes(pool.clone())).unwrap();

        let response = server
            .put(&format!("/requests/{}", request_db.id))
            .json(&json!({
                "name": "new name",
                "method": "INVALID",
                "url": "http://new.com",
                "body": "new body",
                "headers": "new headers",
                "folder_id": 1
            }))
            .await;

        response.assert_status(StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_archive_request_success() {
        let pool = db::create_test_pool().await;
        let req = CreateRequest {
            name: "req to archive".to_string(),
            method: "GET".to_string(),
            url: "http://example.com".to_string(),
            body: None,
            headers: None,
            folder_id: None,
            request_type: "api".to_string(),
            body_type: "none".to_string(),
            body_content: None,
            auth_type: "none".to_string(),
            auth_token: None,
            auth_username: None,
            auth_password: None,
        };
        let request_db = create_test_request(&pool, &req).await;
        let server = TestServer::new(routes(pool.clone())).unwrap();

        let response = server
            .put(&format!("/requests/{}/archive", request_db.id))
            .await;

        response.assert_status(StatusCode::OK);

        let requests: Vec<Request> = server.get("/requests").await.json();
        assert_eq!(requests.len(), 0);
    }

    #[tokio::test]
    async fn test_archive_request_not_found() {
        let pool = db::create_test_pool().await;
        let server = TestServer::new(routes(pool)).unwrap();

        let response = server.put("/requests/999/archive").await;

        response.assert_status(StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_unarchive_request_success() {
        let pool = db::create_test_pool().await;
        let req = CreateRequest {
            name: "req to unarchive".to_string(),
            method: "GET".to_string(),
            url: "http://example.com".to_string(),
            body: None,
            headers: None,
            folder_id: None,
            request_type: "api".to_string(),
            body_type: "none".to_string(),
            body_content: None,
            auth_type: "none".to_string(),
            auth_token: None,
            auth_username: None,
            auth_password: None,
        };
        let request_db = create_test_request(&pool, &req).await;
        sqlx::query("UPDATE requests SET archived_at = ? WHERE id = ?")
            .bind(Utc::now().naive_utc())
            .bind(request_db.id)
            .execute(&pool)
            .await
            .unwrap();
        let server = TestServer::new(routes(pool.clone())).unwrap();

        let response = server
            .put(&format!("/requests/{}/unarchive", request_db.id))
            .await;

        response.assert_status(StatusCode::OK);

        let requests: Vec<Request> = server.get("/requests").await.json();
        assert_eq!(requests.len(), 1);
    }

    #[tokio::test]
    async fn test_unarchive_request_not_found() {
        let pool = db::create_test_pool().await;
        let server = TestServer::new(routes(pool)).unwrap();

        let response = server.put("/requests/999/unarchive").await;

        response.assert_status(StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_delete_request_success() {
        let pool = db::create_test_pool().await;
        let req = CreateRequest {
            name: "req to delete".to_string(),
            method: "GET".to_string(),
            url: "http://example.com".to_string(),
            body: None,
            headers: None,
            folder_id: None,
            request_type: "api".to_string(),
            body_type: "none".to_string(),
            body_content: None,
            auth_type: "none".to_string(),
            auth_token: None,
            auth_username: None,
            auth_password: None,
        };
        let request_db = create_test_request(&pool, &req).await;
        let server = TestServer::new(routes(pool.clone())).unwrap();

        let response = server.delete(&format!("/requests/{}", request_db.id)).await;

        response.assert_status(StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_delete_request_not_found() {
        let pool = db::create_test_pool().await;
        let server = TestServer::new(routes(pool)).unwrap();

        let response = server.delete("/requests/999").await;

        response.assert_status(StatusCode::NOT_FOUND);
    }
}
