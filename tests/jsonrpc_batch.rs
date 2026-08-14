// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # JSON-RPC 2.0 Batch Requests & Protocol Compliance Integration Tests
//!
//! Verifies complete JSON-RPC 2.0 specification compliance for batch requests and notifications:
//! - Standard batch processing with concurrent execution across all MCP endpoints
//! - Mixed batch containing calls, notifications, spec errors, and invalid objects (Spec Example 10)
//! - All-notification batches returning HTTP 204 No Content (Spec Example 11)
//! - Single notification requests returning HTTP 204 No Content
//! - Empty batch array `[]` returning a single Invalid Request error (Spec Example 7)
//! - Invalid batch elements like `[1]` and `[1, 2, 3]` (Spec Examples 8 & 9)
//! - Malformed batch JSON syntax returning a single Parse Error (Spec Example 6)
//! - Top-level JSON primitive payloads returning Invalid Request
//! - Correlation header (`Mcp-Session-Id`) and shared state propagation across batch calls
//! - Header fallback for batch items omitting `method` or `name`

mod common;

use axum::body::Body;
use http::{Request, StatusCode};
use mcp_routing::{
    McpRouter,
    extract::{Extension, SessionId},
    types::jsonrpc::{
        INVALID_PARAMS_CODE, INVALID_REQUEST_CODE, JsonRpcErrorResponse, METHOD_NOT_FOUND_CODE,
        PARSE_ERROR_CODE,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Deserialize, Serialize)]
struct EchoArgs {
    message: String,
}

async fn echo_tool(args: EchoArgs) -> Result<String, String> {
    Ok(format!("echo: {}", args.message))
}

#[derive(Clone)]
struct AppState {
    app_name: String,
}

async fn stateful_tool(
    session: Option<SessionId>,
    Extension(state): Extension<AppState>,
    args: EchoArgs,
) -> Result<String, String> {
    let sid = session
        .map(|s| s.to_string())
        .unwrap_or_else(|| "none".to_string());
    Ok(format!("{}: [{}] -> {}", state.app_name, sid, args.message))
}

/// Helper creating a standard router with tools and prompts configured.
fn create_test_app() -> McpRouter {
    McpRouter::new(common::sample_server_info())
        .register_tool("echo", echo_tool)
        .register_prompt("greeting", || async { "Hello from prompt!" })
}

/// Tests that a standard batch with multiple distinct MCP requests executes all calls and returns an array of matching responses.
#[tokio::test]
async fn test_batch_all_successful_requests() {
    let app = create_test_app();

    let batch_req = json!([
        {
            "jsonrpc": "2.0",
            "id": "req-1",
            "method": "server/discover"
        },
        {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        },
        {
            "jsonrpc": "2.0",
            "id": "req-3",
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": {
                    "message": "batch hello"
                }
            }
        },
        {
            "jsonrpc": "2.0",
            "id": 4,
            "method": "prompts/list"
        },
        {
            "jsonrpc": "2.0",
            "id": "req-5",
            "method": "prompts/get",
            "params": {
                "name": "greeting"
            }
        }
    ]);

    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(batch_req.to_string()))
        .unwrap();

    let (status, headers, body) = common::execute_request(app, req).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get("content-type").unwrap().to_str().unwrap(),
        "application/json"
    );

    let arr = body.as_array().expect("Response must be a JSON array");
    assert_eq!(arr.len(), 5);

    // 1. server/discover response
    assert_eq!(arr[0]["jsonrpc"], "2.0");
    assert_eq!(arr[0]["id"], "req-1");
    assert!(arr[0]["result"]["capabilities"].is_object());

    // 2. tools/list response
    assert_eq!(arr[1]["jsonrpc"], "2.0");
    assert_eq!(arr[1]["id"], 2.0);
    assert_eq!(arr[1]["result"]["tools"].as_array().unwrap().len(), 1);

    // 3. tools/call response
    assert_eq!(arr[2]["jsonrpc"], "2.0");
    assert_eq!(arr[2]["id"], "req-3");
    assert_eq!(arr[2]["result"]["content"][0]["text"], "echo: batch hello");

    // 4. prompts/list response
    assert_eq!(arr[3]["jsonrpc"], "2.0");
    assert_eq!(arr[3]["id"], 4.0);
    assert_eq!(arr[3]["result"]["prompts"].as_array().unwrap().len(), 1);

    // 5. prompts/get response
    assert_eq!(arr[4]["jsonrpc"], "2.0");
    assert_eq!(arr[4]["id"], "req-5");
    assert_eq!(
        arr[4]["result"]["messages"][0]["content"]["text"],
        "Hello from prompt!"
    );
}

