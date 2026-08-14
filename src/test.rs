use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    routing::post,
};
use serde_json::json;
use tower::ServiceExt;

use crate::{
    McpRouter,
    types::mcp::{
        Implementation,
        server::discover::ServerDiscoverResultResponse,
        tools::{
            Tool,
            list::ListToolsResultResponse,
        },
    },
};

fn test_server_info() -> Implementation {
    Implementation::new("test-server", "1.0.0")
}

async fn mock_handler() -> &'static str {
    "ok"
}

#[tokio::test]
async fn test_mcp_router_builtin_tools_list() {
    let tool = Tool {
        icons: Vec::new(),
        name: "test_tool".to_string(),
        title: Some("Test Tool".to_string()),
        description: Some("A test tool".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "input": { "type": "string" }
            }
        }),
        output_schema: None,
        annotations: None,
        meta: None,
    };

    let app = McpRouter::new(test_server_info()).register_tool(tool, mock_handler);
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

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let res: ListToolsResultResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(res.result.tools.len(), 1);
    assert_eq!(res.result.tools[0].name, "test_tool");
    assert_eq!(res.result.tools[0].title.as_deref(), Some("Test Tool"));
}

#[tokio::test]
async fn test_mcp_router_builtin_server_discover() {
    let app = McpRouter::new(test_server_info())
        .instructions("Test instructions");

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

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let res: ServerDiscoverResultResponse = serde_json::from_slice(&bytes).unwrap();
    let server_info = res.result.meta.unwrap().server_info.unwrap();
    assert_eq!(server_info.name, "test-server");
    assert_eq!(server_info.version, "1.0.0");
    assert_eq!(res.result.instructions.as_deref(), Some("Test instructions"));
}

#[tokio::test]
async fn test_mcp_router_header_routing_with_name() {
    let app = McpRouter::new(test_server_info()).register_tool("echo", mock_handler);
    let request = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "echo")
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "id": 1,
                "method": "tools/call/echo",
                "params": {
                    "name": "echo",
                    "arguments": { "value": "test" }
                }
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_mcp_router_missing_header_returns_bad_request() {
    let app = McpRouter::new(test_server_info());
    let request = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({"id": 1, "method": "tools/list"}).to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_mcp_router_nested_in_axum() {
    let mcp_router = McpRouter::new(test_server_info()).register_tool("hello", mock_handler);
    let app = Router::new().nest_service("/mcp", mcp_router);

    let request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "hello")
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "id": 1,
                "method": "tools/call/hello",
                "params": {
                    "name": "hello",
                    "arguments": { "value": "nested" }
                }
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_mcp_router_custom_route_override() {
    async fn custom_discover() -> &'static str {
        "custom_discover"
    }

    let app = McpRouter::new(test_server_info())
        .route("/server/discover", post(custom_discover));

    let request = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "server/discover")
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(bytes.as_ref(), b"custom_discover");
}
