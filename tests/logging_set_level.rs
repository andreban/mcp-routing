// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::body::Body;
use http::{HeaderMap, Request, StatusCode};
use http_body_util::BodyExt;
use mcp_routing::{
    CurrentLoggingLevel, McpRouter,
    extract::{BearerAuth, SessionId, State},
    logging::{LoggingError, SetLevelParams, SetLevelResult},
    types::mcp::{
        CacheScope, Implementation, LoggingLevel, server::discover::ServerDiscoverResultResponse,
    },
};
use serde_json::json;
use tower::Service;

fn create_base_router() -> McpRouter {
    let server_info = Implementation::new("logging-test-server", "1.0.0");
    McpRouter::new(server_info)
}

async fn send_mcp_request(
    router: &mut McpRouter,
    body: serde_json::Value,
    headers: Option<Vec<(&'static str, &'static str)>>,
) -> (StatusCode, serde_json::Value, HeaderMap) {
    let body_str = serde_json::to_string(&body).unwrap();
    let mut req_builder = Request::post("/")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28");

    let mut has_mcp_method = false;
    let mut has_mcp_name = false;
    let mut has_mcp_uri = false;
    if let Some(ref hdrs) = headers {
        for (k, _) in hdrs {
            if k.eq_ignore_ascii_case("Mcp-Method") {
                has_mcp_method = true;
            }
            if k.eq_ignore_ascii_case("Mcp-Name") {
                has_mcp_name = true;
            }
            if k.eq_ignore_ascii_case("Mcp-Uri") {
                has_mcp_uri = true;
            }
        }
    }

    if !has_mcp_method && let Some(m) = body.get("method").and_then(|v| v.as_str()) {
        req_builder = req_builder.header("Mcp-Method", m);
    }

    if !has_mcp_name
        && let Some(name) = body
            .get("params")
            .and_then(|p| p.get("name"))
            .and_then(|v| v.as_str())
    {
        req_builder = req_builder.header("Mcp-Name", name);
    }

    if !has_mcp_uri
        && let Some(uri) = body
            .get("params")
            .and_then(|p| p.get("uri"))
            .and_then(|v| v.as_str())
    {
        req_builder = req_builder.header("Mcp-Uri", uri);
    }

    if let Some(hdrs) = headers {
        for (k, v) in hdrs {
            req_builder = req_builder.header(k, v);
        }
    }

    let req = req_builder.body(Body::from(body_str)).unwrap();
    let response = router.call(req).await.unwrap();
    let status = response.status();
    let resp_headers = response.headers().clone();
    let resp_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = if resp_bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&resp_bytes).unwrap_or(serde_json::Value::Null)
    };
    (status, json, resp_headers)
}

/// Tests that configuring logging level advertises the `logging` capability in `server/discover`.
#[tokio::test]
async fn test_logging_capability_advertisement_in_discover() {
    let mut router = create_base_router().logging_level(LoggingLevel::Debug);

    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "server/discover"
    });

    let (status, body, _) = send_mcp_request(&mut router, payload, None).await;
    assert_eq!(status, StatusCode::OK);

    let discover_resp: ServerDiscoverResultResponse = serde_json::from_value(body).unwrap();
    assert!(discover_resp.result.capabilities.logging.is_some());
    assert_eq!(router.current_logging_level(), LoggingLevel::Debug);
}

/// Tests `logging/setLevel` endpoint using the default handler dispatched via `Mcp-Method` header.
#[tokio::test]
async fn test_logging_set_level_default_handler_via_header() {
    let mut router = create_base_router().logging_level(LoggingLevel::Info);
    assert_eq!(router.current_logging_level(), LoggingLevel::Info);

    let payload = json!({
        "jsonrpc": "2.0",
        "id": "log-1",
        "params": {
            "level": "debug"
        }
    });

    let (status, body, _) = send_mcp_request(
        &mut router,
        payload,
        Some(vec![("Mcp-Method", "logging/setLevel")]),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], "log-1");
    assert_eq!(body["result"], json!({}));
    assert_eq!(router.current_logging_level(), LoggingLevel::Debug);
}

