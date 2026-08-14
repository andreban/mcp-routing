// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Structured Output Helpers Integration Tests
//!
//! Verifies:
//! - Construction of tools with `output_schema` and `ToolAnnotations` via fluent builder APIs
//! - Advertisement of `output_schema` and annotations in `tools/list`
//! - Handlers returning [`Json<T>`](mcp_routing::extract::Json) structured output wrappers
//! - Handlers returning raw [`serde_json::Value`] structured outputs
//! - Handlers returning typed [`CallToolResult<T>`](mcp_routing::types::mcp::tools::call::CallToolResult)
//! - Handlers returning tuple conversions `(Json<T>, &str)`, `(Json<T>, String)`, `(Json<T>, Vec<ContentBlock>)`
//! - Handlers returning `Result<Json<T>, E>` for both success and error paths
//! - Convenience constructors `structured`, `structured_with_text`, `structured_with_content`, `structured_json`

mod common;

use http::StatusCode;
use mcp_routing::{
    Json, McpRouter,
    types::mcp::{
        ContentBlock,
        tools::{
            Tool, ToolAnnotations,
            call::{CallToolResult, CallToolResultResponse},
            list::ListToolsResultResponse,
        },
    },
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct DatabaseRecord {
    id: u64,
    username: String,
    email: String,
    active: bool,
}

#[derive(Serialize, Deserialize)]
struct QueryParams {
    user_id: u64,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct SearchMetrics {
    total_found: usize,
    took_ms: u64,
}

/// Handler returning `Json<DatabaseRecord>` directly.
async fn handle_get_user(params: QueryParams) -> Result<Json<DatabaseRecord>, String> {
    if params.user_id == 0 {
        return Err("User ID cannot be zero".to_string());
    }
    Ok(Json(DatabaseRecord {
        id: params.user_id,
        username: "testuser".to_string(),
        email: "test@example.com".to_string(),
        active: true,
    }))
}

/// Handler returning a raw `serde_json::Value`.
async fn handle_raw_value_output() -> serde_json::Value {
    json!({
        "status": "healthy",
        "uptime": 3600,
        "nodes": ["node-1", "node-2"]
    })
}

/// Handler returning a typed `CallToolResult<DatabaseRecord>`.
async fn handle_typed_call_tool_result(params: QueryParams) -> CallToolResult<DatabaseRecord> {
    let record = DatabaseRecord {
        id: params.user_id,
        username: "superadmin".to_string(),
        email: "admin@example.com".to_string(),
        active: true,
    };
    CallToolResult::structured(record).with_text(format!("Retrieved user #{}", params.user_id))
}

/// Handler returning `(Json<SearchMetrics>, &'static str)`.
async fn handle_tuple_json_str() -> (Json<SearchMetrics>, &'static str) {
    (
        Json(SearchMetrics {
            total_found: 42,
            took_ms: 15,
        }),
        "Query completed in 15ms",
    )
}

/// Handler returning `(String, Json<SearchMetrics>)`.
async fn handle_tuple_str_json() -> (String, Json<SearchMetrics>) {
    (
        "Search successful".to_string(),
        Json(SearchMetrics {
            total_found: 100,
            took_ms: 30,
        }),
    )
}

/// Handler returning `(serde_json::Value, &'static str)`.
async fn handle_tuple_value_str() -> (serde_json::Value, &'static str) {
    (json!({ "version": "2.0.0" }), "Version information")
}

/// Tests that tool definitions built with `Tool::new`, `.output_schema()`, and `.annotations()`
/// properly advertise their structure in `tools/list`.
#[tokio::test]
async fn test_tool_builder_output_schema_in_tools_list() {
    let tool = Tool::new("get_user")
        .title("Get User Record")
        .description("Fetches a user by ID from the database")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "user_id": { "type": "integer", "description": "User identifier" }
            },
            "required": ["user_id"]
        }))
        .output_schema(json!({
            "type": "object",
            "properties": {
                "id": { "type": "integer" },
                "username": { "type": "string" },
                "email": { "type": "string" },
                "active": { "type": "boolean" }
            },
            "required": ["id", "username", "email", "active"]
        }))
        .annotations(
            ToolAnnotations::new()
                .title("Read-only user query")
                .read_only(true)
                .idempotent(true)
                .destructive(false)
                .open_world(false),
        );

    let app = McpRouter::new(common::sample_server_info()).register_tool(tool, handle_get_user);

    let req = common::build_request(
        Some("tools/list"),
        None,
        json!({
            "jsonrpc": "2.0",
            "id": "list-req",
            "method": "tools/list"
        }),
    );

    let (status, _, body) = common::execute_request(app, req).await;
    assert_eq!(status, StatusCode::OK);

    let res: ListToolsResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res.result.tools.len(), 1);
    let listed = &res.result.tools[0];
    assert_eq!(listed.name, "get_user");
    assert_eq!(listed.title.as_deref(), Some("Get User Record"));
    assert_eq!(
        listed.description.as_deref(),
        Some("Fetches a user by ID from the database")
    );
    assert!(listed.output_schema.is_some());
    let output_schema = listed.output_schema.as_ref().unwrap();
    assert_eq!(output_schema["properties"]["username"]["type"], "string");

    let ann = listed.annotations.as_ref().unwrap();
    assert_eq!(ann.read_only_hint, Some(true));
    assert_eq!(ann.idempotent_hint, Some(true));
    assert_eq!(ann.destructive_hint, Some(false));
    assert_eq!(ann.open_world_hint, Some(false));
}

