// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Extractor Integration Tests
//!
//! Black-box integration tests verifying MCP handler extractors:
//! - `Extension<T>` for shared dependencies injected via request extensions
//! - `Meta` for per-request `_meta` object propagation
//! - `RequestContext` for comprehensive request introspection
//! - `State<T>` for application state shared via `.with_state()`
//! - Axum shared state integration between web and MCP routes

use bytes::Bytes;
use http::{Request, StatusCode};
use http_body_util::{BodyExt, Full};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower::Service;

use mcp_routing::{
    Extension, McpRouter, Meta, RequestContext,
    types::mcp::{
        Implementation,
        prompts::{Prompt, get::GetPromptResult},
        tools::Tool,
    },
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct DatabaseConnection {
    url: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AuthUser {
    user_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ExecuteToolParams {
    query: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PromptFormatParams {
    topic: String,
}

fn create_test_server() -> McpRouter {
    let server_info = Implementation::new("extractor-test-server", "1.0.0");

    let query_tool = Tool {
        icons: Vec::new(),
        name: "query_db".to_string(),
        title: Some("DB Query".to_string()),
        description: Some("Executes a database query".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": { "query": { "type": "string" } },
            "required": ["query"]
        }),
        output_schema: None,
        annotations: None,
        meta: None,
    };

    let context_tool = Tool {
        icons: Vec::new(),
        name: "context_info".to_string(),
        title: Some("Context Info".to_string()),
        description: Some("Returns full context details".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": { "query": { "type": "string" } }
        }),
        output_schema: None,
        annotations: None,
        meta: None,
    };

    let prompt_item = Prompt {
        icons: Vec::new(),
        name: "format_prompt".to_string(),
        title: Some("Format Prompt".to_string()),
        description: Some("Formats a topic prompt".to_string()),
        arguments: Vec::new(),
        meta: None,
    };

    // Tool handler with 2 Extension extractors + Args
    async fn handle_query_db(
        Extension(db): Extension<DatabaseConnection>,
        Extension(auth): Extension<AuthUser>,
        params: ExecuteToolParams,
    ) -> Result<String, String> {
        Ok(format!(
            "User: {}, DB: {}, Query: {}",
            auth.user_id, db.url, params.query
        ))
    }

    // Tool handler with RequestContext + Args
    async fn handle_context_info(
        ctx: RequestContext,
        params: ExecuteToolParams,
    ) -> Result<String, String> {
        let client_name = ctx
            .client_info()
            .map(|c| c.name.as_str())
            .unwrap_or("unknown");
        let proto = ctx.protocol_version().unwrap_or("unknown");
        let has_custom_header = ctx.headers().contains_key("X-Custom-Trace");
        Ok(format!(
            "Client: {client_name}, Proto: {proto}, CustomHeader: {has_custom_header}, Query: {}",
            params.query
        ))
    }

    // Prompt handler with Meta + Args
    async fn handle_format_prompt(
        Meta(meta): Meta,
        params: PromptFormatParams,
    ) -> Result<GetPromptResult, String> {
        let level = meta
            .log_level
            .map(|l| format!("{l:?}"))
            .unwrap_or_else(|| "default".to_string());
        Ok(GetPromptResult::user(format!(
            "[log:{level}] Tell me about: {}",
            params.topic
        )))
    }

    McpRouter::new(server_info)
        .register_tool(query_tool, handle_query_db)
        .register_tool(context_tool, handle_context_info)
        .register_prompt(prompt_item, handle_format_prompt)
}

/// Tests that multiple `Extension<T>` extractors work together with typed tool arguments.
#[tokio::test]
async fn test_multiple_extractors_with_extensions() {
    let mut router = create_test_server();

    let request_payload = json!({
        "jsonrpc": "2.0",
        "id": 10,
        "method": "tools/call",
        "params": {
            "name": "query_db",
            "arguments": {
                "query": "SELECT * FROM users;"
            }
        }
    });

    let mut request = Request::builder()
        .method(http::Method::POST)
        .header(http::header::CONTENT_TYPE, "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "query_db")
        .body(Full::new(Bytes::from(
            serde_json::to_vec(&request_payload).unwrap(),
        )))
        .unwrap();

    // Attach extensions as middleware/framework would
    request.extensions_mut().insert(DatabaseConnection {
        url: "postgres://localhost:5432/testdb".to_string(),
    });
    request.extensions_mut().insert(AuthUser {
        user_id: "user_42".to_string(),
    });

    let response = router.call(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json_val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(json_val["result"]["isError"], false);
    let text = json_val["result"]["content"][0]["text"].as_str().unwrap();
    assert_eq!(
        text,
        "User: user_42, DB: postgres://localhost:5432/testdb, Query: SELECT * FROM users;"
    );
}

/// Tests that a missing required `Extension<T>` returns an extraction error in the tool result.
#[tokio::test]
async fn test_missing_extension_returns_extraction_error() {
    let mut router = create_test_server();

    let request_payload = json!({
        "jsonrpc": "2.0",
        "id": 11,
        "method": "tools/call",
        "params": {
            "name": "query_db",
            "arguments": {
                "query": "SELECT 1;"
            }
        }
    });

    let mut request = Request::builder()
        .method(http::Method::POST)
        .header(http::header::CONTENT_TYPE, "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "query_db")
        .body(Full::new(Bytes::from(
            serde_json::to_vec(&request_payload).unwrap(),
        )))
        .unwrap();

    // Insert only DatabaseConnection, missing AuthUser
    request.extensions_mut().insert(DatabaseConnection {
        url: "sqlite://memory".to_string(),
    });

    let response = router.call(request).await.unwrap();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json_val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(json_val["result"]["isError"], true);
    let text = json_val["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Extraction error: Missing request extension"));
}

/// Tests that per-request `_meta` object is propagated to prompt handlers via the `Meta` extractor.
#[tokio::test]
async fn test_per_request_meta_propagation_in_prompts_get() {
    let mut router = create_test_server();

    let request_payload = json!({
        "jsonrpc": "2.0",
        "id": 13,
        "method": "prompts/get",
        "params": {
            "_meta": {
                "io.modelcontextprotocol/logLevel": "debug"
            },
            "name": "format_prompt",
            "arguments": {
                "topic": "Rust Concurrency"
            }
        }
    });

    let request = Request::builder()
        .method(http::Method::POST)
        .header(http::header::CONTENT_TYPE, "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "prompts/get")
        .header("Mcp-Name", "format_prompt")
        .body(Full::new(Bytes::from(
            serde_json::to_vec(&request_payload).unwrap(),
        )))
        .unwrap();

    let response = router.call(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json_val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    let message_text = json_val["result"]["messages"][0]["content"]["text"]
        .as_str()
        .unwrap();
    assert_eq!(
        message_text,
        "[log:Debug] Tell me about: Rust Concurrency"
    );
}

/// Tests `RequestContext` extractor with client info, protocol version, and custom headers.
#[tokio::test]
async fn test_request_context_extractor_comprehensive() {
    let mut router = create_test_server();

    let request_payload = json!({
        "jsonrpc": "2.0",
        "id": 14,
        "method": "tools/call",
        "params": {
            "_meta": {
                "io.modelcontextprotocol/clientInfo": {
                    "name": "claude-desktop",
                    "version": "2.0.0"
                },
                "io.modelcontextprotocol/protocolVersion": "2026-07-28"
            },
            "name": "context_info",
            "arguments": {
                "query": "hello"
            }
        }
    });

    let request = Request::builder()
        .method(http::Method::POST)
        .header(http::header::CONTENT_TYPE, "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "context_info")
        .header("X-Custom-Trace", "trace-xyz")
        .body(Full::new(Bytes::from(
            serde_json::to_vec(&request_payload).unwrap(),
        )))
        .unwrap();

    let response = router.call(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json_val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(json_val["result"]["isError"], false);
    let text = json_val["result"]["content"][0]["text"].as_str().unwrap();
    assert_eq!(
        text,
        "Client: claude-desktop, Proto: 2026-07-28, CustomHeader: true, Query: hello"
    );
}

/// Tests `State<T>` extractor with application state injected via `.with_state()`.
#[tokio::test]
async fn test_with_state_and_state_extractor() {
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct AppConfig {
        environment: String,
        pool_size: usize,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct ConfigParams {
        key: String,
    }

    async fn handle_config(
        mcp_routing::State(config): mcp_routing::State<AppConfig>,
        params: ConfigParams,
    ) -> Result<String, String> {
        Ok(format!(
            "Env: {}, Pool: {}, Key: {}",
            config.environment, config.pool_size, params.key
        ))
    }

    let server_info = Implementation::new("state-test-server", "1.0.0");
    let config_tool = Tool {
        icons: Vec::new(),
        name: "get_config".to_string(),
        title: Some("Config".to_string()),
        description: Some("Reads config".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": { "key": { "type": "string" } },
            "required": ["key"]
        }),
        output_schema: None,
        annotations: None,
        meta: None,
    };

    let config = AppConfig {
        environment: "production".to_string(),
        pool_size: 64,
    };

    let mut router = McpRouter::new(server_info)
        .with_state(config)
        .register_tool(config_tool, handle_config);

    let request_payload = json!({
        "jsonrpc": "2.0",
        "id": 20,
        "method": "tools/call",
        "params": {
            "name": "get_config",
            "arguments": {
                "key": "timeout"
            }
        }
    });

    let request = Request::builder()
        .method(http::Method::POST)
        .header(http::header::CONTENT_TYPE, "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "get_config")
        .body(Full::new(Bytes::from(
            serde_json::to_vec(&request_payload).unwrap(),
        )))
        .unwrap();

    let response = router.call(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json_val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(json_val["result"]["isError"], false);
    let text = json_val["result"]["content"][0]["text"].as_str().unwrap();
    assert_eq!(text, "Env: production, Pool: 64, Key: timeout");
}

/// Tests that `State<T>` works across both Axum web routes and MCP tool routes sharing the same state.
#[tokio::test]
async fn test_axum_shared_state_between_web_and_mcp() {
    use axum::Router as AxumRouter;
    use axum::extract::State as AxumState;
    use axum::routing::get;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Clone)]
    struct SharedCounter {
        count: std::sync::Arc<AtomicUsize>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct IncParams {
        amount: usize,
    }

    async fn handle_inc(
        mcp_routing::State(counter): mcp_routing::State<SharedCounter>,
        params: IncParams,
    ) -> Result<String, String> {
        let prev = counter.count.fetch_add(params.amount, Ordering::SeqCst);
        Ok(format!("New total: {}", prev + params.amount))
    }

    async fn handle_web_get(AxumState(counter): AxumState<SharedCounter>) -> String {
        format!("Current: {}", counter.count.load(Ordering::SeqCst))
    }

    let shared_state = SharedCounter {
        count: std::sync::Arc::new(AtomicUsize::new(10)),
    };

    let server_info = Implementation::new("shared-state-test", "1.0.0");
    let inc_tool = Tool {
        icons: Vec::new(),
        name: "increment".to_string(),
        title: Some("Increment".to_string()),
        description: Some("Increments counter".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": { "amount": { "type": "number" } },
            "required": ["amount"]
        }),
        output_schema: None,
        annotations: None,
        meta: None,
    };

    let mcp_router = McpRouter::new(server_info)
        .with_state(shared_state.clone())
        .register_tool(inc_tool, handle_inc);

    let mut app = AxumRouter::new()
        .route("/web/count", get(handle_web_get))
        .nest_service("/mcp", mcp_router)
        .with_state(shared_state);

    // Call MCP tool to increment counter by 5
    let mcp_payload = json!({
        "jsonrpc": "2.0",
        "id": 30,
        "method": "tools/call",
        "params": {
            "name": "increment",
            "arguments": { "amount": 5 }
        }
    });

    let mcp_req = Request::builder()
        .uri("/mcp")
        .method(http::Method::POST)
        .header(http::header::CONTENT_TYPE, "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "increment")
        .body(axum::body::Body::from(
            serde_json::to_vec(&mcp_payload).unwrap(),
        ))
        .unwrap();

    let mcp_resp = app.call(mcp_req).await.unwrap();
    assert_eq!(mcp_resp.status(), StatusCode::OK);
    let bytes = mcp_resp.into_body().collect().await.unwrap().to_bytes();
    let json_val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json_val["result"]["content"][0]["text"], "New total: 15");

    // Call web GET endpoint to verify state mutated by MCP is visible to web route
    let web_req = Request::builder()
        .uri("/web/count")
        .method(http::Method::GET)
        .body(axum::body::Body::empty())
        .unwrap();

    let web_resp = app.call(web_req).await.unwrap();
    assert_eq!(web_resp.status(), StatusCode::OK);
    let web_bytes = web_resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&web_bytes[..], b"Current: 15");
}
