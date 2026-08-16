// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Argument Autocompletion (`completion/complete`) Integration Tests
//!
//! Verifies the behavior of the Model Context Protocol (MCP) `completion/complete` endpoint, including:
//! - Argument completions for prompt references (`ref/prompt`) with specific argument names or fallback handlers
//! - Argument completions for resource template references (`ref/resource`) with template context arguments
//! - Global default fallback completion handlers
//! - Extractor integration (`State`, `BearerAuth`) within completion handlers
//! - Result clamping (max 100 items), total count tracking, and pagination flags
//! - Capability advertisement in `server/discover` responses
//! - Custom cache TTL and cache scope configuration
//! - Error handling for unhandled targets and malformed/missing parameters
//! - Batch request processing and header-based routing fallback

mod common;

use axum::body::Body;
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use mcp_routing::{
    McpRouter,
    extract::{BearerAuth, State},
    types::mcp::{
        CacheScope, Implementation,
        completion::{
            CompleteArgument, CompleteContext, CompleteParams, CompleteResult, Reference,
        },
        prompts::{Prompt, PromptArgument},
        resources::ResourceTemplate,
    },
};
use tower::Service;

fn create_base_router() -> McpRouter {
    let server_info = Implementation::new("test-server", "1.0.0");
    McpRouter::new(server_info)
}

async fn send_mcp_request(
    router: &mut McpRouter,
    body: serde_json::Value,
    headers: Option<Vec<(&'static str, &'static str)>>,
) -> (StatusCode, serde_json::Value, http::HeaderMap) {
    let body_str = serde_json::to_string(&body).unwrap();
    let mut req_builder = Request::post("/")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28");

    let mut has_mcp_method = false;
    let mut has_mcp_name = false;
    let mut has_mcp_uri = false;
    if let Some(ref hdrs) = headers {
        for (k, _) in hdrs {
            if k.eq_ignore_ascii_case("Mcp-Method") {
                has_mcp_method = true;
            }
            if k.eq_ignore_ascii_case("Mcp-Name") {
                has_mcp_name = true;
            }
            if k.eq_ignore_ascii_case("Mcp-Uri") {
                has_mcp_uri = true;
            }
        }
    }

    if !has_mcp_method && let Some(m) = body.get("method").and_then(|v| v.as_str()) {
        req_builder = req_builder.header("Mcp-Method", m);
    }

    if !has_mcp_name
        && let Some(name) = body
            .get("params")
            .and_then(|p| p.get("name"))
            .and_then(|v| v.as_str())
    {
        req_builder = req_builder.header("Mcp-Name", name);
    }

    if !has_mcp_uri
        && let Some(uri) = body
            .get("params")
            .and_then(|p| p.get("uri"))
            .and_then(|v| v.as_str())
    {
        req_builder = req_builder.header("Mcp-Uri", uri);
    }

    if let Some(hdrs) = headers {
        for (k, v) in hdrs {
            req_builder = req_builder.header(k, v);
        }
    }

    let req = req_builder.body(Body::from(body_str)).unwrap();
    let response = router.call(req).await.unwrap();
    let status = response.status();
    let resp_headers = response.headers().clone();
    let resp_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = if resp_bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&resp_bytes).unwrap_or(serde_json::Value::Null)
    };
    (status, json, resp_headers)
}

/// Tests autocompletion for a specific named argument of a registered prompt.
#[tokio::test]
async fn test_completion_prompt_specific_argument() {
    let mut router = create_base_router()
        .register_prompt(
            Prompt::new("review").argument(PromptArgument::new("language")),
            || async { "review prompt" },
        )
        .register_prompt_arg_completion("review", "language", |arg: CompleteArgument| async move {
            let languages = vec!["python", "pyside", "pytorch", "rust", "ruby"];
            languages
                .into_iter()
                .filter(|l| l.starts_with(&arg.value))
                .collect::<Vec<_>>()
        });

    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "completion/complete",
        "params": {
            "ref": {
                "type": "ref/prompt",
                "name": "review"
            },
            "argument": {
                "name": "language",
                "value": "py"
            }
        }
    });

    let (status, body, _) = send_mcp_request(&mut router, payload, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], 1.0);
    assert_eq!(
        body["result"]["completion"]["values"],
        serde_json::json!(["python", "pyside", "pytorch"])
    );
}

