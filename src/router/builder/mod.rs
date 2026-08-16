// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Builder methods for configuring the [`McpRouter`].

use std::sync::Arc;

use crate::completion::CompletionRegistry;
use crate::prompts::PromptRegistry;
use crate::resources::ResourceRegistry;
use crate::router::{McpRouter, McpRouterInner, StateInjector};
use crate::server::{IntoServerDiscoveryHandler, ServerConfig};
use crate::subscriptions::SubscriptionsRegistry;
use crate::tools::ToolRegistry;
use crate::types::mcp::{
    CacheScope, Implementation, LoggingCapability, LoggingLevel, ServerCapabilities,
};

pub mod completion;
pub mod prompts;
pub mod resources;
pub mod tools;

#[cfg(test)]
mod tests;

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
                subscriptions: SubscriptionsRegistry::new(),
                logging_level: LoggingLevel::Info,
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

    /// Configures allowed origins for DNS rebinding protection.
    ///
    /// When configured, incoming HTTP requests containing an `Origin` header that does not
    /// match any of the allowed origins will be rejected with `HTTP 403 Forbidden`.
    pub fn allowed_origins(
        mut self,
        origins: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Arc::make_mut(&mut self.inner)
            .server
            .set_allowed_origins(origins);
        self
    }

    /// Registers a handler for generating server discovery metadata (`server/discover`).
    ///
    /// The handler function can take request extractors (such as [`RequestContext`](crate::extract::RequestContext),
    /// [`State`](crate::extract::State), [`Extension`](crate::extract::Extension),
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

    /// Sets the initial logging level and advertises the logging capability in `server/discover`.
    pub fn logging_level(mut self, level: LoggingLevel) -> Self {
        let inner = Arc::make_mut(&mut self.inner);
        if inner.server.capabilities.logging.is_none() {
            inner.server.capabilities.logging = Some(LoggingCapability {});
        }
        inner.logging_level = level;
        self
    }

    /// Returns the server's configured default logging level.
    pub fn current_logging_level(&self) -> LoggingLevel {
        self.inner.logging_level
    }
}