/// Tests `logging/setLevel` endpoint dispatched via JSON-RPC body method fallback.
#[tokio::test]
async fn test_logging_set_level_default_handler_via_body_fallback() {
    let mut router = create_base_router().logging_level(LoggingLevel::Info);

    let payload = json!({
        "jsonrpc": "2.0",
        "id": 42,
        "method": "logging/setLevel",
        "params": {
            "level": "error"
        }
    });

    let (status, body, _) = send_mcp_request(&mut router, payload, None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], 42.0);
    assert_eq!(body["result"], json!({}));
    assert_eq!(router.current_logging_level(), LoggingLevel::Error);
}

/// Tests `logging/setLevel` sent as a JSON-RPC notification (no `id`).
#[tokio::test]
async fn test_logging_set_level_notification() {
    let mut router = create_base_router().logging_level(LoggingLevel::Info);

    let payload = json!({
        "jsonrpc": "2.0",
        "method": "logging/setLevel",
        "params": {
            "level": "warning"
        }
    });

    let (status, body, _) = send_mcp_request(&mut router, payload, None).await;

    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(body, serde_json::Value::Null);
    assert_eq!(router.current_logging_level(), LoggingLevel::Warning);
}

/// Tests `logging/setLevel` with a custom async handler receiving typed parameters and extractors.
#[tokio::test]
async fn test_logging_set_level_custom_handler_with_extractors() {
    let handler_called = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&handler_called);

    #[derive(Clone)]
    struct AppState {
        env: String,
    }

    let mut router = create_base_router()
        .with_state(AppState {
            env: "production".to_string(),
        })
        .logging_handler(
            move |SessionId(sid): SessionId,
                  bearer: BearerAuth,
                  State(state): State<AppState>,
                  params: SetLevelParams| {
                let flag = Arc::clone(&flag);
                async move {
                    assert_eq!(sid, "sess-admin-1");
                    assert_eq!(bearer.token(), "secret-admin-token");
                    assert_eq!(state.env, "production");
                    assert_eq!(params.level, LoggingLevel::Critical);
                    flag.store(true, Ordering::SeqCst);
                    Ok::<SetLevelResult, LoggingError>(SetLevelResult::default())
                }
            },
        );

    let payload = json!({
        "jsonrpc": "2.0",
        "id": "custom-1",
        "method": "logging/setLevel",
        "params": {
            "level": "critical"
        }
    });

    let (status, body, _) = send_mcp_request(
        &mut router,
        payload,
        Some(vec![
            ("Mcp-Session-Id", "sess-admin-1"),
            ("Authorization", "Bearer secret-admin-token"),
        ]),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], "custom-1");
    assert_eq!(body["result"], json!({}));
    assert!(handler_called.load(Ordering::SeqCst));
    assert_eq!(router.current_logging_level(), LoggingLevel::Critical);
}

/// Tests `logging/setLevel` custom handler error propagation.
#[tokio::test]
async fn test_logging_set_level_custom_handler_errors() {
    let mut router_invalid_params =
        create_base_router().logging_handler(|params: SetLevelParams| async move {
            if params.level == LoggingLevel::Debug {
                Err(LoggingError::InvalidParams(
                    "Debug logging not allowed in production".to_string(),
                ))
            } else {
                Ok(())
            }
        });

    let payload_invalid = json!({
        "jsonrpc": "2.0",
        "id": "err-1",
        "method": "logging/setLevel",
        "params": {
            "level": "debug"
        }
    });

    let (status, body, _) =
        send_mcp_request(&mut router_invalid_params, payload_invalid, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["error"]["code"], -32602);
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Debug logging not allowed in production")
    );

    let mut router_internal =
        create_base_router().logging_handler(|_params: SetLevelParams| async move {
            Err::<(), LoggingError>(LoggingError::Internal(
                "Database logger crashed".to_string(),
            ))
        });

    let payload_internal = json!({
        "jsonrpc": "2.0",
        "id": "err-2",
        "method": "logging/setLevel",
        "params": {
            "level": "warning"
        }
    });

    let (status, body, _) = send_mcp_request(&mut router_internal, payload_internal, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["error"]["code"], -32603);
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Database logger crashed")
    );
}