/// Tests fallback autocompletion across all arguments for a prompt.
#[tokio::test]
async fn test_completion_prompt_all_arguments_fallback() {
    let mut router = create_base_router().register_prompt_completion(
        "greet",
        |arg: CompleteArgument| async move {
            if arg.name == "title" {
                vec!["Mr", "Ms", "Dr"]
            } else {
                vec!["Alice", "Bob"]
            }
        },
    );

    let payload_title = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "completion/complete",
        "params": {
            "ref": { "type": "ref/prompt", "name": "greet" },
            "argument": { "name": "title", "value": "" }
        }
    });

    let (status, body, _) = send_mcp_request(&mut router, payload_title, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["result"]["completion"]["values"],
        serde_json::json!(["Mr", "Ms", "Dr"])
    );

    let payload_name = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "completion/complete",
        "params": {
            "ref": { "type": "ref/prompt", "name": "greet" },
            "argument": { "name": "name", "value": "" }
        }
    });

    let (status, body, _) = send_mcp_request(&mut router, payload_name, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["result"]["completion"]["values"],
        serde_json::json!(["Alice", "Bob"])
    );
}

/// Tests autocompletion for resource templates with context argument inspection.
#[tokio::test]
async fn test_completion_resource_template_with_context() {
    let mut router = create_base_router()
        .register_resource_template(
            ResourceTemplate::new("postgres://{db}/{table}", "Database Tables"),
            |_uri: String| async { "data" },
        )
        .register_resource_arg_completion(
            "postgres://{db}/{table}",
            "table",
            |arg: CompleteArgument, ctx: Option<CompleteContext>| async move {
                let db = ctx
                    .as_ref()
                    .and_then(|c| c.get_argument("db"))
                    .unwrap_or("public");
                vec![
                    format!("{db}_{}_a", arg.value),
                    format!("{db}_{}_b", arg.value),
                ]
            },
        );

    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 10,
        "method": "completion/complete",
        "params": {
            "ref": {
                "type": "ref/resource",
                "uri": "postgres://production/users"
            },
            "argument": {
                "name": "table",
                "value": "user"
            },
            "context": {
                "arguments": {
                    "db": "production"
                }
            }
        }
    });

    let (status, body, _) = send_mcp_request(&mut router, payload, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["result"]["completion"]["values"],
        serde_json::json!(["production_user_a", "production_user_b"])
    );
}

/// Tests the default fallback completion provider when no resource- or prompt-specific completion handler matches.
#[tokio::test]
async fn test_completion_default_fallback_provider() {
    let mut router = create_base_router().completion(|params: CompleteParams| async move {
        match params.reference {
            Reference::Prompt { name } => vec![format!("prompt_{name}_{}", params.argument.name)],
            Reference::Resource { uri } => vec![format!("resource_{uri}_{}", params.argument.name)],
        }
    });

    let payload_prompt = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "completion/complete",
        "params": {
            "ref": { "type": "ref/prompt", "name": "dynamic_prompt" },
            "argument": { "name": "topic", "value": "ai" }
        }
    });

    let (_, body1, _) = send_mcp_request(&mut router, payload_prompt, None).await;
    assert_eq!(
        body1["result"]["completion"]["values"],
        serde_json::json!(["prompt_dynamic_prompt_topic"])
    );

    let payload_res = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "completion/complete",
        "params": {
            "ref": { "type": "ref/resource", "uri": "custom://file.txt" },
            "argument": { "name": "line", "value": "1" }
        }
    });

    let (_, body2, _) = send_mcp_request(&mut router, payload_res, None).await;
    assert_eq!(
        body2["result"]["completion"]["values"],
        serde_json::json!(["resource_custom://file.txt_line"])
    );
}

/// Tests completion handlers receiving request extractors such as `State` and `BearerAuth`.
#[tokio::test]
async fn test_completion_with_extractors_and_state() {
    #[derive(Clone)]
    struct AppConfig {
        prefix: String,
    }

    let mut router =
        create_base_router()
            .with_state(AppConfig {
                prefix: "corp_".to_string(),
            })
            .register_prompt_arg_completion(
                "sql_query",
                "table",
                |State(cfg): State<AppConfig>,
                 BearerAuth(token): BearerAuth,
                 arg: CompleteArgument| async move {
                    vec![format!("{}:{}{}", token, cfg.prefix, arg.value)]
                },
            );

    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "completion/complete",
        "params": {
            "ref": { "type": "ref/prompt", "name": "sql_query" },
            "argument": { "name": "table", "value": "orders" }
        }
    });

    let headers = vec![("Authorization", "Bearer secret-token-456")];

    let (status, body, _) = send_mcp_request(&mut router, payload, Some(headers)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["result"]["completion"]["values"],
        serde_json::json!(["secret-token-456:corp_orders"])
    );
}

/// Tests clamping of completion results to the maximum limit (100) and pagination flags.
#[tokio::test]
async fn test_completion_clamping_and_pagination() {
    let mut router = create_base_router().completion(|| async {
        let mut items = Vec::new();
        for i in 0..120 {
            items.push(format!("item_{i:03}"));
        }
        CompleteResult::new(items)
    });

    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "completion/complete",
        "params": {
            "ref": { "type": "ref/prompt", "name": "bulk" },
            "argument": { "name": "item", "value": "" }
        }
    });

    let (status, body, _) = send_mcp_request(&mut router, payload, None).await;
    assert_eq!(status, StatusCode::OK);
    let values = body["result"]["completion"]["values"].as_array().unwrap();
    assert_eq!(values.len(), 100);
    assert_eq!(values[0], "item_000");
    assert_eq!(values[99], "item_099");
    assert_eq!(body["result"]["completion"]["total"], 120);
    assert_eq!(body["result"]["completion"]["hasMore"], true);
}

