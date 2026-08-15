// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Axum Framework & End-to-End TCP Integration Tests
//!
//! Verifies the integration between [`McpRouter`](mcp_routing::McpRouter) and web frameworks (such as [Axum](https://crates.io/crates/axum)):
//! - Mounting [`McpRouter`] as a nested service via `axum::Router::nest_service`
//! - Mounting multiple independent [`McpRouter`] instances on distinct sub-routes (e.g. `/mcp/v1` and `/mcp/v2`)
//! - Running a live HTTP server bound to a real local TCP socket (`127.0.0.1:0`) and executing HTTP/1.1 requests end-to-end

mod common;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use mcp_routing::{
    McpRouter,
    types::mcp::{
        Implementation,
        server::discover::ServerDiscoverResultResponse,
        tools::{call::CallToolResultResponse, list::ListToolsResultResponse},
    },
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower::ServiceExt;

#[derive(Serialize, Deserialize)]
struct GreetParams {
    name: String,
}

async fn handle_greet(params: GreetParams) -> Result<String, String> {
    Ok(format!("Hello, {}!", params.name))
}

/// Tests nesting an [`McpRouter`] service under an Axum route (`/api/mcp`) using `oneshot`.
///
/// Verifies:
/// - Nested discovery endpoint (`server/discover`)
/// - Nested tool catalog endpoint (`tools/list`)
/// - Nested tool execution endpoint (`tools/call`)
#[tokio::test]
async fn test_axum_nested_service_oneshot() {
    let mcp_router = McpRouter::new(Implementation::new("nested-server", "1.0.0"))
        .instructions("Nested in Axum")
        .register_tool("greet", handle_greet);

    let app = Router::new().nest_service("/api/mcp", mcp_router);

    // 1. Discover endpoint
    let discover_req = Request::builder()
        .method("POST")
        .uri("/api/mcp")
        .header("Mcp-Method", "server/discover")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(
            json!({
                "jsonrpc": "2.0",
                "id": "axum-disc",
                "method": "server/discover"
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(discover_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let disc_res: ServerDiscoverResultResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        disc_res.result.instructions.as_deref(),
        Some("Nested in Axum")
    );

    // 2. Tools list endpoint
    let list_req = Request::builder()
        .method("POST")
        .uri("/api/mcp")
        .header("Mcp-Method", "tools/list")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(
            json!({
                "jsonrpc": "2.0",
                "id": "axum-list",
                "method": "tools/list"
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(list_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let list_res: ListToolsResultResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(list_res.result.tools.len(), 1);
    assert_eq!(list_res.result.tools[0].name, "greet");

    // 3. Tools call endpoint
    let call_req = Request::builder()
        .method("POST")
        .uri("/api/mcp")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "greet")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(
            json!({
                "jsonrpc": "2.0",
                "id": "axum-call",
                "method": "tools/call",
                "params": {
                    "name": "greet",
                    "arguments": {
                        "name": "Rustacean"
                    }
                }
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(call_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let call_res: CallToolResultResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(call_res.result.is_error, Some(false));
    if let mcp_routing::types::mcp::ContentBlock::Text(ref t) = call_res.result.content[0] {
        assert_eq!(t.text, "Hello, Rustacean!");
    } else {
        panic!("Expected ContentBlock::Text");
    }
}

/// Tests nesting multiple distinct [`McpRouter`] instances in the same Axum application at different sub-paths.
///
/// Verifies:
/// - Independent router state and tool registries between `/mcp/v1` and `/mcp/v2`
#[tokio::test]
async fn test_axum_multiple_nested_mcp_routers() {
    let v1_router = McpRouter::new(Implementation::new("v1-server", "1.0.0"))
        .register_tool("v1_tool", || async { "from_v1" });

    let v2_router = McpRouter::new(Implementation::new("v2-server", "2.0.0"))
        .register_tool("v2_tool", || async { "from_v2" });

    let app = Router::new()
        .nest_service("/mcp/v1", v1_router)
        .nest_service("/mcp/v2", v2_router);

    // Call v1
    let req_v1 = Request::builder()
        .method("POST")
        .uri("/mcp/v1")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "v1_tool")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": { "name": "v1_tool" }
            })
            .to_string(),
        ))
        .unwrap();

    let resp_v1 = app.clone().oneshot(req_v1).await.unwrap();
    assert_eq!(resp_v1.status(), StatusCode::OK);
    let bytes_v1 = resp_v1.into_body().collect().await.unwrap().to_bytes();
    let res_v1: CallToolResultResponse = serde_json::from_slice(&bytes_v1).unwrap();
    if let mcp_routing::types::mcp::ContentBlock::Text(ref t) = res_v1.result.content[0] {
        assert_eq!(t.text, "from_v1");
    } else {
        panic!("Expected ContentBlock::Text");
    }

    // Call v2
    let req_v2 = Request::builder()
        .method("POST")
        .uri("/mcp/v2")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "v2_tool")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": { "name": "v2_tool" }
            })
            .to_string(),
        ))
        .unwrap();

    let resp_v2 = app.clone().oneshot(req_v2).await.unwrap();
    assert_eq!(resp_v2.status(), StatusCode::OK);
    let bytes_v2 = resp_v2.into_body().collect().await.unwrap().to_bytes();
    let res_v2: CallToolResultResponse = serde_json::from_slice(&bytes_v2).unwrap();
    if let mcp_routing::types::mcp::ContentBlock::Text(ref t) = res_v2.result.content[0] {
        assert_eq!(t.text, "from_v2");
    } else {
        panic!("Expected ContentBlock::Text");
    }
}

/// Tests end-to-end HTTP/1.1 communication over a real TCP socket with `axum::serve`.
///
/// Verifies:
/// - Binding an ephemeral port with `TcpListener` and serving an MCP router in background
/// - Live discovery request over TCP stream
/// - Live tool execution request over TCP stream
/// - Proper handling of unknown tools (`404 Not Found`) over TCP
#[tokio::test]
async fn test_axum_real_tcp_server_e2e() {
    let server_info = Implementation::new("real-tcp-server", "1.0.0");
    let mcp_router = McpRouter::new(server_info)
        .instructions("Real TCP Server Test")
        .register_tool("greet", handle_greet);

    let app = Router::new().nest_service("/mcp", mcp_router);

    // Bind to ephemeral port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    // Spawn server in background
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // 1. Test server/discover over real TCP
    let disc_body = json!({
        "jsonrpc": "2.0",
        "id": "tcp-disc-1",
        "method": "server/discover"
    })
    .to_string();

    let (status, headers, body) = common::send_raw_http_request(
        addr,
        "POST",
        "/mcp",
        &[
            ("Mcp-Method", "server/discover"),
            ("Content-Type", "application/json"),
            ("MCP-Protocol-Version", "2026-07-28"),
        ],
        &disc_body,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get("content-type").unwrap().to_str().unwrap(),
        "application/json"
    );
    let disc_res: ServerDiscoverResultResponse = serde_json::from_str(&body).unwrap();
    assert_eq!(disc_res.id, "tcp-disc-1".into());
    assert_eq!(
        disc_res.result.instructions.as_deref(),
        Some("Real TCP Server Test")
    );

    // 2. Test tools/call over real TCP
    let call_body = json!({
        "jsonrpc": "2.0",
        "id": 42,
        "method": "tools/call",
        "params": {
            "name": "greet",
            "arguments": {
                "name": "TCP Client"
            }
        }
    })
    .to_string();

    let (status, _headers, body) = common::send_raw_http_request(
        addr,
        "POST",
        "/mcp",
        &[
            ("Mcp-Method", "tools/call"),
            ("Mcp-Name", "greet"),
            ("Content-Type", "application/json"),
            ("MCP-Protocol-Version", "2026-07-28"),
        ],
        &call_body,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let call_res: CallToolResultResponse = serde_json::from_str(&body).unwrap();
    assert_eq!(call_res.id, 42.into());
    assert_eq!(call_res.result.is_error, Some(false));
    if let mcp_routing::types::mcp::ContentBlock::Text(ref t) = call_res.result.content[0] {
        assert_eq!(t.text, "Hello, TCP Client!");
    } else {
        panic!("Expected text block");
    }

    // 3. Test JSON-RPC Invalid Params error (-32602) for unknown tool over real TCP
    let (status, _headers, body) = common::send_raw_http_request(
        addr,
        "POST",
        "/mcp",
        &[
            ("Mcp-Method", "tools/call"),
            ("Mcp-Name", "unknown"),
            ("Content-Type", "application/json"),
            ("MCP-Protocol-Version", "2026-07-28"),
        ],
        &json!({
            "jsonrpc": "2.0",
            "id": 99,
            "method": "tools/call",
            "params": { "name": "unknown" }
        })
        .to_string(),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let err_res: mcp_routing::types::jsonrpc::JsonRpcErrorResponse =
        serde_json::from_str(&body).unwrap();
    assert_eq!(err_res.jsonrpc, "2.0");
    assert_eq!(err_res.id, Some(99.into()));
    assert_eq!(
        err_res.error.code.code(),
        mcp_routing::types::jsonrpc::INVALID_PARAMS_CODE
    );

    // 4. Test HTTP 405 Method Not Allowed on non-POST methods over real TCP
    let (status_get, headers_get, body_get) = common::send_raw_http_request(
        addr,
        "GET",
        "/mcp",
        &[("Content-Type", "application/json")],
        "",
    )
    .await;
    assert_eq!(status_get, StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(
        headers_get.get("allow").and_then(|h| h.to_str().ok()),
        Some("POST")
    );
    assert!(body_get.is_empty());

    // 5. Test HTTP 415 Unsupported Media Type on unsupported Content-Type over real TCP
    let (status_ct, _headers_ct, body_ct) = common::send_raw_http_request(
        addr,
        "POST",
        "/mcp",
        &[("Content-Type", "text/plain")],
        "hello",
    )
    .await;
    assert_eq!(status_ct, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert!(body_ct.is_empty());
}
