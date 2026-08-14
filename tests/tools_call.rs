// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Tool Execution (`tools/call`) Integration Tests
//!
//! Verifies the behavior of the Model Context Protocol (MCP) `tools/call` endpoint, including:
//! - Header-based routing via `Mcp-Method: tools/call` and `Mcp-Name: <name>`
//! - Body-based fallback for tool names when `Mcp-Name` is omitted
//! - Pure body-based fallback routing (when both `Mcp-Method` and `Mcp-Name` headers are omitted)
//! - Support for no-argument tool handlers returning strings, results, or error variants
//! - Automatic deserialization of typed argument structures into handler parameters
//! - Error handling when handler business logic fails vs. argument deserialization failure
//! - Support for optional and default argument fields

mod common;

use http::StatusCode;
use mcp_routing::{
    McpRouter,
    types::mcp::{
        ContentBlock,
        tools::{
            Tool,
            call::{CallToolResult, CallToolResultResponse},
        },
    },
};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Typed parameters for the echo tool.
#[derive(Serialize, Deserialize)]
struct EchoParams {
    message: String,
}

/// Typed parameters for arithmetic calculator operations.
#[derive(Serialize, Deserialize)]
struct CalculatorParams {
    a: i64,
    b: i64,
    operation: String,
}

/// Typed parameters demonstrating required, optional, and default collection fields.
#[derive(Serialize, Deserialize)]
struct OptionalFieldParams {
    required_key: String,
    optional_key: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

/// Typed parameters representing empty argument payloads.
#[derive(Serialize, Deserialize)]
struct EmptyParams {}

/// No-args handler returning a static string slice.
async fn handle_no_args_ok() -> &'static str {
    "no_args_response"
}

/// No-args handler returning an owned [`String`].
async fn handle_no_args_string() -> String {
    "dynamic_no_args_string".to_string()
}

/// No-args handler returning an `Err` result to test error formatting.
async fn handle_no_args_result_err() -> Result<String, String> {
    Err("handler failed intentionally".to_string())
}

/// Typed handler echoing the provided message parameter.
async fn handle_echo(params: EchoParams) -> Result<String, String> {
    if params.message.is_empty() {
        return Err("message cannot be empty".to_string());
    }
    Ok(format!("Echo: {}", params.message))
}

/// Typed handler executing basic arithmetic operations.
async fn handle_calculator(params: CalculatorParams) -> Result<String, String> {
    match params.operation.as_str() {
        "add" => Ok((params.a + params.b).to_string()),
        "sub" => Ok((params.a - params.b).to_string()),
        "mul" => Ok((params.a * params.b).to_string()),
        "div" => {
            if params.b == 0 {
                Err("division by zero".to_string())
            } else {
                Ok((params.a / params.b).to_string())
            }
        }
        unknown => Err(format!("unknown operation: {unknown}")),
    }
}

/// Typed handler testing optional and default field extraction.
async fn handle_optional_fields(params: OptionalFieldParams) -> CallToolResult {
    let text = format!(
        "req={}, opt={:?}, tags_len={}",
        params.required_key,
        params.optional_key,
        params.tags.len()
    );
    CallToolResult::text(text)
}

/// Typed handler accepting empty object arguments `{}`.
async fn handle_empty_params(_params: EmptyParams) -> &'static str {
    "empty_params_ok"
}

