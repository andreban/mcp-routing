// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use crate::completion::{CompletionRegistry, IntoCompletionHandler};
use crate::prompts::{IntoPromptHandler, IntoPromptsListHandler, PromptRegistry};
use crate::resources::{
    IntoResourceHandler, IntoResourcesListHandler, IntoResourceTemplatesListHandler,
    ResourceRegistry,
};
use crate::router::{McpRouter, McpRouterInner, StateInjector};
use crate::server::{IntoServerDiscoveryHandler, ServerConfig};
use crate::tools::{IntoToolHandler, IntoToolsListHandler, ToolRegistry};
use crate::types::mcp::{
    CacheScope, CompletionsCapability, Implementation, PromptsCapability, ResourcesCapability,
    ServerCapabilities,
    prompts::Prompt,
    resources::{Resource, ResourceTemplate},
    tools::Tool,
};

impl McpRouter {
    /// Creates a new [`McpRouter`] initialized with the given server [`Implementation`] metadata.
    pub fn new(server_info: Implementation) -> Self {
        Self {
            inner: Arc::new(McpRouterInner {
                server: ServerConfig::new(server_info),
                tools: ToolRegistry::new(),
                prompts: PromptRegistry::new(),
                resources: ResourceRegistry::new(),
                completion: CompletionRegistry::new(),
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

    /// Sets the time-to-live (`ttl_ms`) and cache scope for `resources/list` responses.
    pub fn resources_list_cache(
        mut self,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) -> Self {
        Arc::make_mut(&mut self.inner)
            .resources
            .set_list_cache(ttl_ms, cache_scope);
        self
    }

    /// Sets the time-to-live (`ttl_ms`) in milliseconds for `resources/list` responses.
    pub fn resources_list_ttl(mut self, ttl_ms: u64) -> Self {
        Arc::make_mut(&mut self.inner).resources.list_ttl_ms = Some(ttl_ms);
        self
    }

    /// Sets the cache scope for `resources/list` responses.
    pub fn resources_list_cache_scope(mut self, cache_scope: CacheScope) -> Self {
        Arc::make_mut(&mut self.inner).resources.list_cache_scope = Some(cache_scope);
        self
    }

    /// Registers a custom handler function for generating the resources list (`resources/list`).
    ///
    /// The handler function can take request extractors (such as [`RequestContext`](crate::extract::RequestContext),
    /// [`SessionId`](crate::extract::SessionId), [`State`](crate::extract::State), [`Extension`](crate::extract::Extension),
    /// [`Authorization`](crate::extract::Authorization), or [`BearerAuth`](crate::extract::BearerAuth))
    /// and optionally a `cursor: Option<String>` parameter, and return any type implementing [`IntoResourcesListResult`](crate::resources::IntoResourcesListResult).
    pub fn resources_list<H, T>(mut self, handler: H) -> Self
    where
        H: IntoResourcesListHandler<T>,
        T: 'static,
    {
        let inner = Arc::make_mut(&mut self.inner);
        if inner.server.capabilities.resources.is_none() {
            inner.server.capabilities.resources = Some(ResourcesCapability {
                subscribe: None,
                list_changed: None,
            });
        }
        inner.resources.set_list_handler(handler);
        self
    }

    /// Sets the time-to-live (`ttl_ms`) and cache scope for `resources/templates/list` responses.
    pub fn resource_templates_list_cache(
        mut self,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) -> Self {
        Arc::make_mut(&mut self.inner)
            .resources
            .set_templates_list_cache(ttl_ms, cache_scope);
        self
    }

    /// Sets the time-to-live (`ttl_ms`) in milliseconds for `resources/templates/list` responses.
    pub fn resource_templates_list_ttl(mut self, ttl_ms: u64) -> Self {
        Arc::make_mut(&mut self.inner).resources.templates_list_ttl_ms = Some(ttl_ms);
        self
    }

    /// Sets the cache scope for `resources/templates/list` responses.
    pub fn resource_templates_list_cache_scope(mut self, cache_scope: CacheScope) -> Self {
        Arc::make_mut(&mut self.inner)
            .resources
            .templates_list_cache_scope = Some(cache_scope);
        self
    }

    /// Registers a custom handler function for generating the resource templates list (`resources/templates/list`).
    ///
    /// The handler function can take request extractors and optionally a `cursor: Option<String>` parameter,
    /// and return any type implementing [`IntoResourceTemplatesListResult`](crate::resources::IntoResourceTemplatesListResult).
    pub fn resource_templates_list<H, T>(mut self, handler: H) -> Self
    where
        H: IntoResourceTemplatesListHandler<T>,
        T: 'static,
    {
        let inner = Arc::make_mut(&mut self.inner);
        if inner.server.capabilities.resources.is_none() {
            inner.server.capabilities.resources = Some(ResourcesCapability {
                subscribe: None,
                list_changed: None,
            });
        }
        inner.resources.set_templates_list_handler(handler);
        self
    }

    /// Registers a direct resource definition alongside a typed asynchronous handler function.
    ///
    /// The handler function can take request extractors and optionally a `uri: String` parameter,
    /// and return any type implementing [`IntoResourceResult`](crate::resources::IntoResourceResult).
    pub fn register_resource<TResource, H, T>(mut self, resource: TResource, handler: H) -> Self
    where
        TResource: Into<Resource>,
        H: IntoResourceHandler<T>,
        T: 'static,
    {
        let inner = Arc::make_mut(&mut self.inner);
        if inner.server.capabilities.resources.is_none() {
            inner.server.capabilities.resources = Some(ResourcesCapability {
                subscribe: None,
                list_changed: None,
            });
        }
        inner.resources.register(resource, handler);
        self
    }

    /// Registers a direct resource definition alongside a typed asynchronous handler and caching directives.
    pub fn register_resource_with_cache<TResource, H, T>(
        mut self,
        resource: TResource,
        handler: H,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) -> Self
    where
        TResource: Into<Resource>,
        H: IntoResourceHandler<T>,
        T: 'static,
    {
        let inner = Arc::make_mut(&mut self.inner);
        if inner.server.capabilities.resources.is_none() {
            inner.server.capabilities.resources = Some(ResourcesCapability {
                subscribe: None,
                list_changed: None,
            });
        }
        inner
            .resources
            .register_with_cache(resource, handler, ttl_ms, cache_scope);
        self
    }

    /// Registers a resource template definition alongside a typed asynchronous handler function.
    ///
    /// The handler function can take request extractors and optionally a `uri: String` parameter,
    /// and return any type implementing [`IntoResourceResult`](crate::resources::IntoResourceResult).
    pub fn register_resource_template<TTemplate, H, T>(
        mut self,
        template: TTemplate,
        handler: H,
    ) -> Self
    where
        TTemplate: Into<ResourceTemplate>,
        H: IntoResourceHandler<T>,
        T: 'static,
    {
        let inner = Arc::make_mut(&mut self.inner);
        if inner.server.capabilities.resources.is_none() {
            inner.server.capabilities.resources = Some(ResourcesCapability {
                subscribe: None,
                list_changed: None,
            });
        }
        inner.resources.register_template(template, handler);
        self
    }

    /// Registers a resource template definition alongside a typed asynchronous handler and caching directives.
    pub fn register_resource_template_with_cache<TTemplate, H, T>(
        mut self,
        template: TTemplate,
        handler: H,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) -> Self
    where
        TTemplate: Into<ResourceTemplate>,
        H: IntoResourceHandler<T>,
        T: 'static,
    {
        let inner = Arc::make_mut(&mut self.inner);
        if inner.server.capabilities.resources.is_none() {
            inner.server.capabilities.resources = Some(ResourcesCapability {
                subscribe: None,
                list_changed: None,
            });
        }
        inner.resources.register_template_with_cache(
            template,
            handler,
            ttl_ms,
            cache_scope,
        );
        self
    }

    /// Sets the cache configuration (`ttl_ms` and `cache_scope`) for a specific registered resource or template by URI.
    pub fn resource_cache(
        mut self,
        uri: impl Into<String>,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) -> Self {
        Arc::make_mut(&mut self.inner)
            .resources
            .set_resource_cache(uri, ttl_ms, cache_scope);
        self
    }

    /// Sets the time-to-live (`ttl_ms`) and cache scope for `completion/complete` responses.
    pub fn completion_cache(
        mut self,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) -> Self {
        Arc::make_mut(&mut self.inner)
            .completion
            .set_cache(ttl_ms, cache_scope);
        self
    }

    /// Sets the time-to-live (`ttl_ms`) in milliseconds for `completion/complete` responses.
    pub fn completion_ttl(mut self, ttl_ms: u64) -> Self {
        Arc::make_mut(&mut self.inner).completion.cache_ttl_ms = Some(ttl_ms);
        self
    }

    /// Sets the cache scope for `completion/complete` responses.
    pub fn completion_cache_scope(mut self, cache_scope: CacheScope) -> Self {
        Arc::make_mut(&mut self.inner).completion.cache_scope = Some(cache_scope);
        self
    }

    /// Registers a default or fallback autocompletion handler function (`completion/complete`).
    ///
    /// The handler function can accept request extractors, [`CompleteParams`](crate::types::mcp::completion::CompleteParams),
    /// or [`CompleteArgument`](crate::types::mcp::completion::CompleteArgument), and return any type
    /// implementing [`IntoCompletionResult`](crate::completion::IntoCompletionResult).
    pub fn completion<H, T>(mut self, handler: H) -> Self
    where
        H: IntoCompletionHandler<T>,
        T: 'static,
    {
        let inner = Arc::make_mut(&mut self.inner);
        if inner.server.capabilities.completions.is_none() {
            inner.server.capabilities.completions = Some(CompletionsCapability {});
        }
        inner.completion.set_default_handler(handler);
        self
    }

    /// Registers an autocompletion handler function for all arguments of a prompt template.
    pub fn register_prompt_completion<H, T>(
        mut self,
        prompt_name: impl Into<String>,
        handler: H,
    ) -> Self
    where
        H: IntoCompletionHandler<T>,
        T: 'static,
    {
        let inner = Arc::make_mut(&mut self.inner);
        if inner.server.capabilities.completions.is_none() {
            inner.server.capabilities.completions = Some(CompletionsCapability {});
        }
        inner.completion.register_prompt(prompt_name, handler);
        self
    }

    /// Registers an autocompletion handler function for a specific argument of a prompt template.
    pub fn register_prompt_arg_completion<H, T>(
        mut self,
        prompt_name: impl Into<String>,
        arg_name: impl Into<String>,
        handler: H,
    ) -> Self
    where
        H: IntoCompletionHandler<T>,
        T: 'static,
    {
        let inner = Arc::make_mut(&mut self.inner);
        if inner.server.capabilities.completions.is_none() {
            inner.server.capabilities.completions = Some(CompletionsCapability {});
        }
        inner
            .completion
            .register_prompt_arg(prompt_name, arg_name, handler);
        self
    }

    /// Registers an autocompletion handler function for all variables of a resource URI or URI template.
    pub fn register_resource_completion<H, T>(
        mut self,
        uri_or_template: impl Into<String>,
        handler: H,
    ) -> Self
    where
        H: IntoCompletionHandler<T>,
        T: 'static,
    {
        let inner = Arc::make_mut(&mut self.inner);
        if inner.server.capabilities.completions.is_none() {
            inner.server.capabilities.completions = Some(CompletionsCapability {});
        }
        inner
            .completion
            .register_resource(uri_or_template, handler);
        self
    }

    /// Registers an autocompletion handler function for a specific variable of a resource URI or URI template.
    pub fn register_resource_arg_completion<H, T>(
        mut self,
        uri_or_template: impl Into<String>,
        arg_name: impl Into<String>,
        handler: H,
    ) -> Self
    where
        H: IntoCompletionHandler<T>,
        T: 'static,
    {
        let inner = Arc::make_mut(&mut self.inner);
        if inner.server.capabilities.completions.is_none() {
            inner.server.capabilities.completions = Some(CompletionsCapability {});
        }
        inner
            .completion
            .register_resource_arg(uri_or_template, arg_name, handler);
        self
    }
}

