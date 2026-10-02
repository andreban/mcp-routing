// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Multi-Round-Trip (MRTR) Protocol Integration Tests
//!
//! Verifies stateless Multi-Round-Trip Request (MRTR) semantics across Model Context Protocol (MCP) endpoints:
//! - Interactive elicitation and sampling requests (`inputRequests`, `requestState`, `inputResponses`)
//! - Extractor-based access to `RequestState` and `InputResponses` in handlers
//! - Load-shedding patterns via `InputRequiredResult::load_shed`
//! - Result tagging (`resultType: complete` vs `resultType: input_required`)
//! - Caching rules: `input_required` and retry results carry `Cache-Control: no-store`
//! - Multi-round-trip flows across `tools/call`, `prompts/get`, `resources/read`, and `completion/complete`

mod common;

use common::{build_request, execute_request, sample_server_info};
use http::{Request, StatusCode};
use stateless_mcp::{
    InputResponses, IntoPromptResult, IntoResourceResult, IntoToolResult, McpRouter, PromptError,
    RequestContext, RequestState, ResourceError, ToolError,
    types::mcp::{
        CacheScope, CompleteArgument, InputRequest, InputRequiredResult,
        prompts::{Prompt, PromptArgument, get::GetPromptResult},
        resources::read::ReadResourceResult,
        tools::{Tool, call::CallToolResult},
    },
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize)]
struct ConfirmationResponse {
    approved: bool,
}

/// Client `ElicitResult` for an elicitation whose form content is `T`.
#[derive(Serialize, Deserialize)]
struct ElicitResult<T> {
    action: String,
    content: Option<T>,
}

/// Tests multi-round-trip tool execution with user confirmation elicitation.
#[tokio::test]
async fn test_tool_call_multi_round_trip_elicitation() {
    let server_info = sample_server_info();

    let tool = Tool {
        icons: Vec::new(),
        name: "dangerous_exec".to_string(),
        title: Some("Dangerous Execution".to_string()),
        description: Some("Executes an action with user confirmation".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": { "action": { "type": "string" } },
            "required": ["action"]
        }),
        output_schema: None,
        annotations: None,
        meta: None,
    };

    let router =
        McpRouter::new(server_info).register_tool(tool, |ctx: RequestContext| async move {
            let state = ctx.request_state();
            let responses = ctx.input_responses();

            match state {
                Some("step_1_confirmation") => {
                    let responses = responses.expect("missing responses on retry");
                    let confirm_resp = responses
                        .get("confirm_action")
                        .expect("missing confirm response");
                    let res: ElicitResult<ConfirmationResponse> =
                        confirm_resp.get_result().unwrap();
                    if res.action == "accept" && res.content.is_some_and(|c| c.approved) {
                        CallToolResult::text("Action executed successfully after confirmation")
                    } else {
                        CallToolResult::error("Action rejected by user")
                    }
                }
                _ => {
                    let elicitation_req = InputRequest::elicitation(&json!({
                        "message": "Are you sure you want to execute this dangerous action?",
                        "requestedSchema": {
                            "type": "object",
                            "properties": { "approved": { "type": "boolean" } },
                            "required": ["approved"]
                        }
                    }))
                    .unwrap();

                    InputRequiredResult::new()
                        .with_request_state("step_1_confirmation")
                        .with_input_request("confirm_action", elicitation_req)
                        .into_tool_result()
                }
            }
        });

    // 1. Initial Request (Round 1)
    let req1 = build_request(
        Some("tools/call"),
        Some("dangerous_exec"),
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "_meta": common::meta(),
                "name": "dangerous_exec",
                "arguments": { "action": "wipe_cache" }
            }
        }),
    );

    let (status1, _, json1) = execute_request(router.clone(), req1).await;
    assert_eq!(status1, StatusCode::OK);
    assert_eq!(json1["result"]["resultType"], "input_required");
    assert_eq!(json1["result"]["requestState"], "step_1_confirmation");
    assert_eq!(
        json1["result"]["inputRequests"]["confirm_action"]["method"],
        "elicitation/create"
    );

    // 2. Retry Request with InputResponses and RequestState (Round 2)
    let req2 = build_request(
        Some("tools/call"),
        Some("dangerous_exec"),
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "_meta": common::meta(),
                "name": "dangerous_exec",
                "arguments": { "action": "wipe_cache" },
                "requestState": "step_1_confirmation",
                "inputResponses": {
                    "confirm_action": {
                        "action": "accept",
                        "content": { "approved": true }
                    }
                }
            }
        }),
    );

    let (status2, _, json2) = execute_request(router, req2).await;
    assert_eq!(status2, StatusCode::OK);
    assert_eq!(json2["result"]["resultType"], "complete");
    assert_eq!(json2["result"]["isError"], false);
    assert_eq!(
        json2["result"]["content"][0]["text"],
        "Action executed successfully after confirmation"
    );
}

