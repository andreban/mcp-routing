// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Internal McpRouter Unit Tests
//!
//! Unit test suite covering core router routing logic, handler dispatch, fallback resolution,
//! and error condition handling.

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
    types::{
        jsonrpc::{JsonRpcErrorCode, JsonRpcErrorResponse},
        mcp::{
            Implementation,
            server::discover::ServerDiscoverResultResponse,
            tools::{
                Tool,
                list::ListToolsResultResponse,
            },
        },
    },
};

fn test_server_info() -> Implementation {
    Implementation::new("test-server", "1.0.0")
}

async fn mock_handler() -> &'static str {
    "ok"
}

/// Tests that `tools/list` returns the registered tool with its title and schema.
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

/// Tests that `server/discover` returns the configured server metadata and instructions.
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

/// Tests tool call routing using `Mcp-Method: tools/call` and `Mcp-Name: echo` headers.
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
                "method": "tools/call",
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

/// Tests that `tools/list` falls back to the body method when `Mcp-Method` header is omitted.
#[tokio::test]
async fn test_mcp_router_body_method_fallback_tools_list() {
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
    // Request without Mcp-Method header
    let request = Request::builder()
        .method("POST")
        .uri("/")
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
}

/// Tests that `server/discover` falls back to the body method when `Mcp-Method` header is omitted.
#[tokio::test]
async fn test_mcp_router_body_method_fallback_server_discover() {
    let app = McpRouter::new(test_server_info()).instructions("Test instructions");
    // Request without Mcp-Method header
    let request = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({"id": 1, "method": "server/discover"}).to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let res: ServerDiscoverResultResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(res.result.instructions.as_deref(), Some("Test instructions"));
}

