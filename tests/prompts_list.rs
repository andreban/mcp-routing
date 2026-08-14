// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Prompt Discovery (`prompts/list`) Integration Tests
//!
//! Verifies the behavior of the Model Context Protocol (MCP) `prompts/list` endpoint, including:
//! - Empty prompt catalog responses
//! - Registration and advertisement of multiple prompts with arguments, icons, and metadata
//! - Fallback dispatch when the `Mcp-Method` header is omitted in favor of the request body
//! - Handling of pagination cursor parameters and metadata in `ListPromptsParams`
//! - Custom cache TTL and cache scope configuration
//! - Server capability advertisement verification

mod common;

use std::borrow::Cow;
use http::StatusCode;
use mcp_routing::{
    McpRouter,
    types::mcp::{
        CacheScope, IconTheme,
        prompts::{Prompt, PromptArgument, list::ListPromptsResultResponse},
        server::discover::ServerDiscoverResultResponse,
    },
};
use serde_json::json;

async fn dummy_prompt_handler() -> &'static str {
    "prompt text"
}

/// Tests that a router with no registered prompts returns an empty `prompts` list.
#[tokio::test]
async fn test_prompts_list_empty() {
    let app = McpRouter::new(common::sample_server_info());

    let req = common::build_request(
        Some("prompts/list"),
        None,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "prompts/list"
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

    let res: ListPromptsResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res.jsonrpc, "2.0");
    assert_eq!(res.id, 1.into());
    assert_eq!(res.result.prompts.len(), 0);
    assert_eq!(res.result.ttl_ms, Some(0));
    assert!(matches!(res.result.cache_scope, Some(CacheScope::Public)));
    assert_eq!(res.result.next_cursor, None);
}

/// Tests registering multiple prompts using various styles and verifying their advertisement in `prompts/list`.
#[tokio::test]
async fn test_prompts_list_multiple_rich_prompts() {
    let prompt1 = common::sample_prompt("code_review");
    let prompt2 = Prompt {
        icons: vec![],
        name: "summarize".to_string(),
        title: Some("Summarize Article".to_string()),
        description: Some("Summarizes a long article".to_string()),
        arguments: vec![
            PromptArgument::new("text")
                .title("Article Text")
                .description("The full text to summarize")
                .required(true),
            PromptArgument::new("max_length")
                .title("Max Length")
                .required(false),
        ],
        meta: None,
    };

    let app = McpRouter::new(common::sample_server_info())
        .register_prompt(prompt1, dummy_prompt_handler)
        .register_prompt(prompt2, dummy_prompt_handler)
        .register_prompt("inline_str_prompt", dummy_prompt_handler)
        .register_prompt(Cow::Borrowed("cow_prompt"), dummy_prompt_handler);

    let req = common::build_request(
        Some("prompts/list"),
        None,
        json!({
            "jsonrpc": "2.0",
            "id": "list-rich-prompts",
            "method": "prompts/list"
        }),
    );

    let (status, _headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    let res: ListPromptsResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res.id, "list-rich-prompts".into());
    assert_eq!(res.result.prompts.len(), 4);

    // Verify first prompt
    let p0 = &res.result.prompts[0];
    assert_eq!(p0.name, "code_review");
    assert_eq!(p0.title.as_deref(), Some("Title for code_review"));
    assert_eq!(p0.description.as_deref(), Some("Description for code_review"));
    assert_eq!(p0.icons.len(), 1);
    assert_eq!(p0.icons[0].src, "https://example.com/prompt_icon.png");
    assert_eq!(p0.icons[0].mime_type.as_deref(), Some("image/png"));
    assert!(matches!(p0.icons[0].theme, Some(IconTheme::Dark)));
    assert_eq!(p0.arguments.len(), 2);
    assert_eq!(p0.arguments[0].name, "topic");
    assert_eq!(p0.arguments[0].required, Some(true));
    assert_eq!(p0.arguments[1].name, "style");
    assert_eq!(p0.arguments[1].required, Some(false));
    assert_eq!(
        p0.meta.as_ref().unwrap().get("customPromptMeta").unwrap(),
        "promptMetaVal"
    );

    // Verify second prompt
    let p1 = &res.result.prompts[1];
    assert_eq!(p1.name, "summarize");
    assert_eq!(p1.title.as_deref(), Some("Summarize Article"));
    assert_eq!(p1.arguments.len(), 2);

    // Verify prompt registered via &str
    let p2 = &res.result.prompts[2];
    assert_eq!(p2.name, "inline_str_prompt");
    assert!(p2.arguments.is_empty());

    // Verify prompt registered via Cow
    let p3 = &res.result.prompts[3];
    assert_eq!(p3.name, "cow_prompt");
}