/// Tests calling a tool with explicit `Mcp-Method: tools/call` and `Mcp-Name: echo` HTTP headers.
///
/// Verifies:
/// - Request is dispatched to the registered handler for `echo`
/// - String arguments are automatically deserialized into [`EchoParams`]
/// - Result is formatted as a JSON-RPC success response with `is_error: false`
#[tokio::test]
async fn test_tools_call_header_routing_with_name() {
    let app = McpRouter::new(common::sample_server_info()).register_tool("echo", handle_echo);

    let req = common::build_request(
        Some("tools/call"),
        Some("echo"),
        json!({
            "jsonrpc": "2.0",
            "id": "req-1",
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": {
                    "message": "Hello MCP!"
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
        assert_eq!(text_block.text, "Echo: Hello MCP!");
    } else {
        panic!("Expected ContentBlock::Text");
    }
}

/// Tests tool call routing when `Mcp-Method: tools/call` is present, but `Mcp-Name` header is omitted.
///
/// Verifies:
/// - Router extracts the target tool name from `params.name` inside the JSON-RPC body
/// - Handler executes and returns the expected result
#[tokio::test]
async fn test_tools_call_header_method_body_tool_name_fallback() {
    let app = McpRouter::new(common::sample_server_info()).register_tool("echo", handle_echo);

    // Mcp-Method header is present, but Mcp-Name header is OMITTED
    let req = common::build_request(
        Some("tools/call"),
        None,
        json!({
            "jsonrpc": "2.0",
            "id": 100,
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": {
                    "message": "Fallback tool name from body"
                }
            }
        }),
    );

    let (status, _headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    let res: CallToolResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res.id, 100.into());
    assert_eq!(res.result.is_error, Some(false));

    if let ContentBlock::Text(ref text_block) = res.result.content[0] {
        assert_eq!(text_block.text, "Echo: Fallback tool name from body");
    } else {
        panic!("Expected ContentBlock::Text");
    }
}

/// Tests pure body-based tool dispatch when neither `Mcp-Method` nor `Mcp-Name` headers are provided.
///
/// Verifies:
/// - Router falls back to `method` and `params.name` in JSON-RPC payload
/// - Float request IDs (`200.5`) are supported
/// - Arithmetic handler executes and returns correct calculation output
#[tokio::test]
async fn test_tools_call_body_method_and_body_name_fallback() {
    let app =
        McpRouter::new(common::sample_server_info()).register_tool("calculator", handle_calculator);

    // Both Mcp-Method and Mcp-Name headers are OMITTED
    let req = common::build_request(
        None,
        None,
        json!({
            "jsonrpc": "2.0",
            "id": 200.5,
            "method": "tools/call",
            "params": {
                "name": "calculator",
                "arguments": {
                    "a": 15,
                    "b": 3,
                    "operation": "mul"
                }
            }
        }),
    );

    let (status, _headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    let res: CallToolResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res.id, 200.5.into());
    assert_eq!(res.result.is_error, Some(false));

    if let ContentBlock::Text(ref text_block) = res.result.content[0] {
        assert_eq!(text_block.text, "45");
    } else {
        panic!("Expected ContentBlock::Text");
    }
}

/// Tests registering and invoking handlers with no arguments across return types (`&str`, `String`, `Result<T, E>`).
///
/// Verifies:
/// - Static string handlers wrap output into [`TextContent`](mcp_routing::types::mcp::TextContent) with `is_error: false`
/// - Dynamic string handlers wrap output with `is_error: false`
/// - `Result::Err` produces `is_error: true` containing the error description
#[tokio::test]
async fn test_tools_call_no_args_handlers() {
    let app = McpRouter::new(common::sample_server_info())
        .register_tool("no_args_static", handle_no_args_ok)
        .register_tool("no_args_string", handle_no_args_string)
        .register_tool("no_args_err", handle_no_args_result_err);

    // Call static str handler
    let req1 = common::build_request(
        Some("tools/call"),
        Some("no_args_static"),
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": { "name": "no_args_static" }
        }),
    );
    let (status, _, body) = common::execute_request(app.clone(), req1).await;
    assert_eq!(status, StatusCode::OK);
    let res1: CallToolResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res1.result.is_error, Some(false));
    if let ContentBlock::Text(ref t) = res1.result.content[0] {
        assert_eq!(t.text, "no_args_response");
    }

    // Call dynamic string handler
    let req2 = common::build_request(
        Some("tools/call"),
        Some("no_args_string"),
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": { "name": "no_args_string" }
        }),
    );
    let (status, _, body) = common::execute_request(app.clone(), req2).await;
    assert_eq!(status, StatusCode::OK);
    let res2: CallToolResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res2.result.is_error, Some(false));
    if let ContentBlock::Text(ref t) = res2.result.content[0] {
        assert_eq!(t.text, "dynamic_no_args_string");
    }

    // Call handler returning error
    let req3 = common::build_request(
        Some("tools/call"),
        Some("no_args_err"),
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": { "name": "no_args_err" }
        }),
    );
    let (status, _, body) = common::execute_request(app, req3).await;
    assert_eq!(status, StatusCode::OK);
    let res3: CallToolResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res3.result.is_error, Some(true));
    if let ContentBlock::Text(ref t) = res3.result.content[0] {
        assert_eq!(t.text, "handler failed intentionally");
    }
}

