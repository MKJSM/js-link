use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;

// Intermediate structures for parsing and preview
#[derive(Debug, Serialize, Clone)]
pub struct ParsedFolder {
    pub name: String,
    pub requests: Vec<ParsedRequest>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ParsedRequest {
    pub name: String,
    pub method: String,
    pub url: String,
    pub body: Option<String>,
    pub body_type: String,
    pub headers: HashMap<String, String>,
    pub auth_type: String,
    pub auth_token: Option<String>,
    pub auth_username: Option<String>,
    pub auth_password: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CollectionSummary {
    pub name: String,
    pub request_count: usize,
}

// --- Import Logic ---

pub fn parse_import_file(
    content: &[u8],
    file_name: &str,
) -> Result<Vec<ParsedFolder>, anyhow::Error> {
    let content_str = String::from_utf8_lossy(content);

    // Order matters here. Specific formats first.

    if content_str.contains("\"clientName\": \"Thunder Client\"") {
        parse_thunder_client(&content_str).context("Failed to parse Thunder Client export")
    } else if content_str.contains("\"_postman_id\"")
        || content_str.contains("\"schema\": \"https://schema.getpostman.com/json/collection/v2")
    {
        parse_postman_v2(&content_str).context("Failed to parse Postman v2 export")
    } else if content_str.contains("\"requests\": [") && content_str.contains("\"folders\": [") {
        // Likely Postman v1
        parse_postman_v1(&content_str).context("Failed to parse Postman v1 export")
    } else if content_str.contains("collection.insomnia.rest")
        || content_str.contains("_type\": \"request_group\"")
        || file_name.ends_with(".yaml")
        || file_name.ends_with(".yml")
    {
        // Insomnia export (JSON Export or YAML Collection)
        // Try parsing as Export first (JSON)
        if let Ok(export) = serde_json::from_str::<InsomniaExport>(&content_str) {
            return parse_insomnia(export).context("Failed to parse Insomnia JSON export");
        }

        // Try parsing as V5 Collection (YAML or JSON)
        // Since content_str is a lossy string, we can try parsing.
        if let Ok(export) = serde_yaml::from_str::<InsomniaV5>(&content_str) {
            return parse_insomnia_v5(export).context("Failed to parse Insomnia V5/YAML export");
        }

        // Try generic YAML Export
        if let Ok(export) = serde_yaml::from_str::<InsomniaExport>(&content_str) {
            return parse_insomnia(export).context("Failed to parse Insomnia YAML export");
        }

        Err(anyhow::anyhow!("Detected Insomnia format but failed to parse as JSON export, YAML collection, or YAML export"))
    } else {
        Err(anyhow::anyhow!(
            "Unknown file format. Please use Postman (v1/v2), Insomnia, or Thunder Client exports."
        ))
    }
}

pub async fn save_import(
    pool: &SqlitePool,
    folders: Vec<ParsedFolder>,
) -> Result<String, anyhow::Error> {
    let mut total_requests = 0;

    if folders.is_empty() {
        return Ok("No collections found to import".to_string());
    }

    for folder in folders {
        // Use "import" if name is empty
        let folder_name = if folder.name.trim().is_empty() {
            "import"
        } else {
            &folder.name
        };
        let folder_id = create_folder(pool, folder_name)
            .await
            .context(format!("Failed to create folder '{}'", folder_name))?;

        for req in folder.requests {
            create_request(
                pool,
                &req.name,
                &req.method,
                &req.url,
                req.body.as_deref(),
                &req.headers,
                Some(folder_id),
                &req.body_type,
                &req.auth_type,
                req.auth_token.as_deref(),
                req.auth_username.as_deref(),
                req.auth_password.as_deref(),
            )
            .await
            .context(format!("Failed to create request '{}'", req.name))?;
            total_requests += 1;
        }
    }

    Ok(format!("Successfully imported {} requests", total_requests))
}

// --- Parsers ---

fn parse_postman_v2(content: &str) -> Result<Vec<ParsedFolder>, anyhow::Error> {
    let collection: PostmanCollectionV2 = serde_json::from_str(content)?;
    let mut all_requests = Vec::new();
    flatten_postman_v2_items(&collection.item, &mut all_requests);

    Ok(vec![ParsedFolder {
        name: collection.info.name,
        requests: all_requests,
    }])
}

fn flatten_postman_v2_items(items: &[PostmanItemV2], results: &mut Vec<ParsedRequest>) {
    for item in items {
        if let Some(req) = &item.request {
            let url = match &req.url {
                Some(PostmanUrlV2::String(s)) => s.clone(),
                Some(PostmanUrlV2::Object { raw }) => raw.clone(),
                None => String::new(),
            };

            let headers: HashMap<String, String> = req
                .header
                .as_ref()
                .map(|h| {
                    h.iter()
                        .map(|header| (header.key.clone(), header.value.clone()))
                        .collect()
                })
                .unwrap_or_default();

            let (body_type, body_content) = match &req.body {
                Some(b) => {
                    if let Some(raw) = &b.raw {
                        ("json", Some(raw.clone()))
                    } else {
                        ("none", None)
                    }
                }
                None => ("none", None),
            };

            let (auth_type, auth_token, auth_user, auth_pass) = if let Some(auth) = &req.auth {
                match auth.r#type.as_str() {
                    "bearer" => {
                        let token = auth
                            .bearer
                            .as_ref()
                            .and_then(|params| params.iter().find(|p| p.key == "token"))
                            .map(|p| match &p.value {
                                Value::String(s) => s.clone(),
                                v => v.to_string(),
                            });
                        ("bearer".to_string(), token, None, None)
                    }
                    "basic" => {
                        let user = auth
                            .basic
                            .as_ref()
                            .and_then(|params| params.iter().find(|p| p.key == "username"))
                            .map(|p| match &p.value {
                                Value::String(s) => s.clone(),
                                v => v.to_string(),
                            });
                        let pass = auth
                            .basic
                            .as_ref()
                            .and_then(|params| params.iter().find(|p| p.key == "password"))
                            .map(|p| match &p.value {
                                Value::String(s) => s.clone(),
                                v => v.to_string(),
                            });
                        ("basic".to_string(), None, user, pass)
                    }
                    _ => ("none".to_string(), None, None, None),
                }
            } else {
                ("none".to_string(), None, None, None)
            };

            results.push(ParsedRequest {
                name: item.name.clone(),
                method: req.method.clone(),
                url,
                body: body_content,
                body_type: body_type.to_string(),
                headers,
                auth_type,
                auth_token,
                auth_username: auth_user,
                auth_password: auth_pass,
            });
        } else if let Some(sub_items) = &item.item {
            flatten_postman_v2_items(sub_items, results);
        }
    }
}

fn parse_postman_v1(content: &str) -> Result<Vec<ParsedFolder>, anyhow::Error> {
    let collection: PostmanCollectionV1 = serde_json::from_str(content)?;
    let mut requests = Vec::new();

    for req in collection.requests {
        let mut headers = HashMap::new();
        // Postman v1 headers are often a string
        for line in req.headers.lines() {
            if let Some((key, value)) = line.split_once(':') {
                headers.insert(key.trim().to_string(), value.trim().to_string());
            }
        }

        requests.push(ParsedRequest {
            name: req.name,
            method: req.method,
            url: req.url,
            body: req.rawModeData,
            body_type: "json".to_string(),
            headers,
            auth_type: "none".to_string(),
            auth_token: None,
            auth_username: None,
            auth_password: None,
        });
    }

    Ok(vec![ParsedFolder {
        name: collection.name,
        requests,
    }])
}

fn parse_thunder_client(content: &str) -> Result<Vec<ParsedFolder>, anyhow::Error> {
    let collection: ThunderCollection = serde_json::from_str(content)?;
    let mut folders_map: HashMap<String, ParsedFolder> = HashMap::new();

    for folder in &collection.folders {
        folders_map.insert(
            folder._id.clone(),
            ParsedFolder {
                name: folder.name.clone(),
                requests: Vec::new(),
            },
        );
    }

    let mut root_requests = Vec::new();

    for req in &collection.requests {
        let headers: HashMap<String, String> = req
            .headers
            .iter()
            .map(|h| (h.name.clone(), h.value.clone()))
            .collect();

        let body_content = req.body.as_ref().and_then(|b| b.raw.clone());
        let body_type = req
            .body
            .as_ref()
            .map(|b| b.body_type.clone())
            .unwrap_or_else(|| "none".to_string());

        let (auth_type, auth_token, auth_user, auth_pass) = if let Some(auth) = &req.auth {
            match auth.r#type.as_str() {
                "bearer" => ("bearer".to_string(), auth.bearer.clone(), None, None),
                "basic" => (
                    "basic".to_string(),
                    None,
                    auth.username.clone(),
                    auth.password.clone(),
                ),
                _ => ("none".to_string(), None, None, None),
            }
        } else {
            ("none".to_string(), None, None, None)
        };

        let parsed_req = ParsedRequest {
            name: req.name.clone(),
            method: req.method.clone(),
            url: req.url.clone(),
            body: body_content,
            body_type,
            headers,
            auth_type,
            auth_token,
            auth_username: auth_user,
            auth_password: auth_pass,
        };

        if let Some(folder) = folders_map.get_mut(&req.container_id) {
            folder.requests.push(parsed_req);
        } else {
            root_requests.push(parsed_req);
        }
    }

