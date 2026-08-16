// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Unit tests for `tools/call` request and result serialization and builders.

use super::*;
use crate::types::mcp::{ContentBlock, TextContent};

/// Tests serialization and deserialization of `CallToolRequest` payloads.
#[test]
fn test_call_tool_request_serde() {
    let json_data = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "echo",
            "arguments": {
                "value": "hello"
            }
        }
    });

    let req: CallToolRequest = serde_json::from_value(json_data).unwrap();
    assert_eq!(req.method, "tools/call");
    let params = req.params.unwrap();
    assert_eq!(params.name, "echo");
    assert_eq!(params.arguments.unwrap()["value"], "hello");
}

/// Tests serialization and deserialization of `CallToolResult` payloads.
#[test]
fn test_call_tool_result_serde() {
    let json_data = serde_json::json!({
        "resultType": "complete",
        "content": [{
            "type": "text",
            "text": "Hello"
        }],
        "isError": false,
        "structuredContent": {
            "key": "value"
        }
    });

    let result: CallToolResult = serde_json::from_value(json_data).unwrap();
    assert_eq!(result.result_type.as_deref(), Some("complete"));
    assert_eq!(result.content.len(), 1);
    if let ContentBlock::Text(ref text_content) = result.content[0] {
        assert_eq!(text_content.text, "Hello");
    } else {
        panic!("Expected ContentBlock::Text");
    }
    assert_eq!(result.is_error, Some(false));
    assert_eq!(result.structured_content.as_ref().unwrap()["key"], "value");

    let reserialized = serde_json::to_value(&result).unwrap();
    assert_eq!(reserialized["isError"], false);
    assert_eq!(reserialized["content"][0]["type"], "text");
    assert_eq!(reserialized["content"][0]["text"], "Hello");
    assert_eq!(reserialized["structuredContent"]["key"], "value");
}

/// Tests [`CallToolResult`] convenience constructors and builder methods.
#[test]
fn test_call_tool_result_builder_constructors() {
    let text_res = CallToolResult::<serde_json::Value>::text("Hello result");
    assert_eq!(text_res.result_type.as_deref(), Some("complete"));
    assert_eq!(text_res.is_error, Some(false));
    assert_eq!(text_res.content.len(), 1);

    let err_res = CallToolResult::<serde_json::Value>::error("Failure message");
    assert_eq!(err_res.result_type.as_deref(), Some("complete"));
    assert_eq!(err_res.is_error, Some(true));
    assert_eq!(err_res.content.len(), 1);

    let block = ContentBlock::Text(TextContent {
        text: "block".to_string(),
        annotations: None,
        meta: None,
    });
    let with_content_res = CallToolResult::<serde_json::Value>::with_content(vec![block.clone()]);
    assert_eq!(with_content_res.is_error, Some(false));
    assert_eq!(with_content_res.content.len(), 1);

    // Structured constructors
    let struct_res = CallToolResult::structured(serde_json::json!({ "status": "ok" }));
    assert_eq!(struct_res.structured_content.unwrap()["status"], "ok");
    assert!(struct_res.content.is_empty());

    let struct_text_res =
        CallToolResult::structured_with_text(serde_json::json!({ "count": 10 }), "Found 10 items");
    assert_eq!(struct_text_res.structured_content.unwrap()["count"], 10);
    assert_eq!(struct_text_res.content.len(), 1);

    let struct_content_res =
        CallToolResult::structured_with_content(serde_json::json!({ "id": 1 }), vec![block]);
    assert_eq!(struct_content_res.structured_content.unwrap()["id"], 1);
    assert_eq!(struct_content_res.content.len(), 1);

    // JSON helper constructors
    #[derive(serde::Serialize)]
    struct Person {
        name: String,
        age: u32,
    }
    let person = Person {
        name: "Alice".to_string(),
        age: 30,
    };

    let json_res = CallToolResult::structured_json(&person).unwrap();
    assert_eq!(json_res.structured_content.unwrap()["name"], "Alice");

    let json_text_res =
        CallToolResult::structured_json_with_text(&person, "Person created").unwrap();
    assert_eq!(json_text_res.structured_content.unwrap()["age"], 30);
    assert_eq!(json_text_res.content.len(), 1);

    // Fluent builders
    let chained = CallToolResult::new()
        .with_text("Chained text")
        .with_image("aW1hZ2U=", "image/png")
        .with_audio("YXVkaW8=", "audio/wav")
        .with_extra("custom", serde_json::json!(42))
        .with_structured(serde_json::json!({ "chained": true }));

    assert_eq!(chained.content.len(), 3);
    assert_eq!(chained.extras.get("custom").unwrap(), 42);
    assert_eq!(chained.structured_content.unwrap()["chained"], true);
}