/// Tests MRTR tool execution utilizing `RequestState` and `InputResponses` extractors.
#[tokio::test]
async fn test_tool_call_mrtr_with_extractors() {
    let server_info = sample_server_info();

    let tool = Tool {
        icons: Vec::new(),
        name: "multi_step".to_string(),
        title: Some("Multi Step Tool".to_string()),
        description: Some("Uses extractors for MRTR".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": { "step": { "type": "integer" } }
        }),
        output_schema: None,
        annotations: None,
        meta: None,
    };

    let router = McpRouter::new(server_info).register_tool(
        tool,
        |state: Option<RequestState>, responses: Option<InputResponses>| async move {
            if let Some(state) = state {
                assert_eq!(state.as_str(), "step_token_abc");
                let responses = responses.expect("responses should be extracted");
                let val: Option<serde_json::Value> = responses.get_result("sample_step").unwrap();
                assert_eq!(val.unwrap()["content"]["text"], "42");
                Ok::<_, ToolError>(CallToolResult::text("Final answer computed: 42"))
            } else {
                let sampling_req = InputRequest::sampling(&json!({
                    "messages": [{"role": "user", "content": {"type": "text", "text": "Compute 6 * 7"}}]
                }))
                .unwrap();

                Ok(InputRequiredResult::new()
                    .with_request_state("step_token_abc")
                    .with_input_request("sample_step", sampling_req)
                    .into_tool_result())
            }
        },
    );

    // Initial call
    let req1 = build_request(
        Some("tools/call"),
        Some("multi_step"),
        json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "tools/call",
            "params": {
                "_meta": common::meta(),
                "name": "multi_step",
                "arguments": {}
            }
        }),
    );

    let (status1, _, json1) = execute_request(router.clone(), req1).await;
    assert_eq!(status1, StatusCode::OK);
    assert_eq!(json1["result"]["resultType"], "input_required");
    assert_eq!(json1["result"]["requestState"], "step_token_abc");
    assert_eq!(
        json1["result"]["inputRequests"]["sample_step"]["method"],
        "sampling/createMessage"
    );

    // Resume call
    let req2 = build_request(
        Some("tools/call"),
        Some("multi_step"),
        json!({
            "jsonrpc": "2.0",
            "id": 11,
            "method": "tools/call",
            "params": {
                "_meta": common::meta(),
                "name": "multi_step",
                "arguments": {},
                "requestState": "step_token_abc",
                "inputResponses": {
                    "sample_step": {
                        "role": "assistant",
                        "content": { "type": "text", "text": "42" },
                        "model": "test-model",
                        "stopReason": "endTurn"
                    }
                }
            }
        }),
    );

    let (status2, _, json2) = execute_request(router, req2).await;
    assert_eq!(status2, StatusCode::OK);
    assert_eq!(json2["result"]["resultType"], "complete");
    assert_eq!(
        json2["result"]["content"][0]["text"],
        "Final answer computed: 42"
    );
}

