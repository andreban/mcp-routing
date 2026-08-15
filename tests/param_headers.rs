// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Custom Parameter Headers (`x-mcp-header` & `Mcp-Param-{Name}`) Integration Tests
//!
//! Verifies the behavior of the Model Context Protocol (MCP) Streamable HTTP parameter headers:
//! - Extracting `x-mcp-header: true` annotations from tool `inputSchema`
//! - Validating `Mcp-Param-{Name}` HTTP request headers against `tools/call` arguments
//! - Rejection with HTTP 400 Bad Request and error code -32020 (`HEADER_MISMATCH`) on missing or mismatched headers
//! - RFC 2047-style Base64 sentinel decoding (`=?base64?...?=`) for parameter headers
//! - Support for string, numeric, and boolean parameter types
//! - Proper handling of optional parameters and batch requests

mod common;

use axum::body::Body;
use http::{Request, StatusCode};
use mcp_routing::{
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
                "x-mcp-header": true
            },
            "path": {
                "type": "string"
            },
            "branch": {
                "type": "string",
                "x-mcp-header": true
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
                "x-mcp-header": true
            },
            "active": {
                "type": "boolean",
                "x-mcp-header": true
            }
        },
        "required": ["count", "active"]
    });
    tool
}

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

#[tokio::test]
async fn test_param_header_optional_field_omitted_success() {
    let app =
        McpRouter::new(sample_server_info()).register_tool(file_query_tool(), handle_file_query);

    // Optional field "branch" has x-mcp-header: true, but is omitted in both body and headers
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

#[tokio::test]
async fn test_param_header_batch_request_without_header_success() {
    let app =
        McpRouter::new(sample_server_info()).register_tool(file_query_tool(), handle_file_query);

    // Batch requests do not enforce top-level Mcp-Param-* headers if omitted
    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .body(Body::from(
            json!([
                {
                    "jsonrpc": "2.0",
                    "id": 10,
                    "method": "tools/call",
                    "params": {
                        "name": "file_query",
                        "arguments": {
                            "repo": "mcp-routing-1",
                            "path": "src/lib.rs"
                        }
                    }
                },
                {
                    "jsonrpc": "2.0",
                    "id": 11,
                    "method": "tools/call",
                    "params": {
                        "name": "file_query",
                        "arguments": {
                            "repo": "mcp-routing-2",
                            "path": "src/main.rs"
                        }
                    }
                }
            ])
            .to_string(),
        ))
        .unwrap();

    let (status, _, body) = execute_request(app, req).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.is_array());
    assert_eq!(body.as_array().unwrap().len(), 2);
    assert_eq!(
        body[0]["result"]["content"][0]["text"],
        "repo=mcp-routing-1, path=src/lib.rs, branch=main"
    );
    assert_eq!(
        body[1]["result"]["content"][0]["text"],
        "repo=mcp-routing-2, path=src/main.rs, branch=main"
    );
}