/// Tests tool call routing when both `Mcp-Method` and `Mcp-Name` headers are omitted.
#[tokio::test]
async fn test_mcp_router_body_method_and_tool_name_fallback() {
    let app = McpRouter::new(test_server_info()).register_tool("echo", mock_handler);
    // Request without Mcp-Method or Mcp-Name headers
    let request = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "id": 1,
                "method": "tools/call",
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

/// Tests tool call routing with `Mcp-Method: tools/call` header but falling back to body for tool name.
#[tokio::test]
async fn test_mcp_router_tool_name_fallback_with_header_method() {
    let app = McpRouter::new(test_server_info()).register_tool("echo", mock_handler);
    // Request with Mcp-Method header but WITHOUT Mcp-Name header
    let request = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "tools/call")
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "id": 1,
                "method": "tools/call",
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

/// Tests that non-standard method suffix strings return a JSON-RPC Method Not Found error (-32601).
#[tokio::test]
async fn test_mcp_router_invalid_method_suffix_returns_not_found() {
    let app = McpRouter::new(test_server_info()).register_tool("echo", mock_handler);
    let request = Request::builder()
        .method("POST")
        .uri("/")
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

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let res: JsonRpcErrorResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(res.jsonrpc, "2.0");
    assert_eq!(res.id, Some(1.into()));
    assert_eq!(res.error.code, JsonRpcErrorCode::MethodNotFound);
}

/// Tests that missing method in both header and body returns a JSON-RPC Invalid Request error (-32600).
#[tokio::test]
async fn test_mcp_router_missing_method_in_header_and_body_returns_bad_request() {
    let app = McpRouter::new(test_server_info());
    let request = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .body(Body::from(json!({"id": 1}).to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let res: JsonRpcErrorResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(res.jsonrpc, "2.0");
    assert_eq!(res.id, Some(1.into()));
    assert_eq!(res.error.code, JsonRpcErrorCode::InvalidRequest);
}

/// Tests that missing tool name in `tools/call` returns a JSON-RPC Invalid Params error (-32602).
#[tokio::test]
async fn test_mcp_router_missing_tool_name_returns_bad_request() {
    let app = McpRouter::new(test_server_info());
    let request = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "id": 1,
                "method": "tools/call",
                "params": {}
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let res: JsonRpcErrorResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(res.jsonrpc, "2.0");
    assert_eq!(res.id, Some(1.into()));
    assert_eq!(res.error.code, JsonRpcErrorCode::InvalidParams);
}

/// Tests that calling an unregistered tool returns a JSON-RPC Method Not Found error (-32601).
#[tokio::test]
async fn test_mcp_router_unknown_tool_returns_not_found() {
    let app = McpRouter::new(test_server_info());
    let request = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "id": 1,
                "method": "tools/call",
                "params": {
                    "name": "non_existent_tool"
                }
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let res: JsonRpcErrorResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(res.jsonrpc, "2.0");
    assert_eq!(res.id, Some(1.into()));
    assert_eq!(res.error.code, JsonRpcErrorCode::MethodNotFound);
}

/// Tests that an unknown method returns a JSON-RPC Method Not Found error (-32601).
#[tokio::test]
async fn test_mcp_router_unknown_method_returns_not_found() {
    let app = McpRouter::new(test_server_info());
    let request = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "id": 1,
                "method": "unknown/method"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let res: JsonRpcErrorResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(res.jsonrpc, "2.0");
    assert_eq!(res.id, Some(1.into()));
    assert_eq!(res.error.code, JsonRpcErrorCode::MethodNotFound);
}

/// Tests mounting an `McpRouter` in Axum with `nest_service`.
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
                "method": "tools/call",
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

/// Tests typed tool handler argument deserialization and success return wrapping.
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
                "method": "tools/call",
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

/// Tests typed tool handler error result wrapping with `is_error: true`.
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
                "method": "tools/call",
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

/// Tests that non-POST HTTP methods (GET, PUT, DELETE, PATCH, HEAD, OPTIONS) return 405 Method Not Allowed with `Allow: POST`.
#[tokio::test]
async fn test_mcp_router_rejects_non_post_methods() {
    let app = McpRouter::new(test_server_info());

    let methods = [
        http::Method::GET,
        http::Method::PUT,
        http::Method::DELETE,
        http::Method::PATCH,
        http::Method::HEAD,
        http::Method::OPTIONS,
    ];

    for method in methods {
        let request = Request::builder()
            .method(method)
            .uri("/")
            .header("Content-Type", "application/json")
            .body(Body::from(
                json!({"id": 1, "method": "server/discover"}).to_string(),
            ))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::METHOD_NOT_ALLOWED,
            "Expected 405 Method Not Allowed"
        );
        assert_eq!(
            response.headers().get("allow").and_then(|h| h.to_str().ok()),
            Some("POST")
        );
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        assert!(bytes.is_empty());
    }
}

/// Tests that requests with missing or unsupported Content-Type return 415 Unsupported Media Type.
#[tokio::test]
async fn test_mcp_router_rejects_unsupported_content_types() {
    let app = McpRouter::new(test_server_info());

    // 1. Missing Content-Type
    let req_missing = Request::builder()
        .method("POST")
        .uri("/")
        .body(Body::from(
            json!({"id": 1, "method": "server/discover"}).to_string(),
        ))
        .unwrap();

    let resp_missing = app.clone().oneshot(req_missing).await.unwrap();
    assert_eq!(
        resp_missing.status(),
        StatusCode::UNSUPPORTED_MEDIA_TYPE,
        "Expected 415 for missing Content-Type"
    );
    let bytes = resp_missing.into_body().collect().await.unwrap().to_bytes();
    assert!(bytes.is_empty());

    // 2. text/plain
    let req_text = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "text/plain")
        .body(Body::from(
            json!({"id": 2, "method": "server/discover"}).to_string(),
        ))
        .unwrap();

    let resp_text = app.clone().oneshot(req_text).await.unwrap();
    assert_eq!(
        resp_text.status(),
        StatusCode::UNSUPPORTED_MEDIA_TYPE,
        "Expected 415 for text/plain"
    );

    // 3. application/xml
    let req_xml = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/xml")
        .body(Body::from(
            json!({"id": 3, "method": "server/discover"}).to_string(),
        ))
        .unwrap();

    let resp_xml = app.clone().oneshot(req_xml).await.unwrap();
    assert_eq!(
        resp_xml.status(),
        StatusCode::UNSUPPORTED_MEDIA_TYPE,
        "Expected 415 for application/xml"
    );
}

/// Tests that requests with Content-Type containing charset parameters (e.g. application/json; charset=utf-8) are accepted.
#[tokio::test]
async fn test_mcp_router_accepts_valid_content_types_with_charset() {
    let app = McpRouter::new(test_server_info()).instructions("Valid Content Type");

    let request = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "server/discover")
        .header("Content-Type", "application/json; charset=utf-8")
        .body(Body::from(
            json!({"id": "charset-test", "method": "server/discover"}).to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let res: ServerDiscoverResultResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        res.result.instructions.as_deref(),
        Some("Valid Content Type")
    );
}

