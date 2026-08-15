// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Base64 Sentinel Value Decoding Integration Tests (SEP-2243)
//!
//! Verifies RFC 2047-style Base64 sentinel header decoding (`=?base64?<encoded>?=`)
//! across Streamable HTTP endpoints (`tools/call`, `prompts/get`, `resources/read`).

mod common;

use http::StatusCode;
use mcp_routing::{
    McpRouter,
    types::mcp::{
        ContentBlock,
        resources::Resource,
        tools::call::{CallToolResult, CallToolResultResponse},
    },
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize)]
struct EchoArgs {
    message: String,
}

async fn handle_echo(args: EchoArgs) -> CallToolResult {
    CallToolResult::text(format!("Echo: {}", args.message))
}

async fn handle_unicode_echo(args: EchoArgs) -> CallToolResult {
    CallToolResult::text(format!("Unicode: {}", args.message))
}

/// Tests calling a tool where `Mcp-Name` is Base64 sentinel encoded (`my_tool`).
#[tokio::test]
async fn test_tools_call_sentinel_encoded_ascii_name() {
    let app = McpRouter::new(common::sample_server_info()).register_tool("my_tool", handle_echo);

    // "my_tool" in base64 is "bXlfdG9vbA=="
    let req = common::build_request(
        Some("tools/call"),
        Some("=?base64?bXlfdG9vbA==?="),
        json!({
            "jsonrpc": "2.0",
            "id": "req-1",
            "method": "tools/call",
            "params": {
                "name": "my_tool",
                "arguments": {
                    "message": "sentinel test"
                }
            }
        }),
    );

    let (status, headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get("content-type").unwrap().to_str().unwrap(),
        "application/json"
    );

    let res: CallToolResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res.jsonrpc, "2.0");
    assert_eq!(res.id, "req-1".into());
    assert_eq!(res.result.is_error, Some(false));
    assert_eq!(res.result.content.len(), 1);

    if let ContentBlock::Text(ref text_block) = res.result.content[0] {
        assert_eq!(text_block.text, "Echo: sentinel test");
    } else {
        panic!("Expected ContentBlock::Text");
    }
}

/// Tests calling a tool where `Mcp-Name` contains non-ASCII characters encoded via Base64 sentinel (`echo_世界`).
#[tokio::test]
async fn test_tools_call_sentinel_encoded_unicode_name() {
    let app = McpRouter::new(common::sample_server_info())
        .register_tool("echo_世界", handle_unicode_echo);

    // "echo_世界" in base64 is "ZWNob1/kuJbnlYw="
    let req = common::build_request(
        Some("tools/call"),
        Some("=?base64?ZWNob1/kuJbnlYw=?="),
        json!({
            "jsonrpc": "2.0",
            "id": "req-unicode",
            "method": "tools/call",
            "params": {
                "name": "echo_世界",
                "arguments": {
                    "message": "testing unicode tool name"
                }
            }
        }),
    );

    let (status, _headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);

    let res: CallToolResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res.jsonrpc, "2.0");
    assert_eq!(res.id, "req-unicode".into());
    if let ContentBlock::Text(ref text_block) = res.result.content[0] {
        assert_eq!(text_block.text, "Unicode: testing unicode tool name");
    } else {
        panic!("Expected ContentBlock::Text");
    }
}

/// Tests calling a tool where `Mcp-Name` is Base64 sentinel encoded with leading and trailing slashes (`/my_tool/`).
#[tokio::test]
async fn test_tools_call_sentinel_encoded_with_slashes() {
    let app = McpRouter::new(common::sample_server_info()).register_tool("my_tool", handle_echo);

    // "/my_tool/" in base64 is "L215X3Rvb2wv"
    let req = common::build_request(
        Some("tools/call"),
        Some("=?base64?L215X3Rvb2wv?="),
        json!({
            "jsonrpc": "2.0",
            "id": "req-slashes",
            "method": "tools/call",
            "params": {
                "name": "my_tool",
                "arguments": {
                    "message": "trimmed slashes"
                }
            }
        }),
    );

    let (status, _headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    let res: CallToolResultResponse = serde_json::from_value(body).unwrap();
    if let ContentBlock::Text(ref text_block) = res.result.content[0] {
        assert_eq!(text_block.text, "Echo: trimmed slashes");
    } else {
        panic!("Expected ContentBlock::Text");
    }
}

/// Tests `prompts/get` where `Mcp-Name` is Base64 sentinel encoded.
#[tokio::test]
async fn test_prompts_get_sentinel_encoded_name() {
    let prompt_def = common::sample_prompt("prompt_test");
    let app = McpRouter::new(common::sample_server_info())
        .register_prompt(prompt_def, || async { "Hello from prompt!" });

    // "prompt_test" in base64 is "cHJvbXB0X3Rlc3Q="
    let req = common::build_request(
        Some("prompts/get"),
        Some("=?base64?cHJvbXB0X3Rlc3Q=?="),
        json!({
            "jsonrpc": "2.0",
            "id": "req-prompt",
            "method": "prompts/get",
            "params": {
                "name": "prompt_test"
            }
        }),
    );

    let (status, _headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], "req-prompt");
    assert_eq!(
        body["result"]["messages"][0]["content"]["text"],
        "Hello from prompt!"
    );
}

/// Tests `resources/read` where `Mcp-Uri` is Base64 sentinel encoded.
#[tokio::test]
async fn test_resources_read_sentinel_encoded_uri() {
    let res = Resource::new("file:///doc/sample.txt", "Sample Resource");
    let app = McpRouter::new(common::sample_server_info())
        .register_resource(res, || async { "Sample resource content" });

    // "file:///doc/sample.txt" in base64 is "ZmlsZTovLy9kb2Mvc2FtcGxlLnR4dA=="
    let req = http::Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "resources/read")
        .header("Mcp-Uri", "=?base64?ZmlsZTovLy9kb2Mvc2FtcGxlLnR4dA==?=")
        .body(axum::body::Body::from(
            json!({
                "jsonrpc": "2.0",
                "id": "req-res",
                "method": "resources/read",
                "params": {
                    "uri": "file:///doc/sample.txt"
                }
            })
            .to_string(),
        ))
        .unwrap();

    let (status, _headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], "req-res");
    assert_eq!(
        body["result"]["contents"][0]["text"],
        "Sample resource content"
    );
}

/// Tests that sentinel decoded header value mismatch with body value returns HTTP 400 with `-32020` (`HeaderMismatch`).
#[tokio::test]
async fn test_sentinel_encoded_mismatch_returns_header_mismatch() {
    let app = McpRouter::new(common::sample_server_info()).register_tool("my_tool", handle_echo);

    // Header says "other_tool" ("b3RoZXJfdG9vbA=="), body says "my_tool"
    let req = common::build_request(
        Some("tools/call"),
        Some("=?base64?b3RoZXJfdG9vbA==?="),
        json!({
            "jsonrpc": "2.0",
            "id": "req-mismatch",
            "method": "tools/call",
            "params": {
                "name": "my_tool",
                "arguments": {
                    "message": "mismatch"
                }
            }
        }),
    );

    let (status, _headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], -32020);
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Header mismatch")
    );
}
