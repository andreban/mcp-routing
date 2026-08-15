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
            CacheScope, Implementation,
            prompts::{
                Prompt, PromptArgument, get::GetPromptResultResponse,
                list::ListPromptsResultResponse,
            },
            server::discover::ServerDiscoverResultResponse,
            tools::{Tool, list::ListToolsResultResponse},
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
        .header("MCP-Protocol-Version", "2026-07-28")
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
    let app = McpRouter::new(test_server_info()).instructions("Test instructions");

    let request = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "server/discover")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
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
    assert_eq!(
        res.result.instructions.as_deref(),
        Some("Test instructions")
    );
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
        .header("MCP-Protocol-Version", "2026-07-28")
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
    // Request without Mcp-Method header -> rejected with HeaderMismatch (-32020)
    let request = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(
            json!({"id": 1, "method": "tools/list"}).to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let res: JsonRpcErrorResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(res.error.code.code(), crate::types::mcp::HEADER_MISMATCH);
}

/// Tests that a mismatch between `Mcp-Method` header and body `method` returns a HeaderMismatch error (-32020).
#[tokio::test]
async fn test_mcp_router_mcp_method_mismatch_returns_header_mismatch() {
    let app = McpRouter::new(test_server_info()).instructions("Test instructions");
    let request = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "server/discover")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(
            json!({"id": 1, "method": "tools/list"}).to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let res: JsonRpcErrorResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(res.error.code.code(), crate::types::mcp::HEADER_MISMATCH);
}

/// Tests that missing `Mcp-Name` header on `tools/call` returns a HeaderMismatch error (-32020).
#[tokio::test]
async fn test_mcp_router_missing_mcp_name_header_returns_header_mismatch() {
    let app = McpRouter::new(test_server_info()).register_tool("echo", mock_handler);
    let request = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "tools/call")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
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
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let res: JsonRpcErrorResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(res.error.code.code(), crate::types::mcp::HEADER_MISMATCH);
}

