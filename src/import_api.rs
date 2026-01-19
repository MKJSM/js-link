use crate::db::DbPool;
use crate::importers::{parse_import_file, save_import, CollectionSummary};
use axum::{
    extract::{Multipart, Query, State},
    response::{IntoResponse, Json},
    routing::post,
    Router,
};
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize)]
struct ImportParams {
    preview: Option<bool>,
}

pub fn routes(pool: DbPool) -> Router {
    Router::new()
        .route("/import", post(handle_import))
        .with_state(pool)
}

async fn handle_import(
    State(pool): State<DbPool>,
    Query(params): Query<ImportParams>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut message = String::new();
    let is_preview = params.preview.unwrap_or(false);

    // For preview, we collect summaries. For execute, we collect status messages.
    let mut preview_collections = Vec::new();

    while let Some(field) = multipart.next_field().await.unwrap() {
        let file_name = field.file_name().unwrap_or("unknown").to_string();
        let data = field.bytes().await.unwrap();

        match parse_import_file(&data, &file_name) {
            Ok(folders) => {
                if is_preview {
                    for folder in folders {
                        preview_collections.push(CollectionSummary {
                            name: folder.name,
                            request_count: folder.requests.len(),
                        });
                    }
                } else {
                    match save_import(&pool, folders).await {
                        Ok(msg) => message.push_str(&format!("Success: {}\n", msg)),
                        Err(e) => message.push_str(&format!("Error saving {}: {}\n", file_name, e)),
                    }
                }
            }
            Err(e) => {
                if !is_preview {
                    message.push_str(&format!("Error parsing {}: {}\n", file_name, e));
                } else {
                    // In preview mode, maybe just return error for that file?
                    // For now we just ignore or could return error structure.
                    // Let's just return what we have.
                }
            }
        }
    }

    if is_preview {
        Json(json!({
            "preview": true,
            "collections": preview_collections
        }))
    } else {
        Json(json!({
            "preview": false,
            "message": message
        }))
    }
}
