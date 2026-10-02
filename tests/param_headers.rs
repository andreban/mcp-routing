// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Custom Parameter Headers (`x-mcp-header` & `Mcp-Param-{Name}`) Integration Tests
//!
//! Verifies the behavior of the Model Context Protocol (MCP) Streamable HTTP parameter headers:
//! - Extracting string `x-mcp-header` annotations (the `Mcp-Param-{Name}` header name) from tool `inputSchema`
//! - Header names that differ from the annotated property name, including nested properties
//! - Validating `Mcp-Param-{Name}` HTTP request headers against `tools/call` arguments
//! - Rejection with HTTP 400 Bad Request and error code -32020 (`HEADER_MISMATCH`) on missing or mismatched headers
//! - RFC 2047-style Base64 sentinel decoding (`=?base64?...?=`) for parameter headers
//! - Support for string, numeric, and boolean parameter types (numbers compared numerically)
//! - Rejection of malformed Base64 sentinel parameter values
//! - Proper handling of optional parameters

mod common;

use axum::body::Body;
use http::{Request, StatusCode};
use stateless_mcp::{
    McpRouter,
    types::mcp::{
        HEADER_MISMATCH,
        tools::{Tool, call::CallToolResult},
    },
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use common::{execute_request, sample_server_info};

#[derive(Serialize, Deserialize)]
struct FileQueryParams {
    repo: String,
    path: String,
    #[serde(default)]
    branch: Option<String>,
}

async fn handle_file_query(params: FileQueryParams) -> CallToolResult {
    let branch = params.branch.as_deref().unwrap_or("main");
    CallToolResult::text(format!(
        "repo={}, path={}, branch={}",
        params.repo, params.path, branch
    ))
}

fn file_query_tool() -> Tool {
    let mut tool = Tool::new("file_query");
    tool.input_schema = json!({
        "type": "object",
        "properties": {
            "repo": {
                "type": "string",
                "x-mcp-header": "Repo"
            },
            "path": {
                "type": "string"
            },
            "branch": {
                "type": "string",
                "x-mcp-header": "Branch"
            }
        },
        "required": ["repo", "path"]
    });
    tool
}

#[derive(Serialize, Deserialize)]
struct TypedParams {
    count: i64,
    active: bool,
}

async fn handle_typed_params(params: TypedParams) -> CallToolResult {
    CallToolResult::text(format!("count={}, active={}", params.count, params.active))
}

fn typed_params_tool() -> Tool {
    let mut tool = Tool::new("typed_params");
    tool.input_schema = json!({
        "type": "object",
        "properties": {
            "count": {
                "type": "integer",
                "x-mcp-header": "Count"
            },
            "active": {
                "type": "boolean",
                "x-mcp-header": "Active"
            }
        },
        "required": ["count", "active"]
    });
    tool
}

/// Tests matching parameter header against tool arguments with successful execution.
#[tokio::test]
async fn test_param_header_matching_success() {
    let app =
        McpRouter::new(sample_server_info()).register_tool(file_query_tool(), handle_file_query);

    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "file_query")
        .header("Mcp-Param-Repo", "mcp-routing")
        .body(Body::from(
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": {
                    "_meta": common::meta(),
                    "name": "file_query",
                    "arguments": {
                        "repo": "mcp-routing",
                        "path": "src/lib.rs"
                    }
                }
            })
            .to_string(),
        ))
        .unwrap();

    let (status, _, body) = execute_request(app, req).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["result"]["content"][0]["text"],
        "repo=mcp-routing, path=src/lib.rs, branch=main"
    );
}