/// Tests a mixed batch of method calls, notifications, method-not-found errors, invalid objects, and bad params.
///
/// Corresponds to JSON-RPC 2.0 Specification Example 10:
/// - Notifications must NOT produce a response in the array.
/// - Invalid non-method objects produce Invalid Request (-32600) with `id: null`.
/// - Method not found produces (-32601) with the request ID.
/// - Invalid params produce (-32602) with the request ID.
/// - Successful requests produce standard result responses.
#[tokio::test]
async fn test_batch_mixed_calls_notifications_and_errors() {
    let app = create_test_app();

    let batch_req = json!([
        // 1. Valid tool call
        {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": { "message": "msg1" }
            }
        },
        // 2. Notification (no id) -> Should NOT produce a response
        {
            "jsonrpc": "2.0",
            "method": "notifications/ping",
            "params": { "data": 123 }
        },
        // 3. Unknown method
        {
            "jsonrpc": "2.0",
            "id": "err-unknown",
            "method": "non_existent_method"
        },
        // 4. Invalid object missing method and id
        {
            "foo": "boo"
        },
        // 5. Invalid params (tools/call without tool name)
        {
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "arguments": { "message": "missing name" }
            }
        },
        // 6. Valid server/discover
        {
            "jsonrpc": "2.0",
            "id": "disc-9",
            "method": "server/discover"
        }
    ]);

    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(batch_req.to_string()))
        .unwrap();

    let (status, _, body) = common::execute_request(app, req).await;
    assert_eq!(status, StatusCode::OK);

    let arr = body.as_array().expect("Response must be a JSON array");
    // 6 items sent, 1 was a notification -> 5 responses returned
    assert_eq!(arr.len(), 5);

    // Response 1: Success (id: 1)
    assert_eq!(arr[0]["id"], 1.0);
    assert_eq!(arr[0]["result"]["content"][0]["text"], "echo: msg1");

    // Response 2: Method Not Found (id: "err-unknown")
    assert_eq!(arr[1]["id"], "err-unknown");
    assert_eq!(arr[1]["error"]["code"], METHOD_NOT_FOUND_CODE);

    // Response 3: Invalid Request for {"foo":"boo"} (id: null)
    assert_eq!(arr[2]["id"], serde_json::Value::Null);
    assert_eq!(arr[2]["error"]["code"], INVALID_REQUEST_CODE);

    // Response 4: Invalid Params for missing tool name (id: 5)
    assert_eq!(arr[3]["id"], 5.0);
    assert_eq!(arr[3]["error"]["code"], INVALID_PARAMS_CODE);

    // Response 5: Success for server/discover (id: "disc-9")
    assert_eq!(arr[4]["id"], "disc-9");
    assert!(arr[4]["result"]["capabilities"].is_object());
}

/// Tests that a batch containing exclusively notifications returns HTTP 204 No Content with an empty body.
///
/// Corresponds to JSON-RPC 2.0 Specification Example 11.
#[tokio::test]
async fn test_batch_all_notifications_returns_204() {
    let app = create_test_app();

    let batch_req = json!([
        {
            "jsonrpc": "2.0",
            "method": "notifications/notify_one",
            "params": { "val": 1 }
        },
        {
            "jsonrpc": "2.0",
            "method": "notifications/notify_two",
            "params": { "val": 2 }
        }
    ]);

    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(batch_req.to_string()))
        .unwrap();

    let (status, _, body_bytes) = common::execute_request_raw(app, req).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert!(body_bytes.is_empty());
}