/// Tests `prompts/list` dispatch when the `Mcp-Method` HTTP header is omitted.
#[tokio::test]
async fn test_prompts_list_via_body_fallback() {
    let app = McpRouter::new(common::sample_server_info())
        .register_prompt(common::sample_prompt("body_prompt"), dummy_prompt_handler);

    let req = common::build_request(
        None,
        None,
        json!({
            "jsonrpc": "2.0",
            "id": 100,
            "method": "prompts/list"
        }),
    );

    let (status, _headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    let res: ListPromptsResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res.id, 100.into());
    assert_eq!(res.result.prompts.len(), 1);
    assert_eq!(res.result.prompts[0].name, "body_prompt");
}

/// Tests passing pagination `cursor` and protocol `_meta` in `prompts/list` requests.
#[tokio::test]
async fn test_prompts_list_with_pagination_cursor_and_meta() {
    let app = McpRouter::new(common::sample_server_info())
        .register_prompt("prompt_p", dummy_prompt_handler);

    let req = common::build_request(
        Some("prompts/list"),
        None,
        json!({
            "jsonrpc": "2.0",
            "id": "cursor-prompt-req",
            "method": "prompts/list",
            "params": {
                "cursor": "cursor_page_3",
                "_meta": {
                    "io.modelcontextprotocol/protocolVersion": "2026-07-28"
                }
            }
        }),
    );

    let (status, _headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    let res: ListPromptsResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res.id, "cursor-prompt-req".into());
    assert_eq!(res.result.prompts.len(), 1);
    assert_eq!(res.result.prompts[0].name, "prompt_p");
}

/// Tests custom TTL and cache scope configuration on `prompts/list`.
#[tokio::test]
async fn test_prompts_list_custom_caching_parameters() {
    let app = McpRouter::new(common::sample_server_info())
        .prompts_list_cache(Some(120000), Some(CacheScope::Public))
        .register_prompt(common::sample_prompt("cached_prompt"), dummy_prompt_handler);

    let req = common::build_request(
        Some("prompts/list"),
        None,
        json!({
            "jsonrpc": "2.0",
            "id": "cache-prompts-test",
            "method": "prompts/list"
        }),
    );

    let (status, headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get("cache-control").unwrap().to_str().unwrap(),
        "public, max-age=120"
    );
    assert!(headers.contains_key("etag"));

    let res: ListPromptsResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res.result.ttl_ms, Some(120000));
    assert!(matches!(res.result.cache_scope, Some(CacheScope::Public)));
}

/// Tests that registering a prompt automatically advertises prompts capability in `server/discover`.
#[tokio::test]
async fn test_prompts_capability_advertisement_in_discover() {
    let app = McpRouter::new(common::sample_server_info())
        .register_prompt("my_prompt", dummy_prompt_handler);

    let req = common::build_request(
        Some("server/discover"),
        None,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "server/discover"
        }),
    );

    let (status, _headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    let res: ServerDiscoverResultResponse = serde_json::from_value(body).unwrap();
    assert!(res.result.capabilities.prompts.is_some());
}