/// Tests advertisement of `completions` capability in `server/discover` responses.
#[tokio::test]
async fn test_completion_capability_advertisement_in_discover() {
    let mut router = create_base_router().completion(|| async { vec!["opt1"] });

    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "server/discover"
    });

    let (status, body, _) = send_mcp_request(&mut router, payload, None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["result"]["capabilities"]["completions"].is_object());
}

/// Tests HTTP caching headers (`Cache-Control`, `ETag`) returned for completion responses when configured.
#[tokio::test]
async fn test_completion_caching_directives() {
    let mut router = create_base_router()
        .completion(|| async { vec!["cached_suggestion"] })
        .completion_cache(Some(300_000), Some(CacheScope::Private));

    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "completion/complete",
        "params": {
            "ref": { "type": "ref/prompt", "name": "cached_prompt" },
            "argument": { "name": "query", "value": "a" }
        }
    });

    let (status, _, headers) = send_mcp_request(&mut router, payload, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get("Cache-Control").unwrap().to_str().unwrap(),
        "private, max-age=300"
    );
    assert!(headers.get("ETag").is_some());
}

/// Tests error handling when requesting completion for an unhandled target.
#[tokio::test]
async fn test_completion_unhandled_target_returns_invalid_params() {
    let mut router = create_base_router().register_prompt_arg_completion(
        "registered_prompt",
        "known_arg",
        || async { vec!["ok"] },
    );

    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 99,
        "method": "completion/complete",
        "params": {
            "ref": { "type": "ref/prompt", "name": "unknown_prompt" },
            "argument": { "name": "any_arg", "value": "test" }
        }
    });

    let (status, body, _) = send_mcp_request(&mut router, payload, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["error"]["code"], -32602);
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("no completion handler registered")
    );
}

/// Tests validation failure when `completion/complete` parameters are missing or malformed.
#[tokio::test]
async fn test_completion_invalid_params() {
    let mut router = create_base_router().completion(|| async { vec!["suggestion"] });

    // Missing ref
    let payload_missing_ref = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "completion/complete",
        "params": {
            "argument": { "name": "query", "value": "val" }
        }
    });

    let (status, body1, _) = send_mcp_request(&mut router, payload_missing_ref, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body1["error"]["code"], -32602);

    // Missing params completely
    let payload_missing_params = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "completion/complete"
    });

    let (status, body2, _) = send_mcp_request(&mut router, payload_missing_params, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body2["error"]["code"], -32602);
}

/// Tests batch execution of multiple `completion/complete` requests in a single JSON-RPC batch.
#[tokio::test]
async fn test_completion_batch_request() {
    let mut router = create_base_router().register_prompt_arg_completion(
        "batch_prompt",
        "arg",
        |arg: CompleteArgument| async move { vec![format!("res_{}", arg.value)] },
    );

    let batch_payload = serde_json::json!([
        {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "completion/complete",
            "params": {
                "ref": { "type": "ref/prompt", "name": "batch_prompt" },
                "argument": { "name": "arg", "value": "first" }
            }
        },
        {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "completion/complete",
            "params": {
                "ref": { "type": "ref/prompt", "name": "batch_prompt" },
                "argument": { "name": "arg", "value": "second" }
            }
        }
    ]);

    let (status, body, _) = send_mcp_request(&mut router, batch_payload, None).await;
    assert_eq!(status, StatusCode::OK);
    let array = body.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(
        array[0]["result"]["completion"]["values"],
        serde_json::json!(["res_first"])
    );
    assert_eq!(
        array[1]["result"]["completion"]["values"],
        serde_json::json!(["res_second"])
    );
}

/// Tests `completion/complete` routing when the method is passed via `Mcp-Method` header.
#[tokio::test]
async fn test_completion_via_header_routing() {
    let mut router = create_base_router().register_prompt_arg_completion(
        "hdr_prompt",
        "arg",
        |arg: CompleteArgument| async move { vec![format!("hdr_{}", arg.value)] },
    );

    // method omitted in body, provided in Mcp-Method header
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "params": {
            "ref": { "type": "ref/prompt", "name": "hdr_prompt" },
            "argument": { "name": "arg", "value": "val" }
        }
    });

    let headers = vec![("Mcp-Method", "completion/complete")];
    let (status, body, _) = send_mcp_request(&mut router, payload, Some(headers)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["result"]["completion"]["values"],
        serde_json::json!(["hdr_val"])
    );
}
