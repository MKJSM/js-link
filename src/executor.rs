use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use reqwest::{Client, Proxy};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{
    db::DbPool, environments::EnvironmentDb, network::NetworkSettings, requests::RequestDb,
};
use std::fmt;

#[derive(Debug)]
pub enum ExecutorError {
    RequestNotFound,
    NetworkError(String),
    SubstitutionError(String),
    DatabaseError(#[allow(dead_code)] sqlx::Error),
}

impl fmt::Display for ExecutorError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ExecutorError::RequestNotFound => write!(f, "Request not found"),
            ExecutorError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            ExecutorError::SubstitutionError(msg) => {
                write!(f, "Variable substitution error: {}", msg)
            }
            ExecutorError::DatabaseError(_) => write!(f, "Database error"),
        }
    }
}

impl From<reqwest::Error> for ExecutorError {
    fn from(e: reqwest::Error) -> Self {
        ExecutorError::NetworkError(e.to_string())
    }
}

impl From<sqlx::Error> for ExecutorError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => ExecutorError::RequestNotFound,
            _ => ExecutorError::DatabaseError(e),
        }
    }
}

impl IntoResponse for ExecutorError {
    fn into_response(self) -> Response {
        match self {
            ExecutorError::RequestNotFound => {
                (StatusCode::NOT_FOUND, "Request not found").into_response()
            }
            ExecutorError::NetworkError(msg) => {
                (StatusCode::BAD_GATEWAY, format!("Network error: {}", msg)).into_response()
            }
            ExecutorError::SubstitutionError(msg) => (
                StatusCode::BAD_REQUEST,
                format!("Variable substitution error: {}", msg),
            )
                .into_response(),
            ExecutorError::DatabaseError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ExecuteRequestPayload {
    request_id: Option<i64>,
    environment_id: Option<i64>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    method: Option<String>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    headers: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ExecuteResponse {
    status: u16,
    headers: HashMap<String, String>,
    body: String,
    request_name: String,
    request_url: String,
}

// Function to substitute variables in a string
fn substitute_variables(
    template: &str,
    variables: &HashMap<String, String>,
) -> Result<String, ExecutorError> {
    log::debug!("Substituting variables in template: {}", template);
    log::debug!(
        "Available variables: {:?}",
        variables.keys().collect::<Vec<_>>()
    );

    let mut result = template.to_string();
    for (key, value) in variables {
        let placeholder = format!("{{{{{}}}}}", key);
        if result.contains(&placeholder) {
            log::debug!("Replacing {} with {}", placeholder, value);
            result = result.replace(&placeholder, value);
        }
    }
    // Check if any placeholders remain
    if result.contains("{{") && result.contains("}}") {
        log::warn!("Unresolved variables found in result: {}", result);
        // This is a basic check; a more robust solution might use regex to find unmatched placeholders
        Err(ExecutorError::SubstitutionError(
            "Unresolved variables found".to_string(),
        ))
    } else {
        log::debug!("Variable substitution complete: {}", result);
        Ok(result)
    }
}

// Function to build reqwest client with network settings
async fn build_reqwest_client(pool: &DbPool) -> Result<Client, ExecutorError> {
    log::debug!("Building reqwest client with network settings");

    let network_settings = sqlx::query_as!(
        NetworkSettings,
        "SELECT id, auto_proxy, http_proxy, https_proxy, no_proxy FROM network_settings WHERE id = 1"
    )
    .fetch_one(pool)
    .await
    .unwrap_or_else(|_| {
        log::debug!("Using default network settings (auto_proxy=true)");
        NetworkSettings {
            id: 1,
            auto_proxy: true,
            http_proxy: None,
            https_proxy: None,
            no_proxy: None,
        }
    });

    log::debug!(
        "Network settings: auto_proxy={}, http_proxy={:?}, https_proxy={:?}",
        network_settings.auto_proxy,
        network_settings.http_proxy,
        network_settings.https_proxy
    );

    let mut client_builder = Client::builder();

    if !network_settings.auto_proxy {
        log::debug!("Manual proxy configuration enabled");
        if let Some(http_proxy_str) = network_settings.http_proxy {
            log::debug!("Setting HTTP proxy: {}", http_proxy_str);
            if let Ok(proxy) = Proxy::http(&http_proxy_str) {
                client_builder = client_builder.proxy(proxy);
            } else {
                log::error!("Invalid HTTP proxy: {}", http_proxy_str);
                return Err(ExecutorError::NetworkError(format!(
                    "Invalid HTTP proxy: {}",
                    http_proxy_str
                )));
            }
        }
        if let Some(https_proxy_str) = network_settings.https_proxy {
            log::debug!("Setting HTTPS proxy: {}", https_proxy_str);
            if let Ok(proxy) = Proxy::https(&https_proxy_str) {
                client_builder = client_builder.proxy(proxy);
            } else {
                log::error!("Invalid HTTPS proxy: {}", https_proxy_str);
                return Err(ExecutorError::NetworkError(format!(
                    "Invalid HTTPS proxy: {}",
                    https_proxy_str
                )));
            }
        }
    } else {
        log::debug!("Auto proxy detection enabled");
    }

    let client = client_builder.build()?;
    log::debug!("Reqwest client built successfully");
    Ok(client)
}

async fn execute_request_handler(
    State(pool): State<DbPool>,
    Json(payload): Json<ExecuteRequestPayload>,
) -> Result<impl IntoResponse, ExecutorError> {
    log::info!(
        "Executing request: request_id={:?}, environment_id={:?}",
        payload.request_id,
        payload.environment_id
    );

    // 1. Fetch Request Details or use provided values
    let mut request = if let Some(request_id) = payload.request_id {
        log::debug!("Fetching request details for id: {}", request_id);
        let request_db = sqlx::query_as!(
            RequestDb,
            "SELECT id, name, method, url, body, headers, folder_id, request_type, body_type, body_content, auth_type, auth_token, auth_username, auth_password, created_at, updated_at, archived_at FROM requests WHERE id = ?",
            request_id
        )
        .fetch_one(&pool)
        .await?;
        let mut req = crate::requests::Request::from(request_db);

        // Override with provided values if they exist
        if let Some(url) = payload.url {
            req.url = url;
        }
        if let Some(method) = payload.method {
            req.method = method;
        }
        if let Some(body) = payload.body {
            req.body = Some(body);
        }
        // Always use provided headers (even if empty) to allow clearing headers
        if let Some(headers_map) = &payload.headers {
            if headers_map.is_empty() {
                // Empty headers map means no headers should be sent
                req.headers = None;
            } else {
                req.headers = Some(serde_json::to_string(&headers_map).map_err(|e| {
                    ExecutorError::SubstitutionError(format!("Failed to serialize headers: {}", e))
                })?);
            }
        }
        req
    } else {
        // Direct execution without saved request
        if payload.url.is_none() || payload.method.is_none() {
            return Err(ExecutorError::NetworkError(
                "URL and method are required for direct execution".to_string(),
            ));
        }
        crate::requests::Request {
            id: 0,
            name: "Direct Request".to_string(),
            method: payload.method.unwrap(),
            url: payload.url.unwrap(),
            body: payload.body,
            headers: payload
                .headers
                .as_ref()
                .and_then(|h| serde_json::to_string(h).ok()),
            folder_id: None,
            request_type: "api".to_string(),
            body_type: "none".to_string(),
            body_content: None,
            auth_type: "none".to_string(),
            auth_token: None,
            auth_username: None,
            auth_password: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            archived_at: None,
        }
    };

    log::debug!(
        "Request loaded: name={}, method={}, url={}",
        request.name,
        request.method,
        request.url
    );

    // 2. Fetch Environment Variables
    let mut variables: HashMap<String, String> = HashMap::new();
    if let Some(env_id) = payload.environment_id {
        log::debug!(
            "Loading environment variables for environment_id: {}",
            env_id
        );
        let environment_db = sqlx::query_as!(
            EnvironmentDb,
            "SELECT id, name, variables, created_at, updated_at, archived_at FROM environments WHERE id = ?",
            env_id
        )
        .fetch_one(&pool)
        .await?;
        log::debug!("Environment loaded: name={}", environment_db.name);
        let env_vars: HashMap<String, String> = serde_json::from_str(&environment_db.variables)
            .map_err(|e| {
                log::error!("Failed to parse environment variables: {}", e);
                ExecutorError::SubstitutionError(format!(
                    "Failed to parse environment variables: {}",
                    e
                ))
            })?;
        log::debug!("Loaded {} environment variables", env_vars.len());
        variables.extend(env_vars);
    } else {
        log::debug!("No environment specified, using empty variable set");
    }

    // 3. Perform Variable Substitution
    log::debug!("Performing variable substitution");
    let resolved_url = substitute_variables(&request.url, &variables)?;
    let resolved_body = request
        .body
        .as_ref()
        .map(|b| substitute_variables(b, &variables))
        .transpose()?;
    let resolved_headers = request
        .headers
        .as_ref()
        .map(|h| substitute_variables(h, &variables))
        .transpose()?;

    let resolved_auth_token = request
        .auth_token
        .as_ref()
        .map(|t| substitute_variables(t, &variables))
        .transpose()?;
    let resolved_auth_username = request
        .auth_username
        .as_ref()
        .map(|u| substitute_variables(u, &variables))
        .transpose()?;
    let resolved_auth_password = request
        .auth_password
        .as_ref()
        .map(|p| substitute_variables(p, &variables))
        .transpose()?;

    request.url = resolved_url.clone();
    request.body = resolved_body.clone();
    request.headers = resolved_headers.clone();
    request.auth_token = resolved_auth_token;
    request.auth_username = resolved_auth_username;
    request.auth_password = resolved_auth_password;

    log::debug!("Resolved URL: {}", request.url);
    if let Some(ref body) = resolved_body {
        log::debug!("Resolved body length: {} bytes", body.len());
    }

    // 4. Build Reqwest Client with Network Settings
    let client = build_reqwest_client(&pool).await?;

    // 5. Execute HTTP Request
    log::info!("Executing {} request to: {}", request.method, request.url);
    let mut req_builder = client.request(
        reqwest::Method::from_bytes(request.method.as_bytes()).map_err(|e| {
            log::error!("Invalid HTTP method: {}", e);
            ExecutorError::NetworkError(format!("Invalid HTTP method: {}", e))
        })?,
        &request.url,
    );

    // Apply authentication
    match request.auth_type.as_str() {
        "bearer" => {
            if let Some(token) = &request.auth_token {
                log::debug!("Applying Bearer token authentication");
                req_builder = req_builder.header("Authorization", format!("Bearer {}", token));
            }
        }
        "basic" => {
            if let (Some(username), Some(password)) =
                (&request.auth_username, &request.auth_password)
            {
                log::debug!("Applying Basic authentication");
                req_builder = req_builder.basic_auth(username, Some(password));
            }
        }
        _ => {
            log::debug!("No authentication applied");
        }
    }

    // Handle body based on body_type
    if let Some(body_content) = &request.body_content {
        log::debug!(
            "Adding request body (type: {}): {} bytes",
            request.body_type,
            body_content.len()
        );

        match request.body_type.as_str() {
            "json" => {
                req_builder = req_builder
                    .header("Content-Type", "application/json")
                    .body(body_content.clone());
            }
            "xml" => {
                req_builder = req_builder
                    .header("Content-Type", "application/xml")
                    .body(body_content.clone());
            }
            "text" => {
                req_builder = req_builder
                    .header("Content-Type", "text/plain")
                    .body(body_content.clone());
            }
            "form" => {
                // Parse form data from JSON format {"key1": "value1", "key2": "value2"}
                let form_data: HashMap<String, String> = serde_json::from_str(body_content)
                    .map_err(|e| {
                        log::error!("Failed to parse form data: {}", e);
                        ExecutorError::SubstitutionError(format!(
                            "Failed to parse form data: {}",
                            e
                        ))
                    })?;
                // Build URL-encoded form data manually
                let form_string: Vec<String> = form_data
                    .iter()
                    .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
                    .collect();
                req_builder = req_builder
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .body(form_string.join("&"));
            }
            "multipart" => {
                // Parse multipart data from JSON format {"key1": "value1", "key2": "value2"}
                let multipart_data: HashMap<String, String> = serde_json::from_str(body_content)
                    .map_err(|e| {
                        log::error!("Failed to parse multipart data: {}", e);
                        ExecutorError::SubstitutionError(format!(
                            "Failed to parse multipart data: {}",
                            e
                        ))
                    })?;
                let mut form = reqwest::multipart::Form::new();
                for (key, value) in multipart_data {
                    form = form.text(key, value);
                }
                req_builder = req_builder.multipart(form);
            }
            "binary" => {
                req_builder = req_builder
                    .header("Content-Type", "application/octet-stream")
                    .body(body_content.clone());
            }
            _ => {
                log::debug!("No body type specified or unknown type");
            }
        }
    } else if let Some(body) = &request.body {
        // Fallback to old body field for backward compatibility
        log::debug!("Adding request body (legacy): {} bytes", body.len());
        req_builder = req_builder.body(body.clone());
    }

    if let Some(headers_str) = &request.headers {
        log::debug!("Parsing and adding request headers");
        let headers_map: HashMap<String, String> =
            serde_json::from_str(headers_str).map_err(|e| {
                log::error!("Failed to parse request headers: {}", e);
                ExecutorError::SubstitutionError(format!("Failed to parse request headers: {}", e))
            })?;
        log::debug!("Adding {} headers", headers_map.len());
        for (key, value) in headers_map {
            req_builder = req_builder.header(&key, &value);
        }
    }

    log::debug!("Sending HTTP request...");
    let response = req_builder.send().await.map_err(|e| {
        log::error!("Request execution failed: {}", e);
        ExecutorError::NetworkError(e.to_string())
    })?;

    // 6. Format Response
    let status = response.status().as_u16();
    log::info!("Request completed with status: {}", status);
    let mut headers = HashMap::new();
    for (name, value) in response.headers().iter() {
        headers.insert(name.to_string(), value.to_str().unwrap_or("").to_string());
    }
    log::debug!("Response has {} headers", headers.len());

    let body = response.text().await?;
    log::debug!("Response body length: {} bytes", body.len());

    log::info!(
        "Request execution successful: {} {} -> {}",
        request.method,
        request.url,
        status
    );

    Ok(Json(ExecuteResponse {
        status,
        headers,
        body,
        request_name: request.name,
        request_url: request.url,
    }))
}

pub fn routes(pool: DbPool) -> Router {
    Router::new()
        .route("/execute", post(execute_request_handler))
        .route("/execute-direct", post(execute_request_handler))
        .with_state(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::requests::CreateRequest;
    use axum_test::TestServer;
    use httpmock::MockServer;
    use serde_json::json;

    // Helper to ensure default network settings are present
    async fn ensure_default_network_settings(pool: &DbPool) {
        let _ = sqlx::query!(
            "INSERT OR IGNORE INTO network_settings (id, auto_proxy, http_proxy, https_proxy, no_proxy) VALUES (1, TRUE, NULL, NULL, NULL)"
        )
        .execute(pool)
        .await;
    }

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

    #[allow(dead_code)]
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
    async fn test_substitute_variables_success() {
        let mut variables = HashMap::new();
        variables.insert("base_url".to_string(), "http://example.com".to_string());
        variables.insert("path".to_string(), "/api/data".to_string());

        let template = "{{base_url}}{{path}}?query=1".to_string();
        let result = substitute_variables(&template, &variables).unwrap();
        assert_eq!(result, "http://example.com/api/data?query=1");
    }

    #[tokio::test]
    async fn test_substitute_variables_unresolved() {
        let mut variables = HashMap::new();
        variables.insert("base_url".to_string(), "http://example.com".to_string());

        let template = "{{base_url}}{{path}}?query=1".to_string();
        let result = substitute_variables(&template, &variables);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            ExecutorError::SubstitutionError("Unresolved variables found".to_string()).to_string()
        );
    }

    // Mock server for external requests
    async fn start_mock_server() -> MockServer {
        MockServer::start_async().await
    }

    #[tokio::test]
    async fn test_execute_request_handler_success() {
        let pool = db::create_test_pool().await;
        ensure_default_network_settings(&pool).await;

        let mock_server = start_mock_server().await;
        let _mock = mock_server.mock(|when, then| {
            when.method(httpmock::Method::GET).path("/test");
            then.status(200)
                .header("Content-Type", "application/json")
                .json_body(json!({ "message": "hello" }));
        });

        let req = CreateRequest {
            name: "Test Request".to_string(),
            method: "GET".to_string(),
            url: format!("{}/test", mock_server.base_url()),
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

        let server = TestServer::new(routes(pool)).unwrap();
        let response = server
            .post("/execute")
            .json(&json!({ "request_id": request_db.id }))
            .await;

        response.assert_status(StatusCode::OK);
        let exec_response: ExecuteResponse = response.json();
        assert_eq!(exec_response.status, 200);
        assert_eq!(exec_response.body, "{\"message\":\"hello\"}");
        assert_eq!(exec_response.request_name, "Test Request");
        assert_eq!(
            exec_response.request_url,
            format!("{}/test", mock_server.base_url())
        );
    }

    // #[tokio::test]
    // async fn test_execute_request_handler_with_variables() {
    //     let pool = db::create_test_pool().await;
    //     ensure_default_network_settings(&pool).await;

    //     let mock_server = start_mock_server().await;
    //     let _mock = mock_server.mock(|when, then| {
    //         when.method(httpmock::Method::POST).path("/api/data");
    //         then.status(201)
    //             .json_body(json!({ "status": "created" }));
    //     });

    //     let env_vars_map: HashMap<String, String> = [
    //         ("base_url".to_string(), mock_server.base_url()),
    //         ("endpoint".to_string(), "/api/data".to_string()),
    //     ]
    //     .iter()
    //     .cloned()
    //     .collect();
    //     let env_vars_json = serde_json::to_string(&env_vars_map).unwrap();

    //     let environment_db = create_test_environment(&pool, "Dev Env", &env_vars_json).await;

    //     let req_headers_map: HashMap<String, String> = [
    //         ("Content-Type".to_string(), "application/json".to_string()),
    //         ("X-Custom-Header".to_string(), "value".to_string()),
    //     ]
    //     .iter()
    //     .cloned()
    //     .collect();
    //     let req_headers_json = serde_json::to_string(&req_headers_map).unwrap();

    //     let req = CreateRequest {
    //         name: "Variable Request".to_string(),
    //         method: "POST".to_string(),
    //         url: "{{base_url}}{{endpoint}}".to_string(),
    //         body: Some("{\"key\": \"{{dynamic_value}}\"}".to_string()), // This variable is intentionally unresolved
    //         headers: Some(req_headers_json),
    //         folder_id: None,
    //     };
    //     let request_db = create_test_request(&pool, &req).await;

    //     let server = TestServer::new(routes(pool)).unwrap();
    //     let response = server
    //         .post("/execute")
    //         .json(&json!({ "request_id": request_db.id, "environment_id": environment_db.id }))
    //         .await;

    //     // This test should fail due to unresolved variable in body
    //     response.assert_status(StatusCode::BAD_REQUEST);
    //     let response_text = response.text().await.unwrap();
    //     assert!(response_text.contains("Unresolved variables found"));
    // }
}