/// Tests MRTR load shedding using `InputRequiredResult::load_shed` and request resumption.
#[tokio::test]
async fn test_load_shedding_mrtr() {
    let server_info = sample_server_info();

    let tool = Tool {
        icons: Vec::new(),
        name: "busy_tool".to_string(),
        title: Some("Busy Tool".to_string()),
        description: Some("Performs load shedding".to_string()),
        input_schema: json!({ "type": "object" }),
        output_schema: None,
        annotations: None,
        meta: None,
    };

    let router =
        McpRouter::new(server_info).register_tool(tool, |state: Option<RequestState>| async move {
            if let Some(state) = state {
                assert_eq!(state.as_str(), "ticket_shed_888");
                CallToolResult::text("Processed after load shedding resumption")
            } else {
                InputRequiredResult::load_shed("ticket_shed_888").into_tool_result()
            }
        });

    // Initial Request
    let req1 = build_request(
        Some("tools/call"),
        Some("busy_tool"),
        json!({
            "jsonrpc": "2.0",
            "id": 20,
            "method": "tools/call",
            "params": {
                "_meta": common::meta(),
                "name": "busy_tool",
                "arguments": {}
            }
        }),
    );

    let (status1, _, json1) = execute_request(router.clone(), req1).await;
    assert_eq!(status1, StatusCode::OK);
    assert_eq!(json1["result"]["resultType"], "input_required");
    assert_eq!(json1["result"]["requestState"], "ticket_shed_888");
    assert!(json1["result"].get("inputRequests").is_none());

    // Resumed Request
    let req2 = build_request(
        Some("tools/call"),
        Some("busy_tool"),
        json!({
            "jsonrpc": "2.0",
            "id": 21,
            "method": "tools/call",
            "params": {
                "_meta": common::meta(),
                "name": "busy_tool",
                "arguments": {},
                "requestState": "ticket_shed_888"
            }
        }),
    );

    let (status2, _, json2) = execute_request(router, req2).await;
    assert_eq!(status2, StatusCode::OK);
    assert_eq!(json2["result"]["resultType"], "complete");
    assert_eq!(
        json2["result"]["content"][0]["text"],
        "Processed after load shedding resumption"
    );
}

/// Tests that completion result payloads include the `resultType: complete` tag.
#[tokio::test]
async fn test_completion_complete_result_type() {
    let server_info = sample_server_info();

    let router = McpRouter::new(server_info)
        .register_prompt_completion("generate_code", |_arg: CompleteArgument| async move {
            vec!["rust", "python", "typescript"]
        });

    let req = build_request(
        Some("completion/complete"),
        None,
        json!({
            "jsonrpc": "2.0",
            "id": 30,
            "method": "completion/complete",
            "params": {
                "_meta": common::meta(),
                "ref": {
                    "type": "ref/prompt",
                    "name": "generate_code"
                },
                "argument": {
                    "name": "language",
                    "value": "ru"
                }
            }
        }),
    );

    let (status, _, json_res) = execute_request(router, req).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json_res["result"]["resultType"], "complete");
    assert_eq!(json_res["result"]["completion"]["values"][0], "rust");
    assert_eq!(json_res["result"]["completion"]["values"][1], "python");
    assert_eq!(json_res["result"]["completion"]["values"][2], "typescript");
}

