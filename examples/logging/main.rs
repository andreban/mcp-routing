// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Logging & Diagnostics Example
//!
//! Demonstrates configuring default server logging thresholds, advertising logging capabilities,
//! handling per-request `_meta.io.modelcontextprotocol/logLevel`, and inspecting server log levels.

use std::error::Error;

use axum::Router;
use mcp_routing::{
    CurrentLoggingLevel, McpRouter,
    types::mcp::{Implementation, LoggingLevel, tools::Tool},
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize)]
pub struct ProcessParams {
    pub task_name: String,
}

async fn process_task(
    opt_level: Option<LoggingLevel>,
    current_level: CurrentLoggingLevel,
    params: ProcessParams,
) -> Result<String, String> {
    let effective_level = opt_level.unwrap_or(current_level.level());
    tracing::info!(
        task = %params.task_name,
        %effective_level,
        server_level = %current_level,
        "Executing task"
    );

    Ok(format!(
        "Processed task '{}' with effective log level: {effective_level} (server threshold: {current_level})",
        params.task_name
    ))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let server_info = Implementation::new("logging-mcp-server", "1.0.0");

    let process_tool = Tool {
        icons: Vec::new(),
        name: "process_task".to_string(),
        title: Some("Process Task".to_string()),
        description: Some(
            "Processes a task while respecting client/server log level thresholds".to_string(),
        ),
        input_schema: json!({
            "type": "object",
            "properties": {
                "task_name": { "type": "string", "description": "Name of the task to process" }
            },
            "required": ["task_name"]
        }),
        output_schema: None,
        annotations: None,
        meta: None,
    };

    let mcp_router = McpRouter::new(server_info)
        .instructions("MCP server demonstrating logging capabilities and per-request log levels")
        // Initialize default logging level to Info and advertise logging capability
        .logging_level(LoggingLevel::Info)
        .register_tool(process_tool, process_task);

    let app = Router::new().nest_service("/mcp", mcp_router);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    println!("Logging MCP server listening on http://127.0.0.1:3000/mcp");
    axum::serve(listener, app).await?;
    Ok(())
}
