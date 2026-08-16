// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Subscriptions MCP Server Example
//!
//! Demonstrates event-driven notifications streaming (`subscriptions/listen` per SEP-2575)
//! over Server-Sent Events (SSE) in an Axum application.
//!
//! Notifications are triggered **only** when state changes actually occur (e.g. tools
//! list modified or a specific resource edited), and are filtered so each client only
//! receives notifications for the specific events and resource URIs they subscribed to.

use std::collections::HashMap;
use std::error::Error;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use axum::Router;
use bytes::Bytes;
use http_body::{Body, Frame};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::{RwLock, broadcast, mpsc};

use mcp_routing::{
    BoxError, McpRouter, ResponseBody, format_sse_message,
    extract::{Json, RequestContext, State},
    types::mcp::{
        Implementation, NotificationSubscriptions,
        resources::Resource,
        subscriptions::{
            ResourceUpdatedParams, resource_updated_notification, tools_list_changed_notification,
        },
        tools::Tool,
    },
};

/// Domain events emitted only when real state changes occur.
#[derive(Clone, Debug)]
enum DomainEvent {
    /// The available tools list has changed (e.g., dynamic tools registered or enabled).
    ToolsListChanged,
    /// A specific resource URI was modified.
    ResourceUpdated(String),
}

/// A custom [`http_body::Body`] streaming SSE frames from an [`mpsc::Receiver`].
struct SubscriberBody {
    rx: mpsc::Receiver<Bytes>,
}

impl Body for SubscriberBody {
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match self.rx.poll_recv(cx) {
            Poll::Ready(Some(bytes)) => Poll::Ready(Some(Ok(Frame::data(bytes)))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[derive(Clone)]
struct AppState {
    event_tx: broadcast::Sender<DomainEvent>,
    logs: Arc<RwLock<Vec<String>>>,
    config: Arc<RwLock<HashMap<String, String>>>,
    advanced_mode_enabled: Arc<RwLock<bool>>,
}

#[derive(Serialize, Deserialize)]
struct AppendLogArgs {
    entry: String,
}

#[derive(Serialize, Deserialize)]
struct UpdateConfigArgs {
    key: String,
    value: String,
}

/// Tool handler that appends a log entry to `file:///logs/app.log`.
/// Emits `DomainEvent::ResourceUpdated("file:///logs/app.log")` only because that resource changed.
async fn append_log(
    state: State<AppState>,
    Json(args): Json<AppendLogArgs>,
) -> Result<String, String> {
    {
        let mut logs = state.logs.write().await;
        logs.push(format!("{}\n", args.entry));
    }

    // Emit event strictly for the resource that was modified
    let _ = state
        .event_tx
        .send(DomainEvent::ResourceUpdated("file:///logs/app.log".to_string()));

    Ok(format!("Logged: {}", args.entry))
}

/// Tool handler that updates a configuration key in `file:///config/settings.json`.
/// Emits `DomainEvent::ResourceUpdated("file:///config/settings.json")` only because that resource changed.
async fn update_config(
    state: State<AppState>,
    Json(args): Json<UpdateConfigArgs>,
) -> Result<String, String> {
    {
        let mut config = state.config.write().await;
        config.insert(args.key.clone(), args.value.clone());
    }

    // Emit event strictly for the configuration resource that was modified
    let _ = state
        .event_tx
        .send(DomainEvent::ResourceUpdated("file:///config/settings.json".to_string()));

    Ok(format!("Updated config: {} = {}", args.key, args.value))
}

/// Tool handler that toggles advanced mode on/off, altering the set of active capabilities.
/// Emits `DomainEvent::ToolsListChanged` because the tools list was modified.
async fn toggle_advanced_mode(state: State<AppState>) -> Result<String, String> {
    let new_mode = {
        let mut mode = state.advanced_mode_enabled.write().await;
        *mode = !*mode;
        *mode
    };

    // Emit event strictly because the toolset availability changed
    let _ = state.event_tx.send(DomainEvent::ToolsListChanged);

    Ok(format!("Advanced mode is now: {}", if new_mode { "ENABLED" } else { "DISABLED" }))
}

/// Handles `subscriptions/listen` requests, acknowledges supported filters, and spawns a
/// filtered stream adapter so the client receives **only** notifications matching their subscription.
async fn handle_listen(
    _ctx: RequestContext,
    state: State<AppState>,
) -> Result<(NotificationSubscriptions, ResponseBody), String> {
    // 1. Establish the filter rules for this client
    let subscribed_filters = NotificationSubscriptions::new()
        .with_tools_list_changed(true)
        .with_resources_list_changed(true)
        .with_resource_subscriptions(vec![
            "file:///logs/app.log".to_string(),
            "file:///config/settings.json".to_string(),
        ]);

    // 2. Set up the per-connection channel
    let (tx, rx) = mpsc::channel::<Bytes>(100);
    let mut event_rx = state.event_tx.subscribe();
    let client_filters = subscribed_filters.clone();

    // 3. Filtered forwarding task: Only deliver events matching what this client subscribed to
    tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            let notification_bytes = match event {
                DomainEvent::ToolsListChanged => {
                    if client_filters.tools_list_changed == Some(true) {
                        let notif = tools_list_changed_notification(None);
                        format_sse_message(&notif).ok()
                    } else {
                        None
                    }
                }
                DomainEvent::ResourceUpdated(ref uri) => {
                    let is_subscribed_uri = client_filters
                        .resource_subscriptions
                        .as_ref()
                        .is_some_and(|uris| uris.contains(uri));

                    if is_subscribed_uri {
                        let notif = resource_updated_notification(ResourceUpdatedParams::new(uri));
                        format_sse_message(&notif).ok()
                    } else {
                        None
                    }
                }
            };

            if let Some(bytes) = notification_bytes {
                if tx.send(bytes).await.is_err() {
                    // Client disconnected
                    break;
                }
            }
        }
    });

