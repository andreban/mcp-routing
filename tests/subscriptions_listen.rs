// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Subscriptions Listen Integration Tests
//!
//! Tests for the MCP `2026-07-28` stateless `subscriptions/listen` notification stream (SEP-2575):
//! - The acknowledgment and closure carry the listen request's JSON-RPC ID as `subscriptionId`
//! - Only notification types backed by a declared capability are acknowledged
//! - The stream ends with a graceful `resultType: "complete"` response
//! - Handlers receive the subscription ID and acknowledged filter via the `Subscription` extractor

mod common;

use axum::body::Body;
use http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::json;
use tower::ServiceExt;

use stateless_mcp::{
    McpRouter,
    ResponseBody,
    extract::{BearerAuth, State, Subscription},
    format_sse_message,
    types::mcp::{
        NotificationSubscriptions, ServerCapabilities,
        resources::Resource,
        subscriptions::{ListChangedParams, tools_list_changed_notification},
    },
};

use common::sample_server_info;

/// Sends a `subscriptions/listen` request and returns the JSON messages from the SSE stream.
async fn listen(app: McpRouter, req_body: serde_json::Value) -> Vec<serde_json::Value> {
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
    std::str::from_utf8(&body_bytes)
        .unwrap()
        .split("\n\n")
        .filter_map(|frame| frame.lines().find_map(|l| l.strip_prefix("data: ")))
        .map(|data| serde_json::from_str(data).unwrap())
        .collect()
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
        .subscriptions_listen(|auth: BearerAuth, state: State<AuthConfig>| async move {
            if auth.token() == state.0.required_token {
                Ok(NotificationSubscriptions::new().with_tools_list_changed(true))
            } else {
                Err("Unauthorized subscription".to_string())
            }
        });

    // Authorized request
    let req_body = json!({
        "jsonrpc": "2.0",
        "id": 10,
        "method": "subscriptions/listen",
        "params": {
            "_meta": common::meta(),
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

/// Tests that `subscriptions/listen` sent without an `id` is rejected with `-32600` and HTTP 400.
#[tokio::test]
async fn test_subscriptions_listen_without_id_is_rejected() {
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
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["error"]["code"], -32600);
}

/// Tests the acknowledgment and graceful closure of a `subscriptions/listen` stream.
///
/// Verifies:
/// - The first message is `notifications/subscriptions/acknowledged`
/// - Only `toolsListChanged` is acknowledged when the server declares `tools.listChanged` alone
/// - The acknowledgment and the closing response carry the request ID as `subscriptionId`
/// - The stream ends with a `resultType: "complete"` response correlated by the request ID
#[tokio::test]
async fn test_subscriptions_listen_acknowledgment_and_closure() {
    let app = McpRouter::new(sample_server_info())
        .capabilities(ServerCapabilities::empty().with_tools(Some(true)));

    let messages = listen(
        app,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "subscriptions/listen",
            "params": {
                "_meta": common::meta(),
                "notifications": {
                    "toolsListChanged": true,
                    "promptsListChanged": true,
                    "resourcesListChanged": true
                }
            }
        }),
    )
    .await;

    assert_eq!(messages.len(), 2);

    let ack = &messages[0];
    assert_eq!(ack["method"], "notifications/subscriptions/acknowledged");
    assert_eq!(ack["params"]["_meta"]["io.modelcontextprotocol/subscriptionId"], 1);
    assert_eq!(
        ack["params"]["notifications"],
        json!({ "toolsListChanged": true })
    );

    let closure = &messages[1];
    assert_eq!(closure["id"], 1);
    assert_eq!(closure["result"]["resultType"], "complete");
    assert_eq!(
        closure["result"]["_meta"]["io.modelcontextprotocol/subscriptionId"],
        1
    );
}

/// Tests that nothing is acknowledged when the server declares no `listChanged` or `subscribe` capability.
#[tokio::test]
async fn test_subscriptions_listen_without_capabilities_acknowledges_nothing() {
    let app = McpRouter::new(sample_server_info()).register_resource(
        Resource::new("file:///logs/app.log", "App Logs"),
        || async { "log content" },
    );

    let messages = listen(
        app,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "subscriptions/listen",
            "params": {
                "_meta": common::meta(),
                "notifications": {
                    "toolsListChanged": true,
                    "promptsListChanged": true,
                    "resourcesListChanged": true,
                    "resourceSubscriptions": ["file:///logs/app.log"]
                }
            }
        }),
    )
    .await;

    assert_eq!(messages[0]["params"]["notifications"], json!({}));
}

