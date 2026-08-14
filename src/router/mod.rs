// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # MCP Router
//!
//! A [Tower](tower)-native router for the Model Context Protocol (MCP).
//!
//! `McpRouter` implements [`tower::Service`] for HTTP requests and handles routing for:
//! - Built-in `server/discover` discovery endpoint
//! - Built-in `tools/list` tool discovery endpoint
//! - `tools/call` tool execution endpoints (delegating to typed handlers)
//! - Built-in `prompts/list` prompt discovery endpoint
//! - `prompts/get` prompt retrieval endpoints (delegating to typed handlers)
//! - Built-in `resources/list` direct resource discovery endpoint
//! - Built-in `resources/templates/list` resource template discovery endpoint
//! - `resources/read` resource content retrieval endpoints (delegating to typed handlers)
//! - `completion/complete` autocompletion endpoints (delegating to typed handlers)
//! - `logging/setLevel` logging level configuration endpoints (delegating to dynamic state and typed handlers)
//! - JSON-RPC 2.0 batch requests and notifications

mod builder;
mod dispatch;
mod outcome;
mod service;

use std::sync::Arc;

pub(crate) use outcome::{DispatchOutcome, MethodContext};

use crate::completion::CompletionRegistry;
use crate::logging::LoggingRegistry;
use crate::prompts::PromptRegistry;
use crate::resources::ResourceRegistry;
use crate::server::ServerConfig;
use crate::tools::ToolRegistry;

type StateInjector = Arc<dyn Fn(&mut http::Extensions) + Send + Sync>;

/// A [Tower](tower)-native router for the Model Context Protocol (MCP).
#[derive(Clone)]
pub struct McpRouter {
    inner: Arc<McpRouterInner>,
}

#[derive(Clone)]
pub(crate) struct McpRouterInner {
    pub(crate) server: ServerConfig,
    pub(crate) tools: ToolRegistry,
    pub(crate) prompts: PromptRegistry,
    pub(crate) resources: ResourceRegistry,
    pub(crate) completion: CompletionRegistry,
    pub(crate) logging: LoggingRegistry,
    pub(crate) state_injectors: Vec<StateInjector>,
}

