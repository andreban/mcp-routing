// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Shared Integration Test Utilities
//!
//! Helper functions for building sample metadata, generating mock tool definitions,
//! constructing Tower HTTP requests with MCP headers, executing requests via `tower::ServiceExt::oneshot`,
//! and issuing raw HTTP/1.1 requests over live TCP sockets.

#![allow(dead_code)]

use std::collections::HashMap;
use std::net::SocketAddr;

use axum::body::Body;
use bytes::Bytes;
use http::{HeaderMap, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tower::ServiceExt;

use mcp_routing::{
    McpRouter,
    types::mcp::{
        Icon, IconTheme, Implementation,
        tools::{Tool, ToolAnnotations},
    },
};

/// Returns a pre-configured [`Implementation`] with name, title, version, description, website URL, and icon.
pub fn sample_server_info() -> Implementation {
    Implementation {
        icons: vec![Icon {
            src: "https://example.com/icon.png".to_string(),
            mime_type: Some("image/png".into()),
            sizes: vec!["64x64".to_string()],
            theme: Some(IconTheme::Dark),
        }],
        name: "test-mcp-server".to_string(),
        title: Some("Test MCP Server".to_string()),
        version: "1.2.3".to_string(),
        description: Some("Integration test server".to_string()),
        website_url: Some("https://example.com".to_string()),
    }
}

/// Returns a fully specified [`Tool`] definition including JSON schemas, behavioral hints, icons, and metadata.
pub fn sample_tool(name: &str) -> Tool {
    Tool {
        icons: vec![Icon {
            src: "https://example.com/tool_icon.png".to_string(),
            mime_type: Some("image/png".into()),
            sizes: vec!["32x32".to_string()],
            theme: Some(IconTheme::Light),
        }],
        name: name.to_string(),
        title: Some(format!("Title for {}", name)),
        description: Some(format!("Description for {}", name)),
        input_schema: json!({
            "type": "object",
            "properties": {
                "input": { "type": "string", "description": "Sample input string" },
                "count": { "type": "integer", "description": "Sample count" }
            },
            "required": ["input"]
        }),
        output_schema: Some(json!({
            "type": "object",
            "properties": {
                "result": { "type": "string" }
            }
        })),
        annotations: Some(ToolAnnotations {
            title: Some(format!("Annotation title for {}", name)),
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(true),
            open_world_hint: Some(false),
        }),
        meta: {
            let mut meta = HashMap::new();
            meta.insert("customMeta".to_string(), json!("customValue"));
            Some(meta)
        },
    }
}

/// Returns a fully specified [`Prompt`] definition including arguments, icons, and metadata.
pub fn sample_prompt(name: &str) -> mcp_routing::types::mcp::prompts::Prompt {
    use mcp_routing::types::mcp::prompts::{Prompt, PromptArgument};
    let mut meta = HashMap::new();
    meta.insert("customPromptMeta".to_string(), json!("promptMetaVal"));

    Prompt {
        icons: vec![Icon {
            src: "https://example.com/prompt_icon.png".to_string(),
            mime_type: Some("image/png".into()),
            sizes: vec!["48x48".to_string()],
            theme: Some(IconTheme::Dark),
        }],
        name: name.to_string(),
        title: Some(format!("Title for {}", name)),
        description: Some(format!("Description for {}", name)),
        arguments: vec![
            PromptArgument::new("topic")
                .title("Topic")
                .description("The main subject of discussion")
                .required(true),
            PromptArgument::new("style")
                .title("Style")
                .description("Tone or style")
                .required(false),
        ],
        meta: Some(meta),
    }
}

/// Builds an HTTP `POST /` request containing the optional `Mcp-Method` and `Mcp-Name` headers with a JSON body.
pub fn build_request(
    method_header: Option<&str>,
    name_header: Option<&str>,
    body: Value,
) -> Request<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json");

    if let Some(m) = method_header {
        builder = builder.header("Mcp-Method", m);
    }
    if let Some(n) = name_header {
        builder = builder.header("Mcp-Name", n);
    }

    builder.body(Body::from(body.to_string())).unwrap()
}

/// Executes a request against an [`McpRouter`] using `oneshot`, returning the status code, header map, and parsed JSON value.
pub async fn execute_request(
    app: McpRouter,
    req: Request<Body>,
) -> (StatusCode, HeaderMap, Value) {
    let response = app.oneshot(req).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json_body: Value = if body_bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&body_bytes).unwrap_or(Value::Null)
    };

    (status, headers, json_body)
}

/// Executes a request against an [`McpRouter`] using `oneshot`, returning the raw [`Bytes`] body.
pub async fn execute_request_raw(
    app: McpRouter,
    req: Request<Body>,
) -> (StatusCode, HeaderMap, Bytes) {
    let response = app.oneshot(req).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();

    (status, headers, body_bytes)
}

/// Sends a raw HTTP/1.1 request directly over a TCP socket connection, reading the complete response.
pub async fn send_raw_http_request(
    addr: SocketAddr,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &str,
) -> (StatusCode, HeaderMap, String) {
    let mut stream = TcpStream::connect(addr).await.unwrap();

    let mut request_raw = format!("{method} {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n");
    request_raw.push_str(&format!("Content-Length: {}\r\n", body.len()));
    for (k, v) in headers {
        request_raw.push_str(&format!("{k}: {v}\r\n"));
    }
    request_raw.push_str("\r\n");
    request_raw.push_str(body);

    stream.write_all(request_raw.as_bytes()).await.unwrap();

    let mut response_buf = Vec::new();
    stream.read_to_end(&mut response_buf).await.unwrap();
    let response_str = String::from_utf8_lossy(&response_buf);

    let mut parts = response_str.split("\r\n\r\n");
    let header_part = parts.next().unwrap_or("");
    let body_part = parts.next().unwrap_or("").to_string();

    let mut header_lines = header_part.lines();
    let status_line = header_lines.next().unwrap_or("");
    let status_code_num: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(500);
    let status = StatusCode::from_u16(status_code_num).unwrap();

    let mut header_map = HeaderMap::new();
    for line in header_lines {
        if let Some((k, v)) = line.split_once(':')
            && let Ok(name) = http::header::HeaderName::from_bytes(k.trim().as_bytes())
            && let Ok(val) = http::HeaderValue::from_str(v.trim())
        {
            header_map.insert(name, val);
        }
    }

    (status, header_map, body_part)
}
