// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Context & Extractors MCP Server Example
//!
//! Demonstrates sharing application state between standard Axum web routes
//! and MCP handlers using `with_state` and the `State<T>` extractor.

use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::Router;
use axum::extract::State as AxumState;
use axum::routing::get;
use mcp_routing::{
    McpRouter, Meta, SessionId, State,
    types::mcp::{Implementation, tools::Tool},
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone)]
struct AppState {
    request_counter: Arc<AtomicUsize>,
}

#[derive(Serialize, Deserialize)]
struct GreetingParams {
    name: String,
}

// MCP tool handler using mcp_routing::State<AppState>
async fn greet(
    session: Option<SessionId>,
    State(state): State<AppState>,
    meta: Option<Meta>,
    params: GreetingParams,
) -> Result<String, String> {
    let count = state.request_counter.fetch_add(1, Ordering::SeqCst) + 1;
    let session_str = session.as_deref().unwrap_or("no-session");
    let client_name = meta
        .as_ref()
        .and_then(|m| m.client_info.as_ref())
        .map(|c| c.name.as_str())
        .unwrap_or("anonymous client");

    Ok(format!(
        "Hello, {}! (Request #{count}, Session: {session_str}, Client: {client_name})",
        params.name
    ))
}

// Standard Axum HTTP route sharing the same AppState via axum::extract::State
async fn stats(AxumState(state): AxumState<AppState>) -> String {
    let count = state.request_counter.load(Ordering::SeqCst);
    format!("Total requests processed: {count}")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let server_info = Implementation::new("extractors-mcp-server", "0.1.0");

    let greet_tool = Tool {
        icons: Vec::new(),
        name: "greet".to_string(),
        title: Some("Greeting Tool".to_string()),
        description: Some("Greets a user with session and request statistics".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "description": "Name of the person to greet" }
            },
            "required": ["name"]
        }),
        output_schema: None,
        annotations: None,
        meta: None,
    };

    let state = AppState {
        request_counter: Arc::new(AtomicUsize::new(0)),
    };

    // Attach state directly to the MCP router using with_state
    let mcp_router = McpRouter::new(server_info)
        .instructions("MCP server demonstrating extractors and session correlation")
        .with_state(state.clone())
        .register_tool(greet_tool, greet);

    // Share the same AppState between Axum web routes and MCP router
    let app = Router::new()
        .route("/stats", get(stats))
        .nest_service("/mcp", mcp_router)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    println!("Extractors MCP server listening on http://127.0.0.1:3000/mcp");
    println!("Web statistics endpoint at http://127.0.0.1:3000/stats");
    axum::serve(listener, app).await?;
    Ok(())
}