/// Tests executing a tool whose handler returns `Json<T>`.
#[tokio::test]
async fn test_handler_returning_json_wrapper() {
    let app =
        McpRouter::new(common::sample_server_info()).register_tool("get_user", handle_get_user);

    let req = common::build_request(
        Some("tools/call"),
        Some("get_user"),
        json!({
            "jsonrpc": "2.0",
            "id": 101,
            "method": "tools/call",
            "params": {
                "name": "get_user",
                "arguments": { "user_id": 42 }
            }
        }),
    );

    let (status, _, body) = common::execute_request(app, req).await;
    assert_eq!(status, StatusCode::OK);

    let res: CallToolResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res.id, 101.into());
    assert_eq!(res.result.is_error, Some(false));

    let structured = res
        .result
        .structured_content
        .expect("Must have structured content");
    assert_eq!(structured["id"], 42);
    assert_eq!(structured["username"], "testuser");
    assert_eq!(structured["email"], "test@example.com");
    assert_eq!(structured["active"], true);
}

/// Tests executing a tool whose handler returns `Result<Json<T>, E>` where `Err` is produced.
#[tokio::test]
async fn test_handler_returning_json_wrapper_error() {
    let app =
        McpRouter::new(common::sample_server_info()).register_tool("get_user", handle_get_user);

    let req = common::build_request(
        Some("tools/call"),
        Some("get_user"),
        json!({
            "jsonrpc": "2.0",
            "id": 102,
            "method": "tools/call",
            "params": {
                "name": "get_user",
                "arguments": { "user_id": 0 }
            }
        }),
    );

    let (status, _, body) = common::execute_request(app, req).await;
    assert_eq!(status, StatusCode::OK);

    let res: CallToolResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res.id, 102.into());
    assert_eq!(res.result.is_error, Some(true));
    if let ContentBlock::Text(ref t) = res.result.content[0] {
        assert_eq!(t.text, "User ID cannot be zero");
    } else {
        panic!("Expected text error block");
    }
}

/// Tests executing a tool whose handler returns a raw `serde_json::Value`.
#[tokio::test]
async fn test_handler_returning_serde_json_value() {
    let app = McpRouter::new(common::sample_server_info())
        .register_tool("get_health", handle_raw_value_output);

    let req = common::build_request(
        Some("tools/call"),
        Some("get_health"),
        json!({
            "jsonrpc": "2.0",
            "id": "health-check",
            "method": "tools/call",
            "params": { "name": "get_health" }
        }),
    );

    let (status, _, body) = common::execute_request(app, req).await;
    assert_eq!(status, StatusCode::OK);

    let res: CallToolResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res.result.is_error, Some(false));

    let structured = res
        .result
        .structured_content
        .expect("Must have structured content");
    assert_eq!(structured["status"], "healthy");
    assert_eq!(structured["uptime"], 3600);
    assert_eq!(structured["nodes"][0], "node-1");
    assert_eq!(structured["nodes"][1], "node-2");
}

/// Tests executing a tool whose handler returns `CallToolResult<DatabaseRecord>` with both text and structured content.
#[tokio::test]
async fn test_handler_returning_typed_call_tool_result() {
    let app = McpRouter::new(common::sample_server_info())
        .register_tool("typed_result", handle_typed_call_tool_result);

    let req = common::build_request(
        Some("tools/call"),
        Some("typed_result"),
        json!({
            "jsonrpc": "2.0",
            "id": 200,
            "method": "tools/call",
            "params": {
                "name": "typed_result",
                "arguments": { "user_id": 99 }
            }
        }),
    );

    let (status, _, body) = common::execute_request(app, req).await;
    assert_eq!(status, StatusCode::OK);

    let res: CallToolResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res.result.is_error, Some(false));
    assert_eq!(res.result.content.len(), 1);

    if let ContentBlock::Text(ref t) = res.result.content[0] {
        assert_eq!(t.text, "Retrieved user #99");
    } else {
        panic!("Expected text block");
    }

    let structured = res
        .result
        .structured_content
        .expect("Must have structured content");
    assert_eq!(structured["id"], 99);
    assert_eq!(structured["username"], "superadmin");
}

