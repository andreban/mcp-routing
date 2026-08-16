// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Resource Content Retrieval (`resources/read`) Integration Tests
//!
//! Verifies the behavior of the Model Context Protocol (MCP) `resources/read` endpoint, including:
//! - Exact URI matching for registered resource reading
//! - Header-based routing via `Mcp-Method: resources/read` and `Mcp-Uri`
//! - Fallback support for `Mcp-Name` header when `Mcp-Uri` is omitted
//! - Rejection on header/body URI mismatch (`HEADER_MISMATCH`)
//! - Text vs binary blob content serialization and MIME type reporting
//! - Request extractor integration (`BearerAuth`, `State`, etc.) in resource read handlers
//! - Per-resource HTTP caching directives (`ttlMs`, `cacheScope`, `Cache-Control`)
//! - Error handling for missing URIs, unregistered resources, and internal handler failures

use http::Request;
use http_body_util::BodyExt;
use mcp_routing::{
    BearerAuth, McpRouter, State,
    types::mcp::{
        CacheScope, Implementation,
        resources::{ReadResourceResult, Resource},
    },
};
use tower::Service;

fn create_test_server() -> Implementation {
    Implementation::new("test-resource-read-server", "1.0.0")
        .with_title("Test Resource Read Server")
}

/// Tests reading a resource by exact URI matching.
#[tokio::test]
async fn test_resources_read_exact_match_success() {
    let res = Resource::new("file:///project/README.md", "README");
    let mut router =
        McpRouter::new(create_test_server()).register_resource(res, |uri: String| async move {
            format!("Content of {uri}")
        });

    let req_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "resources/read",
        "params": {
            "uri": "file:///project/README.md"
        }
    });

    let request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "resources/read")
        .header("Mcp-Uri", "file:///project/README.md")
        .body(req_body.to_string())
        .unwrap();

    let response = router.call(request).await.unwrap();
    assert_eq!(response.status(), 200);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(resp_json["jsonrpc"], "2.0");
    assert_eq!(resp_json["id"], 1.0);
    assert_eq!(resp_json["result"]["ttlMs"], 0);
    assert_eq!(resp_json["result"]["cacheScope"], "public");
    let contents = resp_json["result"]["contents"].as_array().unwrap();
    assert_eq!(contents.len(), 1);
    assert_eq!(contents[0]["uri"], "file:///project/README.md");
    assert_eq!(contents[0]["text"], "Content of file:///project/README.md");
}

/// Tests routing `resources/read` using `Mcp-Uri` HTTP header.
#[tokio::test]
async fn test_resources_read_header_routing_with_uri() {
    let res = Resource::new("memo://meeting-notes", "Meeting Notes");
    let mut router = McpRouter::new(create_test_server())
        .register_resource(res, || async { "Notes from 2026-08-15" });

    let request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "resources/read")
        .header("Mcp-Uri", "memo://meeting-notes")
        .body(
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 42,
                "method": "resources/read",
                "params": {
                    "uri": "memo://meeting-notes"
                }
            })
            .to_string(),
        )
        .unwrap();

    let response = router.call(request).await.unwrap();
    assert_eq!(response.status(), 200);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(resp_json["id"], 42.0);
    assert_eq!(
        resp_json["result"]["contents"][0]["text"],
        "Notes from 2026-08-15"
    );
}

/// Tests routing `resources/read` using fallback `Mcp-Name` header when `Mcp-Uri` is omitted.
#[tokio::test]
async fn test_resources_read_header_routing_with_name_header_fallback() {
    let res = Resource::new("memo://system-status", "System Status");
    let mut router = McpRouter::new(create_test_server())
        .register_resource(res, || async { "All systems nominal" });

    let request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "resources/read")
        .header("Mcp-Name", "memo://system-status")
        .body(
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 10,
                "method": "resources/read",
                "params": {
                    "uri": "memo://system-status"
                }
            })
            .to_string(),
        )
        .unwrap();

    let response = router.call(request).await.unwrap();
    assert_eq!(response.status(), 200);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(resp_json["id"], 10.0);
    assert_eq!(
        resp_json["result"]["contents"][0]["text"],
        "All systems nominal"
    );
}

/// Tests that a mismatch between `Mcp-Uri` header and body `uri` returns HTTP 400 with `HEADER_MISMATCH`.
#[tokio::test]
async fn test_resources_read_header_body_mismatch_returns_header_mismatch() {
    let res1 = Resource::new("memo://primary", "Primary");
    let res2 = Resource::new("memo://secondary", "Secondary");

    let mut router = McpRouter::new(create_test_server())
        .register_resource(res1, || async { "primary content" })
        .register_resource(res2, || async { "secondary content" });

    let request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "resources/read")
        .header("Mcp-Uri", "memo://primary")
        .body(
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "resources/read",
                "params": {
                    "uri": "memo://secondary"
                }
            })
            .to_string(),
        )
        .unwrap();

    let response = router.call(request).await.unwrap();
    assert_eq!(response.status(), 400);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(
        resp_json["error"]["code"],
        mcp_routing::types::mcp::HEADER_MISMATCH
    );
}

