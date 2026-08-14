use std::collections::HashMap;

use axum::{
    Json,
    body::Body,
    http::{Request, Response, StatusCode},
    response::IntoResponse,
};
use tracing::error;

use crate::types::mcp::{
    CacheScope, Implementation, ResultMetaObject, ServerCapabilities,
    server::discover::{
        ServerDiscoverRequest, ServerDiscoverResult, ServerDiscoverResultResponse,
    },
};

pub async fn handle_server_discover(
    req: Request<Body>,
    server_info: Implementation,
    instructions: Option<String>,
    capabilities: ServerCapabilities,
    supported_versions: Vec<String>,
) -> Response<Body> {
    let body_bytes = match axum::body::to_bytes(req.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(err) => {
            error!(?err, "Failed to read request body");
            return StatusCode::BAD_REQUEST.into_response();
        }
    };

    let request: ServerDiscoverRequest = match serde_json::from_slice(&body_bytes) {
        Ok(req) => req,
        Err(err) => {
            error!(?err, "Failed to parse ServerDiscoverRequest");
            return StatusCode::BAD_REQUEST.into_response();
        }
    };

    let response = ServerDiscoverResultResponse::new(
        request.id,
        ServerDiscoverResult {
            meta: Some(ResultMetaObject {
                server_info: Some(server_info),
                extra: HashMap::new(),
            }),
            result_type: Some("complete".to_string()),
            supported_versions,
            capabilities,
            instructions,
            ttl_ms: Some(0),
            cache_scope: Some(CacheScope::Public),
            extras: HashMap::new(),
        },
    );

    Json(response).into_response()
}