/// Tests that `Mcp-Name` header mismatch with body `params.name` returns a HeaderMismatch error (-32020).
#[tokio::test]
async fn test_mcp_router_mcp_name_mismatch_returns_header_mismatch() {
    let app = McpRouter::new(test_server_info()).register_tool("echo", mock_handler);
    let request = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "echo")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(
            json!({
                "id": 1,
                "method": "tools/call",
                "params": {
                    "name": "other_tool",
                    "arguments": { "value": "test" }
                }
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let res: JsonRpcErrorResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(res.error.code.code(), crate::types::mcp::HEADER_MISMATCH);
}

/// Tests that non-standard method suffix strings return a JSON-RPC Method Not Found error (-32601).
#[tokio::test]
async fn test_mcp_router_invalid_method_suffix_returns_not_found() {
    let app = McpRouter::new(test_server_info()).register_tool("echo", mock_handler);
    let request = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "tools/call/echo")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
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
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let res: JsonRpcErrorResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(res.jsonrpc, "2.0");
    assert_eq!(res.id, Some(1.into()));
    assert_eq!(res.error.code, JsonRpcErrorCode::MethodNotFound);
}

/// Tests that missing method in header returns a Header Mismatch error (-32020).
#[tokio::test]
async fn test_mcp_router_missing_method_in_header_and_body_returns_bad_request() {
    let app = McpRouter::new(test_server_info());
    let request = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(json!({"id": 1}).to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let res: JsonRpcErrorResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(res.jsonrpc, "2.0");
    assert_eq!(res.id, Some(1.into()));
    assert_eq!(res.error.code.code(), crate::types::mcp::HEADER_MISMATCH);
}

/// Tests that empty tool name in `tools/call` returns a JSON-RPC Invalid Params error (-32602).
#[tokio::test]
async fn test_mcp_router_empty_tool_name_returns_bad_request() {
    let app = McpRouter::new(test_server_info());
    let request = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
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

/// Tests that calling an unregistered tool returns a JSON-RPC Invalid Params error (-32602).
#[tokio::test]
async fn test_mcp_router_unknown_tool_returns_invalid_params() {
    let app = McpRouter::new(test_server_info());
    let request = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "non_existent_tool")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
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
    assert_eq!(res.error.code, JsonRpcErrorCode::InvalidParams);
}

/// Tests that an unknown method returns a JSON-RPC Method Not Found error (-32601).
#[tokio::test]
async fn test_mcp_router_unknown_method_returns_not_found() {
    let app = McpRouter::new(test_server_info());
    let request = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "unknown/method")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(
            json!({
                "id": 1,
                "method": "unknown/method"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

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
        .header("MCP-Protocol-Version", "2026-07-28")
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
    use crate::types::mcp::{ContentBlock, tools::call::CallToolResultResponse};
    use serde::{Deserialize, Serialize};

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
        .header("MCP-Protocol-Version", "2026-07-28")
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
    use crate::types::mcp::{ContentBlock, tools::call::CallToolResultResponse};
    use serde::{Deserialize, Serialize};

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
        .header("MCP-Protocol-Version", "2026-07-28")
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
            response
                .headers()
                .get("allow")
                .and_then(|h| h.to_str().ok()),
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
        .header("MCP-Protocol-Version", "2026-07-28")
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

/// Tests default `Cache-Control: public, max-age=0` and `ETag` headers on `server/discover`.
#[tokio::test]
async fn test_mcp_router_server_discover_caching_headers_default() {
    let app = McpRouter::new(test_server_info());

    let request = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "server/discover")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(
            json!({"id": 1, "method": "server/discover"}).to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("cache-control")
            .and_then(|h| h.to_str().ok()),
        Some("public, max-age=0")
    );
    assert!(response.headers().contains_key("etag"));

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let expected_etag = crate::body::compute_etag(&bytes);
    assert_eq!(expected_etag, crate::body::compute_etag(&bytes));
}

/// Tests default `Cache-Control: public, max-age=0` and `ETag` headers on `tools/list`.
#[tokio::test]
async fn test_mcp_router_tools_list_caching_headers_default() {
    let app = McpRouter::new(test_server_info()).register_tool("echo", mock_handler);

    let request = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "tools/list")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(
            json!({"id": 1, "method": "tools/list"}).to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("cache-control")
            .and_then(|h| h.to_str().ok()),
        Some("public, max-age=0")
    );
    assert!(response.headers().contains_key("etag"));
}

/// Tests custom caching configuration on `server/discover` using builder methods.
#[tokio::test]
async fn test_mcp_router_server_discover_custom_caching_headers() {
    use crate::types::mcp::CacheScope;

    let app = McpRouter::new(test_server_info())
        .server_discover_cache(Some(60000), Some(CacheScope::Private));

    let request = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "server/discover")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(
            json!({"id": 1, "method": "server/discover"}).to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("cache-control")
            .and_then(|h| h.to_str().ok()),
        Some("private, max-age=60")
    );
    assert!(response.headers().contains_key("etag"));

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let res: ServerDiscoverResultResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(res.result.ttl_ms, Some(60000));
    assert!(matches!(res.result.cache_scope, Some(CacheScope::Private)));
}

/// Tests custom caching configuration on `tools/list` using individual TTL and scope builders.
#[tokio::test]
async fn test_mcp_router_tools_list_custom_caching_headers() {
    use crate::types::mcp::CacheScope;

    let app = McpRouter::new(test_server_info())
        .tools_list_ttl(120000)
        .tools_list_cache_scope(CacheScope::Public)
        .register_tool("echo", mock_handler);

    let request = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "tools/list")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(
            json!({"id": 1, "method": "tools/list"}).to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("cache-control")
            .and_then(|h| h.to_str().ok()),
        Some("public, max-age=120")
    );
    assert!(response.headers().contains_key("etag"));

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let res: ListToolsResultResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(res.result.ttl_ms, Some(120000));
    assert!(matches!(res.result.cache_scope, Some(CacheScope::Public)));
}

/// Tests disabling caching directives (no `Cache-Control` header) while preserving `ETag`.
#[tokio::test]
async fn test_mcp_router_disabled_caching_headers() {
    let app = McpRouter::new(test_server_info()).server_discover_cache(None, None);

    let request = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "server/discover")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(
            json!({"id": 1, "method": "server/discover"}).to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers().get("cache-control"), None);
    assert!(response.headers().contains_key("etag"));

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let res: ServerDiscoverResultResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(res.result.ttl_ms, None);
    assert_eq!(res.result.cache_scope, None);
}