    let body = ResponseBody::new(SubscriberBody { rx });
    Ok((subscribed_filters, body))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let (event_tx, _) = broadcast::channel::<DomainEvent>(100);
    let mut initial_config = HashMap::new();
    initial_config.insert("environment".to_string(), "production".to_string());
    initial_config.insert("debug_mode".to_string(), "false".to_string());

    let app_state = AppState {
        event_tx,
        logs: Arc::new(RwLock::new(vec![
            "2026-08-16 08:00:00 [INFO] Server started\n".to_string(),
        ])),
        config: Arc::new(RwLock::new(initial_config)),
        advanced_mode_enabled: Arc::new(RwLock::new(false)),
    };

    let server_info = Implementation::new("subscriptions-mcp-server", "1.0.0");

    let append_tool = Tool {
        icons: Vec::new(),
        name: "append_log".to_string(),
        title: Some("Append Log Entry".to_string()),
        description: Some("Appends a log line to file:///logs/app.log and emits a resource updated notification".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "entry": { "type": "string", "description": "Log message text" }
            },
            "required": ["entry"]
        }),
        output_schema: None,
        annotations: None,
        meta: None,
    };

    let update_config_tool = Tool {
        icons: Vec::new(),
        name: "update_config".to_string(),
        title: Some("Update Configuration".to_string()),
        description: Some("Updates a configuration value in file:///config/settings.json and emits a resource updated notification".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "key": { "type": "string" },
                "value": { "type": "string" }
            },
            "required": ["key", "value"]
        }),
        output_schema: None,
        annotations: None,
        meta: None,
    };

    let toggle_advanced_tool = Tool {
        icons: Vec::new(),
        name: "toggle_advanced_mode".to_string(),
        title: Some("Toggle Advanced Mode".to_string()),
        description: Some("Toggles advanced tools and emits a tools list changed notification".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {}
        }),
        output_schema: None,
        annotations: None,
        meta: None,
    };

    let log_resource = Resource::new("file:///logs/app.log", "Application Log File");
    let config_resource = Resource::new("file:///config/settings.json", "Application Configuration");

    let log_state = app_state.clone();
    let config_state = app_state.clone();

    let mcp_router = McpRouter::new(server_info)
        .instructions("MCP server demonstrating event-driven subscriptions/listen notification streams")
        .with_state(app_state)
        .register_tool(append_tool, append_log)
        .register_tool(update_config_tool, update_config)
        .register_tool(toggle_advanced_tool, toggle_advanced_mode)
        .register_resource(log_resource, move || {
            let state = log_state.clone();
            async move {
                let l = state.logs.read().await;
                Ok::<String, String>(l.join(""))
            }
        })
        .register_resource(config_resource, move || {
            let state = config_state.clone();
            async move {
                let c = state.config.read().await;
                let json_str = serde_json::to_string_pretty(&*c).unwrap_or_default();
                Ok::<String, String>(json_str)
            }
        })
        .subscriptions_listen(handle_listen);

    let app = Router::new().nest_service("/mcp", mcp_router);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    println!("============================================================");
    println!("Event-Driven Subscriptions Server on http://127.0.0.1:3000/mcp");
    println!("1. Open an SSE subscription stream:");
    println!("   POST /mcp (Header: Mcp-Method: subscriptions/listen)");
    println!("2. Notifications are triggered ONLY when actual state changes occur:");
    println!("   - Call `append_log` -> signals `notifications/resources/updated` for `file:///logs/app.log`");
    println!("   - Call `update_config` -> signals `notifications/resources/updated` for `file:///config/settings.json`");
    println!("   - Call `toggle_advanced_mode` -> signals `notifications/tools/list_changed`");
    println!("============================================================");

    axum::serve(listener, app).await?;
    Ok(())
}
