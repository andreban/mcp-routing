// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for server logging capabilities, per-request log levels, and rejection of deprecated `logging/setLevel`.

mod common;

use axum::body::Body;
use http::{HeaderMap, Request, StatusCode};
use http_body_util::BodyExt;
use mcp_routing::{
    CurrentLoggingLevel, McpRouter,
    types::mcp::{
        Implementation, LoggingLevel, server::discover::ServerDiscoverResultResponse, tools::Tool,
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

/// Tests that calling deprecated `logging/setLevel` method via header is rejected with 404 Method Not Found.
#[tokio::test]
async fn test_logging_set_level_rejected_via_header() {
    let mut router = create_base_router().logging_level(LoggingLevel::Info);

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

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["id"], "log-1");
    assert_eq!(body["error"]["code"], -32601);
}

/// Tests that calling deprecated `logging/setLevel` method via body is rejected with 404 Method Not Found.
#[tokio::test]
async fn test_logging_set_level_rejected_via_body() {
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

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["id"], 42.0);
    assert_eq!(body["error"]["code"], -32601);
}

/// Tests per-request `_meta.io.modelcontextprotocol/logLevel` extraction and `CurrentLoggingLevel`.
#[tokio::test]
async fn test_per_request_log_level_and_current_logging_level_extractors() {
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
