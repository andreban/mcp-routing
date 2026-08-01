use axum::{
    Json, Router,
    body::Body,
    http::{HeaderMap, Request, StatusCode, Uri},
    middleware::{self, Next},
    response::Response,
    routing::post,
};
use serde_json::{Value, json};
use tracing::{debug, error, info};

use crate::mcp::{JsonRpcRequest, JsonRpcResultResponse, default_jsonrpc};

async fn rewrite_mcp_path(
    headers: HeaderMap,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Because of .nest("/mcp", ...), req.uri().path() is relative to /mcp (e.g., "/" or "")
    let path = req.uri().path();

    if path == "/" || path.is_empty() {
        let method = headers.get("Mcp-Method").and_then(|v| v.to_str().ok());
        let name = headers.get("Mcp-Name").and_then(|v| v.to_str().ok());

        let Some(method) = method else {
            debug!("Invalid MCP request. Missing 'Mcp-Method' header");
            return Err(StatusCode::BAD_REQUEST);
        };

        let method = method.trim_matches('/');
        let new_path = match name {
            Some(name) => {
                let name = name.trim_matches('/');
                format!("/{method}/{name}")
            }
            None => format!("/{method}"),
        };

        let new_uri_str = match req.uri().query() {
            Some(query) => format!("{new_path}?{query}"),
            None => new_path,
        };

        let Ok(new_uri) = new_uri_str.parse::<Uri>() else {
            error!(new_uri_str, "Generated invalid Uri");
            return Err(StatusCode::BAD_REQUEST);
        };
        info!(?new_uri, "Forwarding MCP request");
        *req.uri_mut() = new_uri;
    }

    Ok(next.run(req).await)
}

async fn noop() {}

pub fn router() -> Router {
    Router::new()
        .route("/tools/call/echo", post(echo))
        .route("/tools/list", post(list_tools))
        .route("/", post(noop))
        .layer(middleware::from_fn(rewrite_mcp_path))
}

pub async fn list_tools(Json(request): Json<JsonRpcRequest>) -> Json<JsonRpcResultResponse<Value>> {
    Json(JsonRpcResultResponse {
        id: request.id,
        jsonrpc: default_jsonrpc(),
        result: json!({
            "resultType": "complete",
            "tools": [],
            "ttlMs": 0,
            "cacheScope": "public",
        }),
    })
}

pub async fn echo(Json(_params): Json<JsonRpcRequest<Value>>) -> Json<Value> {
    Json(json!({"result": "ok"}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_header_routing_without_name() {
        let app = router();
        let request = Request::builder()
            .method("POST")
            .uri("/")
            .header("Mcp-Method", "tools/list")
            .header("Content-Type", "application/json")
            .body(Body::from(
                json!({"id": 1, "method": "tools/list"}).to_string(),
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_header_routing_with_name() {
        let app = router();
        let request = Request::builder()
            .method("POST")
            .uri("/")
            .header("Mcp-Method", "tools/call")
            .header("Mcp-Name", "echo")
            .header("Content-Type", "application/json")
            .body(Body::from(
                json!({"id": 1, "method": "tools/call/echo"}).to_string(),
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