    let mut result_folders: Vec<ParsedFolder> = folders_map
        .into_values()
        .filter(|f| !f.requests.is_empty())
        .collect();

    if !root_requests.is_empty() {
        result_folders.push(ParsedFolder {
            name: if collection.collectionName.is_empty() {
                "import".to_string()
            } else {
                collection.collectionName.clone()
            },
            requests: root_requests,
        });
    }

    Ok(result_folders)
}

fn parse_insomnia(export: InsomniaExport) -> Result<Vec<ParsedFolder>, anyhow::Error> {
    let mut folders_map: HashMap<String, ParsedFolder> = HashMap::new();
    let mut request_map: HashMap<String, Vec<ParsedRequest>> = HashMap::new();

    for res in &export.resources {
        if res.resource_type == "request_group" {
            folders_map.insert(
                res._id.clone(),
                ParsedFolder {
                    name: res.name.clone().unwrap_or_else(|| "import".to_string()),
                    requests: Vec::new(),
                },
            );
        }
    }

    for res in &export.resources {
        if res.resource_type == "request" {
            let method = res.method.clone().unwrap_or_else(|| "GET".to_string());
            let url = res.url.clone().unwrap_or_default();
            let name = res
                .name
                .clone()
                .unwrap_or_else(|| "Unnamed Request".to_string());

            let headers: HashMap<String, String> = res
                .headers
                .as_ref()
                .map(|h| {
                    h.iter()
                        .map(|header| (header.name.clone(), header.value.clone()))
                        .collect()
                })
                .unwrap_or_default();

            let (body_type, real_body) = if let Some(body_obj) = &res.body {
                if let Some(text) = body_obj.get("text").and_then(|t| t.as_str()) {
                    ("json", Some(text.to_string()))
                } else {
                    ("none", None)
                }
            } else {
                ("none", None)
            };

            let (auth_type, auth_token, auth_user, auth_pass) =
                if let Some(auth) = &res.authentication {
                    if let Some(type_str) = auth.get("type").and_then(|t| t.as_str()) {
                        match type_str {
                            "bearer" => {
                                let token = auth
                                    .get("token")
                                    .and_then(|t| t.as_str())
                                    .map(|s| s.to_string());
                                ("bearer".to_string(), token, None, None)
                            }
                            "basic" => {
                                let user = auth
                                    .get("username")
                                    .and_then(|t| t.as_str())
                                    .map(|s| s.to_string());
                                let pass = auth
                                    .get("password")
                                    .and_then(|t| t.as_str())
                                    .map(|s| s.to_string());
                                ("basic".to_string(), None, user, pass)
                            }
                            _ => ("none".to_string(), None, None, None),
                        }
                    } else {
                        ("none".to_string(), None, None, None)
                    }
                } else {
                    ("none".to_string(), None, None, None)
                };

            let req = ParsedRequest {
                name,
                method,
                url,
                body: real_body,
                body_type: body_type.to_string(),
                headers,
                auth_type,
                auth_token,
                auth_username: auth_user,
                auth_password: auth_pass,
            };

            let parent = res.parent_id.clone().unwrap_or_default();
            request_map.entry(parent).or_default().push(req);
        }
    }

    let mut final_folders = Vec::new();
    let mut root_requests = Vec::new();

    for (id, mut folder) in folders_map {
        if let Some(reqs) = request_map.remove(&id) {
            folder.requests = reqs;
        }
        if !folder.requests.is_empty() {
            final_folders.push(folder);
        }
    }

    for (_, reqs) in request_map {
        root_requests.extend(reqs);
    }

    if !root_requests.is_empty() {
        final_folders.push(ParsedFolder {
            name: "import".to_string(),
            requests: root_requests,
        });
    }

    Ok(final_folders)
}