/// Tests that a single notification request returns HTTP 202 Accepted with an empty body.
#[tokio::test]
async fn test_single_notification_returns_202() {
    let app = create_test_app();

    let notif_req = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(notif_req.to_string()))
        .unwrap();

    let (status, _, body_bytes) = common::execute_request_raw(app, req).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert!(body_bytes.is_empty());
}

/// Tests that an empty batch array `[]` returns a single Invalid Request (-32600) error object with `id: null`.
///
/// Corresponds to JSON-RPC 2.0 Specification Example 7:
/// --> []
/// <-- {"jsonrpc": "2.0", "error": {"code": -32600, "message": "Invalid Request"}, "id": null}
#[tokio::test]
async fn test_empty_batch_array_returns_invalid_request() {
    let app = create_test_app();

    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from("[]"))
        .unwrap();

    let (status, _, body) = common::execute_request(app, req).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        body.is_object(),
        "Empty batch must return a single JSON object, NOT an array"
    );

    let err_resp: JsonRpcErrorResponse = serde_json::from_value(body).unwrap();
    assert_eq!(err_resp.jsonrpc, "2.0");
    assert_eq!(err_resp.id, None);
    assert_eq!(err_resp.error.code.code(), INVALID_REQUEST_CODE);
}

/// Tests invalid batch arrays containing non-object primitives.
///
/// Corresponds to JSON-RPC 2.0 Specification Examples 8 & 9:
/// - `[1]` -> `[ {"jsonrpc": "2.0", "error": {"code": -32600, "message": "Invalid Request"}, "id": null} ]`
/// - `[1, 2, 3]` -> Array of 3 error objects
/// - `[1, {"jsonrpc": "2.0", "method": "tools/list", "id": 1}]` -> Mixed invalid primitive and valid call
#[tokio::test]
async fn test_batch_invalid_primitive_elements() {
    let app = create_test_app();

    // 1. [1]
    let req1 = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from("[1]"))
        .unwrap();

    let (status1, _, body1) = common::execute_request(app.clone(), req1).await;
    assert_eq!(status1, StatusCode::OK);
    let arr1 = body1.as_array().expect("Must return a JSON array");
    assert_eq!(arr1.len(), 1);
    assert_eq!(arr1[0]["jsonrpc"], "2.0");
    assert_eq!(arr1[0]["id"], serde_json::Value::Null);
    assert_eq!(arr1[0]["error"]["code"], INVALID_REQUEST_CODE);

    // 2. [1, 2, 3]
    let req2 = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from("[1, 2, 3]"))
        .unwrap();

    let (status2, _, body2) = common::execute_request(app.clone(), req2).await;
    assert_eq!(status2, StatusCode::OK);
    let arr2 = body2.as_array().expect("Must return a JSON array");
    assert_eq!(arr2.len(), 3);
    for item in arr2 {
        assert_eq!(item["jsonrpc"], "2.0");
        assert_eq!(item["id"], serde_json::Value::Null);
        assert_eq!(item["error"]["code"], INVALID_REQUEST_CODE);
    }

    // 3. Mixed [1, valid_request]
    let req3 = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(
            json!([
                1,
                {
                    "jsonrpc": "2.0",
                    "id": "valid-1",
                    "method": "tools/list"
                }
            ])
            .to_string(),
        ))
        .unwrap();

    let (status3, _, body3) = common::execute_request(app, req3).await;
    assert_eq!(status3, StatusCode::OK);
    let arr3 = body3.as_array().expect("Must return a JSON array");
    assert_eq!(arr3.len(), 2);
    assert_eq!(arr3[0]["id"], serde_json::Value::Null);
    assert_eq!(arr3[0]["error"]["code"], INVALID_REQUEST_CODE);
    assert_eq!(arr3[1]["id"], "valid-1");
    assert!(arr3[1]["result"]["tools"].is_array());
}