/// Tests that a missing required parameter header returns HTTP 400 Bad Request with `HEADER_MISMATCH`.
#[tokio::test]
async fn test_param_header_missing_returns_header_mismatch() {
    let app =
        McpRouter::new(sample_server_info()).register_tool(file_query_tool(), handle_file_query);

    // Missing required Mcp-Param-Repo header
    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "file_query")
        .body(Body::from(
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": {
                    "_meta": common::meta(),
                    "name": "file_query",
                    "arguments": {
                        "repo": "mcp-routing",
                        "path": "src/lib.rs"
                    }
                }
            })
            .to_string(),
        ))
        .unwrap();

    let (status, _, body) = execute_request(app, req).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], HEADER_MISMATCH);
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Mcp-Param-repo")
            || body["error"]["message"]
                .as_str()
                .unwrap()
                .contains("Mcp-Param-Repo")
    );
}

/// Tests that a mismatched parameter header value returns HTTP 400 Bad Request with `HEADER_MISMATCH`.
#[tokio::test]
async fn test_param_header_value_mismatch_returns_header_mismatch() {
    let app =
        McpRouter::new(sample_server_info()).register_tool(file_query_tool(), handle_file_query);

    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "file_query")
        .header("Mcp-Param-Repo", "conflicting-repo")
        .body(Body::from(
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {
                    "_meta": common::meta(),
                    "name": "file_query",
                    "arguments": {
                        "repo": "mcp-routing",
                        "path": "src/lib.rs"
                    }
                }
            })
            .to_string(),
        ))
        .unwrap();

    let (status, _, body) = execute_request(app, req).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], HEADER_MISMATCH);
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("does not match")
    );
}

/// Tests successful Base64 sentinel header decoding for parameter headers containing special characters and unicode.
#[tokio::test]
async fn test_param_header_sentinel_encoded_success() {
    let app =
        McpRouter::new(sample_server_info()).register_tool(file_query_tool(), handle_file_query);

    // "mcp-routing / workspace 🚀" in base64: "bWNwLXJvdXRpbmcgLyB3b3Jrc3BhY2Ug8J+agA=="
    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "file_query")
        .header(
            "Mcp-Param-Repo",
            "=?base64?bWNwLXJvdXRpbmcgLyB3b3Jrc3BhY2Ug8J+agA==?=",
        )
        .body(Body::from(
            json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "tools/call",
                "params": {
                    "_meta": common::meta(),
                    "name": "file_query",
                    "arguments": {
                        "repo": "mcp-routing / workspace 🚀",
                        "path": "src/main.rs"
                    }
                }
            })
            .to_string(),
        ))
        .unwrap();

    let (status, _, body) = execute_request(app, req).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["result"]["content"][0]["text"],
        "repo=mcp-routing / workspace 🚀, path=src/main.rs, branch=main"
    );
}

/// Tests that a Base64 sentinel header value that mismatches the body parameter returns `HEADER_MISMATCH`.
#[tokio::test]
async fn test_param_header_sentinel_encoded_mismatch() {
    let app =
        McpRouter::new(sample_server_info()).register_tool(file_query_tool(), handle_file_query);

    // Sentinel encoding of "wrong-repo" in base64: "d3JvbmctcmVwbw=="
    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "file_query")
        .header("Mcp-Param-Repo", "=?base64?d3JvbmctcmVwbw==?=")
        .body(Body::from(
            json!({
                "jsonrpc": "2.0",
                "id": 5,
                "method": "tools/call",
                "params": {
                    "_meta": common::meta(),
                    "name": "file_query",
                    "arguments": {
                        "repo": "mcp-routing",
                        "path": "src/lib.rs"
                    }
                }
            })
            .to_string(),
        ))
        .unwrap();

    let (status, _, body) = execute_request(app, req).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], HEADER_MISMATCH);
}