fn parse_insomnia_v5(export: InsomniaV5) -> Result<Vec<ParsedFolder>, anyhow::Error> {
    let mut folders = Vec::new();

    for item in export.collection {
        collect_insomnia_v5_items(&item, String::new(), &mut folders);
    }

    Ok(folders)
}

fn collect_insomnia_v5_items(item: &InsomniaV5Item, path: String, folders: &mut Vec<ParsedFolder>) {
    let current_name = item.name.clone().unwrap_or_default();

    // If it has a URL, it's a request. If it has children, it's a folder.
    // However, folders can be nested.

    if let Some(children) = &item.children {
        // It's a folder (or a request group)
        let mut requests = Vec::new();
        let folder_name = if path.is_empty() {
            current_name.clone()
        } else {
            format!("{} / {}", path, current_name)
        };

        for child in children {
            if child.url.is_some() {
                // Request
                requests.push(parse_insomnia_v5_request(child));
            } else {
                // Sub-folder - recurse
                collect_insomnia_v5_items(child, folder_name.clone(), folders);
            }
        }

        if !requests.is_empty() {
            folders.push(ParsedFolder {
                name: folder_name,
                requests,
            });
        }
    } else if item.url.is_some() {
        // Root request without parent? Or encountered during recursion?
        // If we are here, we are at root level item which is a request but has no children.
        // It wasn't collected by a parent folder loop because we are iterating roots.
        // We can put it in a root folder.

        // However, this function is called for roots.
        // If we want to support root requests, we should probably check if it's a request or folder at top level.
    }
}

