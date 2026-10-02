// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # JSON-RPC Message Framing Integration Tests
//!
//! Verifies that each Streamable HTTP POST body carries exactly one JSON-RPC message:
//! - JSON-RPC batch arrays (including empty `[]`) are rejected with HTTP 400 and `-32600`
//!   without dispatching any element
//! - Single notifications return HTTP 202 Accepted with an empty body
//! - Request methods sent without an `id` are rejected with HTTP 400 and `-32600`, and their handlers never run
//! - Malformed JSON returns a Parse Error (`-32700`) with `id: null`
//! - Top-level JSON primitive payloads return Invalid Request (`-32600`)

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::body::Body;
use http::{Request, StatusCode};
use stateless_mcp::{
    McpRouter,
    types::jsonrpc::{INVALID_REQUEST_CODE, JsonRpcErrorResponse, PARSE_ERROR_CODE},
};
use serde_json::json;

/// Builds a POST request with the standard MCP headers and a raw body.
fn raw_request(method_header: Option<&str>, body: impl Into<Body>) -> Request<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28");
    if let Some(method) = method_header {
        builder = builder.header("Mcp-Method", method);
    }
    builder.body(body.into()).unwrap()
}

/// Tests that batch arrays are rejected with `-32600` and that no batch element is executed.
///
/// Verifies:
/// - A batch of valid `tools/call` requests returns HTTP 400 with a single error object
/// - The tool handler is never invoked
/// - An empty batch `[]` is rejected the same way
#[tokio::test]
async fn test_batch_array_is_rejected() {
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&calls);
    let app = McpRouter::new(common::sample_server_info()).register_tool("count", move || {
        let counter = Arc::clone(&counter);
        async move {
            counter.fetch_add(1, Ordering::SeqCst);
            "counted"
        }
    });

    let batch = json!([
        {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": { "_meta": common::meta(), "name": "count" }
        },
        {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": { "_meta": common::meta(), "name": "count" }
        }
    ]);

    for body in [batch.to_string(), "[]".to_string()] {
        let req = raw_request(Some("tools/call"), body.clone());
        let (status, _, response) = common::execute_request(app.clone(), req).await;

        assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
        let err: JsonRpcErrorResponse = serde_json::from_value(response).unwrap();
        assert_eq!(err.id, None);
        assert_eq!(err.error.code.code(), INVALID_REQUEST_CODE);
    }

    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

/// Tests that a single notification request returns HTTP 202 Accepted with an empty body.
#[tokio::test]
async fn test_single_notification_returns_202() {
    let app = McpRouter::new(common::sample_server_info());

    let req = raw_request(
        Some("notifications/cancelled"),
        json!({
            "jsonrpc": "2.0",
            "method": "notifications/cancelled",
            "params": { "requestId": 1 }
        })
        .to_string(),
    );

    let (status, _, body_bytes) = common::execute_request_raw(app, req).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert!(body_bytes.is_empty());
}

/// Tests that malformed JSON syntax returns a Parse Error (`-32700`) with `id: null`.
#[tokio::test]
async fn test_malformed_json_returns_parse_error() {
    let app = McpRouter::new(common::sample_server_info());

    let req = raw_request(
        Some("tools/list"),
        r#"{"jsonrpc": "2.0", "method": "tools/list", "id": "#,
    );

    let (status, _, body) = common::execute_request(app, req).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let err: JsonRpcErrorResponse = serde_json::from_value(body).unwrap();
    assert_eq!(err.jsonrpc, "2.0");
    assert_eq!(err.id, None);
    assert_eq!(err.error.code.code(), PARSE_ERROR_CODE);
}

/// Tests that a top-level JSON primitive payload (e.g. `12345`) returns Invalid Request (`-32600`).
#[tokio::test]
async fn test_top_level_primitive_returns_invalid_request() {
    let app = McpRouter::new(common::sample_server_info());

    let req = raw_request(None, "12345");

    let (status, _, body) = common::execute_request(app, req).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let err: JsonRpcErrorResponse = serde_json::from_value(body).unwrap();
    assert_eq!(err.jsonrpc, "2.0");
    assert_eq!(err.id, None);
    assert_eq!(err.error.code.code(), INVALID_REQUEST_CODE);
}

/// Tests that a request method sent without an `id` is rejected and its handler never runs.
///
/// Verifies:
/// - `tools/call` without `id` returns HTTP 400 with `-32600` and `id: null`
/// - The tool handler is not invoked
#[tokio::test]
async fn test_request_method_without_id_is_rejected() {
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&calls);
    let app = McpRouter::new(common::sample_server_info()).register_tool("count", move || {
        let counter = Arc::clone(&counter);
        async move {
            counter.fetch_add(1, Ordering::SeqCst);
            "counted"
        }
    });

    let mut req = raw_request(
        Some("tools/call"),
        json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": { "_meta": common::meta(), "name": "count" }
        })
        .to_string(),
    );
    req.headers_mut()
        .insert("Mcp-Name", "count".parse().unwrap());

    let (status, _, body) = common::execute_request(app, req).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let err: JsonRpcErrorResponse = serde_json::from_value(body).unwrap();
    assert_eq!(err.id, None);
    assert_eq!(err.error.code.code(), INVALID_REQUEST_CODE);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}
