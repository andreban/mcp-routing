// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use bytes::Bytes;
use http::{Request, StatusCode};
use http_body_util::{BodyExt, Full};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower::Service;

use mcp_routing::{
    Extension, McpRouter, Meta, RequestContext, SessionId,
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

    let session_only_tool = Tool {
        icons: Vec::new(),
        name: "session_info".to_string(),
        title: Some("Session Info".to_string()),
        description: Some("Returns session info".to_string()),
        input_schema: json!({"type": "object"}),
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

    // Tool handler with 3 extractors + Args
    async fn handle_query_db(
        session: SessionId,
        Extension(db): Extension<DatabaseConnection>,
        Extension(auth): Extension<AuthUser>,
        params: ExecuteToolParams,
    ) -> Result<String, String> {
        Ok(format!(
            "Session: {session}, User: {}, DB: {}, Query: {}",
            auth.user_id, db.url, params.query
        ))
    }

    // Tool handler with 1 extractor, 0 Args
    async fn handle_session_info(session: SessionId) -> Result<String, String> {
        Ok(format!("Active session: {session}"))
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
        let sid = ctx.session_id_str().unwrap_or("none");
        let has_custom_header = ctx.headers().contains_key("X-Custom-Trace");
        Ok(format!(
            "Client: {client_name}, Proto: {proto}, Session: {sid}, CustomHeader: {has_custom_header}, Query: {}",
            params.query
        ))
    }

    // Prompt handler with SessionId + Meta + Args
    async fn handle_format_prompt(
        session: SessionId,
        Meta(meta): Meta,
        params: PromptFormatParams,
    ) -> Result<GetPromptResult, String> {
        let level = meta
            .log_level
            .map(|l| format!("{l:?}"))
            .unwrap_or_else(|| "default".to_string());
        Ok(GetPromptResult::user(format!(
            "[{session}][log:{level}] Tell me about: {}",
            params.topic
        )))
    }

    McpRouter::new(server_info)
        .register_tool(query_tool, handle_query_db)
        .register_tool(session_only_tool, handle_session_info)
        .register_tool(context_tool, handle_context_info)
        .register_prompt(prompt_item, handle_format_prompt)
}

#[tokio::test]
async fn test_mcp_session_id_header_propagation_on_discover() {
    let mut router = create_test_server();

    let request_payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "server/discover"
    });

    let request = Request::builder()
        .method(http::Method::POST)
        .header(http::header::CONTENT_TYPE, "application/json")
        .header("Mcp-Session-Id", "sess-alpha-123")
        .body(Full::new(Bytes::from(
            serde_json::to_vec(&request_payload).unwrap(),
        )))
        .unwrap();

    let response = router.call(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("Mcp-Session-Id")
            .unwrap()
            .to_str()
            .unwrap(),
        "sess-alpha-123"
    );
}

#[tokio::test]
async fn test_mcp_session_id_header_propagation_on_tools_list() {
    let mut router = create_test_server();

    let request_payload = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    });

    let request = Request::builder()
        .method(http::Method::POST)
        .header(http::header::CONTENT_TYPE, "application/json")
        .header("Mcp-Session-Id", "sess-beta-456")
        .body(Full::new(Bytes::from(
            serde_json::to_vec(&request_payload).unwrap(),
        )))
        .unwrap();

    let response = router.call(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("Mcp-Session-Id")
            .unwrap()
            .to_str()
            .unwrap(),
        "sess-beta-456"
    );
}

#[tokio::test]
async fn test_mcp_session_id_propagation_on_error_responses() {
    let mut router = create_test_server();

    // 405 Method Not Allowed
    let req_get = Request::builder()
        .method(http::Method::GET)
        .header("Mcp-Session-Id", "sess-err-1")
        .body(Full::new(Bytes::new()))
        .unwrap();
    let resp = router.call(req_get).await.unwrap();
    assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(
        resp.headers()
            .get("Mcp-Session-Id")
            .unwrap()
            .to_str()
            .unwrap(),
        "sess-err-1"
    );

    // 415 Unsupported Media Type
    let req_unsupported = Request::builder()
        .method(http::Method::POST)
        .header(http::header::CONTENT_TYPE, "text/plain")
        .header("Mcp-Session-Id", "sess-err-2")
        .body(Full::new(Bytes::from_static(b"hello")))
        .unwrap();
    let resp = router.call(req_unsupported).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_eq!(
        resp.headers()
            .get("Mcp-Session-Id")
            .unwrap()
            .to_str()
            .unwrap(),
        "sess-err-2"
    );

    // Parse Error
    let req_parse_err = Request::builder()
        .method(http::Method::POST)
        .header(http::header::CONTENT_TYPE, "application/json")
        .header("Mcp-Session-Id", "sess-err-3")
        .body(Full::new(Bytes::from_static(b"not json")))
        .unwrap();
    let resp = router.call(req_parse_err).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        resp.headers()
            .get("Mcp-Session-Id")
            .unwrap()
            .to_str()
            .unwrap(),
        "sess-err-3"
    );
}

#[tokio::test]
async fn test_multiple_extractors_with_extensions_and_session() {
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
        .header("Mcp-Session-Id", "sess-db-777")
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
    assert_eq!(
        response
            .headers()
            .get("Mcp-Session-Id")
            .unwrap()
            .to_str()
            .unwrap(),
        "sess-db-777"
    );

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json_val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(json_val["result"]["isError"], false);
    let text = json_val["result"]["content"][0]["text"].as_str().unwrap();
    assert_eq!(
        text,
        "Session: sess-db-777, User: user_42, DB: postgres://localhost:5432/testdb, Query: SELECT * FROM users;"
    );
}

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
        .header("Mcp-Session-Id", "sess-db-777")
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

#[tokio::test]
async fn test_missing_required_session_id_returns_extraction_error() {
    let mut router = create_test_server();

    let request_payload = json!({
        "jsonrpc": "2.0",
        "id": 12,
        "method": "tools/call",
        "params": {
            "name": "session_info"
        }
    });

    // Omit Mcp-Session-Id header
    let request = Request::builder()
        .method(http::Method::POST)
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(
            serde_json::to_vec(&request_payload).unwrap(),
        )))
        .unwrap();

    let response = router.call(request).await.unwrap();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json_val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(json_val["result"]["isError"], true);
    let text = json_val["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Extraction error: Missing required Mcp-Session-Id header"));
}

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
        .header("Mcp-Session-Id", "sess-prompt-99")
        .body(Full::new(Bytes::from(
            serde_json::to_vec(&request_payload).unwrap(),
        )))
        .unwrap();

    let response = router.call(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("Mcp-Session-Id")
            .unwrap()
            .to_str()
            .unwrap(),
        "sess-prompt-99"
    );

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json_val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    let message_text = json_val["result"]["messages"][0]["content"]["text"]
        .as_str()
        .unwrap();
    assert_eq!(
        message_text,
        "[sess-prompt-99][log:Debug] Tell me about: Rust Concurrency"
    );
}

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
        .header("Mcp-Session-Id", "sess-ctx-1")
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
        "Client: claude-desktop, Proto: 2026-07-28, Session: sess-ctx-1, CustomHeader: true, Query: hello"
    );
}

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