/// Tests `resourceSubscriptions` filtering known vs unknown resources when `resources.subscribe` is declared.
#[tokio::test]
async fn test_subscriptions_listen_with_resource_subscriptions() {
    let app = McpRouter::new(sample_server_info())
        .capabilities(ServerCapabilities::empty().with_resources(Some(true), None))
        .register_resource(
            Resource::new("file:///logs/app.log", "App Logs"),
            || async { "log content" },
        );

    let messages = listen(
        app,
        json!({
            "jsonrpc": "2.0",
            "id": "sub-req-2",
            "method": "subscriptions/listen",
            "params": {
                "_meta": common::meta(),
                "notifications": {
                    "resourceSubscriptions": [
                        "file:///logs/app.log",
                        "file:///unknown/missing.log"
                    ]
                }
            }
        }),
    )
    .await;

    let ack = &messages[0];
    assert_eq!(
        ack["params"]["_meta"]["io.modelcontextprotocol/subscriptionId"],
        "sub-req-2"
    );
    assert_eq!(
        ack["params"]["notifications"],
        json!({ "resourceSubscriptions": ["file:///logs/app.log"] })
    );
}

/// Tests that a client-supplied `subscriptionId` in request `_meta` is ignored in favor of the request ID.
#[tokio::test]
async fn test_subscriptions_listen_uses_request_id_as_subscription_id() {
    let app = McpRouter::new(sample_server_info());

    let messages = listen(
        app,
        json!({
            "jsonrpc": "2.0",
            "id": 42,
            "method": "subscriptions/listen",
            "params": {
                "_meta": {
                    "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                    "io.modelcontextprotocol/clientCapabilities": {},
                    "io.modelcontextprotocol/subscriptionId": "custom-client-sub-123"
                },
                "notifications": { "toolsListChanged": true }
            }
        }),
    )
    .await;

    assert_eq!(
        messages[0]["params"]["_meta"]["io.modelcontextprotocol/subscriptionId"],
        42
    );
}

/// Tests that a listen handler receives the subscription ID and acknowledged filter via [`Subscription`].
///
/// Verifies:
/// - `Subscription::notifications` reflects the acknowledged (capability-filtered) types
/// - Notifications tagged with `Subscription::meta()` carry the request ID as `subscriptionId`
/// - The handler's stream is followed by the graceful closure response
#[tokio::test]
async fn test_subscriptions_listen_handler_receives_subscription() {
    let app = McpRouter::new(sample_server_info())
        .capabilities(ServerCapabilities::empty().with_tools(Some(true)))
        .subscriptions_listen(|subscription: Subscription| async move {
            assert_eq!(subscription.notifications.tools_list_changed, Some(true));
            assert_eq!(subscription.notifications.prompts_list_changed, None);
            let notif = tools_list_changed_notification(Some(
                ListChangedParams::new().with_meta(subscription.meta()),
            ));
            ResponseBody::from_bytes(format_sse_message(&notif).unwrap())
        });

    let messages = listen(
        app,
        json!({
            "jsonrpc": "2.0",
            "id": "listen-9",
            "method": "subscriptions/listen",
            "params": {
                "_meta": common::meta(),
                "notifications": { "toolsListChanged": true, "promptsListChanged": true }
            }
        }),
    )
    .await;

    assert_eq!(messages.len(), 3);
    assert_eq!(messages[1]["method"], "notifications/tools/list_changed");
    assert_eq!(
        messages[1]["params"]["_meta"]["io.modelcontextprotocol/subscriptionId"],
        "listen-9"
    );
    assert_eq!(messages[2]["id"], "listen-9");
    assert_eq!(messages[2]["result"]["resultType"], "complete");
}
