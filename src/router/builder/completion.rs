// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Builder methods for completion and subscription handlers.

use std::sync::Arc;

use crate::completion::IntoCompletionHandler;
use crate::router::McpRouter;
use crate::subscriptions::IntoSubscriptionsListenHandler;
use crate::types::mcp::{CacheScope, CompletionsCapability};

impl McpRouter {
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
        inner.completion.register_resource(uri_or_template, handler);
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

    /// Registers a custom handler function for `subscriptions/listen` requests.
    ///
    /// The handler function can take request extractors (such as [`RequestContext`](crate::extract::RequestContext),
    /// [`State`](crate::extract::State), [`Extension`](crate::extract::Extension),
    /// [`Authorization`](crate::extract::Authorization), or [`BearerAuth`](crate::extract::BearerAuth))
    /// and return any type implementing [`IntoSubscriptionsListenResult`](crate::subscriptions::IntoSubscriptionsListenResult).
    pub fn subscriptions_listen<H, T>(mut self, handler: H) -> Self
    where
        H: IntoSubscriptionsListenHandler<T>,
        T: 'static,
    {
        Arc::make_mut(&mut self.inner)
            .subscriptions
            .set_listen_handler(handler);
        self
    }
}