/// Tests per-tool caching headers on `tools/call` via `register_tool_with_cache` and `tool_cache`.
#[tokio::test]
async fn test_mcp_router_per_tool_caching_headers() {
    use crate::types::mcp::CacheScope;

    let app = McpRouter::new(test_server_info())
        .register_tool_with_cache(
            "cached_tool",
            mock_handler,
            Some(45000),
            Some(CacheScope::Public),
        )
        .register_tool("configured_tool", mock_handler)
        .tool_cache("configured_tool", Some(90000), Some(CacheScope::Private))
        .register_tool("uncached_tool", mock_handler);

    // 1. Tool registered with cache (public, 45s)
    let req1 = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "cached_tool")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(
            json!({"id": 1, "method": "tools/call", "params": {"name": "cached_tool"}}).to_string(),
        ))
        .unwrap();

    let resp1 = app.clone().oneshot(req1).await.unwrap();
    assert_eq!(resp1.status(), StatusCode::OK);
    assert_eq!(
        resp1
            .headers()
            .get("cache-control")
            .and_then(|h| h.to_str().ok()),
        Some("public, max-age=45")
    );
    assert!(resp1.headers().contains_key("etag"));

    // 2. Tool configured via .tool_cache() (private, 90s)
    let req2 = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "configured_tool")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(
            json!({"id": 2, "method": "tools/call", "params": {"name": "configured_tool"}})
                .to_string(),
        ))
        .unwrap();

    let resp2 = app.clone().oneshot(req2).await.unwrap();
    assert_eq!(resp2.status(), StatusCode::OK);
    assert_eq!(
        resp2
            .headers()
            .get("cache-control")
            .and_then(|h| h.to_str().ok()),
        Some("private, max-age=90")
    );
    assert!(resp2.headers().contains_key("etag"));

    // 3. Tool without cache settings (ETag only, no Cache-Control)
    let req3 = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "uncached_tool")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(
            json!({"id": 3, "method": "tools/call", "params": {"name": "uncached_tool"}})
                .to_string(),
        ))
        .unwrap();

    let resp3 = app.oneshot(req3).await.unwrap();
    assert_eq!(resp3.status(), StatusCode::OK);
    assert_eq!(resp3.headers().get("cache-control"), None);
    assert!(resp3.headers().contains_key("etag"));
}

/// Tests that `prompts/list` returns the registered prompt with its metadata.
#[tokio::test]
async fn test_mcp_router_builtin_prompts_list() {
    let prompt = Prompt::new("test_prompt")
        .title("Test Prompt")
        .description("A test prompt template")
        .argument(PromptArgument::new("arg1").required(true));

    let app = McpRouter::new(test_server_info()).register_prompt(prompt, mock_handler);
    let request = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "prompts/list")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(
            json!({"id": 1, "method": "prompts/list"}).to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let res: ListPromptsResultResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(res.result.prompts.len(), 1);
    assert_eq!(res.result.prompts[0].name, "test_prompt");
    assert_eq!(res.result.prompts[0].title.as_deref(), Some("Test Prompt"));
    assert_eq!(res.result.prompts[0].arguments.len(), 1);
}

/// Tests that `prompts/get` retrieves prompt messages from a typed handler.
#[tokio::test]
async fn test_mcp_router_prompts_get_success() {
    let app = McpRouter::new(test_server_info())
        .register_prompt("greeting", || async { "Hello, world!" });

    let request = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "prompts/get")
        .header("Mcp-Name", "greeting")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(
            json!({"id": "p-1", "method": "prompts/get"}).to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let res: GetPromptResultResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(res.id, "p-1".into());
    assert_eq!(res.result.messages.len(), 1);
}