/// Tests `logging/setLevel` input validation errors (missing params, malformed level).
#[tokio::test]
async fn test_logging_set_level_validation_errors() {
    let mut router = create_base_router().logging_level(LoggingLevel::Info);

    // Missing params object
    let payload_missing = json!({
        "jsonrpc": "2.0",
        "id": "v-1",
        "method": "logging/setLevel"
    });
    let (status, body, _) = send_mcp_request(&mut router, payload_missing, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["error"]["code"], -32602);

    // Missing level field in params
    let payload_no_level = json!({
        "jsonrpc": "2.0",
        "id": "v-2",
        "method": "logging/setLevel",
        "params": {}
    });
    let (status, body, _) = send_mcp_request(&mut router, payload_no_level, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["error"]["code"], -32602);

    // Invalid level variant
    let payload_bad_level = json!({
        "jsonrpc": "2.0",
        "id": "v-3",
        "method": "logging/setLevel",
        "params": {
            "level": "verbose_ultra"
        }
    });
    let (status, body, _) = send_mcp_request(&mut router, payload_bad_level, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["error"]["code"], -32602);
}

/// Tests caching directives on `logging/setLevel` responses.
#[tokio::test]
async fn test_logging_set_level_caching_directives() {
    let mut router = create_base_router()
        .logging_level(LoggingLevel::Info)
        .logging_cache(Some(120_000), Some(CacheScope::Private));

    let payload = json!({
        "jsonrpc": "2.0",
        "id": "cache-1",
        "method": "logging/setLevel",
        "params": {
            "level": "notice"
        }
    });

    let (status, body, headers) = send_mcp_request(&mut router, payload, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], "cache-1");
    assert_eq!(
        headers.get("Cache-Control").unwrap().to_str().unwrap(),
        "private, max-age=120"
    );
    assert!(headers.contains_key("ETag"));
}

/// Tests per-request `_meta.io.modelcontextprotocol/logLevel` extraction and `CurrentLoggingLevel`.
#[tokio::test]
async fn test_per_request_log_level_and_current_logging_level_extractors() {
    use mcp_routing::types::mcp::tools::Tool;

    let echo_tool = Tool {
        icons: Vec::new(),
        name: "echo_log".to_string(),
        title: None,
        description: None,
        input_schema: json!({ "type": "object" }),
        output_schema: None,
        annotations: None,
        meta: None,
    };

    let mut router = create_base_router()
        .logging_level(LoggingLevel::Warning)
        .register_tool(
            echo_tool,
            |opt_level: Option<LoggingLevel>, current: CurrentLoggingLevel| async move {
                assert_eq!(current.level(), LoggingLevel::Warning);
                match opt_level {
                    Some(lvl) => format!("Request level: {lvl}, server level: {}", current.level()),
                    None => format!("No request level, server level: {}", current.level()),
                }
            },
        );

    // Request with _meta.logLevel = debug
    let payload_with_meta = json!({
        "jsonrpc": "2.0",
        "id": "tool-1",
        "method": "tools/call",
        "params": {
            "name": "echo_log",
            "arguments": {},
            "_meta": {
                "io.modelcontextprotocol/logLevel": "debug"
            }
        }
    });

    let (status, body, _) = send_mcp_request(&mut router, payload_with_meta, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["result"]["content"][0]["text"],
        "Request level: debug, server level: warning"
    );

    // Request without _meta.logLevel
    let payload_without_meta = json!({
        "jsonrpc": "2.0",
        "id": "tool-2",
        "method": "tools/call",
        "params": {
            "name": "echo_log",
            "arguments": {}
        }
    });

    let (status2, body2, _) = send_mcp_request(&mut router, payload_without_meta, None).await;
    assert_eq!(status2, StatusCode::OK);
    assert_eq!(
        body2["result"]["content"][0]["text"],
        "No request level, server level: warning"
    );
}

/// Tests `logging/setLevel` executed as part of a JSON-RPC batch request.
#[tokio::test]
async fn test_logging_set_level_in_jsonrpc_batch() {
    let mut router = create_base_router().logging_level(LoggingLevel::Info);

    let batch_payload = json!([
        {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "server/discover"
        },
        {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "logging/setLevel",
            "params": {
                "level": "emergency"
            }
        },
        {
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/list"
        }
    ]);

    let (status, body, _) = send_mcp_request(&mut router, batch_payload, None).await;
    assert_eq!(status, StatusCode::OK);

    let batch = body.as_array().expect("expected array response");
    assert_eq!(batch.len(), 3);

    assert_eq!(batch[0]["id"], 1.0);
    assert_eq!(batch[1]["id"], 2.0);
    assert_eq!(batch[1]["result"], json!({}));
    assert_eq!(batch[2]["id"], 3.0);

    assert_eq!(router.current_logging_level(), LoggingLevel::Emergency);
}
