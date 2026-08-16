// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Registry managing prompt templates, typed handlers, and prompt cache configurations.

use std::collections::HashMap;
use std::sync::Arc;

use crate::prompts::{
    IntoPromptHandler, IntoPromptsListHandler, PromptHandler, PromptsListHandler,
};
use crate::types::mcp::{CacheScope, prompts::Prompt};

pub mod dispatch;

#[cfg(test)]
mod tests;

/// Registry managing prompt templates, typed handlers, and prompt cache configurations.
#[derive(Clone)]
pub struct PromptRegistry {
    pub(crate) prompts: Arc<Vec<Prompt>>,
    pub(crate) prompt_handlers: HashMap<String, Arc<dyn PromptHandler>>,
    pub(crate) prompt_cache_settings: HashMap<String, (Option<u64>, Option<CacheScope>)>,
    pub(crate) list_ttl_ms: Option<u64>,
    pub(crate) list_cache_scope: Option<CacheScope>,
    pub(crate) list_handler: Option<Arc<dyn PromptsListHandler>>,
}

impl Default for PromptRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PromptRegistry {
    /// Creates a new empty [`PromptRegistry`].
    pub fn new() -> Self {
        Self {
            prompts: Arc::new(Vec::new()),
            prompt_handlers: HashMap::new(),
            prompt_cache_settings: HashMap::new(),
            list_ttl_ms: Some(0),
            list_cache_scope: Some(CacheScope::Public),
            list_handler: None,
        }
    }

    /// Sets a custom handler for `prompts/list` requests.
    pub fn set_list_handler<H, T>(&mut self, handler: H)
    where
        H: IntoPromptsListHandler<T>,
        T: 'static,
    {
        self.list_handler = Some(handler.into_prompts_list_handler());
    }

    /// Registers a prompt template alongside a typed asynchronous handler.
    pub fn register<TPrompt, H, T>(&mut self, prompt: TPrompt, handler: H)
    where
        TPrompt: Into<Prompt>,
        H: IntoPromptHandler<T>,
        T: 'static,
    {
        self.register_with_cache(prompt, handler, None, None);
    }

    /// Registers a prompt template alongside a typed asynchronous handler and prompt-specific caching directives.
    pub fn register_with_cache<TPrompt, H, T>(
        &mut self,
        prompt: TPrompt,
        handler: H,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) where
        TPrompt: Into<Prompt>,
        H: IntoPromptHandler<T>,
        T: 'static,
    {
        let prompt = prompt.into();
        let name = prompt.name.clone();
        self.prompt_handlers
            .insert(name.clone(), handler.into_prompt_handler());
        self.prompt_cache_settings
            .insert(name, (ttl_ms, cache_scope));
        Arc::make_mut(&mut self.prompts).push(prompt);
    }

    /// Sets caching directives for a specific registered prompt.
    pub fn set_prompt_cache(
        &mut self,
        prompt_name: impl Into<String>,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) {
        self.prompt_cache_settings
            .insert(prompt_name.into(), (ttl_ms, cache_scope));
    }

    /// Sets caching directives for `prompts/list` responses.
    pub fn set_list_cache(&mut self, ttl_ms: Option<u64>, cache_scope: Option<CacheScope>) {
        self.list_ttl_ms = ttl_ms;
        self.list_cache_scope = cache_scope;
    }
}