/// Tests that `prompts/get` returns error code for unknown prompt.
#[tokio::test]
async fn test_mcp_router_prompts_get_unknown() {
    let app = McpRouter::new(test_server_info());

    let request = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "prompts/get")
        .header("Mcp-Name", "unknown_prompt")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(
            json!({"id": 99, "method": "prompts/get"}).to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let err_resp: JsonRpcErrorResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(err_resp.error.code, JsonRpcErrorCode::InvalidParams);
}

/// Tests caching headers for `prompts/list` and `prompts/get`.
#[tokio::test]
async fn test_mcp_router_prompts_caching_headers() {
    let app = McpRouter::new(test_server_info())
        .prompts_list_ttl(180_000)
        .register_prompt_with_cache(
            "cached_p",
            || async { "Cached prompt text" },
            Some(60_000),
            Some(CacheScope::Public),
        );

    // List cache
    let req_list = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "prompts/list")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(
            json!({"id": 1, "method": "prompts/list"}).to_string(),
        ))
        .unwrap();

    let resp_list = app.clone().oneshot(req_list).await.unwrap();
    assert_eq!(
        resp_list
            .headers()
            .get("cache-control")
            .and_then(|h| h.to_str().ok()),
        Some("public, max-age=180")
    );

    // Get cache
    let req_get = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "prompts/get")
        .header("Mcp-Name", "cached_p")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(
            json!({"id": 2, "method": "prompts/get"}).to_string(),
        ))
        .unwrap();

    let resp_get = app.oneshot(req_get).await.unwrap();
    assert_eq!(
        resp_get
            .headers()
            .get("cache-control")
            .and_then(|h| h.to_str().ok()),
        Some("public, max-age=60")
    );
}

/// Tests that omitting `MCP-Protocol-Version` header returns HTTP 400 Bad Request with HeaderMismatch (-32020).
#[tokio::test]
async fn test_mcp_router_missing_protocol_version_header_returns_header_mismatch() {
    let app = McpRouter::new(test_server_info());

    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "server/discover")
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({"id": 1, "method": "server/discover"}).to_string(),
        ))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let err_resp: JsonRpcErrorResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        err_resp.error.code.code(),
        crate::types::mcp::HEADER_MISMATCH
    );
    assert!(
        err_resp
            .error
            .message
            .contains("missing required MCP-Protocol-Version header")
    );
}

/// Tests that an unsupported `MCP-Protocol-Version` header returns HTTP 400 Bad Request with UnsupportedProtocolVersion (-32022).
#[tokio::test]
async fn test_mcp_router_unsupported_protocol_version_header_returns_unsupported_version() {
    let app = McpRouter::new(test_server_info());

    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "server/discover")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2024-11-05")
        .body(Body::from(
            json!({"id": 1, "method": "server/discover"}).to_string(),
        ))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let err_resp: JsonRpcErrorResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        err_resp.error.code.code(),
        crate::types::mcp::UNSUPPORTED_PROTOCOL_VERSION
    );
    assert!(
        err_resp
            .error
            .message
            .contains("Unsupported protocol version '2024-11-05'")
    );
    let data = err_resp.error.data.unwrap();
    assert_eq!(data["supported"][0], "2026-07-28");
    assert_eq!(data["requested"], "2024-11-05");
}

/// Tests that a mismatch between `MCP-Protocol-Version` header and body `_meta` returns HTTP 400 Bad Request with HeaderMismatch (-32020).
#[tokio::test]
async fn test_mcp_router_protocol_version_header_body_mismatch_returns_header_mismatch() {
    let app = McpRouter::new(test_server_info());

    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "server/discover")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(
            json!({
                "id": 1,
                "method": "server/discover",
                "params": {
                    "_meta": {
                        "io.modelcontextprotocol/protocolVersion": "2025-06-18"
                    }
                }
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let err_resp: JsonRpcErrorResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        err_resp.error.code.code(),
        crate::types::mcp::HEADER_MISMATCH
    );
    assert!(err_resp.error.message.contains(
        "MCP-Protocol-Version header value '2026-07-28' does not match body value '2025-06-18'"
    ));
}

/// Tests that disabling protocol version validation accepts omitted header or custom version strings.
#[tokio::test]
async fn test_mcp_router_disabled_protocol_version_validation() {
    let app = McpRouter::new(test_server_info())
        .validate_protocol_version(false)
        .instructions("Bypass validation");

    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "server/discover")
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({"id": 1, "method": "server/discover"}).to_string(),
        ))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

