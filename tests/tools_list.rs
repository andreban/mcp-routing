// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Tool Discovery (`tools/list`) Integration Tests
//!
//! Verifies the behavior of the Model Context Protocol (MCP) `tools/list` endpoint, including:
//! - Empty tool catalog responses
//! - Registration and advertisement of multiple tools with rich JSON schemas, icons, and behavioral annotations
//! - Fallback dispatch when the `Mcp-Method` header is omitted in favor of the request body
//! - Handling of pagination cursor parameters and metadata in `ListToolsParams`

mod common;

use std::borrow::Cow;
use http::StatusCode;
use mcp_routing::{
    McpRouter,
    types::mcp::{
        CacheScope, IconTheme,
        tools::{Tool, ToolAnnotations, list::ListToolsResultResponse},
    },
};
use serde_json::json;

async fn dummy_handler() -> &'static str {
    "ok"
}

/// Tests that a router with no registered tools returns an empty `tools` list.
///
/// Verifies:
/// - HTTP `200 OK` status and `Content-Type: application/json`
/// - JSON-RPC response with empty array `tools: []`
/// - Default caching properties (`ttl_ms: Some(0)`, `cache_scope: Public`, `next_cursor: None`)
#[tokio::test]
async fn test_tools_list_empty() {
    let app = McpRouter::new(common::sample_server_info());

    let req = common::build_request(
        Some("tools/list"),
        None,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list"
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

    let res: ListToolsResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res.jsonrpc, "2.0");
    assert_eq!(res.id, 1.into());
    assert_eq!(res.result.tools.len(), 0);
    assert_eq!(res.result.ttl_ms, Some(0));
    assert!(matches!(res.result.cache_scope, Some(CacheScope::Public)));
    assert_eq!(res.result.next_cursor, None);
}

/// Tests registering multiple tools using different definition styles and verifying their advertisement in `tools/list`.
///
/// Verifies:
/// - Rich [`Tool`] definitions with custom `input_schema`, `output_schema`, [`ToolAnnotations`], sized icons, and metadata
/// - Ergonomic tool registration using `&str` and `Cow<'static, str>` (auto-wrapped into [`Tool`])
/// - Preserving all tool schema fields and ordering in the JSON-RPC response
#[tokio::test]
async fn test_tools_list_multiple_rich_tools() {
    let tool1 = common::sample_tool("search_database");
    let tool2 = Tool {
        icons: vec![],
        name: "calculate_tax".to_string(),
        title: Some("Calculate Tax".to_string()),
        description: Some("Calculates state and local taxes".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "amount": { "type": "number" },
                "state": { "type": "string" }
            },
            "required": ["amount", "state"]
        }),
        output_schema: Some(json!({
            "type": "object",
            "properties": {
                "tax": { "type": "number" }
            }
        })),
        annotations: Some(ToolAnnotations {
            title: Some("Tax Calculator".to_string()),
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(true),
            open_world_hint: Some(false),
        }),
        meta: None,
    };

    let app = McpRouter::new(common::sample_server_info())
        .register_tool(tool1, dummy_handler)
        .register_tool(tool2, dummy_handler)
        .register_tool("inline_str_tool", dummy_handler)
        .register_tool(Cow::Borrowed("cow_tool"), dummy_handler);

    let req = common::build_request(
        Some("tools/list"),
        None,
        json!({
            "jsonrpc": "2.0",
            "id": "list-rich-tools",
            "method": "tools/list"
        }),
    );

    let (status, _headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    let res: ListToolsResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res.id, "list-rich-tools".into());
    assert_eq!(res.result.tools.len(), 4);

    // Verify first tool
    let t0 = &res.result.tools[0];
    assert_eq!(t0.name, "search_database");
    assert_eq!(t0.title.as_deref(), Some("Title for search_database"));
    assert_eq!(
        t0.description.as_deref(),
        Some("Description for search_database")
    );
    assert_eq!(t0.icons.len(), 1);
    assert_eq!(t0.icons[0].src, "https://example.com/tool_icon.png");
    assert_eq!(t0.icons[0].mime_type.as_deref(), Some("image/png"));
    assert_eq!(t0.icons[0].sizes, vec!["32x32".to_string()]);
    assert!(matches!(t0.icons[0].theme, Some(IconTheme::Light)));
    assert_eq!(
        t0.annotations.as_ref().unwrap().read_only_hint,
        Some(true)
    );
    assert_eq!(
        t0.annotations.as_ref().unwrap().destructive_hint,
        Some(false)
    );
    assert_eq!(
        t0.meta.as_ref().unwrap().get("customMeta").unwrap(),
        "customValue"
    );

    // Verify second tool
    let t1 = &res.result.tools[1];
    assert_eq!(t1.name, "calculate_tax");
    assert_eq!(t1.title.as_deref(), Some("Calculate Tax"));
    assert!(t1.output_schema.is_some());

    // Verify tool registered via &str
    let t2 = &res.result.tools[2];
    assert_eq!(t2.name, "inline_str_tool");
    assert_eq!(t2.input_schema, json!({ "type": "object" }));

    // Verify tool registered via Cow
    let t3 = &res.result.tools[3];
    assert_eq!(t3.name, "cow_tool");
}

