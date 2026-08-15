// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

mod common;

use common::{build_request, execute_request, sample_server_info};
use http::{Request, StatusCode};
use mcp_routing::{
    InputResponses, IntoPromptResult, IntoResourceResult, IntoToolResult, McpRouter, PromptError,
    RequestContext, RequestState, ResourceError, ToolError,
    types::mcp::{
        CompleteArgument, InputRequest, InputRequiredResult,
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
                    let res: Option<ConfirmationResponse> = confirm_resp.get_result().unwrap();
                    if res.map(|r| r.approved).unwrap_or(false) {
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
                "name": "dangerous_exec",
                "arguments": { "action": "wipe_cache" },
                "requestState": "step_1_confirmation",
                "inputResponses": {
                    "confirm_action": {
                        "result": { "approved": true }
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
                assert_eq!(val.unwrap()["answer"], 42);
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
                "name": "multi_step",
                "arguments": {},
                "requestState": "step_token_abc",
                "inputResponses": {
                    "sample_step": {
                        "result": { "answer": 42 }
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
                let name = user_context.unwrap()["name"].as_str().unwrap().to_string();
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
                "name": "interactive_prompt",
                "requestState": "prompt_state_1",
                "inputResponses": {
                    "user_name": {
                        "result": { "name": "Alice" }
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
                assert!(roots.is_some());
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
                    "uri": "custom://secure-data",
                    "requestState": "resource_auth_token_99",
                    "inputResponses": {
                        "roots_request": {
                            "result": {
                                "roots": [{"uri": "custom://workspace", "name": "Main"}]
                            }
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