/// Tests validation and conversion of typed numeric and boolean parameter headers.
#[tokio::test]
async fn test_param_header_typed_numeric_and_boolean() {
    let app = McpRouter::new(sample_server_info())
        .register_tool(typed_params_tool(), handle_typed_params);

    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "typed_params")
        .header("Mcp-Param-Count", "42")
        .header("Mcp-Param-Active", "true")
        .body(Body::from(
            json!({
                "jsonrpc": "2.0",
                "id": 6,
                "method": "tools/call",
                "params": {
                    "_meta": common::meta(),
                    "name": "typed_params",
                    "arguments": {
                        "count": 42,
                        "active": true
                    }
                }
            })
            .to_string(),
        ))
        .unwrap();

    let (status, _, body) = execute_request(app, req).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["result"]["content"][0]["text"],
        "count=42, active=true"
    );
}

/// Tests that optional parameter headers omitted from both headers and body succeed.
#[tokio::test]
async fn test_param_header_optional_field_omitted_success() {
    let app =
        McpRouter::new(sample_server_info()).register_tool(file_query_tool(), handle_file_query);

    // Optional field "branch" has x-mcp-header: "Branch", but is omitted in both body and headers
    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "file_query")
        .header("Mcp-Param-Repo", "mcp-routing")
        .body(Body::from(
            json!({
                "jsonrpc": "2.0",
                "id": 7,
                "method": "tools/call",
                "params": {
                    "_meta": common::meta(),
                    "name": "file_query",
                    "arguments": {
                        "repo": "mcp-routing",
                        "path": "src/lib.rs"
                    }
                }
            })
            .to_string(),
        ))
        .unwrap();

    let (status, _, body) = execute_request(app, req).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["result"]["content"][0]["text"],
        "repo=mcp-routing, path=src/lib.rs, branch=main"
    );
}

/// Tests that optional parameter headers provided in both headers and body succeed.
#[tokio::test]
async fn test_param_header_optional_field_provided_in_both_success() {
    let app =
        McpRouter::new(sample_server_info()).register_tool(file_query_tool(), handle_file_query);

    // Optional field "branch" is provided in both body and header
    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "file_query")
        .header("Mcp-Param-Repo", "mcp-routing")
        .header("Mcp-Param-Branch", "develop")
        .body(Body::from(
            json!({
                "jsonrpc": "2.0",
                "id": 8,
                "method": "tools/call",
                "params": {
                    "_meta": common::meta(),
                    "name": "file_query",
                    "arguments": {
                        "repo": "mcp-routing",
                        "path": "src/lib.rs",
                        "branch": "develop"
                    }
                }
            })
            .to_string(),
        ))
        .unwrap();

    let (status, _, body) = execute_request(app, req).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["result"]["content"][0]["text"],
        "repo=mcp-routing, path=src/lib.rs, branch=develop"
    );
}

/// Tests that providing a parameter header when the parameter is omitted in body arguments returns `HEADER_MISMATCH`.
#[tokio::test]
async fn test_param_header_provided_without_body_param_returns_mismatch() {
    let app =
        McpRouter::new(sample_server_info()).register_tool(file_query_tool(), handle_file_query);

    // Mcp-Param-Branch provided in header, but "branch" omitted in body arguments
    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "file_query")
        .header("Mcp-Param-Repo", "mcp-routing")
        .header("Mcp-Param-Branch", "develop")
        .body(Body::from(
            json!({
                "jsonrpc": "2.0",
                "id": 9,
                "method": "tools/call",
                "params": {
                    "_meta": common::meta(),
                    "name": "file_query",
                    "arguments": {
                        "repo": "mcp-routing",
                        "path": "src/lib.rs"
                    }
                }
            })
            .to_string(),
        ))
        .unwrap();

    let (status, _, body) = execute_request(app, req).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], HEADER_MISMATCH);
}


#[derive(Serialize, Deserialize)]
struct SqlParams {
    tenant_id: String,
    query: String,
    options: SqlOptions,
}

#[derive(Serialize, Deserialize)]
struct SqlOptions {
    priority: i64,
}

async fn handle_sql(params: SqlParams) -> CallToolResult {
    CallToolResult::text(format!(
        "tenant={}, query={}, priority={}",
        params.tenant_id, params.query, params.options.priority
    ))
}