/// Tests that a tool returning a business-logic error (e.g. division by zero) returns `is_error: true` with HTTP 200.
///
/// Verifies:
/// - According to MCP spec, tool-level runtime execution errors return standard tool results with `is_error: Some(true)`
#[tokio::test]
async fn test_tools_call_handler_business_logic_error() {
    let app =
        McpRouter::new(common::sample_server_info()).register_tool("calculator", handle_calculator);

    let req = common::build_request(
        Some("tools/call"),
        Some("calculator"),
        json!({
            "jsonrpc": "2.0",
            "id": "div-zero",
            "method": "tools/call",
            "params": {
                "name": "calculator",
                "arguments": {
                    "a": 10,
                    "b": 0,
                    "operation": "div"
                }
            }
        }),
    );

    let (status, _headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    let res: CallToolResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res.id, "div-zero".into());
    assert_eq!(res.result.is_error, Some(true));
    if let ContentBlock::Text(ref t) = res.result.content[0] {
        assert_eq!(t.text, "division by zero");
    } else {
        panic!("Expected ContentBlock::Text");
    }
}

/// Tests that passing argument JSON with invalid field types returns a descriptive tool error.
///
/// Verifies:
/// - JSON deserialization error is caught in [`IntoToolHandler`](mcp_routing::tools::IntoToolHandler)
/// - Response status is `200 OK` with `is_error: true` and text starting with `"Invalid arguments:"`
#[tokio::test]
async fn test_tools_call_invalid_argument_types_returns_tool_error() {
    let app =
        McpRouter::new(common::sample_server_info()).register_tool("calculator", handle_calculator);

    // Pass invalid type for "a" (string instead of integer)
    let req = common::build_request(
        Some("tools/call"),
        Some("calculator"),
        json!({
            "jsonrpc": "2.0",
            "id": "invalid-args-test",
            "method": "tools/call",
            "params": {
                "name": "calculator",
                "arguments": {
                    "a": "not-a-number",
                    "b": 10,
                    "operation": "add"
                }
            }
        }),
    );

    let (status, _headers, body) = common::execute_request(app, req).await;

    // According to JSON-RPC / MCP spec, invalid tool arguments result in CallToolResult with is_error: true and status 200
    assert_eq!(status, StatusCode::OK);
    let res: CallToolResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res.id, "invalid-args-test".into());
    assert_eq!(res.result.is_error, Some(true));

    if let ContentBlock::Text(ref t) = res.result.content[0] {
        assert!(
            t.text.starts_with("Invalid arguments:"),
            "Expected 'Invalid arguments:' prefix, got: {}",
            t.text
        );
    } else {
        panic!("Expected ContentBlock::Text");
    }
}

