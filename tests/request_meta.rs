// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Required Per-Request `_meta` Integration Tests
//!
//! Verifies that [`McpRouter`](stateless_mcp::McpRouter) enforces the per-request protocol fields
//! every MCP request must carry in `params._meta`:
//! - Requests missing `params`, `_meta`, `io.modelcontextprotocol/protocolVersion`, or
//!   `io.modelcontextprotocol/clientCapabilities` are rejected with `-32602` and HTTP `400 Bad Request`
//! - Requests carrying both required fields are accepted
//! - Notifications are not subject to the per-request field requirement

mod common;

use http::StatusCode;
use stateless_mcp::{McpRouter, types::jsonrpc::INVALID_PARAMS_CODE};
use serde_json::{Value, json};

/// Sends a `tools/list` request with the given `params` and returns the status and JSON body.
async fn send_tools_list(params: Option<Value>) -> (StatusCode, Value) {
    let app = McpRouter::new(common::sample_server_info());
    let mut body = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" });
    if let Some(params) = params {
        body["params"] = params;
    }
    let req = common::build_request(Some("tools/list"), None, body);
    let (status, _headers, body) = common::execute_request(app, req).await;
    (status, body)
}

/// Tests that requests without the required `_meta` fields are rejected with `-32602` and HTTP 400.
///
/// Verifies:
/// - Missing `params`, empty `params`, and `_meta` lacking either required field are all rejected
/// - The error echoes the request ID
#[tokio::test]
async fn test_missing_required_meta_is_rejected() {
    let cases = [
        None,
        Some(json!({})),
        Some(json!({ "_meta": {} })),
        Some(json!({
            "_meta": { "io.modelcontextprotocol/clientCapabilities": {} }
        })),
        Some(json!({
            "_meta": { "io.modelcontextprotocol/protocolVersion": "2026-07-28" }
        })),
    ];

    for params in cases {
        let (status, body) = send_tools_list(params.clone()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "params: {params:?}");
        assert_eq!(
            body["error"]["code"], INVALID_PARAMS_CODE,
            "params: {params:?}"
        );
        assert_eq!(body["id"], 1, "params: {params:?}");
    }
}

/// Tests that a request carrying both required `_meta` fields is accepted.
#[tokio::test]
async fn test_required_meta_present_is_accepted() {
    let (status, body) = send_tools_list(Some(json!({ "_meta": common::meta() }))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"]["resultType"], "complete");
}

/// Tests that notifications without `_meta` are still accepted with `202 Accepted`.
#[tokio::test]
async fn test_notification_without_meta_is_accepted() {
    let app = McpRouter::new(common::sample_server_info());
    let req = common::build_request(
        Some("notifications/cancelled"),
        None,
        json!({
            "jsonrpc": "2.0",
            "method": "notifications/cancelled",
            "params": { "requestId": 1 }
        }),
    );
    let (status, _headers, _body) = common::execute_request(app, req).await;
    assert_eq!(status, StatusCode::ACCEPTED);
}
