// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Handler-Based Server Discovery & Capabilities Example
//!
//! Demonstrates how to use first-class handler functions in `mcp-routing` to generate
//! context-aware server instructions, capabilities, tool lists, and prompt lists on a per-request basis
//! using extractors like `BearerAuth`, `SessionId`, `Meta`, and `State`.

use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::Router;
use mcp_routing::{
    BearerAuth, McpRouter, Meta, RegisteredPrompts, RegisteredTools, SessionId, State,
    types::mcp::{
        CacheScope, Implementation, PromptsCapability, ResourcesCapability, ServerCapabilities,
        ToolsCapability,
        prompts::{Prompt, PromptArgument, PromptMessage, get::GetPromptResult},
        server::discover::ServerDiscoverResult,
        tools::{Tool, list::ListToolsResult},
    },
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone)]
struct ServerState {
    active_sessions: Arc<AtomicUsize>,
}

#[derive(Serialize, Deserialize)]
struct StatusParams {
    verbose: Option<bool>,
}

async fn status_tool(
    State(state): State<ServerState>,
    params: StatusParams,
) -> Result<String, String> {
    let count = state.active_sessions.load(Ordering::SeqCst);
    if params.verbose.unwrap_or(false) {
        Ok(format!(
            "Server running in dynamic mode with {count} active sessions."
        ))
    } else {
        Ok(format!("Active sessions: {count}"))
    }
}

/// Admin-only tool handler verifying Bearer authorization token.
async fn admin_restart_tool(auth: BearerAuth) -> Result<String, String> {
    if auth.token() != "admin-secret-token" {
        return Err("Unauthorized: Invalid admin secret token".to_string());
    }
    Ok("Server restart initiated successfully by administrator.".to_string())
}

/// Prompt handler generating diagnostic templates.
async fn admin_diagnostics_prompt() -> GetPromptResult {
    GetPromptResult {
        meta: None,
        result_type: Some("complete".to_string()),
        description: Some("System Diagnostic Report".to_string()),
        messages: vec![PromptMessage::user_text(
            "Please check system CPU, memory, and database connections.",
        )],
        extras: std::collections::HashMap::new(),
    }
}

/// Server discovery handler function.
///
/// Inspects incoming session ID, Bearer auth token, client metadata, and application state
/// to tailor capabilities and instructions.
async fn server_discover_handler(
    auth: Option<BearerAuth>,
    session: Option<SessionId>,
    meta: Option<Meta>,
    State(state): State<ServerState>,
) -> ServerDiscoverResult {
    let session_count = state.active_sessions.fetch_add(1, Ordering::SeqCst) + 1;
    let client_name = meta
        .as_ref()
        .and_then(|m| m.client_info.as_ref())
        .map(|c| c.name.as_str())
        .unwrap_or("standard-client");

    let is_admin = auth.as_ref().map(|a| a.token()) == Some("admin-secret-token")
        || session.as_deref().unwrap_or("").starts_with("admin-");

    let capabilities = ServerCapabilities {
        tools: Some(ToolsCapability {
            list_changed: Some(true),
        }),
        resources: Some(ResourcesCapability {
            subscribe: Some(is_admin),
            list_changed: Some(false),
        }),
        prompts: if is_admin {
            Some(PromptsCapability { list_changed: None })
        } else {
            None
        },
        completions: None,
        logging: None,
        experimental: None,
        extensions: None,
    };

    let instructions = format!(
        "Welcome {client_name}! (Session: {}, Discoveries: {session_count}, Admin mode: {is_admin})",
        session.as_deref().unwrap_or("none")
    );

    ServerDiscoverResult::new(capabilities, vec!["2026-07-28".to_string()])
        .with_instructions(instructions)
        .with_cache(
            Some(60_000),
            Some(if session.is_some() || auth.is_some() {
                CacheScope::Private
            } else {
                CacheScope::Public
            }),
        )
}

/// Tools list handler function filtering the pre-registered tools using `RegisteredTools`.
async fn tools_list_handler(
    auth: Option<BearerAuth>,
    session: Option<SessionId>,
    RegisteredTools(all_tools): RegisteredTools,
) -> ListToolsResult {
    let is_admin = auth.as_ref().map(|a| a.token()) == Some("admin-secret-token")
        || session.as_deref().unwrap_or("").starts_with("admin-");

    let filtered = all_tools
        .into_iter()
        .filter(|t| !t.name.starts_with("admin_") || is_admin)
        .collect();

    ListToolsResult::new(filtered).with_cache(Some(30_000), Some(CacheScope::Private))
}

/// Prompts list handler function filtering the pre-registered prompts using `RegisteredPrompts`.
async fn prompts_list_handler(
    auth: Option<BearerAuth>,
    RegisteredPrompts(all_prompts): RegisteredPrompts,
) -> Result<Vec<Prompt>, String> {
    let is_admin = auth.as_ref().map(|a| a.token()) == Some("admin-secret-token");

    Ok(all_prompts
        .into_iter()
        .filter(|p| !p.name.starts_with("admin_") || is_admin)
        .collect())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let server_info = Implementation::new("discovery-mcp-server", "1.0.0")
        .with_title("Handler-Based Discovery MCP Server")
        .with_description(
            "Demonstrates handler-based discovery, tools/list, and prompts/list with extractors",
        );

    let status_tool_def = Tool {
        icons: Vec::new(),
        name: "status".to_string(),
        title: Some("Status Tool".to_string()),
        description: Some("Returns server status information".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "verbose": { "type": "boolean" }
            }
        }),
        output_schema: None,
        annotations: None,
        meta: None,
    };

    let admin_restart_def = Tool {
        icons: Vec::new(),
        name: "admin_restart".to_string(),
        title: Some("Admin Server Restart".to_string()),
        description: Some("Administrative tool to restart services".to_string()),
        input_schema: json!({ "type": "object" }),
        output_schema: None,
        annotations: None,
        meta: None,
    };

    let admin_prompt_def = Prompt {
        name: "admin_diagnostics".to_string(),
        title: Some("System Diagnostics".to_string()),
        description: Some("Diagnose server subsystem health".to_string()),
        arguments: vec![PromptArgument {
            name: "subsystem".to_string(),
            title: None,
            description: Some("Target subsystem name".to_string()),
            required: Some(false),
        }],
        icons: Vec::new(),
        meta: None,
    };

    let state = ServerState {
        active_sessions: Arc::new(AtomicUsize::new(0)),
    };

    // 1. Single source of truth: register all tool and prompt definitions
    // 2. Attach first-class handlers for discovery, tool listing, and prompt listing
    let mcp_router = McpRouter::new(server_info)
        .with_state(state.clone())
        .validate_protocol_version(true)
        .register_tool(status_tool_def, status_tool)
        .register_tool(admin_restart_def, admin_restart_tool)
        .register_prompt(admin_prompt_def, admin_diagnostics_prompt)
        .discover(server_discover_handler)
        .tools_list(tools_list_handler)
        .prompts_list(prompts_list_handler);

    let app = Router::new().nest_service("/mcp", mcp_router);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    println!("Discovery MCP server listening on http://127.0.0.1:3000/mcp");
    println!("Send requests with headers:");
    println!("  - Authorization: Bearer admin-secret-token (enables admin tools and prompts)");
    println!("  - Mcp-Session-Id: admin-1234 (enables admin capabilities)");
    axum::serve(listener, app).await?;
    Ok(())
}