/// Tests deserialization when arguments contain optional fields (`Option<T>`) and defaulted vectors.
///
/// Verifies:
/// - Optional fields present are deserialized as `Some(...)`
/// - Omitted optional fields default to `None` and `Vec::new()`
#[tokio::test]
async fn test_tools_call_optional_fields_and_defaults() {
    let app = McpRouter::new(common::sample_server_info())
        .register_tool("optional_tool", handle_optional_fields);

    // Case 1: with all fields
    let req1 = common::build_request(
        Some("tools/call"),
        Some("optional_tool"),
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "optional_tool",
                "arguments": {
                    "required_key": "val1",
                    "optional_key": "val2",
                    "tags": ["a", "b"]
                }
            }
        }),
    );
    let (status, _, body) = common::execute_request(app.clone(), req1).await;
    assert_eq!(status, StatusCode::OK);
    let res1: CallToolResultResponse = serde_json::from_value(body).unwrap();
    if let ContentBlock::Text(ref t) = res1.result.content[0] {
        assert_eq!(t.text, "req=val1, opt=Some(\"val2\"), tags_len=2");
    }

    // Case 2: omitting optional fields
    let req2 = common::build_request(
        Some("tools/call"),
        Some("optional_tool"),
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "optional_tool",
                "arguments": {
                    "required_key": "only_required"
                }
            }
        }),
    );
    let (status, _, body) = common::execute_request(app, req2).await;
    assert_eq!(status, StatusCode::OK);
    let res2: CallToolResultResponse = serde_json::from_value(body).unwrap();
    if let ContentBlock::Text(ref t) = res2.result.content[0] {
        assert_eq!(t.text, "req=only_required, opt=None, tags_len=0");
    }
}

/// Tests calling a typed handler with empty arguments object `{}`.
///
/// Verifies:
/// - Handlers expecting empty structs deserialize `{}` successfully
#[tokio::test]
async fn test_tools_call_empty_arguments_object() {
    let app = McpRouter::new(common::sample_server_info())
        .register_tool("empty_args_tool", handle_empty_params);

    let req = common::build_request(
        Some("tools/call"),
        Some("empty_args_tool"),
        json!({
            "jsonrpc": "2.0",
            "id": "empty-args-req",
            "method": "tools/call",
            "params": {
                "name": "empty_args_tool",
                "arguments": {}
            }
        }),
    );

    let (status, _headers, body) = common::execute_request(app, req).await;
    assert_eq!(status, StatusCode::OK);
    let res: CallToolResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res.result.is_error, Some(false));
    if let ContentBlock::Text(ref t) = res.result.content[0] {
        assert_eq!(t.text, "empty_params_ok");
    }
}

/// Tests that tools configured with cache directives return `Cache-Control` and `ETag` headers.
///
/// Verifies:
/// - `register_tool_with_cache` sets HTTP `Cache-Control` header on `tools/call` response
/// - `.tool_cache()` updates cache settings for registered tools
/// - Responses have status 200 OK, valid ETag, and matching JSON-RPC payload
#[tokio::test]
async fn test_tools_call_with_tool_caching_directives() {
    use mcp_routing::types::mcp::CacheScope;

    let app = McpRouter::new(common::sample_server_info())
        .register_tool_with_cache(
            "cached_calculator",
            handle_calculator,
            Some(180_000), // 3 minutes
            Some(CacheScope::Public),
        )
        .register_tool("regular_calculator", handle_calculator);

    // 1. Call cached tool -> expect Cache-Control: public, max-age=180
    let req1 = common::build_request(
        Some("tools/call"),
        Some("cached_calculator"),
        json!({
            "jsonrpc": "2.0",
            "id": "cache-calc-1",
            "method": "tools/call",
            "params": {
                "name": "cached_calculator",
                "arguments": { "a": 5, "b": 3, "operation": "add" }
            }
        }),
    );
    let (status1, headers1, body1) = common::execute_request(app.clone(), req1).await;
    assert_eq!(status1, StatusCode::OK);
    assert_eq!(
        headers1.get("cache-control").unwrap().to_str().unwrap(),
        "public, max-age=180"
    );
    assert!(headers1.contains_key("etag"));

    let res1: CallToolResultResponse = serde_json::from_value(body1).unwrap();
    assert_eq!(res1.result.is_error, Some(false));
    if let ContentBlock::Text(ref t) = res1.result.content[0] {
        assert_eq!(t.text, "8");
    }

    // 2. Call regular tool -> expect ETag but no Cache-Control header
    let req2 = common::build_request(
        Some("tools/call"),
        Some("regular_calculator"),
        json!({
            "jsonrpc": "2.0",
            "id": "reg-calc-2",
            "method": "tools/call",
            "params": {
                "name": "regular_calculator",
                "arguments": { "a": 5, "b": 3, "operation": "add" }
            }
        }),
    );
    let (status2, headers2, _) = common::execute_request(app, req2).await;
    assert_eq!(status2, StatusCode::OK);
    assert_eq!(headers2.get("cache-control"), None);
    assert!(headers2.contains_key("etag"));
}

