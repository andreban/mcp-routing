// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Prompt Retrieval (`prompts/get`) Integration Tests
//!
//! Verifies the behavior of the Model Context Protocol (MCP) `prompts/get` endpoint, including:
//! - Header-based routing via `Mcp-Method: prompts/get` and `Mcp-Name: <name>`
//! - Body-based fallback for prompt names when `Mcp-Name` is omitted
//! - Pure body-based fallback routing (when both `Mcp-Method` and `Mcp-Name` headers are omitted)
//! - Support for no-argument prompt handlers returning strings, prompt messages, results, or error variants
//! - Automatic deserialization of typed argument structures into handler parameters
//! - Error handling when handler business logic fails vs. argument deserialization failure
//! - Multi-turn prompt templates with `Role::User` and `Role::Assistant` messages
//! - Multi-modal prompt message content blocks
//! - Per-prompt HTTP caching directives

mod common;

use http::StatusCode;
use mcp_routing::{
    McpRouter,
    types::mcp::{
        CacheScope, ContentBlock, ImageContent, Role, TextContent,
        prompts::{
            PromptMessage,
            get::{GetPromptResult, GetPromptResultResponse},
        },
    },
};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Typed parameters for a code review prompt.
#[derive(Serialize, Deserialize)]
struct CodeReviewParams {
    code: String,
    language: Option<String>,
}

/// Typed parameters for a translation prompt.
#[derive(Serialize, Deserialize)]
struct TranslateParams {
    text: String,
    target_language: String,
}

/// No-args handler returning a static string slice.
async fn handle_no_args_static_str() -> &'static str {
    "Static prompt template content"
}

/// No-args handler returning an owned [`String`].
async fn handle_no_args_string() -> String {
    "Dynamic prompt string".to_string()
}

/// No-args handler returning a [`PromptMessage`].
async fn handle_no_args_message() -> PromptMessage {
    PromptMessage::assistant_text("You are a helpful assistant.")
}

/// No-args handler returning a multi-turn [`Vec<PromptMessage>`].
async fn handle_multi_turn() -> Vec<PromptMessage> {
    vec![
        PromptMessage::user_text("Hello, can you help me?"),
        PromptMessage::assistant_text("Of course! What do you need help with?"),
        PromptMessage::user_text("I need help with Rust programming."),
    ]
}

/// No-args handler returning a [`GetPromptResult`] with description.
async fn handle_result_with_desc() -> GetPromptResult {
    GetPromptResult::new(vec![
        PromptMessage::user_text("Summarize the following notes:"),
    ])
    .with_description("Notes summarization template")
}

/// Typed handler constructing a code review prompt.
async fn handle_code_review(params: CodeReviewParams) -> Result<GetPromptResult, String> {
    if params.code.trim().is_empty() {
        return Err("code cannot be empty".to_string());
    }

    let lang = params.language.unwrap_or_else(|| "unspecified".to_string());
    let prompt_text = format!("Please review the following {lang} code:\n\n```\n{}\n```", params.code);

    Ok(GetPromptResult::new(vec![
        PromptMessage::user_text(prompt_text),
    ]))
}

/// Typed handler returning a string directly.
async fn handle_translate(params: TranslateParams) -> Result<String, String> {
    if params.text.is_empty() {
        return Err("text cannot be empty".to_string());
    }
    Ok(format!("Translate the following text into {}:\n{}", params.target_language, params.text))
}

/// Multi-modal prompt handler returning text and image blocks.
async fn handle_multimodal_prompt() -> GetPromptResult {
    GetPromptResult::new(vec![
        PromptMessage::user(ContentBlock::Text(TextContent {
            text: "Analyze this image:".to_string(),
            annotations: None,
            meta: None,
        })),
        PromptMessage::user(ContentBlock::Image(ImageContent {
            data: "aGVsbG8=".to_string(),
            mime_type: "image/png".to_string(),
            annotations: None,
            meta: None,
        })),
    ])
}

