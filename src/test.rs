// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
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

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
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

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
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
async fn test_mcp_router_typed_tool_handler_success() {
    use serde::{Deserialize, Serialize};
    use crate::types::mcp::{ContentBlock, tools::call::CallToolResultResponse};

    #[derive(Serialize, Deserialize)]
    struct AddParams {
        a: i32,
        b: i32,
    }

    async fn add_tool(params: AddParams) -> Result<String, String> {
        Ok(format!("sum={}", params.a + params.b))
    }

    let app = McpRouter::new(test_server_info()).register_tool("add", add_tool);
    let request = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "add")
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "id": 42,
                "method": "tools/call/add",
                "params": {
                    "name": "add",
                    "arguments": { "a": 10, "b": 20 }
                }
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let res: CallToolResultResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(res.id, 42.into());
    assert_eq!(res.result.is_error, Some(false));
    if let ContentBlock::Text(ref t) = res.result.content[0] {
        assert_eq!(t.text, "sum=30");
    } else {
        panic!("Expected text block");
    }
}

#[tokio::test]
async fn test_mcp_router_typed_tool_handler_error_result() {
    use serde::{Deserialize, Serialize};
    use crate::types::mcp::{ContentBlock, tools::call::CallToolResultResponse};

    #[derive(Serialize, Deserialize)]
    struct DivideParams {
        a: i32,
        b: i32,
    }

    async fn divide_tool(params: DivideParams) -> Result<String, String> {
        if params.b == 0 {
            return Err("Cannot divide by zero".to_string());
        }
        Ok((params.a / params.b).to_string())
    }

    let app = McpRouter::new(test_server_info()).register_tool("divide", divide_tool);
    let request = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "divide")
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "id": 43,
                "method": "tools/call/divide",
                "params": {
                    "name": "divide",
                    "arguments": { "a": 10, "b": 0 }
                }
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let res: CallToolResultResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(res.id, 43.into());
    assert_eq!(res.result.is_error, Some(true));
    if let ContentBlock::Text(ref t) = res.result.content[0] {
        assert_eq!(t.text, "Cannot divide by zero");
    } else {
        panic!("Expected text block");
    }
}