/// Tests that valid arguments matching a complex JSON Schema pass pre-validation and execute the handler.
#[tokio::test]
async fn test_tools_call_schema_pre_validation_success() {
    let schema_tool = Tool {
        name: "create_user".into(),
        title: Some("Create User".into()),
        description: Some("Creates a new user profile".into()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "username": { "type": "string", "pattern": "^[a-z0-9_]{3,16}$" },
                "age": { "type": "integer", "minimum": 18, "maximum": 120 },
                "role": { "type": "string", "enum": ["admin", "editor", "viewer"] }
            },
            "required": ["username", "age", "role"],
            "additionalProperties": false
        }),
        output_schema: None,
        annotations: None,
        meta: None,
        icons: Vec::new(),
    };

    #[derive(Deserialize)]
    struct CreateUserArgs {
        username: String,
        age: u32,
        role: String,
    }

    let app = McpRouter::new(common::sample_server_info()).register_tool(
        schema_tool,
        |args: CreateUserArgs| async move {
            format!("Created user {} ({}) with role {}", args.username, args.age, args.role)
        },
    );

    let req = common::build_request(
        Some("tools/call"),
        Some("create_user"),
        json!({
            "jsonrpc": "2.0",
            "id": "val-success-1",
            "method": "tools/call",
            "params": {
                "name": "create_user",
                "arguments": {
                    "username": "alice_99",
                    "age": 25,
                    "role": "admin"
                }
            }
        }),
    );

    let (status, _, body) = common::execute_request(app, req).await;
    assert_eq!(status, StatusCode::OK);

    let res: CallToolResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res.result.is_error, Some(false));
    if let ContentBlock::Text(ref t) = res.result.content[0] {
        assert_eq!(t.text, "Created user alice_99 (25) with role admin");
    }
}

