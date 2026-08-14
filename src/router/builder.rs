// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use crate::prompts::{IntoPromptHandler, IntoPromptsListHandler, PromptRegistry};
use crate::router::{McpRouter, McpRouterInner, StateInjector};
use crate::server::{IntoServerDiscoveryHandler, ServerConfig};
use crate::tools::{IntoToolHandler, IntoToolsListHandler, ToolRegistry};
use crate::types::mcp::{
    CacheScope, Implementation, PromptsCapability, ServerCapabilities, prompts::Prompt, tools::Tool,
};

impl McpRouter {
    /// Creates a new [`McpRouter`] initialized with the given server [`Implementation`] metadata.
    pub fn new(server_info: Implementation) -> Self {
        Self {
            inner: Arc::new(McpRouterInner {
                server: ServerConfig::new(server_info),
                tools: ToolRegistry::new(),
                prompts: PromptRegistry::new(),
                state_injectors: Vec::new(),
            }),
        }
    }

    /// Attaches application state to the router.
    ///
    /// The provided state value will be injected into request context for tool and prompt handlers,
    /// making it accessible via the [`State`](crate::extract::State) or [`Extension`](crate::extract::Extension)
    /// extractors.
    pub fn with_state<S: Clone + Send + Sync + 'static>(mut self, state: S) -> Self {
        let injector: StateInjector = Arc::new(move |exts: &mut http::Extensions| {
            if exts.get::<S>().is_none() {
                exts.insert(state.clone());
            }
        });
        Arc::make_mut(&mut self.inner)
            .state_injectors
            .push(injector);
        self
    }

    /// Sets human-readable instructions for the server advertised in `server/discover`.
    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        Arc::make_mut(&mut self.inner).server.instructions = Some(instructions.into());
        self
    }

    /// Sets server capabilities advertised in `server/discover`.
    pub fn capabilities(mut self, capabilities: ServerCapabilities) -> Self {
        Arc::make_mut(&mut self.inner).server.capabilities = capabilities;
        self
    }

    /// Sets supported protocol versions advertised in `server/discover`.
    pub fn supported_versions(mut self, versions: Vec<String>) -> Self {
        Arc::make_mut(&mut self.inner).server.supported_versions = versions;
        self
    }

    /// Configures whether client protocol version must be validated against `supported_versions`.
    ///
    /// Defaults to `true`. When enabled, discovery requests specifying an unsupported
    /// `_meta.protocolVersion` will be rejected with an `InvalidParams` error.
    pub fn validate_protocol_version(mut self, validate: bool) -> Self {
        Arc::make_mut(&mut self.inner)
            .server
            .validate_protocol_version = validate;
        self
    }

    /// Registers a handler for generating server discovery metadata (`server/discover`).
    ///
    /// The handler function can take request extractors (such as [`RequestContext`](crate::extract::RequestContext),
    /// [`SessionId`](crate::extract::SessionId), [`State`](crate::extract::State), [`Extension`](crate::extract::Extension),
    /// [`Authorization`](crate::extract::Authorization), or [`BearerAuth`](crate::extract::BearerAuth))
    /// and return any type implementing [`IntoServerDiscoveryResult`](crate::server::IntoServerDiscoveryResult).
    pub fn discover<H, T>(mut self, handler: H) -> Self
    where
        H: IntoServerDiscoveryHandler<T>,
        T: 'static,
    {
        Arc::make_mut(&mut self.inner)
            .server
            .set_discovery_provider(handler.into_discovery_handler());
        self
    }

    /// Alias for [`discover`](Self::discover) to register a server discovery handler.
    pub fn discovery<H, T>(self, handler: H) -> Self
    where
        H: IntoServerDiscoveryHandler<T>,
        T: 'static,
    {
        self.discover(handler)
    }

    /// Alias for [`discover`](Self::discover) to register a server discovery handler.
    pub fn dynamic_discovery<H, T>(self, handler: H) -> Self
    where
        H: IntoServerDiscoveryHandler<T>,
        T: 'static,
    {
        self.discover(handler)
    }

    /// Alias for [`discover`](Self::discover) to register a server discovery handler.
    pub fn server_discovery_provider<H, T>(self, handler: H) -> Self
    where
        H: IntoServerDiscoveryHandler<T>,
        T: 'static,
    {
        self.discover(handler)
    }

    /// Sets the time-to-live (`ttl_ms`) and cache scope for `server/discover` responses.
    pub fn server_discover_cache(
        mut self,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) -> Self {
        let server = &mut Arc::make_mut(&mut self.inner).server;
        server.discover_ttl_ms = ttl_ms;
        server.discover_cache_scope = cache_scope;
        self
    }

    /// Sets the time-to-live (`ttl_ms`) in milliseconds for `server/discover` responses.
    pub fn server_discover_ttl(mut self, ttl_ms: u64) -> Self {
        Arc::make_mut(&mut self.inner).server.discover_ttl_ms = Some(ttl_ms);
        self
    }

    /// Sets the cache scope for `server/discover` responses.
    pub fn server_discover_cache_scope(mut self, cache_scope: CacheScope) -> Self {
        Arc::make_mut(&mut self.inner).server.discover_cache_scope = Some(cache_scope);
        self
    }

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
    /// [`SessionId`](crate::extract::SessionId), [`State`](crate::extract::State), [`Extension`](crate::extract::Extension),
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

    /// Alias for [`tools_list`](Self::tools_list) to register a tools list handler function.
    pub fn list_tools<H, T>(self, handler: H) -> Self
    where
        H: IntoToolsListHandler<T>,
        T: 'static,
    {
        self.tools_list(handler)
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

    /// Sets the time-to-live (`ttl_ms`) and cache scope for `prompts/list` responses.
    pub fn prompts_list_cache(
        mut self,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) -> Self {
        Arc::make_mut(&mut self.inner)
            .prompts
            .set_list_cache(ttl_ms, cache_scope);
        self
    }

    /// Sets the time-to-live (`ttl_ms`) in milliseconds for `prompts/list` responses.
    pub fn prompts_list_ttl(mut self, ttl_ms: u64) -> Self {
        Arc::make_mut(&mut self.inner).prompts.list_ttl_ms = Some(ttl_ms);
        self
    }

    /// Sets the cache scope for `prompts/list` responses.
    pub fn prompts_list_cache_scope(mut self, cache_scope: CacheScope) -> Self {
        Arc::make_mut(&mut self.inner).prompts.list_cache_scope = Some(cache_scope);
        self
    }

    /// Registers a custom handler function for generating the prompts list (`prompts/list`).
    ///
    /// The handler function can take request extractors (such as [`RequestContext`](crate::extract::RequestContext),
    /// [`SessionId`](crate::extract::SessionId), [`State`](crate::extract::State), [`Extension`](crate::extract::Extension),
    /// [`Authorization`](crate::extract::Authorization), or [`BearerAuth`](crate::extract::BearerAuth))
    /// and optionally a `cursor: Option<String>` parameter, and return any type implementing [`IntoPromptsListResult`](crate::prompts::IntoPromptsListResult).
    pub fn prompts_list<H, T>(mut self, handler: H) -> Self
    where
        H: IntoPromptsListHandler<T>,
        T: 'static,
    {
        let inner = Arc::make_mut(&mut self.inner);
        if inner.server.capabilities.prompts.is_none() {
            inner.server.capabilities.prompts = Some(PromptsCapability { list_changed: None });
        }
        inner.prompts.set_list_handler(handler);
        self
    }

    /// Alias for [`prompts_list`](Self::prompts_list) to register a prompts list handler function.
    pub fn list_prompts<H, T>(self, handler: H) -> Self
    where
        H: IntoPromptsListHandler<T>,
        T: 'static,
    {
        self.prompts_list(handler)
    }

    /// Registers a prompt template alongside a typed asynchronous handler function.
    ///
    /// The handler function can take typed deserializable arguments (or no arguments)
    /// and return any type implementing [`IntoPromptResult`](crate::prompts::IntoPromptResult).
    pub fn register_prompt<TPrompt, H, T>(mut self, prompt: TPrompt, handler: H) -> Self
    where
        TPrompt: Into<Prompt>,
        H: IntoPromptHandler<T>,
        T: 'static,
    {
        let inner = Arc::make_mut(&mut self.inner);
        if inner.server.capabilities.prompts.is_none() {
            inner.server.capabilities.prompts = Some(PromptsCapability { list_changed: None });
        }
        inner.prompts.register(prompt, handler);
        self
    }

    /// Registers a prompt template alongside a typed asynchronous handler and prompt-specific caching directives.
    pub fn register_prompt_with_cache<TPrompt, H, T>(
        mut self,
        prompt: TPrompt,
        handler: H,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) -> Self
    where
        TPrompt: Into<Prompt>,
        H: IntoPromptHandler<T>,
        T: 'static,
    {
        let inner = Arc::make_mut(&mut self.inner);
        if inner.server.capabilities.prompts.is_none() {
            inner.server.capabilities.prompts = Some(PromptsCapability { list_changed: None });
        }
        inner
            .prompts
            .register_with_cache(prompt, handler, ttl_ms, cache_scope);
        self
    }

    /// Sets the cache configuration (`ttl_ms` and `cache_scope`) for a specific registered prompt by name.
    pub fn prompt_cache(
        mut self,
        prompt_name: impl Into<String>,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) -> Self {
        Arc::make_mut(&mut self.inner)
            .prompts
            .set_prompt_cache(prompt_name, ttl_ms, cache_scope);
        self
    }
}
