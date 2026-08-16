// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # MCP Routing
//!
//! `mcp-routing` is a [Tower](https://crates.io/crates/tower)-native routing library for building
//! [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) servers in Rust.
//!
//! This library specifically implements the **stateless** version of the MCP specification
//! ([`2026-07-28`](https://modelcontextprotocol.io/docs/2026-07-28/)). It relies on request-based
//! discovery (`server/discover`) and direct tool execution over HTTP, and does **not** support
//! earlier stateful protocol versions (such as `2024-11-05` session initialization).
//!
//! It provides a composable [`McpRouter`] that implements [`tower::Service`], allowing it to be used
//! directly with Hyper, standalone Tower pipelines, or embedded into web frameworks like [Axum](https://crates.io/crates/axum).
//!
//! ## Features
//!
//! - **Stateless MCP (`2026-07-28`)**: Implements `server/discover` and stateless JSON-RPC request handling.
//! - **Tower-Native**: Implements [`tower::Service`] for any request body implementing [`http_body::Body`].
//! - **Header-Based Routing**: Dispatches requests based on `Mcp-Method` and `Mcp-Name` headers per the MCP HTTP spec.
//! - **Typed Tool Handlers**: Register async functions directly as tools with automatic JSON deserialization
//!   of arguments and serialization of results.
//! - **Zero Framework Lock-in**: No hard dependency on Axum in the core library—use it with any Tower-compatible server.
//!
//! ## Example
//!
//! ```rust,no_run
//! use mcp_routing::{
//!     McpRouter,
//!     types::mcp::{Implementation, tools::Tool},
//! };
//! use serde::{Deserialize, Serialize};
//! use serde_json::json;
//!
//! #[derive(Serialize, Deserialize)]
//! struct EchoParams {
//!     value: String,
//! }
//!
//! async fn echo(params: EchoParams) -> Result<String, String> {
//!     Ok(params.value)
//! }
//!
//! let server_info = Implementation::new("example-mcp-server", "1.0.0");
//! let echo_tool = Tool {
//!     icons: Vec::new(),
//!     name: "echo".to_string(),
//!     title: Some("Echo Tool".to_string()),
//!     description: Some("Echoes the input".to_string()),
//!     input_schema: json!({
//!         "type": "object",
//!         "properties": { "value": { "type": "string" } },
//!         "required": ["value"]
//!     }),
//!     output_schema: None,
//!     annotations: None,
//!     meta: None,
//! };
//!
//! let mcp_router = McpRouter::new(server_info)
//!     .instructions("Example MCP server providing an echo tool")
//!     .server_discover_ttl(3_600_000) // 1 hour TTL
//!     .tools_list_ttl(300_000)        // 5 minutes TTL
//!     .register_tool(echo_tool, echo);
//! ```

pub mod body;
pub mod completion;
pub mod extract;
pub mod prompts;
pub mod resources;
pub mod router;
pub mod server;
pub mod subscriptions;
pub mod tools;
pub mod types;
pub mod utils;

pub use body::{BoxError, ResponseBody, compute_etag, format_cache_control};
pub use completion::{
    CompletionError, CompletionHandler, CompletionRegistry, IntoCompletionHandler,
    IntoCompletionResult,
};
pub use extract::{
    Authorization, BearerAuth, CurrentLoggingLevel, Extension, ExtractionError, FromRequestContext,
    InputResponses, Json, Meta, RegisteredPrompts, RegisteredResourceTemplates,
    RegisteredResources, RegisteredTools, RequestContext, RequestState, State,
};
pub use prompts::{
    IntoPromptHandler, IntoPromptResult, IntoPromptsListHandler, IntoPromptsListResult,
    PromptError, PromptHandler, PromptRegistry, PromptsListHandler,
};
pub use resources::{
    IntoResourceHandler, IntoResourceResult, IntoResourceTemplatesListHandler,
    IntoResourceTemplatesListResult, IntoResourcesListHandler, IntoResourcesListResult,
    ResourceError, ResourceHandler, ResourceRegistry, ResourceTemplatesListHandler,
    ResourcesListHandler,
};
pub use router::McpRouter;
pub use server::{
    DiscoveryError, IntoServerDiscoveryHandler, IntoServerDiscoveryResult, ServerConfig,
    ServerDiscoveryHandler, validate_protocol_version,
};
pub use subscriptions::{
    IntoSubscriptionsListenHandler, IntoSubscriptionsListenResult, SubscriptionError,
    SubscriptionsListenHandler, SubscriptionsListenOutcome, SubscriptionsRegistry,
};
pub use tools::{
    IntoToolHandler, IntoToolResult, IntoToolsListHandler, IntoToolsListResult, Json as ToolJson,
    ToolError, ToolHandler, ToolRegistry, ToolsListHandler,
};
pub use utils::{format_sse_frame, format_sse_message};