/// Tests that invalid arguments violating JSON Schema constraints are rejected before invoking the handler.
#[tokio::test]
async fn test_tools_call_schema_pre_validation_failures() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let handler_called = Arc::new(AtomicBool::new(false));
    let handler_called_clone = Arc::clone(&handler_called);

    let schema_tool = Tool {
        name: "create_user".into(),
        title: Some("Create User".into()),
        description: Some("Creates a new user profile".into()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "username": { "type": "string", "pattern": "^[a-z0-9_]{3,16}$" },
                "age": { "type": "integer", "minimum": 18, "maximum": 120 },
                "role": { "type": "string", "enum": ["admin", "editor", "viewer"] }
            },
            "required": ["username", "age", "role"],
            "additionalProperties": false
        }),
        output_schema: None,
        annotations: None,
        meta: None,
        icons: Vec::new(),
    };

    #[allow(dead_code)]
    #[derive(Deserialize)]
    struct CreateUserArgs {
        username: String,
        age: u32,
        role: String,
    }

    let app = McpRouter::new(common::sample_server_info()).register_tool(
        schema_tool,
        move |_args: CreateUserArgs| {
            let flag = Arc::clone(&handler_called_clone);
            async move {
                flag.store(true, Ordering::SeqCst);
                "should never reach here"
            }
        },
    );

    // 1. Missing required field "role"
    let req1 = common::build_request(
        Some("tools/call"),
        Some("create_user"),
        json!({
            "jsonrpc": "2.0",
            "id": "val-fail-1",
            "method": "tools/call",
            "params": {
                "name": "create_user",
                "arguments": {
                    "username": "alice",
                    "age": 30
                }
            }
        }),
    );
    let (status1, _, body1) = common::execute_request(app.clone(), req1).await;
    assert_eq!(status1, StatusCode::OK);
    let res1: CallToolResultResponse = serde_json::from_value(body1).unwrap();
    assert_eq!(res1.result.is_error, Some(true));
    if let ContentBlock::Text(ref t) = res1.result.content[0] {
        assert!(t.text.starts_with("Input schema validation failed:"));
        assert!(t.text.contains("role"));
    }
    assert!(!handler_called.load(Ordering::SeqCst));

    // 2. Out-of-range "age" (below minimum 18)
    let req2 = common::build_request(
        Some("tools/call"),
        Some("create_user"),
        json!({
            "jsonrpc": "2.0",
            "id": "val-fail-2",
            "method": "tools/call",
            "params": {
                "name": "create_user",
                "arguments": {
                    "username": "bob_underage",
                    "age": 12,
                    "role": "editor"
                }
            }
        }),
    );
    let (status2, _, body2) = common::execute_request(app.clone(), req2).await;
    assert_eq!(status2, StatusCode::OK);
    let res2: CallToolResultResponse = serde_json::from_value(body2).unwrap();
    assert_eq!(res2.result.is_error, Some(true));
    if let ContentBlock::Text(ref t) = res2.result.content[0] {
        assert!(t.text.starts_with("Input schema validation failed:"));
        assert!(t.text.contains("18"));
    }
    assert!(!handler_called.load(Ordering::SeqCst));

    // 3. Invalid enum value for "role"
    let req3 = common::build_request(
        Some("tools/call"),
        Some("create_user"),
        json!({
            "jsonrpc": "2.0",
            "id": "val-fail-3",
            "method": "tools/call",
            "params": {
                "name": "create_user",
                "arguments": {
                    "username": "charlie",
                    "age": 40,
                    "role": "superadmin"
                }
            }
        }),
    );
    let (status3, _, body3) = common::execute_request(app.clone(), req3).await;
    assert_eq!(status3, StatusCode::OK);
    let res3: CallToolResultResponse = serde_json::from_value(body3).unwrap();
    assert_eq!(res3.result.is_error, Some(true));
    if let ContentBlock::Text(ref t) = res3.result.content[0] {
        assert!(t.text.starts_with("Input schema validation failed:"));
    }
    assert!(!handler_called.load(Ordering::SeqCst));

    // 4. Invalid pattern for "username"
    let req4 = common::build_request(
        Some("tools/call"),
        Some("create_user"),
        json!({
            "jsonrpc": "2.0",
            "id": "val-fail-4",
            "method": "tools/call",
            "params": {
                "name": "create_user",
                "arguments": {
                    "username": "INVALID USERNAME WITH SPACES!",
                    "age": 25,
                    "role": "viewer"
                }
            }
        }),
    );
    let (status4, _, body4) = common::execute_request(app.clone(), req4).await;
    assert_eq!(status4, StatusCode::OK);
    let res4: CallToolResultResponse = serde_json::from_value(body4).unwrap();
    assert_eq!(res4.result.is_error, Some(true));
    if let ContentBlock::Text(ref t) = res4.result.content[0] {
        assert!(t.text.starts_with("Input schema validation failed:"));
    }
    assert!(!handler_called.load(Ordering::SeqCst));

    // 5. Additional disallowed property
    let req5 = common::build_request(
        Some("tools/call"),
        Some("create_user"),
        json!({
            "jsonrpc": "2.0",
            "id": "val-fail-5",
            "method": "tools/call",
            "params": {
                "name": "create_user",
                "arguments": {
                    "username": "dave_ok",
                    "age": 35,
                    "role": "viewer",
                    "unauthorized_extra_field": true
                }
            }
        }),
    );
    let (status5, _, body5) = common::execute_request(app.clone(), req5).await;
    assert_eq!(status5, StatusCode::OK);
    let res5: CallToolResultResponse = serde_json::from_value(body5).unwrap();
    assert_eq!(res5.result.is_error, Some(true));
    if let ContentBlock::Text(ref t) = res5.result.content[0] {
        assert!(t.text.starts_with("Input schema validation failed:"));
        assert!(t.text.contains("unauthorized_extra_field"));
    }
    assert!(!handler_called.load(Ordering::SeqCst));
}