/// Tests `tools/list` dispatch when the `Mcp-Method` HTTP header is omitted.
///
/// Verifies:
/// - Request is successfully routed based on `"method": "tools/list"` inside the JSON-RPC body
/// - Numeric JSON-RPC request IDs (`42`) are preserved
#[tokio::test]
async fn test_tools_list_via_body_fallback() {
    let app = McpRouter::new(common::sample_server_info())
        .register_tool(common::sample_tool("fallback_tool"), dummy_handler);

    // No Mcp-Method header
    let req = common::build_request(
        None,
        None,
        json!({
            "jsonrpc": "2.0",
            "id": 42,
            "method": "tools/list"
        }),
    );

    let (status, _headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    let res: ListToolsResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res.id, 42.into());
    assert_eq!(res.result.tools.len(), 1);
    assert_eq!(res.result.tools[0].name, "fallback_tool");
}

/// Tests that passing pagination `cursor` and protocol `_meta` in `tools/list` requests executes without error.
///
/// Verifies:
/// - Request containing `cursor: "page-2-cursor"` and `_meta` object is accepted and parsed
/// - String request IDs (`"cursor-req"`) are preserved in response
#[tokio::test]
async fn test_tools_list_with_pagination_cursor_and_meta() {
    let app = McpRouter::new(common::sample_server_info())
        .register_tool("tool_a", dummy_handler);

    let req = common::build_request(
        Some("tools/list"),
        None,
        json!({
            "jsonrpc": "2.0",
            "id": "cursor-req",
            "method": "tools/list",
            "params": {
                "cursor": "page-2-cursor",
                "_meta": {
                    "io.modelcontextprotocol/protocolVersion": "2026-07-28"
                }
            }
        }),
    );

    let (status, _headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    let res: ListToolsResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res.id, "cursor-req".into());
    assert_eq!(res.result.tools.len(), 1);
    assert_eq!(res.result.tools[0].name, "tool_a");
}

/// Tests custom TTL and cache scope configuration on `tools/list`.
///
/// Verifies:
/// - Custom `tools_list_cache(Some(600000), Some(CacheScope::Public))` sets `Cache-Control: public, max-age=600`
/// - ETag header is present
/// - JSON-RPC response contains configured `ttl_ms` and `cache_scope`
#[tokio::test]
async fn test_tools_list_custom_caching_parameters() {
    let app = McpRouter::new(common::sample_server_info())
        .tools_list_cache(Some(600000), Some(CacheScope::Public))
        .register_tool(common::sample_tool("cache_tool"), dummy_handler);

    let req = common::build_request(
        Some("tools/list"),
        None,
        json!({
            "jsonrpc": "2.0",
            "id": "cache-list-test",
            "method": "tools/list"
        }),
    );

    let (status, headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get("cache-control").unwrap().to_str().unwrap(),
        "public, max-age=600"
    );
    assert!(headers.contains_key("etag"));

    let res: ListToolsResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res.result.ttl_ms, Some(600000));
    assert!(matches!(res.result.cache_scope, Some(CacheScope::Public)));
}