/// Tests header-based routing with `Mcp-Method: prompts/get` and `Mcp-Name: <name>`.
#[tokio::test]
async fn test_prompts_get_header_routing_with_name() {
    let app = McpRouter::new(common::sample_server_info())
        .register_prompt("simple_prompt", handle_no_args_static_str);

    let req = common::build_request(
        Some("prompts/get"),
        Some("simple_prompt"),
        json!({
            "jsonrpc": "2.0",
            "id": "header-req-1",
            "method": "prompts/get"
        }),
    );

    let (status, headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get("content-type").unwrap().to_str().unwrap(),
        "application/json"
    );

    let res: GetPromptResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res.id, "header-req-1".into());
    assert_eq!(res.result.messages.len(), 1);
    assert!(matches!(res.result.messages[0].role, Role::User));
    if let ContentBlock::Text(ref t) = res.result.messages[0].content {
        assert_eq!(t.text, "Static prompt template content");
    } else {
        panic!("Expected text content block");
    }
}

/// Tests header method with body prompt name fallback when `Mcp-Name` header is omitted.
#[tokio::test]
async fn test_prompts_get_header_method_body_prompt_name_fallback() {
    let app = McpRouter::new(common::sample_server_info())
        .register_prompt("code_review", handle_code_review);

    let req = common::build_request(
        Some("prompts/get"),
        None,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "prompts/get",
            "params": {
                "name": "code_review",
                "arguments": {
                    "code": "fn main() {}",
                    "language": "rust"
                }
            }
        }),
    );

    let (status, _headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    let res: GetPromptResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res.id, 2.into());
    assert_eq!(res.result.messages.len(), 1);
    if let ContentBlock::Text(ref t) = res.result.messages[0].content {
        assert!(t.text.contains("Please review the following rust code"));
        assert!(t.text.contains("fn main() {}"));
    }
}

/// Tests pure body-based fallback routing (when both headers are omitted).
#[tokio::test]
async fn test_prompts_get_body_fallback() {
    let app = McpRouter::new(common::sample_server_info())
        .register_prompt("translate", handle_translate);

    let req = common::build_request(
        None,
        None,
        json!({
            "jsonrpc": "2.0",
            "id": "body-fallback-prompt",
            "method": "prompts/get",
            "params": {
                "name": "translate",
                "arguments": {
                    "text": "Hello world",
                    "target_language": "Spanish"
                }
            }
        }),
    );

    let (status, _headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    let res: GetPromptResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res.id, "body-fallback-prompt".into());
    if let ContentBlock::Text(ref t) = res.result.messages[0].content {
        assert_eq!(t.text, "Translate the following text into Spanish:\nHello world");
    }
}

/// Tests no-args prompt handlers returning various types (string, PromptMessage, Vec<PromptMessage>, GetPromptResult).
#[tokio::test]
async fn test_prompts_get_no_args_handlers() {
    let app = McpRouter::new(common::sample_server_info())
        .register_prompt("string_prompt", handle_no_args_string)
        .register_prompt("message_prompt", handle_no_args_message)
        .register_prompt("multi_turn_prompt", handle_multi_turn)
        .register_prompt("desc_prompt", handle_result_with_desc);

    // 1. Owned string
    let req1 = common::build_request(
        Some("prompts/get"),
        Some("string_prompt"),
        json!({ "jsonrpc": "2.0", "id": 1, "method": "prompts/get" }),
    );
    let (_, _, body1) = common::execute_request(app.clone(), req1).await;
    let res1: GetPromptResultResponse = serde_json::from_value(body1).unwrap();
    assert_eq!(res1.result.messages.len(), 1);

    // 2. PromptMessage (assistant)
    let req2 = common::build_request(
        Some("prompts/get"),
        Some("message_prompt"),
        json!({ "jsonrpc": "2.0", "id": 2, "method": "prompts/get" }),
    );
    let (_, _, body2) = common::execute_request(app.clone(), req2).await;
    let res2: GetPromptResultResponse = serde_json::from_value(body2).unwrap();
    assert_eq!(res2.result.messages.len(), 1);
    assert!(matches!(res2.result.messages[0].role, Role::Assistant));

    // 3. Multi-turn messages
    let req3 = common::build_request(
        Some("prompts/get"),
        Some("multi_turn_prompt"),
        json!({ "jsonrpc": "2.0", "id": 3, "method": "prompts/get" }),
    );
    let (_, _, body3) = common::execute_request(app.clone(), req3).await;
    let res3: GetPromptResultResponse = serde_json::from_value(body3).unwrap();
    assert_eq!(res3.result.messages.len(), 3);
    assert!(matches!(res3.result.messages[0].role, Role::User));
    assert!(matches!(res3.result.messages[1].role, Role::Assistant));
    assert!(matches!(res3.result.messages[2].role, Role::User));

    // 4. GetPromptResult with description
    let req4 = common::build_request(
        Some("prompts/get"),
        Some("desc_prompt"),
        json!({ "jsonrpc": "2.0", "id": 4, "method": "prompts/get" }),
    );
    let (_, _, body4) = common::execute_request(app, req4).await;
    let res4: GetPromptResultResponse = serde_json::from_value(body4).unwrap();
    assert_eq!(res4.result.description.as_deref(), Some("Notes summarization template"));
}

