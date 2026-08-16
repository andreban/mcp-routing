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
use std::borrow::Cow;

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
    assert_eq!(
        p0.description.as_deref(),
        Some("Description for code_review")
    );
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

/// Tests that omitting `Mcp-Method` HTTP header on `prompts/list` returns HTTP 400 Bad Request with HeaderMismatch (-32020).
#[tokio::test]
async fn test_prompts_list_missing_method_header_returns_header_mismatch() {
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

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["error"]["code"],
        mcp_routing::types::mcp::HEADER_MISMATCH
    );
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

/// Tests registering a custom `prompts_list` handler using `BearerAuth` and `Meta` extractors.
#[tokio::test]
async fn test_prompts_list_custom_handler_with_bearer_auth_and_extractors() {
    use mcp_routing::extract::{BearerAuth, Meta};
    use mcp_routing::types::mcp::prompts::list::ListPromptsResult;

    async fn custom_prompts_handler(
        BearerAuth(token): BearerAuth,
        meta: Option<Meta>,
    ) -> Result<ListPromptsResult, String> {
        let is_admin = token == "admin-secret";
        let is_vip = meta
            .as_ref()
            .and_then(|m| m.client_info.as_ref())
            .map(|c| c.name == "vip-client")
            .unwrap_or(false);

        let mut prompts = vec![common::sample_prompt("public_prompt")];
        if is_admin {
            prompts.push(common::sample_prompt("admin_prompt"));
        }
        if is_vip {
            prompts.push(common::sample_prompt("vip_prompt"));
        }

        Ok(ListPromptsResult::new(prompts).with_cache(Some(60_000), Some(CacheScope::Private)))
    }

    let app = McpRouter::new(common::sample_server_info()).prompts_list(custom_prompts_handler);

    // Request 1: Regular user
    let mut req1 = common::build_request(
        Some("prompts/list"),
        None,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "prompts/list"
        }),
    );
    req1.headers_mut().insert(
        http::header::AUTHORIZATION,
        "Bearer user-token".parse().unwrap(),
    );
    let (status1, headers1, body1) = common::execute_request(app.clone(), req1).await;
    assert_eq!(status1, StatusCode::OK);
    assert_eq!(
        headers1.get("cache-control").unwrap().to_str().unwrap(),
        "private, max-age=60"
    );
    let res1: ListPromptsResultResponse = serde_json::from_value(body1).unwrap();
    assert_eq!(res1.result.prompts.len(), 1);
    assert_eq!(res1.result.prompts[0].name, "public_prompt");

    // Request 2: Admin user with VIP client info
    let mut req2 = common::build_request(
        Some("prompts/list"),
        None,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "prompts/list",
            "params": {
                "_meta": {
                    "io.modelcontextprotocol/clientInfo": {
                        "name": "vip-client",
                        "version": "1.0.0"
                    }
                }
            }
        }),
    );
    req2.headers_mut().insert(
        http::header::AUTHORIZATION,
        "Bearer admin-secret".parse().unwrap(),
    );
    let (status2, _headers2, body2) = common::execute_request(app.clone(), req2).await;
    assert_eq!(status2, StatusCode::OK);
    let res2: ListPromptsResultResponse = serde_json::from_value(body2).unwrap();
    assert_eq!(res2.result.prompts.len(), 3);
    assert_eq!(res2.result.prompts[0].name, "public_prompt");
    assert_eq!(res2.result.prompts[1].name, "admin_prompt");
    assert_eq!(res2.result.prompts[2].name, "vip_prompt");

    // Request 3: Missing authorization header (fails extractor)
    let req3 = common::build_request(
        Some("prompts/list"),
        None,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "prompts/list"
        }),
    );
    let (status3, _headers3, body3) = common::execute_request(app, req3).await;
    assert_eq!(status3, StatusCode::OK);
    assert_eq!(body3["error"]["code"], -32602);
    assert!(
        body3["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Missing required Authorization header")
    );
}

