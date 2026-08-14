// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Basic MCP Server Example
//!
//! A minimal Model Context Protocol (MCP) server embedded in an Axum web application.

use std::error::Error;

use axum::Router;
use mcp_routing::{
    McpRouter,
    types::mcp::{Implementation, tools::Tool},
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize)]
pub struct EchoParams {
    pub value: String,
}

async fn echo(params: EchoParams) -> Result<String, String> {
    if params.value.is_empty() {
        return Err("Missing required parameter: value".to_string());
    }
    Ok(params.value)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let server_info = Implementation::new("basic-mcp-server", "0.1.0");

    let echo_tool = Tool {
        icons: Vec::new(),
        name: "echo".to_string(),
        title: Some("Echo Tool".to_string()),
        description: Some("Echoes the input string back to the client".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "value": { "type": "string", "description": "Text to echo" }
            },
            "required": ["value"]
        }),
        output_schema: None,
        annotations: None,
        meta: None,
    };

    let mcp_router = McpRouter::new(server_info)
        .instructions("Basic MCP server providing an echo tool")
        .register_tool(echo_tool, echo);

    let app = Router::new().nest_service("/mcp", mcp_router);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    println!("Basic MCP server listening on http://127.0.0.1:3000/mcp");
    axum::serve(listener, app).await?;
    Ok(())
}