fn parse_insomnia_v5_request(item: &InsomniaV5Item) -> ParsedRequest {
    let url = item.url.clone().unwrap_or_default();
    let method = item.method.clone().unwrap_or_else(|| "GET".to_string());
    let name = item
        .name
        .clone()
        .unwrap_or_else(|| "Unnamed Request".to_string());

    let headers: HashMap<String, String> = item
        .headers
        .as_ref()
        .map(|h| {
            h.iter()
                .map(|header| (header.name.clone(), header.value.clone()))
                .collect()
        })
        .unwrap_or_default();

    let (body_type, body_content) = if let Some(body) = &item.body {
        if let Some(text) = &body.text {
            ("json".to_string(), Some(text.clone()))
        } else {
            ("none".to_string(), None)
        }
    } else {
        ("none".to_string(), None)
    };

    let (auth_type, auth_token, auth_user, auth_pass) = if let Some(auth) = &item.authentication {
        match auth.r#type.as_deref() {
            Some("bearer") => ("bearer".to_string(), auth.token.clone(), None, None),
            Some("basic") => (
                "basic".to_string(),
                None,
                auth.username.clone(),
                auth.password.clone(),
            ),
            _ => ("none".to_string(), None, None, None),
        }
    } else {
        ("none".to_string(), None, None, None)
    };

    ParsedRequest {
        name,
        method,
        url,
        body: body_content,
        body_type,
        headers,
        auth_type,
        auth_token,
        auth_username: auth_user,
        auth_password: auth_pass,
    }
}

// --- Structs for Deserialization ---

#[derive(Debug, Deserialize)]
struct PostmanCollectionV2 {
    info: PostmanInfoV2,
    item: Vec<PostmanItemV2>,
}

#[derive(Debug, Deserialize)]
struct PostmanInfoV2 {
    name: String,
}

#[derive(Debug, Deserialize)]
struct PostmanItemV2 {
    name: String,
    request: Option<PostmanRequestV2>,
    item: Option<Vec<PostmanItemV2>>,
}

#[derive(Debug, Deserialize)]
struct PostmanRequestV2 {
    method: String,
    url: Option<PostmanUrlV2>,
    header: Option<Vec<PostmanHeaderV2>>,
    body: Option<PostmanBodyV2>,
    auth: Option<PostmanAuthV2>,
}

#[derive(Debug, Deserialize)]
struct PostmanAuthV2 {
    r#type: String,
    bearer: Option<Vec<PostmanAuthParamV2>>,
    basic: Option<Vec<PostmanAuthParamV2>>,
}