/// Tests custom `prompts_list` handler with pagination cursor parameter.
#[tokio::test]
async fn test_prompts_list_custom_handler_with_pagination_cursor() {
    use mcp_routing::types::mcp::prompts::list::ListPromptsResult;

    async fn paged_prompts_handler(cursor: Option<String>) -> ListPromptsResult {
        match cursor.as_deref() {
            None => ListPromptsResult::new(vec![common::sample_prompt("prompt_page1")])
                .with_next_cursor("next_cursor_page2"),
            Some("next_cursor_page2") => {
                ListPromptsResult::new(vec![common::sample_prompt("prompt_page2")])
            }
            Some(_) => ListPromptsResult::new(vec![]),
        }
    }

    let app = McpRouter::new(common::sample_server_info()).prompts_list(paged_prompts_handler);

    // Page 1
    let req1 = common::build_request(
        Some("prompts/list"),
        None,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "prompts/list"
        }),
    );
    let (status1, _, body1) = common::execute_request(app.clone(), req1).await;
    assert_eq!(status1, StatusCode::OK);
    let res1: ListPromptsResultResponse = serde_json::from_value(body1).unwrap();
    assert_eq!(res1.result.prompts.len(), 1);
    assert_eq!(res1.result.prompts[0].name, "prompt_page1");
    assert_eq!(
        res1.result.next_cursor.as_deref(),
        Some("next_cursor_page2")
    );

    // Page 2
    let req2 = common::build_request(
        Some("prompts/list"),
        None,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "prompts/list",
            "params": { "cursor": "next_cursor_page2" }
        }),
    );
    let (status2, _, body2) = common::execute_request(app, req2).await;
    assert_eq!(status2, StatusCode::OK);
    let res2: ListPromptsResultResponse = serde_json::from_value(body2).unwrap();
    assert_eq!(res2.result.prompts.len(), 1);
    assert_eq!(res2.result.prompts[0].name, "prompt_page2");
    assert_eq!(res2.result.next_cursor, None);
}

/// Tests custom `prompts_list` handler error propagation.
#[tokio::test]
async fn test_prompts_list_custom_handler_error_propagation() {
    use mcp_routing::types::mcp::prompts::Prompt;

    async fn failing_handler() -> Result<Vec<Prompt>, String> {
        Err("Template engine failed to load prompts".to_string())
    }

    let app = McpRouter::new(common::sample_server_info()).prompts_list(failing_handler);

    let req = common::build_request(
        Some("prompts/list"),
        None,
        json!({
            "jsonrpc": "2.0",
            "id": 200,
            "method": "prompts/list"
        }),
    );

    let (status, _, body) = common::execute_request(app, req).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["error"]["code"], -32603);
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Template engine failed to load prompts")
    );
}

/// Tests that a `prompts_list` handler can extract `RegisteredPrompts` to inspect and filter pre-registered prompts.
#[tokio::test]
async fn test_prompts_list_registered_prompts_extractor_filtering() {
    use mcp_routing::extract::{BearerAuth, RegisteredPrompts};
    use mcp_routing::types::mcp::prompts::Prompt;

    async fn filter_prompts(
        auth: Option<BearerAuth>,
        RegisteredPrompts(all_prompts): RegisteredPrompts,
    ) -> Vec<Prompt> {
        let is_admin = auth.as_ref().map(|a| a.token()) == Some("admin-key");
        all_prompts
            .into_iter()
            .filter(|p| !p.name.starts_with("admin_") || is_admin)
            .collect()
    }

    let app = McpRouter::new(common::sample_server_info())
        .register_prompt(
            common::sample_prompt("public_summary"),
            dummy_prompt_handler,
        )
        .register_prompt(
            common::sample_prompt("public_translate"),
            dummy_prompt_handler,
        )
        .register_prompt(
            common::sample_prompt("admin_diagnostics"),
            dummy_prompt_handler,
        )
        .prompts_list(filter_prompts);

    // Standard user request
    let req_user = common::build_request(
        Some("prompts/list"),
        None,
        json!({ "jsonrpc": "2.0", "id": 1, "method": "prompts/list" }),
    );
    let (status_u, _, body_u) = common::execute_request(app.clone(), req_user).await;
    assert_eq!(status_u, StatusCode::OK);
    let res_u: ListPromptsResultResponse = serde_json::from_value(body_u).unwrap();
    assert_eq!(res_u.result.prompts.len(), 2);
    assert_eq!(res_u.result.prompts[0].name, "public_summary");
    assert_eq!(res_u.result.prompts[1].name, "public_translate");

    // Admin request with Bearer token
    let mut req_admin = common::build_request(
        Some("prompts/list"),
        None,
        json!({ "jsonrpc": "2.0", "id": 2, "method": "prompts/list" }),
    );
    req_admin.headers_mut().insert(
        http::header::AUTHORIZATION,
        "Bearer admin-key".parse().unwrap(),
    );
    let (status_a, _, body_a) = common::execute_request(app, req_admin).await;
    assert_eq!(status_a, StatusCode::OK);
    let res_a: ListPromptsResultResponse = serde_json::from_value(body_a).unwrap();
    assert_eq!(res_a.result.prompts.len(), 3);
    assert_eq!(res_a.result.prompts[2].name, "admin_diagnostics");
}