/// Tests MRTR flow for prompt generation with interactive parameter elicitation.
#[tokio::test]
async fn test_prompts_get_mrtr() {
    let server_info = sample_server_info();

    let prompt = Prompt {
        name: "interactive_prompt".to_string(),
        title: None,
        description: None,
        arguments: vec![PromptArgument::new("topic")],
        icons: Vec::new(),
        meta: None,
    };

    let router = McpRouter::new(server_info).register_prompt(
        prompt,
        |state: Option<RequestState>, responses: Option<InputResponses>| async move {
            if let Some(state) = state {
                assert_eq!(state.as_str(), "prompt_state_1");
                let responses = responses.expect("responses required");
                let user_context: Option<serde_json::Value> =
                    responses.get_result("user_name").unwrap();
                let user_context = user_context.unwrap();
                assert_eq!(user_context["action"], "accept");
                let name = user_context["content"]["name"].as_str().unwrap().to_string();
                Ok::<_, PromptError>(GetPromptResult::user(format!("Hello, {name}!")))
            } else {
                let elicit = InputRequest::elicitation(&json!({
                    "message": "What is your name?"
                }))
                .unwrap();

                InputRequiredResult::new()
                    .with_request_state("prompt_state_1")
                    .with_input_request("user_name", elicit)
                    .into_prompt_result()
            }
        },
    );

    // 1. Initial Get Prompt
    let req1 = build_request(
        Some("prompts/get"),
        Some("interactive_prompt"),
        json!({
            "jsonrpc": "2.0",
            "id": 40,
            "method": "prompts/get",
            "params": {
                "_meta": common::meta(),
                "name": "interactive_prompt"
            }
        }),
    );

    let (status1, _, json1) = execute_request(router.clone(), req1).await;
    assert_eq!(status1, StatusCode::OK);
    assert_eq!(json1["result"]["resultType"], "input_required");
    assert_eq!(json1["result"]["requestState"], "prompt_state_1");
    assert!(json1["result"].get("messages").is_none());

    // 2. Retry Get Prompt
    let req2 = build_request(
        Some("prompts/get"),
        Some("interactive_prompt"),
        json!({
            "jsonrpc": "2.0",
            "id": 41,
            "method": "prompts/get",
            "params": {
                "_meta": common::meta(),
                "name": "interactive_prompt",
                "requestState": "prompt_state_1",
                "inputResponses": {
                    "user_name": {
                        "action": "accept",
                        "content": { "name": "Alice" }
                    }
                }
            }
        }),
    );

    let (status2, _, json2) = execute_request(router, req2).await;
    assert_eq!(status2, StatusCode::OK);
    assert_eq!(json2["result"]["resultType"], "complete");
    assert_eq!(
        json2["result"]["messages"][0]["content"]["text"],
        "Hello, Alice!"
    );
}

/// Tests MRTR flow for reading secure resources with client roots elicitation.
#[tokio::test]
async fn test_resources_read_mrtr() {
    let server_info = sample_server_info();

    let router = McpRouter::new(server_info).register_resource(
        ("custom://secure-data", "Secure Data"),
        |state: Option<RequestState>, responses: Option<InputResponses>| async move {
            if let Some(state) = state {
                assert_eq!(state.as_str(), "resource_auth_token_99");
                let responses = responses.expect("responses required");
                let roots: Option<serde_json::Value> =
                    responses.get_result("roots_request").unwrap();
                assert_eq!(roots.unwrap()["roots"][0]["uri"], "custom://workspace");
                Ok::<_, ResourceError>(ReadResourceResult::text(
                    "custom://secure-data",
                    "Confidential content unlocked",
                    None::<String>,
                ))
            } else {
                let roots_req = InputRequest::roots();
                InputRequiredResult::new()
                    .with_request_state("resource_auth_token_99")
                    .with_input_request("roots_request", roots_req)
                    .into_resource_result("custom://secure-data", None, None)
            }
        },
    );

    // 1. Initial Read Resource
    let req1 = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "resources/read")
        .header("Mcp-Uri", "custom://secure-data")
        .body(axum::body::Body::from(
            json!({
                "jsonrpc": "2.0",
                "id": 50,
                "method": "resources/read",
                "params": {
                    "_meta": common::meta(),
                    "uri": "custom://secure-data"
                }
            })
            .to_string(),
        ))
        .unwrap();

    let (status1, _, json1) = execute_request(router.clone(), req1).await;
    assert_eq!(status1, StatusCode::OK);
    assert_eq!(json1["result"]["resultType"], "input_required");
    assert_eq!(json1["result"]["requestState"], "resource_auth_token_99");
    assert_eq!(
        json1["result"]["inputRequests"]["roots_request"]["method"],
        "roots/list"
    );
    assert!(json1["result"].get("contents").is_none());

    // 2. Retry Read Resource
    let req2 = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "resources/read")
        .header("Mcp-Uri", "custom://secure-data")
        .body(axum::body::Body::from(
            json!({
                "jsonrpc": "2.0",
                "id": 51,
                "method": "resources/read",
                "params": {
                    "_meta": common::meta(),
                    "uri": "custom://secure-data",
                    "requestState": "resource_auth_token_99",
                    "inputResponses": {
                        "roots_request": {
                            "roots": [{"uri": "custom://workspace", "name": "Main"}]
                        }
                    }
                }
            })
            .to_string(),
        ))
        .unwrap();

    let (status2, _, json2) = execute_request(router, req2).await;
    assert_eq!(status2, StatusCode::OK);
    assert_eq!(json2["result"]["resultType"], "complete");
    assert_eq!(
        json2["result"]["contents"][0]["text"],
        "Confidential content unlocked"
    );
}