/// Tests reading binary blob resource content and MIME type handling.
#[tokio::test]
async fn test_resources_read_blob_content() {
    let res = Resource::new("file:///data/binary.dat", "Binary Data")
        .mime_type("application/octet-stream");

    let mut router =
        McpRouter::new(create_test_server()).register_resource(res, |uri: String| async move {
            Ok::<_, String>(ReadResourceResult::blob(
                uri,
                "aGVsbG8gd29ybGQ=", // "hello world" in base64
                Some("application/octet-stream"),
            ))
        });

    let request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "resources/read")
        .header("Mcp-Uri", "file:///data/binary.dat")
        .body(
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "resources/read",
                "params": {
                    "uri": "file:///data/binary.dat"
                }
            })
            .to_string(),
        )
        .unwrap();

    let response = router.call(request).await.unwrap();
    assert_eq!(response.status(), 200);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(resp_json["id"], 1.0);
    assert_eq!(
        resp_json["result"]["contents"][0]["blob"],
        "aGVsbG8gd29ybGQ="
    );
    assert_eq!(
        resp_json["result"]["contents"][0]["mimeType"],
        "application/octet-stream"
    );
}

/// Tests resource reading with request extractors (`BearerAuth`, `State`).
#[tokio::test]
async fn test_resources_read_with_extractors() {
    #[derive(Clone)]
    struct AppEnv {
        region: String,
    }

    let res = Resource::new("config://app", "App Config");
    let mut router = McpRouter::new(create_test_server())
        .with_state(AppEnv {
            region: "us-east-1".to_string(),
        })
        .register_resource(
            res,
            |auth: BearerAuth,
             State(env): State<AppEnv>,
             uri: String| async move {
                Ok::<_, String>(format!(
                    "[{}][{}] Content for {uri}",
                    auth.token(),
                    env.region
                ))
            },
        );

    let request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "resources/read")
        .header("Mcp-Uri", "config://app")
        .header("Authorization", "Bearer my-secret-token")
        .body(
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "resources/read",
                "params": {
                    "uri": "config://app"
                }
            })
            .to_string(),
        )
        .unwrap();

    let response = router.call(request).await.unwrap();
    assert_eq!(response.status(), 200);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(resp_json["id"], 1.0);
    assert_eq!(
        resp_json["result"]["contents"][0]["text"],
        "[my-secret-token][us-east-1] Content for config://app"
    );
}

/// Tests resource reading with per-resource caching directives.
#[tokio::test]
async fn test_resources_read_with_caching_directives() {
    let res = Resource::new("file:///cacheable/data.json", "Cacheable Data");
    let mut router = McpRouter::new(create_test_server()).register_resource_with_cache(
        res,
        || async { "cacheable content" },
        Some(300_000),
        Some(CacheScope::Public),
    );

    let request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "resources/read")
        .header("Mcp-Uri", "file:///cacheable/data.json")
        .body(
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "resources/read",
                "params": {
                    "uri": "file:///cacheable/data.json"
                }
            })
            .to_string(),
        )
        .unwrap();

    let response = router.call(request).await.unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(
        response
            .headers()
            .get("Cache-Control")
            .unwrap()
            .to_str()
            .unwrap(),
        "public, max-age=300"
    );

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(resp_json["result"]["ttlMs"], 300_000);
    assert_eq!(resp_json["result"]["cacheScope"], "public");
}

/// Tests that providing an empty URI returns `-32602` Invalid Params.
#[tokio::test]
async fn test_resources_read_missing_uri_returns_invalid_params() {
    let mut router = McpRouter::new(create_test_server());

    let request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "resources/read")
        .header("Mcp-Uri", "")
        .body(
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "resources/read",
                "params": {
                    "uri": ""
                }
            })
            .to_string(),
        )
        .unwrap();

    let response = router.call(request).await.unwrap();
    assert_eq!(response.status(), 200);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(resp_json["id"], 1.0);
    assert_eq!(resp_json["error"]["code"], -32602);
    assert!(
        resp_json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("empty resource uri")
    );
}

/// Tests that attempting to read an unregistered resource URI returns `-32602` Invalid Params.
#[tokio::test]
async fn test_resources_read_unknown_resource_returns_invalid_params() {
    let mut router = McpRouter::new(create_test_server());

    let request = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "resources/read")
        .header("Mcp-Uri", "file:///non_existent.txt")
        .body(
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "resources/read",
                "params": {
                    "uri": "file:///non_existent.txt"
                }
            })
            .to_string(),
        )
        .unwrap();

    let response = router.call(request).await.unwrap();
    assert_eq!(response.status(), 200);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(resp_json["id"], 1.0);
    assert_eq!(resp_json["error"]["code"], -32602);
    assert!(
        resp_json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("resource 'file:///non_existent.txt' not found")
    );
}

/// Tests that business logic failures inside resource read handlers propagate as `-32603` Internal Error.
#[tokio::test]
async fn test_resources_read_business_logic_error_returns_internal_error() {
    let res = Resource::new("file:///error.txt", "Error Resource");
    let mut router = McpRouter::new(create_test_server()).register_resource(res, || async {
        Err::<String, String>("Disk I/O failure".to_string())
    });

    let request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "resources/read")
        .header("Mcp-Uri", "file:///error.txt")
        .body(
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "resources/read",
                "params": {
                    "uri": "file:///error.txt"
                }
            })
            .to_string(),
        )
        .unwrap();

    let response = router.call(request).await.unwrap();
    assert_eq!(response.status(), 200);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(resp_json["id"], 1.0);
    assert_eq!(resp_json["error"]["code"], -32603);
    assert!(
        resp_json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Disk I/O failure")
    );
}
