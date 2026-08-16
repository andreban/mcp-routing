// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Builder methods for tool registration, listing, and caching.

use std::sync::Arc;

use crate::router::McpRouter;
use crate::tools::{IntoToolHandler, IntoToolsListHandler};
use crate::types::mcp::{CacheScope, tools::Tool};

impl McpRouter {
    /// Sets the time-to-live (`ttl_ms`) and cache scope for `tools/list` responses.
    pub fn tools_list_cache(
        mut self,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) -> Self {
        Arc::make_mut(&mut self.inner)
            .tools
            .set_list_cache(ttl_ms, cache_scope);
        self
    }

    /// Sets the time-to-live (`ttl_ms`) in milliseconds for `tools/list` responses.
    pub fn tools_list_ttl(mut self, ttl_ms: u64) -> Self {
        Arc::make_mut(&mut self.inner).tools.list_ttl_ms = Some(ttl_ms);
        self
    }

    /// Sets the cache scope for `tools/list` responses.
    pub fn tools_list_cache_scope(mut self, cache_scope: CacheScope) -> Self {
        Arc::make_mut(&mut self.inner).tools.list_cache_scope = Some(cache_scope);
        self
    }

    /// Registers a custom handler function for generating the tools list (`tools/list`).
    ///
    /// The handler function can take request extractors (such as [`RequestContext`](crate::extract::RequestContext),
    /// [`State`](crate::extract::State), [`Extension`](crate::extract::Extension),
    /// [`Authorization`](crate::extract::Authorization), or [`BearerAuth`](crate::extract::BearerAuth))
    /// and optionally a `cursor: Option<String>` parameter, and return any type implementing [`IntoToolsListResult`](crate::tools::IntoToolsListResult).
    pub fn tools_list<H, T>(mut self, handler: H) -> Self
    where
        H: IntoToolsListHandler<T>,
        T: 'static,
    {
        Arc::make_mut(&mut self.inner)
            .tools
            .set_list_handler(handler);
        self
    }

    /// Registers a tool definition alongside a typed asynchronous handler function.
    ///
    /// The handler function can take typed deserializable arguments (or no arguments)
    /// and return any type implementing [`IntoToolResult`](crate::tools::IntoToolResult).
    pub fn register_tool<TTool, H, T>(mut self, tool: TTool, handler: H) -> Self
    where
        TTool: Into<Tool>,
        H: IntoToolHandler<T>,
        T: 'static,
    {
        Arc::make_mut(&mut self.inner).tools.register(tool, handler);
        self
    }

    /// Registers a tool definition alongside a typed asynchronous handler and tool-specific caching directives.
    ///
    /// The specified `ttl_ms` and `cache_scope` will be propagated as HTTP `Cache-Control` headers
    /// when the tool is executed via `tools/call`.
    pub fn register_tool_with_cache<TTool, H, T>(
        mut self,
        tool: TTool,
        handler: H,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) -> Self
    where
        TTool: Into<Tool>,
        H: IntoToolHandler<T>,
        T: 'static,
    {
        Arc::make_mut(&mut self.inner).tools.register_with_cache(
            tool,
            handler,
            ttl_ms,
            cache_scope,
        );
        self
    }

    /// Sets the cache configuration (`ttl_ms` and `cache_scope`) for a specific registered tool by name.
    pub fn tool_cache(
        mut self,
        tool_name: impl Into<String>,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) -> Self {
        Arc::make_mut(&mut self.inner)
            .tools
            .set_tool_cache(tool_name, ttl_ms, cache_scope);
        self
    }
}