/// Builds a router with a resource cached for 60 seconds that requires a roots round trip.
fn cached_mrtr_router() -> McpRouter {
    McpRouter::new(sample_server_info())
        .register_resource_with_cache(
            ("custom://cached", "Cached"),
            |state: Option<RequestState>| async move {
                if state.is_some() {
                    Ok::<_, ResourceError>(
                        ReadResourceResult::text("custom://cached", "unlocked", None::<String>)
                            .with_cache(Some(60_000), Some(CacheScope::Public)),
                    )
                } else {
                    InputRequiredResult::new()
                        .with_request_state("cached_state")
                        .with_input_request("roots", InputRequest::roots())
                        .into_resource_result("custom://cached", None, None)
                }
            },
            Some(60_000),
            Some(CacheScope::Public),
        )
        .register_resource_with_cache(
            ("custom://plain", "Plain"),
            || async {
                ReadResourceResult::text("custom://plain", "plain", None::<String>)
                    .with_cache(Some(30_000), Some(CacheScope::Private))
            },
            Some(60_000),
            Some(CacheScope::Public),
        )
}

/// Builds a `resources/read` request for `uri` with optional extra params.
fn read_request(uri: &str, extra: serde_json::Value) -> Request<axum::body::Body> {
    let mut params = json!({ "_meta": common::meta(), "uri": uri });
    if let Some(extra) = extra.as_object() {
        for (key, value) in extra {
            params[key] = value.clone();
        }
    }
    build_request(
        Some("resources/read"),
        Some(uri),
        json!({ "jsonrpc": "2.0", "id": 1, "method": "resources/read", "params": params }),
    )
}

/// Tests that an `input_required` result carries no caching hints and is sent with `no-store`.
///
/// Verifies:
/// - `ttlMs` and `cacheScope` are omitted from the interim result
/// - The HTTP response has `Cache-Control: no-store` and no `ETag`
#[tokio::test]
async fn test_input_required_result_is_not_cacheable() {
    let req = read_request("custom://cached", json!({}));
    let (status, headers, body) = execute_request(cached_mrtr_router(), req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"]["resultType"], "input_required");
    assert!(body["result"].get("ttlMs").is_none());
    assert!(body["result"].get("cacheScope").is_none());
    assert_eq!(headers.get("cache-control").unwrap(), "no-store");
    assert!(headers.get("etag").is_none());
}

/// Tests that the result of a multi round-trip retry is sent with `no-store` and `ttlMs: 0`.
#[tokio::test]
async fn test_retry_result_is_not_cacheable() {
    let req = read_request(
        "custom://cached",
        json!({
            "requestState": "cached_state",
            "inputResponses": { "roots": { "roots": [] } }
        }),
    );
    let (status, headers, body) = execute_request(cached_mrtr_router(), req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"]["resultType"], "complete");
    assert_eq!(body["result"]["ttlMs"], 0);
    assert_eq!(headers.get("cache-control").unwrap(), "no-store");
    assert!(headers.get("etag").is_none());
}

/// Tests that HTTP caching directives follow the result's own `ttlMs` and `cacheScope` hints.
#[tokio::test]
async fn test_cache_headers_match_result_hints() {
    let req = read_request("custom://plain", json!({}));
    let (status, headers, body) = execute_request(cached_mrtr_router(), req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"]["ttlMs"], 30_000);
    assert_eq!(body["result"]["cacheScope"], "private");
    assert_eq!(
        headers.get("cache-control").unwrap(),
        "private, max-age=30"
    );
}
