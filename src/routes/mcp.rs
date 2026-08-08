use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::LazyLock;

use axum::{
    Json, Router,
    body::Body,
    debug_handler,
    http::{HeaderMap, Request, Response, StatusCode, Uri},
    response::IntoResponse,
    routing::post,
};
use serde_json::json;
use tower_service::Service;
use tracing::{debug, error, info};

use crate::types::mcp::{
    CacheScope, ContentBlock, Implementation, ResultMetaObject, ServerCapabilities, TextContent,
    ToolsCapability,
    discover::{ServerDiscoverRequest, ServerDiscoverResult, ServerDiscoverResultResponse},
    tools::{
        Tool,
        call::{CallToolRequest, CallToolResult, CallToolResultResponse},
        list::{ListToolsRequest, ListToolsResult, ListToolsResultResponse},
    },
};

static DISPATCHER: LazyLock<Router> = LazyLock::new(|| {
    Router::new()
        .route("/tools/call/echo", post(echo))
        .route("/tools/list", post(list_tools))
        .route("/server/discover", post(server_discover))
});

pub fn router() -> Router {
    Router::new().route("/", post(mcp_dispatcher))
}

// NOTE: We use `mcp_dispatcher` with an internal static `DISPATCHER` router service
// instead of Axum middleware (`axum::middleware::from_fn`).
// In Axum, route matching happens BEFORE route middleware runs. If middleware mutates
// `*req.uri_mut()`, Axum will NOT re-evaluate top-level route matching.
// Therefore, we manually rewrite the request URI based on the `Mcp-Method` / `Mcp-Name`
// headers and dispatch directly to `DISPATCHER.clone().call(req).await`.
#[debug_handler]
pub async fn mcp_dispatcher(
    headers: HeaderMap,
    mut req: Request<Body>,
) -> Result<Response<Body>, Infallible> {
    debug!(?req, "MCP Dispatcher");
    let path = req.uri().path();
    if path == "/" || path.is_empty() {
        debug!("Handling /mcp request");
        let method = headers.get("Mcp-Method").and_then(|v| v.to_str().ok());
        let name = headers.get("Mcp-Name").and_then(|v| v.to_str().ok());

        let Some(method) = method else {
            debug!("Invalid MCP request. Missing 'Mcp-Method' header");
            return Ok(StatusCode::BAD_REQUEST.into_response());
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
            return Ok(StatusCode::BAD_REQUEST.into_response());
        };
        info!(?new_uri, "Forwarding MCP request");
        *req.uri_mut() = new_uri;
    }
    DISPATCHER.clone().call(req).await
}

pub async fn list_tools(Json(request): Json<ListToolsRequest>) -> Json<ListToolsResultResponse> {
    Json(ListToolsResultResponse::new(
        request.id,
        ListToolsResult {
            meta: None,
            result_type: Some("complete".to_string()),
            next_cursor: None,
            ttl_ms: Some(0),
            cache_scope: Some(CacheScope::Public),
            tools: vec![Tool {
                icons: Vec::new(),
                name: "echo".to_string(),
                title: Some("Echo".to_string()),
                description: Some("Echoes the value back to the client".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "value": {
                            "type": "string",
                            "description": "The value to be echoed",
                        }
                    },
                    "required": ["value"],
                }),
                output_schema: None,
                annotations: None,
                meta: None,
            }],
            extras: HashMap::new(),
        },
    ))
}

#[debug_handler]
pub async fn server_discover(
    Json(request): Json<ServerDiscoverRequest>,
) -> Json<ServerDiscoverResultResponse> {
    debug!(?request, "Received server/discover request");
    Json(ServerDiscoverResultResponse::new(
        request.id,
        ServerDiscoverResult {
            meta: Some(ResultMetaObject {
                server_info: Some(Implementation {
                    icons: vec![],
                    name: "mcp-routing server".to_string(),
                    title: None,
                    version: "0.1.0".to_string(),
                    description: None,
                    website_url: None,
                }),
                extra: HashMap::new(),
            }),
            result_type: Some("complete".to_string()),
            supported_versions: vec!["2026-07-28".to_string()],
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability { list_changed: None }),
                resources: None,
                prompts: None,
                completions: None,
                experimental: None,
            },
            instructions: Some("Example server".to_string()),
            ttl_ms: Some(0),
            cache_scope: Some(CacheScope::Public),
            extras: HashMap::new(),
        },
    ))
}

pub async fn echo(Json(request): Json<CallToolRequest>) -> Json<CallToolResultResponse> {
    let value = request
        .params
        .as_ref()
        .and_then(|p| p.arguments.as_ref())
        .and_then(|args| args.get("value"))
        .and_then(|v| v.as_str());

    let (content, is_error) = match value {
        Some(text) => (
            vec![ContentBlock::Text(TextContent {
                text: text.to_string(),
                annotations: None,
                meta: None,
            })],
            false,
        ),
        None => (
            vec![ContentBlock::Text(TextContent {
                text: "Missing required string parameter: value".to_string(),
                annotations: None,
                meta: None,
            })],
            true,
        ),
    };

    Json(CallToolResultResponse::new(
        request.id,
        CallToolResult {
            meta: None,
            result_type: Some("complete".to_string()),
            content,
            is_error: Some(is_error),
            structured_content: None,
            extras: HashMap::new(),
        },
    ))
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

    #[tokio::test]
    async fn test_header_routing_server_discover() {
        let app = router();
        let request = Request::builder()
            .method("POST")
            .uri("/")
            .header("Mcp-Method", "server/discover")
            .header("Content-Type", "application/json")
            .body(Body::from(
                json!({"id": 1, "method": "server/discover"}).to_string(),
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