/// Tests that malformed JSON syntax inside a batch returns a single Parse Error (-32700) with `id: null`.
///
/// Corresponds to JSON-RPC 2.0 Specification Example 6.
#[tokio::test]
async fn test_batch_malformed_json_returns_parse_error() {
    let app = create_test_app();

    let malformed_batch = r#"[
        {"jsonrpc": "2.0", "method": "tools/list", "id": "1"},
        {"jsonrpc": "2.0", "method"
    ]"#;

    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(malformed_batch))
        .unwrap();

    let (status, _, body) = common::execute_request(app, req).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        body.is_object(),
        "Malformed JSON must return a single JSON-RPC error object"
    );

    let err_resp: JsonRpcErrorResponse = serde_json::from_value(body).unwrap();
    assert_eq!(err_resp.jsonrpc, "2.0");
    assert_eq!(err_resp.id, None);
    assert_eq!(err_resp.error.code.code(), PARSE_ERROR_CODE);
}

/// Tests that a top-level JSON primitive payload (e.g. `123` or `"raw string"`) returns Invalid Request (-32600).
#[tokio::test]
async fn test_top_level_primitive_returns_invalid_request() {
    let app = create_test_app();

    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from("12345"))
        .unwrap();

    let (status, _, body) = common::execute_request(app, req).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let err_resp: JsonRpcErrorResponse = serde_json::from_value(body).unwrap();
    assert_eq!(err_resp.jsonrpc, "2.0");
    assert_eq!(err_resp.id, None);
    assert_eq!(err_resp.error.code.code(), INVALID_REQUEST_CODE);
}

/// Tests that `Mcp-Session-Id` header and shared state (`with_state`) propagate correctly to each handler in a batch.
#[tokio::test]
async fn test_batch_session_id_and_state_propagation() {
    let app = McpRouter::new(common::sample_server_info())
        .with_state(AppState {
            app_name: "BATCH_APP".to_string(),
        })
        .register_tool("stateful", stateful_tool);

    let batch_req = json!([
        {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "stateful",
                "arguments": { "message": "call 1" }
            }
        },
        {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "stateful",
                "arguments": { "message": "call 2" }
            }
        }
    ]);

    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Session-Id", "session-batch-777")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(batch_req.to_string()))
        .unwrap();

    let (status, headers, body) = common::execute_request(app, req).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get("mcp-session-id").unwrap().to_str().unwrap(),
        "session-batch-777"
    );

    let arr = body.as_array().expect("Must be JSON array");
    assert_eq!(arr.len(), 2);
    assert_eq!(
        arr[0]["result"]["content"][0]["text"],
        "BATCH_APP: [session-batch-777] -> call 1"
    );
    assert_eq!(
        arr[1]["result"]["content"][0]["text"],
        "BATCH_APP: [session-batch-777] -> call 2"
    );
}

/// Tests header-based fallback inside batch requests when individual elements omit tool name or use header defaults.
#[tokio::test]
async fn test_batch_header_fallback() {
    let app = create_test_app();

    let batch_req = json!([
        // 1. Omit tool name in body -> Falls back to Mcp-Name: echo
        {
            "jsonrpc": "2.0",
            "id": "fallback-1",
            "method": "tools/call",
            "params": {
                "arguments": { "message": "from fallback" }
            }
        },
        // 2. Explicit server/discover method
        {
            "jsonrpc": "2.0",
            "id": "explicit-2",
            "method": "server/discover"
        }
    ]);

    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header("Mcp-Name", "echo")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(batch_req.to_string()))
        .unwrap();

    let (status, _, body) = common::execute_request(app, req).await;
    assert_eq!(status, StatusCode::OK);

    let arr = body.as_array().expect("Must be JSON array");
    assert_eq!(arr.len(), 2);

    // 1. Fallback tool call using Mcp-Name header
    assert_eq!(arr[0]["id"], "fallback-1");
    assert_eq!(
        arr[0]["result"]["content"][0]["text"],
        "echo: from fallback"
    );

    // 2. Explicit server/discover
    assert_eq!(arr[1]["id"], "explicit-2");
    assert!(arr[1]["result"]["capabilities"].is_object());
}
