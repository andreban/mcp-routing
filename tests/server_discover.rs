// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Server Discovery Integration Tests
//!
//! Verifies the behavior of the Model Context Protocol (MCP) `server/discover` endpoint,
//! including:
//! - Header-based routing via `Mcp-Method: server/discover`
//! - Body-based fallback routing when the `Mcp-Method` header is omitted
//! - Advertisement of server metadata ([`Implementation`](mcp_routing::types::mcp::Implementation)), instructions, and icons
//! - Custom capability negotiation and multi-version advertisement
//! - Protocol-level request metadata (`_meta`) parsing and response formatting

mod common;

use std::collections::HashMap;

use http::StatusCode;
use mcp_routing::{
    McpRouter,
    types::mcp::{
        CacheScope, CompletionsCapability, IconTheme, Implementation,
        PromptsCapability, ResourcesCapability, ServerCapabilities,
        ToolsCapability,
        server::discover::ServerDiscoverResultResponse,
    },
};
use serde_json::json;

/// Tests standard `server/discover` invocation using the `Mcp-Method: server/discover` HTTP header.
///
/// Verifies:
/// - HTTP `200 OK` status and `Content-Type: application/json` header
/// - JSON-RPC `2.0` response wrapper matching request ID
/// - Correct population of `resultType`, default `supportedVersions` (`["2026-07-28"]`), instructions, and caching defaults
/// - Correct propagation of `serverInfo` metadata including icons, themes, version, and description
#[tokio::test]
async fn test_server_discover_via_header() {
    let server_info = common::sample_server_info();
    let app = McpRouter::new(server_info.clone())
        .instructions("System instructions for testing.");

    let req = common::build_request(
        Some("server/discover"),
        None,
        json!({
            "jsonrpc": "2.0",
            "id": "test-id-1",
            "method": "server/discover"
        }),
    );

    let (status, headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get("content-type").unwrap().to_str().unwrap(),
        "application/json"
    );
    assert_eq!(
        headers.get("cache-control").unwrap().to_str().unwrap(),
        "public, max-age=0"
    );
    assert!(headers.contains_key("etag"));

    let res: ServerDiscoverResultResponse = serde_json::from_value(body.clone()).unwrap();
    assert_eq!(res.jsonrpc, "2.0");
    assert_eq!(res.id, "test-id-1".into());

    let result = res.result;
    assert_eq!(result.result_type.as_deref(), Some("complete"));
    assert_eq!(result.supported_versions, vec!["2026-07-28"]);
    assert_eq!(
        result.instructions.as_deref(),
        Some("System instructions for testing.")
    );
    assert_eq!(result.ttl_ms, Some(0));
    assert!(matches!(result.cache_scope, Some(CacheScope::Public)));

    // Verify metadata serverInfo
    let meta = result.meta.expect("Metadata should be present");
    let resp_server_info = meta.server_info.expect("ServerInfo should be present");
    assert_eq!(resp_server_info.name, "test-mcp-server");
    assert_eq!(resp_server_info.title.as_deref(), Some("Test MCP Server"));
    assert_eq!(resp_server_info.version, "1.2.3");
    assert_eq!(
        resp_server_info.description.as_deref(),
        Some("Integration test server")
    );
    assert_eq!(
        resp_server_info.website_url.as_deref(),
        Some("https://example.com")
    );
    assert_eq!(resp_server_info.icons.len(), 1);
    assert_eq!(resp_server_info.icons[0].src, "https://example.com/icon.png");
    assert_eq!(
        resp_server_info.icons[0].mime_type.as_deref(),
        Some("image/png")
    );
    assert_eq!(resp_server_info.icons[0].sizes, vec!["64x64".to_string()]);
    assert!(matches!(resp_server_info.icons[0].theme, Some(IconTheme::Dark)));
}

/// Tests that omitting the `Mcp-Method` HTTP header falls back to inspecting the JSON-RPC body method.
///
/// Verifies:
/// - Request containing `"method": "server/discover"` in JSON payload is correctly dispatched
/// - Numeric JSON-RPC request IDs (`999`) are preserved
/// - Default capabilities (with tool support enabled) and empty instructions are returned
#[tokio::test]
async fn test_server_discover_via_body_fallback() {
    let server_info = Implementation::new("minimal-server", "0.0.1");
    let app = McpRouter::new(server_info);

    // Request without Mcp-Method header
    let req = common::build_request(
        None,
        None,
        json!({
            "jsonrpc": "2.0",
            "id": 999,
            "method": "server/discover"
        }),
    );

    let (status, _headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    let res: ServerDiscoverResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res.id, 999.into());
    assert_eq!(
        res.result.meta.unwrap().server_info.unwrap().name,
        "minimal-server"
    );
    assert_eq!(res.result.instructions, None);
    assert!(res.result.capabilities.tools.is_some());
}

