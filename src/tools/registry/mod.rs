// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Registry managing tool definitions, typed handlers, and tool cache configurations.

use std::collections::HashMap;
use std::sync::Arc;

use crate::tools::{
    IntoToolHandler, IntoToolsListHandler, ToolHandler, ToolsListHandler,
};
use crate::types::mcp::{CacheScope, tools::Tool};

pub mod dispatch;
pub mod validation;

#[cfg(test)]
mod tests;

/// Registry managing tool definitions, typed handlers, and tool cache configurations.
#[derive(Clone)]
pub struct ToolRegistry {
    pub(crate) tools: Arc<Vec<Tool>>,
    pub(crate) tool_handlers: HashMap<String, Arc<dyn ToolHandler>>,
    pub(crate) tool_cache_settings: HashMap<String, (Option<u64>, Option<CacheScope>)>,
    pub(crate) tool_validators: HashMap<String, Arc<jsonschema::Validator>>,
    pub(crate) tool_header_params: HashMap<String, Vec<String>>,
    pub(crate) list_ttl_ms: Option<u64>,
    pub(crate) list_cache_scope: Option<CacheScope>,
    pub(crate) list_handler: Option<Arc<dyn ToolsListHandler>>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    /// Creates a new empty [`ToolRegistry`].
    pub fn new() -> Self {
        Self {
            tools: Arc::new(Vec::new()),
            tool_handlers: HashMap::new(),
            tool_cache_settings: HashMap::new(),
            tool_validators: HashMap::new(),
            tool_header_params: HashMap::new(),
            list_ttl_ms: Some(0),
            list_cache_scope: Some(CacheScope::Public),
            list_handler: None,
        }
    }

    /// Sets a custom handler for `tools/list` requests.
    pub fn set_list_handler<H, T>(&mut self, handler: H)
    where
        H: IntoToolsListHandler<T>,
        T: 'static,
    {
        self.list_handler = Some(handler.into_tools_list_handler());
    }

    /// Registers a tool definition alongside a typed asynchronous handler.
    pub fn register<TTool, H, T>(&mut self, tool: TTool, handler: H)
    where
        TTool: Into<Tool>,
        H: IntoToolHandler<T>,
        T: 'static,
    {
        self.register_with_cache(tool, handler, None, None);
    }

    /// Registers a tool definition alongside a typed asynchronous handler and tool-specific caching directives.
    pub fn register_with_cache<TTool, H, T>(
        &mut self,
        tool: TTool,
        handler: H,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) where
        TTool: Into<Tool>,
        H: IntoToolHandler<T>,
        T: 'static,
    {
        let tool = tool.into();
        let name = tool.name.clone();
        match jsonschema::validator_for(&tool.input_schema) {
            Ok(validator) => {
                self.tool_validators
                    .insert(name.clone(), Arc::new(validator));
            }
            Err(err) => {
                tracing::warn!(
                    tool_name = %name,
                    %err,
                    "Failed to compile input schema validator for tool"
                );
            }
        }
        let header_params = crate::utils::extract_header_params_from_schema(&tool.input_schema);
        if !header_params.is_empty() {
            self.tool_header_params.insert(name.clone(), header_params);
        }
        self.tool_handlers
            .insert(name.clone(), handler.into_tool_handler());
        self.tool_cache_settings.insert(name, (ttl_ms, cache_scope));
        Arc::make_mut(&mut self.tools).push(tool);
    }

    /// Sets caching directives for a specific registered tool.
    pub fn set_tool_cache(
        &mut self,
        tool_name: impl Into<String>,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) {
        self.tool_cache_settings
            .insert(tool_name.into(), (ttl_ms, cache_scope));
    }

    /// Sets caching directives for `tools/list` responses.
    pub fn set_list_cache(&mut self, ttl_ms: Option<u64>, cache_scope: Option<CacheScope>) {
        self.list_ttl_ms = ttl_ms;
        self.list_cache_scope = cache_scope;
    }
}