fn sql_tool() -> Tool {
    let mut tool = Tool::new("execute_sql");
    tool.input_schema = json!({
        "type": "object",
        "properties": {
            "tenant_id": { "type": "string", "x-mcp-header": "Tenant" },
            "query": { "type": "string" },
            "options": {
                "type": "object",
                "properties": {
                    "priority": { "type": "integer", "x-mcp-header": "Priority" }
                }
            }
        },
        "required": ["tenant_id", "query", "options"]
    });
    tool
}

fn sql_request(param_headers: &[(&str, &str)]) -> Request<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "execute_sql");
    for (name, value) in param_headers {
        builder = builder.header(*name, *value);
    }
    builder
        .body(Body::from(
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": {
                    "_meta": common::meta(),
                    "name": "execute_sql",
                    "arguments": {
                        "tenant_id": "acme",
                        "query": "SELECT 1",
                        "options": { "priority": 3 }
                    }
                }
            })
            .to_string(),
        ))
        .unwrap()
}

/// Tests that the `x-mcp-header` value (not the property name) names the `Mcp-Param-*` header.
///
/// Verifies:
/// - `tenant_id` annotated with `"Tenant"` is validated against `Mcp-Param-Tenant`
/// - Nested `options.priority` annotated with `"Priority"` is validated against `Mcp-Param-Priority`
#[tokio::test]
async fn test_param_header_name_from_annotation_value() {
    let app = McpRouter::new(sample_server_info()).register_tool(sql_tool(), handle_sql);

    let req = sql_request(&[("Mcp-Param-Tenant", "acme"), ("Mcp-Param-Priority", "3")]);
    let (status, _, body) = execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["result"]["content"][0]["text"],
        "tenant=acme, query=SELECT 1, priority=3"
    );
}

/// Tests that omitting a required `Mcp-Param-*` header for an annotated argument is rejected.
///
/// Verifies:
/// - Missing `Mcp-Param-Tenant` while `tenant_id` is in the body returns HTTP 400 with `-32020`
/// - Missing `Mcp-Param-Priority` for a nested argument returns HTTP 400 with `-32020`
#[tokio::test]
async fn test_param_header_required_when_annotated_value_present() {
    for headers in [
        vec![("Mcp-Param-Priority", "3")],
        vec![("Mcp-Param-Tenant", "acme")],
    ] {
        let app = McpRouter::new(sample_server_info()).register_tool(sql_tool(), handle_sql);
        let (status, _, body) = execute_request(app, sql_request(&headers)).await;

        assert_eq!(status, StatusCode::BAD_REQUEST, "headers: {headers:?}");
        assert_eq!(body["error"]["code"], HEADER_MISMATCH, "headers: {headers:?}");
    }
}

/// Tests that integer `Mcp-Param-*` values are compared numerically (`3.0` matches `3`).
#[tokio::test]
async fn test_param_header_integer_compared_numerically() {
    let app = McpRouter::new(sample_server_info()).register_tool(sql_tool(), handle_sql);

    let req = sql_request(&[("Mcp-Param-Tenant", "acme"), ("Mcp-Param-Priority", "3.0")]);
    let (status, _, body) = execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["result"]["content"][0]["text"],
        "tenant=acme, query=SELECT 1, priority=3"
    );
}

/// Tests that a malformed Base64 sentinel `Mcp-Param-*` value is rejected with `-32020`.
#[tokio::test]
async fn test_param_header_malformed_sentinel_is_rejected() {
    let app = McpRouter::new(sample_server_info()).register_tool(sql_tool(), handle_sql);

    let req = sql_request(&[
        ("Mcp-Param-Tenant", "=?base64?not base64?="),
        ("Mcp-Param-Priority", "3"),
    ]);
    let (status, _, body) = execute_request(app, req).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], HEADER_MISMATCH);
}
