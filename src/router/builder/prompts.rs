// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Builder methods for prompt registration, listing, and caching.

use std::sync::Arc;

use crate::prompts::{IntoPromptHandler, IntoPromptsListHandler};
use crate::router::McpRouter;
use crate::types::mcp::{CacheScope, PromptsCapability, prompts::Prompt};

impl McpRouter {
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
    /// [`State`](crate::extract::State), [`Extension`](crate::extract::Extension),
    /// [`Authorization`](crate::extract::Authorization), or [`BearerAuth`](crate::extract::BearerAuth))
    /// and optionally a `cursor: Option<String>` parameter, and return any type implementing [`IntoPromptsListResult`](crate::prompts::IntoPromptsListResult).
    pub fn prompts_list<H, T>(mut self, handler: H) -> Self
    where
        H: IntoPromptsListHandler<T>,
        T: 'static,
    {
        let inner = Arc::make_mut(&mut self.inner);
        if inner.server.capabilities.prompts.is_none() {
            inner.server.capabilities.prompts = Some(PromptsCapability {
                list_changed: None,
            });
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
            inner.server.capabilities.prompts = Some(PromptsCapability {
                list_changed: None,
            });
        }
        inner.prompts.register(prompt, handler);
        self
    }

    /// Registers a prompt template alongside a typed asynchronous handler and prompt-specific caching directives.
    ///
    /// The specified `ttl_ms` and `cache_scope` will be propagated as HTTP `Cache-Control` headers
    /// when the prompt is executed via `prompts/get`.
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
            inner.server.capabilities.prompts = Some(PromptsCapability {
                list_changed: None,
            });
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
