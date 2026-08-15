// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # DNS Rebinding Protection & Origin Header Validation Integration Tests
//!
//! Verifies HTTP `Origin` header enforcement and DNS rebinding protections of [`McpRouter`](mcp_routing::McpRouter):
//! - Accepting incoming requests with trusted `Origin` headers matching `allowed_origins`
//! - Case insensitivity and trailing slash tolerance during origin matching
//! - Rejecting incoming requests with untrusted `Origin` headers (`403 Forbidden`)
//! - Rejecting blank or whitespace-only `Origin` headers (`403 Forbidden`)
//! - Permitting wildcard `"*"` origin matching
//! - Permitting requests without `Origin` headers (non-browser clients)
//! - Permissive default behavior when `allowed_origins` is unconfigured
//! - Preserving `mcp-session-id` response header on `403 Forbidden` rejections

mod common;

use axum::body::Body;
use http::{Request, StatusCode};
use mcp_routing::McpRouter;
use serde_json::json;

async fn echo_tool() -> &'static str {
    "hello world"
}

/// Builds an HTTP POST request with an optional `Origin` header.
fn build_origin_request(origin_header: Option<&str>, session_id: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "echo");

    if let Some(origin) = origin_header {
        builder = builder.header("Origin", origin);
    }
    if let Some(sid) = session_id {
        builder = builder.header("Mcp-Session-Id", sid);
    }

    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "echo",
            "arguments": {}
        }
    });

    builder.body(Body::from(payload.to_string())).unwrap()
}

/// Tests that requests with a trusted `Origin` matching `allowed_origins` succeed with `200 OK`.
#[tokio::test]
async fn test_origin_allowed_trusted_origin_returns_ok() {
    let app = McpRouter::new(common::sample_server_info())
        .allowed_origins(vec!["http://localhost:3000".to_string()])
        .register_tool("echo", echo_tool);

    let req = build_origin_request(Some("http://localhost:3000"), None);
    let (status, _, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"]["content"][0]["text"], "hello world");
}

/// Tests that origin comparison is case-insensitive and ignores trailing slashes.
#[tokio::test]
async fn test_origin_allowed_case_insensitive_and_trailing_slashes() {
    let app = McpRouter::new(common::sample_server_info())
        .allowed_origins(["https://App.Example.COM"])
        .register_tool("echo", echo_tool);

    let req = build_origin_request(Some("https://app.example.com/"), None);
    let (status, _, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"]["content"][0]["text"], "hello world");
}

/// Tests that requests with an untrusted `Origin` header are rejected with `403 Forbidden`.
#[tokio::test]
async fn test_origin_untrusted_returns_403_forbidden() {
    let app = McpRouter::new(common::sample_server_info())
        .allowed_origins(vec!["http://localhost:3000".to_string()])
        .register_tool("echo", echo_tool);

    let req = build_origin_request(Some("http://malicious-attacker.com"), None);
    let (status, _, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(body.is_null());
}

/// Tests that wildcard `"*"` in `allowed_origins` permits requests from any origin.
#[tokio::test]
async fn test_origin_wildcard_allows_any_origin() {
    let app = McpRouter::new(common::sample_server_info())
        .allowed_origins(vec!["*".to_string()])
        .register_tool("echo", echo_tool);

    let req = build_origin_request(Some("http://any-domain.org"), None);
    let (status, _, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"]["content"][0]["text"], "hello world");
}

/// Tests that requests without an `Origin` header (non-browser clients) are permitted when `allowed_origins` is set.
#[tokio::test]
async fn test_origin_missing_when_allowed_origins_configured_allows_non_browser() {
    let app = McpRouter::new(common::sample_server_info())
        .allowed_origins(vec!["http://localhost:3000".to_string()])
        .register_tool("echo", echo_tool);

    let req = build_origin_request(None, None);
    let (status, _, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"]["content"][0]["text"], "hello world");
}

/// Tests that when `allowed_origins` is not configured, any request passes origin validation.
#[tokio::test]
async fn test_origin_not_configured_allows_any_origin() {
    let app = McpRouter::new(common::sample_server_info()).register_tool("echo", echo_tool);

    let req = build_origin_request(Some("http://some-origin.com"), None);
    let (status, _, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"]["content"][0]["text"], "hello world");
}

/// Tests that blank or whitespace-only `Origin` headers are rejected with `403 Forbidden` when `allowed_origins` is set.
#[tokio::test]
async fn test_origin_blank_returns_403_forbidden_when_configured() {
    let app = McpRouter::new(common::sample_server_info())
        .allowed_origins(vec!["http://localhost:3000".to_string()])
        .register_tool("echo", echo_tool);

    let req = build_origin_request(Some("   "), None);
    let (status, _, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(body.is_null());
}

/// Tests that the `Mcp-Session-Id` header is preserved on `403 Forbidden` responses.
#[tokio::test]
async fn test_origin_session_id_header_propagated_on_403() {
    let app = McpRouter::new(common::sample_server_info())
        .allowed_origins(vec!["http://localhost:3000".to_string()])
        .register_tool("echo", echo_tool);

    let req = build_origin_request(Some("http://malicious.com"), Some("session-xyz-123"));
    let (status, headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(body.is_null());
    assert_eq!(
        headers.get("mcp-session-id").and_then(|h| h.to_str().ok()),
        Some("session-xyz-123")
    );
}
