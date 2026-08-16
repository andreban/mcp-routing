// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Subscriptions Listen Integration Tests
//!
//! Tests for the MCP `2026-07-28` stateless `subscriptions/listen` notification stream (SEP-2575).

mod common;

use axum::body::Body;
use http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::json;
use tower::ServiceExt;

use mcp_routing::{
    McpRouter,
    extract::{BearerAuth, State},
    types::mcp::{NotificationSubscriptions, resources::Resource},
};

use common::sample_server_info;

/// Tests basic `subscriptions/listen` request returning an SSE stream with acknowledgment.
#[tokio::test]
async fn test_subscriptions_listen_basic_acknowledgment() {
    let server_info = sample_server_info();
    let app = McpRouter::new(server_info);

    let req_body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "subscriptions/listen",
        "params": {
            "notifications": {
                "toolsListChanged": true,
                "promptsListChanged": true,
                "resourcesListChanged": true
            }
        }
    });

    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header(header::CONTENT_TYPE, "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "subscriptions/listen")
        .body(Body::from(req_body.to_string()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get(header::CONTENT_TYPE).unwrap(),
        "text/event-stream"
    );
    assert_eq!(
        resp.headers().get(header::CACHE_CONTROL).unwrap(),
        "no-cache"
    );

    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body_str = std::str::from_utf8(&body_bytes).unwrap();

    assert!(body_str.starts_with("event: message\ndata: "));
    assert!(body_str.contains("\"method\":\"notifications/subscriptions/acknowledged\""));
    assert!(body_str.contains("\"io.modelcontextprotocol/subscriptionId\""));
    assert!(body_str.contains("\"toolsListChanged\":true"));
}

/// Tests `subscriptions/listen` with `resourceSubscriptions` filtering known vs unknown resources.
#[tokio::test]
async fn test_subscriptions_listen_with_resource_subscriptions() {
    let server_info = sample_server_info();
    let resource = Resource::new("file:///logs/app.log", "App Logs");

    let app = McpRouter::new(server_info)
        .register_resource(resource, || async { Ok::<String, String>("log content".to_string()) });

    let req_body = json!({
        "jsonrpc": "2.0",
        "id": "sub-req-2",
        "method": "subscriptions/listen",
        "params": {
            "notifications": {
                "resourceSubscriptions": [
                    "file:///logs/app.log",
                    "file:///unknown/missing.log"
                ]
            }
        }
    });

    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header(header::CONTENT_TYPE, "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "subscriptions/listen")
        .body(Body::from(req_body.to_string()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body_str = std::str::from_utf8(&body_bytes).unwrap();

    assert!(body_str.contains("file:///logs/app.log"));
    assert!(!body_str.contains("file:///unknown/missing.log"));
}

/// Tests `subscriptions/listen` preserving client-provided `subscriptionId` in `_meta`.
#[tokio::test]
async fn test_subscriptions_listen_preserves_client_subscription_id() {
    let server_info = sample_server_info();
    let app = McpRouter::new(server_info);

    let req_body = json!({
        "jsonrpc": "2.0",
        "id": 42,
        "method": "subscriptions/listen",
        "params": {
            "_meta": {
                "io.modelcontextprotocol/subscriptionId": "custom-client-sub-123"
            },
            "notifications": {
                "toolsListChanged": true
            }
        }
    });

    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header(header::CONTENT_TYPE, "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "subscriptions/listen")
        .body(Body::from(req_body.to_string()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body_str = std::str::from_utf8(&body_bytes).unwrap();

    assert!(body_str.contains("custom-client-sub-123"));
}

/// Tests custom `subscriptions_listen` handler with `State` and `BearerAuth` extractors.
#[tokio::test]
async fn test_subscriptions_listen_custom_handler_with_extractors() {
    #[derive(Clone)]
    struct AuthConfig {
        required_token: String,
    }

    let server_info = sample_server_info();
    let app = McpRouter::new(server_info)
        .with_state(AuthConfig {
            required_token: "secret-token-999".to_string(),
        })
        .subscriptions_listen(
            |auth: BearerAuth, state: State<AuthConfig>| async move {
                if auth.token() == state.0.required_token {
                    Ok(NotificationSubscriptions::new().with_tools_list_changed(true))
                } else {
                    Err("Unauthorized subscription".to_string())
                }
            },
        );

    // Authorized request
    let req_body = json!({
        "jsonrpc": "2.0",
        "id": 10,
        "method": "subscriptions/listen",
        "params": {
            "notifications": {
                "toolsListChanged": true
            }
        }
    });

    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header(header::CONTENT_TYPE, "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "subscriptions/listen")
        .header(header::AUTHORIZATION, "Bearer secret-token-999")
        .body(Body::from(req_body.to_string()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get(header::CONTENT_TYPE).unwrap(),
        "text/event-stream"
    );

    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body_str = std::str::from_utf8(&body_bytes).unwrap();
    assert!(body_str.contains("\"toolsListChanged\":true"));
}

/// Tests `subscriptions/listen` with invalid parameter types returning an error.
#[tokio::test]
async fn test_subscriptions_listen_invalid_params() {
    let server_info = sample_server_info();
    let app = McpRouter::new(server_info);

    let req_body = json!({
        "jsonrpc": "2.0",
        "id": 99,
        "method": "subscriptions/listen",
        "params": "invalid-string-params"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header(header::CONTENT_TYPE, "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "subscriptions/listen")
        .body(Body::from(req_body.to_string()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body_json["error"]["code"], -32602);
}

/// Tests `subscriptions/listen` notification (omitted `id`) returns HTTP 202 Accepted.
#[tokio::test]
async fn test_subscriptions_listen_notification() {
    let server_info = sample_server_info();
    let app = McpRouter::new(server_info);

    let req_body = json!({
        "jsonrpc": "2.0",
        "method": "subscriptions/listen",
        "params": {
            "notifications": {
                "toolsListChanged": true
            }
        }
    });

    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header(header::CONTENT_TYPE, "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "subscriptions/listen")
        .body(Body::from(req_body.to_string()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    assert!(body_bytes.is_empty());
}