/// Tests multi-modal prompt message content blocks.
#[tokio::test]
async fn test_prompts_get_multimodal_content() {
    let app = McpRouter::new(common::sample_server_info())
        .register_prompt("multimodal", handle_multimodal_prompt);

    let req = common::build_request(
        Some("prompts/get"),
        Some("multimodal"),
        json!({ "jsonrpc": "2.0", "id": 10, "method": "prompts/get" }),
    );
    let (status, _headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    let res: GetPromptResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res.result.messages.len(), 2);
    assert!(matches!(res.result.messages[0].content, ContentBlock::Text(_)));
    assert!(matches!(res.result.messages[1].content, ContentBlock::Image(_)));
}

/// Tests per-prompt caching configuration.
#[tokio::test]
async fn test_prompts_get_with_caching_directives() {
    let app = McpRouter::new(common::sample_server_info())
        .register_prompt_with_cache(
            "cached_prompt",
            handle_no_args_static_str,
            Some(300_000),
            Some(CacheScope::Public),
        );

    let req = common::build_request(
        Some("prompts/get"),
        Some("cached_prompt"),
        json!({ "jsonrpc": "2.0", "id": "cache-test", "method": "prompts/get" }),
    );

    let (status, headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get("cache-control").unwrap().to_str().unwrap(),
        "public, max-age=300"
    );
    assert!(headers.contains_key("etag"));

    let res: GetPromptResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res.id, "cache-test".into());
}

/// Tests error handling for unknown prompt name.
#[tokio::test]
async fn test_prompts_get_unknown_prompt_returns_method_not_found() {
    let app = McpRouter::new(common::sample_server_info())
        .register_prompt("existing_prompt", handle_no_args_static_str);

    let req = common::build_request(
        Some("prompts/get"),
        Some("nonexistent_prompt"),
        json!({ "jsonrpc": "2.0", "id": "unknown-test", "method": "prompts/get" }),
    );

    let (status, _headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["error"]["code"], -32601);
    assert!(body["error"]["message"].as_str().unwrap().contains("prompt 'nonexistent_prompt' not found"));
}

/// Tests error handling when prompt name is omitted.
#[tokio::test]
async fn test_prompts_get_missing_prompt_name_returns_invalid_params() {
    let app = McpRouter::new(common::sample_server_info());

    let req = common::build_request(
        Some("prompts/get"),
        None,
        json!({ "jsonrpc": "2.0", "id": "missing-name", "method": "prompts/get", "params": {} }),
    );

    let (status, _headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["error"]["code"], -32602);
    assert!(body["error"]["message"].as_str().unwrap().contains("empty prompt name"));
}

/// Tests error handling when prompt arguments fail deserialization.
#[tokio::test]
async fn test_prompts_get_invalid_arguments_returns_invalid_params() {
    let app = McpRouter::new(common::sample_server_info())
        .register_prompt("translate", handle_translate);

    let req = common::build_request(
        Some("prompts/get"),
        Some("translate"),
        json!({
            "jsonrpc": "2.0",
            "id": "bad-args",
            "method": "prompts/get",
            "params": {
                "name": "translate",
                "arguments": {
                    "invalid_field": 123
                }
            }
        }),
    );

    let (status, _headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["error"]["code"], -32602);
    assert!(body["error"]["message"].as_str().unwrap().contains("Invalid params"));
}

/// Tests error handling when prompt handler business logic returns an error.
#[tokio::test]
async fn test_prompts_get_business_logic_error_returns_internal_error() {
    let app = McpRouter::new(common::sample_server_info())
        .register_prompt("code_review", handle_code_review);

    let req = common::build_request(
        Some("prompts/get"),
        Some("code_review"),
        json!({
            "jsonrpc": "2.0",
            "id": "logic-err",
            "method": "prompts/get",
            "params": {
                "name": "code_review",
                "arguments": {
                    "code": "   "
                }
            }
        }),
    );

    let (status, _headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["error"]["code"], -32603);
    assert!(body["error"]["message"].as_str().unwrap().contains("code cannot be empty"));
}