/// Tests that missing `Mcp-Name` header on `prompts/get` returns HTTP 400 Bad Request with HeaderMismatch (-32020).
#[tokio::test]
async fn test_mcp_router_missing_mcp_name_header_for_prompts_get() {
    let prompt = Prompt::new("review").description("Review code");
    let app =
        McpRouter::new(test_server_info()).register_prompt(prompt, || async { "prompt content" });

    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "prompts/get")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(
            json!({
                "id": 1,
                "method": "prompts/get",
                "params": {
                    "name": "review"
                }
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let err_resp: JsonRpcErrorResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        err_resp.error.code.code(),
        crate::types::mcp::HEADER_MISMATCH
    );
}

/// Tests that `Mcp-Name` header mismatch with body `params.name` on `prompts/get` returns HTTP 400 Bad Request with HeaderMismatch (-32020).
#[tokio::test]
async fn test_mcp_router_mcp_name_mismatch_for_prompts_get() {
    let prompt = Prompt::new("review").description("Review code");
    let app =
        McpRouter::new(test_server_info()).register_prompt(prompt, || async { "prompt content" });

    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "prompts/get")
        .header("Mcp-Name", "review")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(
            json!({
                "id": 1,
                "method": "prompts/get",
                "params": {
                    "name": "other_prompt"
                }
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let err_resp: JsonRpcErrorResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        err_resp.error.code.code(),
        crate::types::mcp::HEADER_MISMATCH
    );
}

/// Tests that missing `Mcp-Uri` header on `resources/read` returns HTTP 400 Bad Request with HeaderMismatch (-32020).
#[tokio::test]
async fn test_mcp_router_missing_mcp_uri_header_for_resources_read() {
    let app = McpRouter::new(test_server_info())
        .register_resource(("file:///config.json", "Config"), || async {
            "config data"
        });

    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "resources/read")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(
            json!({
                "id": 1,
                "method": "resources/read",
                "params": {
                    "uri": "file:///config.json"
                }
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let err_resp: JsonRpcErrorResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        err_resp.error.code.code(),
        crate::types::mcp::HEADER_MISMATCH
    );
}

/// Tests that `Mcp-Uri` header mismatch with body `params.uri` on `resources/read` returns HTTP 400 Bad Request with HeaderMismatch (-32020).
#[tokio::test]
async fn test_mcp_router_mcp_uri_mismatch_for_resources_read() {
    let app = McpRouter::new(test_server_info())
        .register_resource(("file:///config.json", "Config"), || async {
            "config data"
        });

    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "resources/read")
        .header("Mcp-Uri", "file:///config.json")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(
            json!({
                "id": 1,
                "method": "resources/read",
                "params": {
                    "uri": "file:///other.json"
                }
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let err_resp: JsonRpcErrorResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        err_resp.error.code.code(),
        crate::types::mcp::HEADER_MISMATCH
    );
}

/// Tests batch request handling with individual body methods when Mcp-Method header is omitted on the HTTP request.
#[tokio::test]
async fn test_mcp_router_batch_request_without_header_method() {
    let app = McpRouter::new(test_server_info())
        .instructions("Batch test")
        .register_tool("echo", mock_handler);

    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(
            json!([
                { "id": 1, "method": "server/discover" },
                { "id": 2, "method": "tools/list" }
            ])
            .to_string(),
        ))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let batch_res: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(batch_res.len(), 2);
    assert_eq!(batch_res[0]["id"], 1.0);
    assert_eq!(batch_res[1]["id"], 2.0);
}
