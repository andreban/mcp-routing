use std::collections::HashMap;

use axum::{
    Json,
    body::Body,
    http::{Request, Response, StatusCode},
    response::IntoResponse,
};
use tracing::error;

use crate::types::mcp::{
    CacheScope,
    tools::{
        Tool,
        list::{ListToolsRequest, ListToolsResult, ListToolsResultResponse},
    },
};

pub async fn handle_list_tools(
    req: Request<Body>,
    tools: Vec<Tool>,
) -> Response<Body> {
    let body_bytes = match axum::body::to_bytes(req.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(err) => {
            error!(?err, "Failed to read request body");
            return StatusCode::BAD_REQUEST.into_response();
        }
    };

    let request: ListToolsRequest = match serde_json::from_slice(&body_bytes) {
        Ok(req) => req,
        Err(err) => {
            error!(?err, "Failed to parse ListToolsRequest");
            return StatusCode::BAD_REQUEST.into_response();
        }
    };

    let response = ListToolsResultResponse::new(
        request.id,
        ListToolsResult {
            meta: None,
            result_type: Some("complete".to_string()),
            next_cursor: None,
            ttl_ms: Some(0),
            cache_scope: Some(CacheScope::Public),
            tools,
            extras: HashMap::new(),
        },
    );

    Json(response).into_response()
}
