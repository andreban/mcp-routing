// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Result `_meta` Integration Tests
//!
//! Verifies that [`McpRouter`](stateless_mcp::McpRouter) identifies the server on every result:
//! - `io.modelcontextprotocol/serverInfo` is present in the `_meta` of list, call, and read results
//! - `_meta` set by a handler is preserved alongside the added `serverInfo`

mod common;

use http::StatusCode;
use stateless_mcp::{
    McpRouter,
    types::mcp::{ResultMetaObject, tools::call::CallToolResult},
};
use serde_json::{Value, json};

/// Sends `method` with `params` (plus the required `_meta`) and returns the JSON result.
async fn result_of(app: McpRouter, method: &str, name: Option<&str>, params: Value) -> Value {
    let mut params = params;
    params["_meta"] = common::meta();
    let req = common::build_request(
        Some(method),
        name,
        json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params }),
    );
    let (status, _headers, body) = common::execute_request(app, req).await;
    assert_eq!(status, StatusCode::OK, "{method}: {body}");
    body["result"].clone()
}

/// Tests that list, call, and read results carry the server's `serverInfo` in `_meta`.
#[tokio::test]
async fn test_results_include_server_info() {
    let app = McpRouter::new(common::sample_server_info())
        .register_tool("echo", || async { "hi" })
        .register_resource(("memo://notes", "Notes"), || async { "notes" });

    let results = [
        result_of(app.clone(), "tools/list", None, json!({})).await,
        result_of(
            app.clone(),
            "tools/call",
            Some("echo"),
            json!({ "name": "echo" }),
        )
        .await,
        result_of(app.clone(), "resources/list", None, json!({})).await,
        result_of(
            app.clone(),
            "resources/read",
            Some("memo://notes"),
            json!({ "uri": "memo://notes" }),
        )
        .await,
    ];

    for result in results {
        assert_eq!(
            result["_meta"]["io.modelcontextprotocol/serverInfo"]["name"], "test-mcp-server",
            "result: {result}"
        );
    }
}

/// Tests that `_meta` set by a tool handler is preserved when `serverInfo` is added.
#[tokio::test]
async fn test_handler_meta_is_preserved() {
    let app = McpRouter::new(common::sample_server_info()).register_tool("tagged", || async {
        let mut meta = ResultMetaObject::new(None);
        meta.extra
            .insert("com.example/trace".to_string(), json!("abc"));
        CallToolResult::text("tagged").with_meta(meta)
    });

    let result = result_of(
        app,
        "tools/call",
        Some("tagged"),
        json!({ "name": "tagged" }),
    )
    .await;

    assert_eq!(result["_meta"]["com.example/trace"], "abc");
    assert_eq!(
        result["_meta"]["io.modelcontextprotocol/serverInfo"]["name"],
        "test-mcp-server"
    );
}