/// Tests tuple return types `(Json<T>, &str)` and `(String, Json<T>)`.
#[tokio::test]
async fn test_handler_returning_tuples() {
    let app = McpRouter::new(common::sample_server_info())
        .register_tool("tuple_json_str", handle_tuple_json_str)
        .register_tool("tuple_str_json", handle_tuple_str_json)
        .register_tool("tuple_val_str", handle_tuple_value_str);

    // 1. (Json<T>, &str)
    let req1 = common::build_request(
        Some("tools/call"),
        Some("tuple_json_str"),
        json!({
            "jsonrpc": "2.0",
            "id": "t1",
            "method": "tools/call",
            "params": { "name": "tuple_json_str" }
        }),
    );
    let (status1, _, body1) = common::execute_request(app.clone(), req1).await;
    assert_eq!(status1, StatusCode::OK);
    let res1: CallToolResultResponse = serde_json::from_value(body1).unwrap();
    assert_eq!(
        res1.result.structured_content.as_ref().unwrap()["totalFound"],
        42
    );
    if let ContentBlock::Text(ref t) = res1.result.content[0] {
        assert_eq!(t.text, "Query completed in 15ms");
    }

    // 2. (String, Json<T>)
    let req2 = common::build_request(
        Some("tools/call"),
        Some("tuple_str_json"),
        json!({
            "jsonrpc": "2.0",
            "id": "t2",
            "method": "tools/call",
            "params": { "name": "tuple_str_json" }
        }),
    );
    let (status2, _, body2) = common::execute_request(app.clone(), req2).await;
    assert_eq!(status2, StatusCode::OK);
    let res2: CallToolResultResponse = serde_json::from_value(body2).unwrap();
    assert_eq!(
        res2.result.structured_content.as_ref().unwrap()["totalFound"],
        100
    );
    if let ContentBlock::Text(ref t) = res2.result.content[0] {
        assert_eq!(t.text, "Search successful");
    }

    // 3. (Value, &str)
    let req3 = common::build_request(
        Some("tools/call"),
        Some("tuple_val_str"),
        json!({
            "jsonrpc": "2.0",
            "id": "t3",
            "method": "tools/call",
            "params": { "name": "tuple_val_str" }
        }),
    );
    let (status3, _, body3) = common::execute_request(app, req3).await;
    assert_eq!(status3, StatusCode::OK);
    let res3: CallToolResultResponse = serde_json::from_value(body3).unwrap();
    assert_eq!(
        res3.result.structured_content.as_ref().unwrap()["version"],
        "2.0.0"
    );
    if let ContentBlock::Text(ref t) = res3.result.content[0] {
        assert_eq!(t.text, "Version information");
    }
}

/// Tests [`CallToolResult`] structured JSON helper constructors and fluent chaining.
#[test]
fn test_call_tool_result_helpers_and_fluent_chaining() {
    #[derive(Serialize)]
    struct Summary {
        items: usize,
        status: String,
    }

    let summary = Summary {
        items: 12,
        status: "complete".to_string(),
    };

    // structured_json
    let res = CallToolResult::structured_json(&summary).unwrap();
    assert_eq!(res.structured_content.as_ref().unwrap()["items"], 12);
    assert_eq!(
        res.structured_content.as_ref().unwrap()["status"],
        "complete"
    );
    assert!(res.content.is_empty());

    // structured_json_with_text
    let res_text = CallToolResult::structured_json_with_text(&summary, "Processing done").unwrap();
    assert_eq!(res_text.structured_content.as_ref().unwrap()["items"], 12);
    assert_eq!(res_text.content.len(), 1);

    // fluent chaining with multi-modal elements and extras
    let full = CallToolResult::new()
        .with_text("Introduction")
        .with_image("base64data", "image/png")
        .with_audio("audiodata", "audio/wav")
        .with_structured(json!({ "score": 98.5 }))
        .with_extra("requestId", "req-xyz");

    assert_eq!(full.content.len(), 3);
    assert_eq!(full.structured_content.unwrap()["score"], 98.5);
    assert_eq!(full.extras["requestId"], "req-xyz");
}
