// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Routing Edge Cases & Error Handling Integration Tests
//!
//! Verifies the error boundaries, HTTP status codes, and header normalization of [`McpRouter`](mcp_routing::McpRouter):
//! - Header and body normalization (leading/trailing slash tolerance in `Mcp-Method` and `Mcp-Name`)
//! - Missing or empty method rejection (`400 Bad Request`)
//! - Missing or empty tool name rejection for `tools/call` (`400 Bad Request`)
//! - Unknown method rejection (`404 Not Found`)
//! - Non-standard method path suffixes (`404 Not Found`)
//! - Unregistered tool execution attempts (`404 Not Found`)
//! - Malformed JSON payloads across all endpoints (`400 Bad Request`)

mod common;

use axum::body::Body;
use http::{Request, StatusCode};
use mcp_routing::McpRouter;
use serde_json::json;

async fn dummy_tool() -> &'static str {
    "success"
}

/// Tests that leading and trailing slashes in headers (`/tools/call/`, `/echo_tool/`) and body strings are normalized.
///
/// Verifies:
/// - Method and tool name resolution is robust to surrounding slashes
#[tokio::test]
async fn test_slash_normalization_in_headers_and_body() {
    let app = McpRouter::new(common::sample_server_info())
        .register_tool("echo_tool", dummy_tool);

    // 1. Headers with leading and trailing slashes
    let req1 = common::build_request(
        Some("/tools/call/"),
        Some("/echo_tool/"),
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call"
        }),
    );
    let (status1, _, body1) = common::execute_request(app.clone(), req1).await;
    assert_eq!(status1, StatusCode::OK);
    assert_eq!(body1["result"]["content"][0]["text"], "success");

    // 2. Body method with leading and trailing slashes
    let req2 = common::build_request(
        None,
        None,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "/tools/list/"
        }),
    );
    let (status2, _, body2) = common::execute_request(app.clone(), req2).await;
    assert_eq!(status2, StatusCode::OK);
    assert_eq!(body2["result"]["tools"].as_array().unwrap().len(), 1);

    // 3. Body tool name with leading and trailing slashes
    let req3 = common::build_request(
        Some("tools/call"),
        None,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "/echo_tool/"
            }
        }),
    );
    let (status3, _, body3) = common::execute_request(app.clone(), req3).await;
    assert_eq!(status3, StatusCode::OK);
    assert_eq!(body3["result"]["content"][0]["text"], "success");
}

/// Tests that requests lacking a method in both the `Mcp-Method` header and the JSON body return `400 Bad Request`.
#[tokio::test]
async fn test_missing_method_returns_bad_request() {
    let app = McpRouter::new(common::sample_server_info());

    // No Mcp-Method header and no method in body
    let req = common::build_request(
        None,
        None,
        json!({
            "jsonrpc": "2.0",
            "id": 1
        }),
    );

    let (status, _headers, body_bytes) = common::execute_request_raw(app, req).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body_bytes.is_empty());
}

/// Tests that requests with an empty `Mcp-Method` header string return `400 Bad Request`.
#[tokio::test]
async fn test_empty_method_returns_bad_request() {
    let app = McpRouter::new(common::sample_server_info());

    // Mcp-Method header is empty string
    let req = common::build_request(
        Some(""),
        None,
        json!({
            "jsonrpc": "2.0",
            "id": 1
        }),
    );

    let (status, _headers, body_bytes) = common::execute_request_raw(app, req).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body_bytes.is_empty());
}

/// Tests that a `tools/call` request without a tool name in headers or body returns `400 Bad Request`.
#[tokio::test]
async fn test_missing_tool_name_returns_bad_request() {
    let app = McpRouter::new(common::sample_server_info())
        .register_tool("sample", dummy_tool);

    // Mcp-Method is tools/call, but no Mcp-Name header and no params.name in body
    let req = common::build_request(
        Some("tools/call"),
        None,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "arguments": {}
            }
        }),
    );

    let (status, _headers, body_bytes) = common::execute_request_raw(app, req).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body_bytes.is_empty());
}

/// Tests that a `tools/call` request with an empty tool name string returns `400 Bad Request`.
#[tokio::test]
async fn test_empty_tool_name_returns_bad_request() {
    let app = McpRouter::new(common::sample_server_info())
        .register_tool("sample", dummy_tool);

    // Empty tool name
    let req = common::build_request(
        Some("tools/call"),
        Some(""),
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": ""
            }
        }),
    );

    let (status, _headers, body_bytes) = common::execute_request_raw(app, req).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body_bytes.is_empty());
}

/// Tests that an unrecognized MCP method (e.g. `resources/list` before implementation) returns `404 Not Found`.
#[tokio::test]
async fn test_unknown_method_returns_not_found() {
    let app = McpRouter::new(common::sample_server_info());

    let req = common::build_request(
        Some("resources/list"),
        None,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "resources/list"
        }),
    );

    let (status, _headers, body_bytes) = common::execute_request_raw(app, req).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body_bytes.is_empty());
}

/// Tests that non-standard path suffix methods like `tools/call/echo` return `404 Not Found`.
#[tokio::test]
async fn test_invalid_method_path_suffix_returns_not_found() {
    let app = McpRouter::new(common::sample_server_info())
        .register_tool("echo", dummy_tool);

    let req = common::build_request(
        Some("tools/call/echo"),
        None,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call/echo"
        }),
    );

    let (status, _headers, body_bytes) = common::execute_request_raw(app, req).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body_bytes.is_empty());
}

/// Tests that attempting to call an unregistered tool name returns `404 Not Found`.
#[tokio::test]
async fn test_unknown_tool_returns_not_found() {
    let app = McpRouter::new(common::sample_server_info())
        .register_tool("existing_tool", dummy_tool);

    let req = common::build_request(
        Some("tools/call"),
        Some("non_existent_tool"),
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "non_existent_tool"
            }
        }),
    );

    let (status, _headers, body_bytes) = common::execute_request_raw(app, req).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body_bytes.is_empty());
}

/// Tests that malformed or non-JSON payloads across all endpoints return `400 Bad Request`.
#[tokio::test]
async fn test_malformed_json_body_returns_bad_request() {
    let app = McpRouter::new(common::sample_server_info())
        .register_tool("echo", dummy_tool);

    // 1. Invalid JSON in server/discover
    let req1 = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "server/discover")
        .header("Content-Type", "application/json")
        .body(Body::from("NOT_A_VALID_JSON{"))
        .unwrap();

    let (status1, _, body1) = common::execute_request_raw(app.clone(), req1).await;
    assert_eq!(status1, StatusCode::BAD_REQUEST);
    assert!(body1.is_empty());

    // 2. Invalid JSON in tools/list
    let req2 = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "tools/list")
        .header("Content-Type", "application/json")
        .body(Body::from("{\"jsonrpc\": \"2.0\", \"id\": "))
        .unwrap();

    let (status2, _, body2) = common::execute_request_raw(app.clone(), req2).await;
    assert_eq!(status2, StatusCode::BAD_REQUEST);
    assert!(body2.is_empty());

    // 3. Invalid JSON in tools/call
    let req3 = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "echo")
        .header("Content-Type", "application/json")
        .body(Body::from("<xml>not json</xml>"))
        .unwrap();

    let (status3, _, body3) = common::execute_request_raw(app.clone(), req3).await;
    assert_eq!(status3, StatusCode::BAD_REQUEST);
    assert!(body3.is_empty());
}