#[derive(Debug, Deserialize)]
struct PostmanAuthParamV2 {
    key: String,
    value: Value,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum PostmanUrlV2 {
    String(String),
    Object { raw: String },
}

#[derive(Debug, Deserialize)]
struct PostmanHeaderV2 {
    key: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct PostmanBodyV2 {
    raw: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PostmanCollectionV1 {
    name: String,
    requests: Vec<PostmanRequestV1>,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct PostmanRequestV1 {
    name: String,
    url: String,
    method: String,
    headers: String,
    rawModeData: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct ThunderCollection {
    collectionName: String,
    folders: Vec<ThunderFolder>,
    requests: Vec<ThunderRequest>,
}

#[derive(Debug, Deserialize)]
struct ThunderFolder {
    _id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct ThunderRequest {
    #[serde(rename = "containerId")]
    container_id: String,
    name: String,
    url: String,
    method: String,
    headers: Vec<ThunderHeader>,
    body: Option<ThunderBody>,
    auth: Option<ThunderAuth>,
}

#[derive(Debug, Deserialize)]
struct ThunderAuth {
    r#type: String,
    bearer: Option<String>,
    username: Option<String>,
    password: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ThunderHeader {
    name: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct ThunderBody {
    #[serde(rename = "type")]
    body_type: String,
    raw: Option<String>,
}

#[derive(Debug, Deserialize)]
struct InsomniaExport {
    resources: Vec<InsomniaResource>,
}

#[derive(Debug, Deserialize)]
struct InsomniaResource {
    _id: String,
    #[serde(rename = "_type")]
    resource_type: String,
    #[serde(rename = "parentId")]
    parent_id: Option<String>,
    name: Option<String>,
    method: Option<String>,
    url: Option<String>,
    body: Option<Value>,
    headers: Option<Vec<InsomniaHeader>>,
    authentication: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct InsomniaHeader {
    name: String,
    value: String,
}

// Insomnia V5
#[derive(Debug, Deserialize)]
struct InsomniaV5 {
    collection: Vec<InsomniaV5Item>,
}

#[derive(Debug, Deserialize)]
struct InsomniaV5Item {
    name: Option<String>,
    url: Option<String>,
    method: Option<String>,
    children: Option<Vec<InsomniaV5Item>>,
    headers: Option<Vec<InsomniaHeader>>,
    body: Option<InsomniaV5Body>,
    authentication: Option<InsomniaV5Auth>,
}

#[derive(Debug, Deserialize)]
struct InsomniaV5Body {
    #[allow(dead_code)]
    #[serde(rename = "mimeType")]
    mime_type: Option<String>,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct InsomniaV5Auth {
    r#type: Option<String>,
    token: Option<String>,
    username: Option<String>,
    password: Option<String>,
}

// --- DB Helpers ---

async fn create_folder(pool: &SqlitePool, name: &str) -> Result<i64, anyhow::Error> {
    let row = sqlx::query("INSERT INTO folders (name) VALUES (?) RETURNING id")
        .bind(name)
        .fetch_one(pool)
        .await?;
    Ok(row.get(0))
}

async fn create_request(
    pool: &SqlitePool,
    name: &str,
    method: &str,
    url: &str,
    body: Option<&str>,
    headers: &HashMap<String, String>,
    folder_id: Option<i64>,
    body_type: &str,
    auth_type: &str,
    auth_token: Option<&str>,
    auth_username: Option<&str>,
    auth_password: Option<&str>,
) -> Result<i64, anyhow::Error> {
    let headers_json = serde_json::to_string(headers)?;
    let row = sqlx::query(
        "INSERT INTO requests (name, method, url, body, headers, folder_id, body_type, request_type, auth_type, auth_token, auth_username, auth_password) VALUES (?, ?, ?, ?, ?, ?, ?, 'api', ?, ?, ?, ?) RETURNING id"
    )
        .bind(name)
        .bind(method)
        .bind(url)
        .bind(body)
        .bind(headers_json)
        .bind(folder_id)
        .bind(body_type)
        .bind(auth_type)
        .bind(auth_token)
        .bind(auth_username)
        .bind(auth_password)
        .fetch_one(pool)
        .await?;
    Ok(row.get(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn test_parse_insomnia_file() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push(".import/Insomnia.yaml");
        println!("Reading file: {:?}", path);

        let content = fs::read(&path).expect("Failed to read Insomnia.yaml");
        match parse_import_file(&content, "Insomnia.yaml") {
            Ok(folders) => {
                println!(
                    "Successfully parsed Insomnia file. Found {} folders.",
                    folders.len()
                );
                for folder in folders {
                    println!(
                        "Folder: {}, Requests: {}",
                        folder.name,
                        folder.requests.len()
                    );
                }
            }
            Err(e) => panic!("Failed to parse Insomnia file: {:?}", e),
        }
    }

    #[test]
    fn test_parse_thunder_file() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push(".import/thunder-collection.json");
        println!("Reading file: {:?}", path);

        let content = fs::read(&path).expect("Failed to read thunder-collection.json");
        match parse_import_file(&content, "thunder-collection.json") {
            Ok(folders) => {
                println!(
                    "Successfully parsed Thunder Client file. Found {} folders.",
                    folders.len()
                );
                for folder in folders {
                    println!(
                        "Folder: {}, Requests: {}",
                        folder.name,
                        folder.requests.len()
                    );
                }
            }
            Err(e) => panic!("Failed to parse Thunder Client file: {:?}", e),
        }
    }

    #[test]
    fn test_parse_postman_file() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push(".import/postman_collection.json");
        println!("Reading file: {:?}", path);

        let content = fs::read(&path).expect("Failed to read postman_collection.json");
        match parse_import_file(&content, "postman_collection.json") {
            Ok(folders) => {
                println!(
                    "Successfully parsed Postman file. Found {} folders.",
                    folders.len()
                );
                for folder in &folders {
                    println!(
                        "Folder: {}, Requests: {}",
                        folder.name,
                        folder.requests.len()
                    );
                    for req in &folder.requests {
                        println!("  - {} {} {}", req.method, req.name, req.url);
                    }
                }
            }
            Err(e) => panic!("Failed to parse Postman file: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_save_insomnia_import() {
        use crate::db::create_test_pool;

        let pool = create_test_pool().await;
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push(".import/Insomnia.yaml");

        let content = fs::read(&path).expect("Failed to read Insomnia.yaml");
        let folders = parse_import_file(&content, "Insomnia.yaml").expect("Failed to parse");

        let result = save_import(&pool, folders).await;
        match result {
            Ok(msg) => println!("Save successful: {}", msg),
            Err(e) => panic!("Failed to save import: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_save_thunder_import() {
        use crate::db::create_test_pool;

        let pool = create_test_pool().await;
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push(".import/thunder-collection.json");

        let content = fs::read(&path).expect("Failed to read thunder-collection.json");
        let folders =
            parse_import_file(&content, "thunder-collection.json").expect("Failed to parse");

        let result = save_import(&pool, folders).await;
        match result {
            Ok(msg) => println!("Save successful: {}", msg),
            Err(e) => panic!("Failed to save import: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_save_postman_import() {
        use crate::db::create_test_pool;

        let pool = create_test_pool().await;
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push(".import/postman_collection.json");

        let content = fs::read(&path).expect("Failed to read postman_collection.json");
        let folders =
            parse_import_file(&content, "postman_collection.json").expect("Failed to parse");

        let result = save_import(&pool, folders).await;
        match result {
            Ok(msg) => println!("Save successful: {}", msg),
            Err(e) => panic!("Failed to save import: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_import_data_integrity() {
        use crate::db::create_test_pool;

        let pool = create_test_pool().await;

        // Test Insomnia import with auth fields
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push(".import/Insomnia.yaml");
        let content = fs::read(&path).expect("Failed to read Insomnia.yaml");
        let folders = parse_import_file(&content, "Insomnia.yaml").expect("Failed to parse");

        // Verify auth fields are captured
        let mut has_bearer_auth = false;

        for folder in &folders {
            for req in &folder.requests {
                if req.auth_type == "bearer" && req.auth_token.is_some() {
                    has_bearer_auth = true;
                    println!("Found bearer auth in request: {}", req.name);
                }
            }
        }

        assert!(
            has_bearer_auth,
            "Should have at least one request with bearer auth"
        );

        // Save and verify
        save_import(&pool, folders).await.expect("Failed to save");

        // Verify saved data
        let row = sqlx::query("SELECT COUNT(*) as count FROM requests WHERE auth_type != 'none'")
            .fetch_one(&pool)
            .await
            .expect("Failed to query");
        let count: i64 = row.get(0);
        println!("Requests with authentication: {}", count);
        assert!(count > 0, "Should have saved requests with authentication");
    }
}
