// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::sync::Arc;

use http::Response;

use crate::body::{ResponseBody, json_response, json_response_with_caching};
use crate::extract::{RequestContext, SessionId};
use crate::tools::{IntoToolHandler, ToolHandler};
use crate::types::jsonrpc::{JsonRpcErrorResponse, JsonRpcRequestId};
use crate::types::mcp::{
    CacheScope,
    tools::{
        Tool,
        call::{CallToolRequest, CallToolResultResponse},
        list::ListToolsRequest,
    },
};
use crate::utils::resolve_tool_name;

/// Registry managing tool definitions, typed handlers, and tool cache configurations.
#[derive(Clone)]
pub struct ToolRegistry {
    pub(crate) tools: Vec<Tool>,
    pub(crate) tool_handlers: HashMap<String, Arc<dyn ToolHandler>>,
    pub(crate) tool_cache_settings: HashMap<String, (Option<u64>, Option<CacheScope>)>,
    pub(crate) list_ttl_ms: Option<u64>,
    pub(crate) list_cache_scope: Option<CacheScope>,
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
            tools: Vec::new(),
            tool_handlers: HashMap::new(),
            tool_cache_settings: HashMap::new(),
            list_ttl_ms: Some(0),
            list_cache_scope: Some(CacheScope::Public),
        }
    }

    /// Registers a tool definition alongside a typed asynchronous handler.
    pub fn register<TTool, H, T>(&mut self, tool: TTool, handler: H)
    where
        TTool: Into<Tool>,
        H: IntoToolHandler<T>,
        T: 'static,
    {
        let tool = tool.into();
        let name = tool.name.clone();
        self.tool_handlers.insert(name, handler.into_tool_handler());
        self.tools.push(tool);
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
        self.tool_handlers.insert(name.clone(), handler.into_tool_handler());
        self.tool_cache_settings.insert(name, (ttl_ms, cache_scope));
        self.tools.push(tool);
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

    /// Handles an incoming `tools/list` JSON-RPC request.
    pub fn handle_list(
        &self,
        req_id: Option<JsonRpcRequestId>,
        body: &[u8],
    ) -> Response<ResponseBody> {
        let request: ListToolsRequest = match serde_json::from_slice(body) {
            Ok(r) => r,
            Err(err) => {
                tracing::error!(?err, "Failed to parse ListToolsRequest");
                let error_response = JsonRpcErrorResponse::invalid_params(
                    req_id,
                    format!("Invalid params: {err}"),
                );
                return json_response(&error_response);
            }
        };

        let response = crate::tools::list::handle_list_tools(
            request,
            self.tools.clone(),
            self.list_ttl_ms,
            self.list_cache_scope.clone(),
        );
        json_response_with_caching(
            &response,
            response.result.ttl_ms,
            response.result.cache_scope.as_ref(),
        )
    }

    /// Handles an incoming `tools/call` JSON-RPC request.
    pub async fn handle_call(
        &self,
        req_id: Option<JsonRpcRequestId>,
        header_name: Option<&str>,
        session_id: Option<SessionId>,
        headers: &http::HeaderMap,
        extensions: Arc<http::Extensions>,
        body: &[u8],
    ) -> Response<ResponseBody> {
        let request: CallToolRequest<serde_json::Value> = match serde_json::from_slice(body) {
            Ok(r) => r,
            Err(err) => {
                tracing::error!(?err, "Failed to parse CallToolRequest");
                let error_response = JsonRpcErrorResponse::invalid_params(
                    req_id,
                    format!("Invalid params: {err}"),
                );
                return json_response(&error_response);
            }
        };

        let tool_name = resolve_tool_name(
            header_name,
            request.params.as_ref().map(|p| p.name.as_str()),
        );

        let Some(tool_name) = tool_name else {
            tracing::debug!("Missing tool name for tools/call");
            let error_response = JsonRpcErrorResponse::invalid_params(
                Some(request.id),
                "Invalid params: missing tool name",
            );
            return json_response(&error_response);
        };

        if tool_name.is_empty() {
            tracing::debug!("Empty tool name for tools/call");
            let error_response = JsonRpcErrorResponse::invalid_params(
                Some(request.id),
                "Invalid params: empty tool name",
            );
            return json_response(&error_response);
        }

        if let Some(handler) = self.tool_handlers.get(tool_name) {
            let (tool_ttl, tool_scope) = self
                .tool_cache_settings
                .get(tool_name)
                .cloned()
                .unwrap_or((None, None));
            let req_id = request.id.clone();
            let meta = request.params.as_ref().and_then(|p| p.meta.clone());
            let ctx = RequestContext::new(session_id, meta, headers.clone(), extensions);
            let raw_args = request.params.and_then(|p| p.arguments);
            let result = handler.call(ctx, raw_args).await;
            let response = CallToolResultResponse::new(req_id, result);
            return json_response_with_caching(&response, tool_ttl, tool_scope.as_ref());
        }

        tracing::debug!(tool_name, "Tool not found");
        let error_response = JsonRpcErrorResponse::method_not_found(
            Some(request.id),
            format!("Method not found: tool '{tool_name}' not found"),
        );
        json_response(&error_response)
    }
}