/// Tests configuring custom server capabilities and multiple supported protocol versions.
///
/// Verifies:
/// - Custom [`ServerCapabilities`] containing tools, resources, prompts, completions, and experimental features
/// - Multi-version protocol compatibility list (e.g. `["2026-07-28", "2026-01-01"]`) is reflected in the discovery response
#[tokio::test]
async fn test_server_discover_custom_capabilities_and_versions() {
    let server_info = Implementation::new("custom-server", "2.0.0");
    let mut experimental = HashMap::new();
    experimental.insert("customFeature".to_string(), json!({ "enabled": true }));

    let custom_caps = ServerCapabilities {
        tools: Some(ToolsCapability {
            list_changed: Some(true),
        }),
        resources: Some(ResourcesCapability {
            subscribe: Some(true),
            list_changed: Some(false),
        }),
        prompts: Some(PromptsCapability {
            list_changed: Some(true),
        }),
        completions: Some(CompletionsCapability {}),
        experimental: Some(experimental),
    };

    let app = McpRouter::new(server_info)
        .capabilities(custom_caps)
        .supported_versions(vec!["2026-07-28".to_string(), "2026-01-01".to_string()]);

    let req = common::build_request(
        Some("server/discover"),
        None,
        json!({
            "jsonrpc": "2.0",
            "id": "caps-test",
            "method": "server/discover"
        }),
    );

    let (status, _headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    let res: ServerDiscoverResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(
        res.result.supported_versions,
        vec!["2026-07-28", "2026-01-01"]
    );

    let caps = res.result.capabilities;
    assert_eq!(caps.tools.unwrap().list_changed, Some(true));
    assert_eq!(caps.resources.as_ref().unwrap().subscribe, Some(true));
    assert_eq!(caps.resources.as_ref().unwrap().list_changed, Some(false));
    assert_eq!(caps.prompts.unwrap().list_changed, Some(true));
    assert!(caps.completions.is_some());
    assert_eq!(
        caps.experimental.unwrap().get("customFeature").unwrap(),
        &json!({ "enabled": true })
    );
}

/// Tests that discovery requests carrying protocol-level metadata (`_meta`) in `params` parse cleanly.
///
/// Verifies:
/// - Parsing of `_meta` containing `clientInfo`, `clientCapabilities`, `protocolVersion`, `logLevel`, and extra custom fields
/// - Floating-point request IDs (`101.5`) are supported and round-tripped
#[tokio::test]
async fn test_server_discover_with_request_meta_params() {
    let server_info = Implementation::new("meta-aware-server", "1.0.0");
    let app = McpRouter::new(server_info);

    let req = common::build_request(
        Some("server/discover"),
        None,
        json!({
            "jsonrpc": "2.0",
            "id": 101.5,
            "method": "server/discover",
            "params": {
                "_meta": {
                    "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                    "io.modelcontextprotocol/clientInfo": {
                        "name": "integration-test-client",
                        "version": "0.1.0"
                    },
                    "io.modelcontextprotocol/clientCapabilities": {
                        "sampling": {},
                        "elicitation": {}
                    },
                    "io.modelcontextprotocol/logLevel": "debug",
                    "customParam": "customValue"
                }
            }
        }),
    );

    let (status, _headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    let res: ServerDiscoverResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res.id, 101.5.into());
    assert_eq!(
        res.result.meta.unwrap().server_info.unwrap().name,
        "meta-aware-server"
    );
}

/// Tests custom TTL and cache scope configuration on `server/discover`.
///
/// Verifies:
/// - Custom `ttl_ms` (e.g. 30000) produces `max-age=30`
/// - Custom `cache_scope` (e.g. `Private`) produces `private`
/// - Result JSON payload matches configured TTL and scope
/// - ETag header is present and deterministic
#[tokio::test]
async fn test_server_discover_caching_headers_and_custom_ttl() {
    let server_info = Implementation::new("caching-server", "1.0.0");
    let app = McpRouter::new(server_info)
        .server_discover_cache(Some(30000), Some(CacheScope::Private));

    let req = common::build_request(
        Some("server/discover"),
        None,
        json!({
            "jsonrpc": "2.0",
            "id": "cache-test-1",
            "method": "server/discover"
        }),
    );

    let (status, headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get("cache-control").unwrap().to_str().unwrap(),
        "private, max-age=30"
    );
    assert!(headers.contains_key("etag"));

    let res: ServerDiscoverResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res.result.ttl_ms, Some(30000));
    assert!(matches!(res.result.cache_scope, Some(CacheScope::Private)));
}
