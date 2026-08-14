// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Routing Edge Cases & Error Handling Integration Tests
//!
//! Verifies the error boundaries, JSON-RPC 2.0 error codes, and header normalization of [`McpRouter`](mcp_routing::McpRouter):
//! - Header and body normalization (leading/trailing slash tolerance in `Mcp-Method` and `Mcp-Name`)
//! - Missing or empty method rejection (`-32600 Invalid Request`)
//! - Missing or empty tool name rejection for `tools/call` (`-32602 Invalid Params`)
//! - Unknown method rejection (`-32601 Method Not Found`)
//! - Non-standard method path suffixes (`-32601 Method Not Found`)
//! - Unregistered tool execution attempts (`-32601 Method Not Found`)
//! - Malformed JSON payloads across all endpoints (`-32700 Parse Error` with `id: null`)

mod common;

use axum::body::Body;
use http::{Request, StatusCode};
use mcp_routing::{
    McpRouter,
    types::jsonrpc::{
        INVALID_PARAMS_CODE, INVALID_REQUEST_CODE, METHOD_NOT_FOUND_CODE, PARSE_ERROR_CODE,
        JsonRpcErrorResponse,
    },
};
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

/// Tests that requests lacking a method in both the `Mcp-Method` header and the JSON body return `Invalid Request` (-32600).
#[tokio::test]
async fn test_missing_method_returns_invalid_request() {
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

    let (status, headers, body) = common::execute_request(app, req).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get("content-type").unwrap().to_str().unwrap(),
        "application/json"
    );

    let err_resp: JsonRpcErrorResponse = serde_json::from_value(body).unwrap();
    assert_eq!(err_resp.jsonrpc, "2.0");
    assert_eq!(err_resp.id, Some(1.into()));
    assert_eq!(err_resp.error.code.code(), INVALID_REQUEST_CODE);
}

/// Tests that requests with an empty `Mcp-Method` header string return `Invalid Request` (-32600).
#[tokio::test]
async fn test_empty_method_returns_invalid_request() {
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

    let (status, _headers, body) = common::execute_request(app, req).await;
    assert_eq!(status, StatusCode::OK);

    let err_resp: JsonRpcErrorResponse = serde_json::from_value(body).unwrap();
    assert_eq!(err_resp.jsonrpc, "2.0");
    assert_eq!(err_resp.id, Some(1.into()));
    assert_eq!(err_resp.error.code.code(), INVALID_REQUEST_CODE);
}

/// Tests that a `tools/call` request without a tool name in headers or body returns `Invalid Params` (-32602).
#[tokio::test]
async fn test_missing_tool_name_returns_invalid_params() {
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

    let (status, _headers, body) = common::execute_request(app, req).await;
    assert_eq!(status, StatusCode::OK);

    let err_resp: JsonRpcErrorResponse = serde_json::from_value(body).unwrap();
    assert_eq!(err_resp.jsonrpc, "2.0");
    assert_eq!(err_resp.id, Some(1.into()));
    assert_eq!(err_resp.error.code.code(), INVALID_PARAMS_CODE);
}

/// Tests that a `tools/call` request with an empty tool name string returns `Invalid Params` (-32602).
#[tokio::test]
async fn test_empty_tool_name_returns_invalid_params() {
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

    let (status, _headers, body) = common::execute_request(app, req).await;
    assert_eq!(status, StatusCode::OK);

    let err_resp: JsonRpcErrorResponse = serde_json::from_value(body).unwrap();
    assert_eq!(err_resp.jsonrpc, "2.0");
    assert_eq!(err_resp.id, Some(1.into()));
    assert_eq!(err_resp.error.code.code(), INVALID_PARAMS_CODE);
}

/// Tests that an unrecognized MCP method (e.g. `resources/list` before implementation) returns `Method Not Found` (-32601).
#[tokio::test]
async fn test_unknown_method_returns_method_not_found() {
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

    let (status, _headers, body) = common::execute_request(app, req).await;
    assert_eq!(status, StatusCode::OK);

    let err_resp: JsonRpcErrorResponse = serde_json::from_value(body).unwrap();
    assert_eq!(err_resp.jsonrpc, "2.0");
    assert_eq!(err_resp.id, Some(1.into()));
    assert_eq!(err_resp.error.code.code(), METHOD_NOT_FOUND_CODE);
}

/// Tests that non-standard path suffix methods like `tools/call/echo` return `Method Not Found` (-32601).
#[tokio::test]
async fn test_invalid_method_path_suffix_returns_method_not_found() {
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

    let (status, _headers, body) = common::execute_request(app, req).await;
    assert_eq!(status, StatusCode::OK);

    let err_resp: JsonRpcErrorResponse = serde_json::from_value(body).unwrap();
    assert_eq!(err_resp.jsonrpc, "2.0");
    assert_eq!(err_resp.id, Some(1.into()));
    assert_eq!(err_resp.error.code.code(), METHOD_NOT_FOUND_CODE);
}

/// Tests that attempting to call an unregistered tool name returns `Method Not Found` (-32601).
#[tokio::test]
async fn test_unknown_tool_returns_method_not_found() {
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

    let (status, _headers, body) = common::execute_request(app, req).await;
    assert_eq!(status, StatusCode::OK);

    let err_resp: JsonRpcErrorResponse = serde_json::from_value(body).unwrap();
    assert_eq!(err_resp.jsonrpc, "2.0");
    assert_eq!(err_resp.id, Some(1.into()));
    assert_eq!(err_resp.error.code.code(), METHOD_NOT_FOUND_CODE);
}

/// Tests that malformed or non-JSON payloads across all endpoints return `Parse Error` (-32700) with `id: null`.
#[tokio::test]
async fn test_malformed_json_body_returns_parse_error() {
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

    let (status1, _, body1) = common::execute_request(app.clone(), req1).await;
    assert_eq!(status1, StatusCode::OK);
    assert_eq!(body1["jsonrpc"], "2.0");
    assert_eq!(body1["id"], serde_json::Value::Null);
    assert_eq!(body1["error"]["code"], PARSE_ERROR_CODE);

    // 2. Invalid JSON in tools/list
    let req2 = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "tools/list")
        .header("Content-Type", "application/json")
        .body(Body::from("{\"jsonrpc\": \"2.0\", \"id\": "))
        .unwrap();

    let (status2, _, body2) = common::execute_request(app.clone(), req2).await;
    assert_eq!(status2, StatusCode::OK);
    assert_eq!(body2["jsonrpc"], "2.0");
    assert_eq!(body2["id"], serde_json::Value::Null);
    assert_eq!(body2["error"]["code"], PARSE_ERROR_CODE);

    // 3. Invalid JSON in tools/call
    let req3 = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "echo")
        .header("Content-Type", "application/json")
        .body(Body::from("<xml>not json</xml>"))
        .unwrap();

    let (status3, _, body3) = common::execute_request(app.clone(), req3).await;
    assert_eq!(status3, StatusCode::OK);
    assert_eq!(body3["jsonrpc"], "2.0");
    assert_eq!(body3["id"], serde_json::Value::Null);
    assert_eq!(body3["error"]["code"], PARSE_ERROR_CODE);
}
