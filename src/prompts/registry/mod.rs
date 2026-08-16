// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Registry managing prompt templates, typed handlers, and prompt cache configurations.

use std::collections::HashMap;
use std::sync::Arc;

use http::Response;

use crate::body::{ResponseBody, json_response, json_response_with_caching};
use crate::extract::RequestContext;
use crate::prompts::{
    IntoPromptHandler, IntoPromptsListHandler, PromptError, PromptHandler, PromptsListHandler,
};
use crate::types::jsonrpc::{JsonRpcErrorResponse, JsonRpcRequestId};
use crate::types::mcp::{
    CacheScope,
    prompts::{
        Prompt,
        get::{GetPromptRequest, GetPromptResultResponse},
        list::ListPromptsRequest,
    },
};
use crate::utils::resolve_prompt_name;

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
        let prompt = prompt.into();
        let name = prompt.name.clone();
        self.prompt_handlers
            .insert(name, handler.into_prompt_handler());
        Arc::make_mut(&mut self.prompts).push(prompt);
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

    /// Handles an incoming `prompts/list` JSON-RPC request.
    pub fn handle_list(
        &self,
        req_id: Option<JsonRpcRequestId>,
        body: &[u8],
    ) -> Response<ResponseBody> {
        let request: ListPromptsRequest = match serde_json::from_slice(body) {
            Ok(r) => r,
            Err(err) => {
                tracing::error!(?err, "Failed to parse ListPromptsRequest");
                let error_response =
                    JsonRpcErrorResponse::invalid_params(req_id, format!("Invalid params: {err}"));
                return json_response(&error_response);
            }
        };

        let response = crate::prompts::list::handle_list_prompts(
            request,
            (*self.prompts).clone(),
            self.list_ttl_ms,
            self.list_cache_scope.clone(),
        );
        json_response_with_caching(
            &response,
            response.result.ttl_ms,
            response.result.cache_scope.as_ref(),
        )
    }

    /// Handles an incoming `prompts/get` JSON-RPC request.
    pub async fn handle_get(
        &self,
        req_id: Option<JsonRpcRequestId>,
        header_name: Option<&str>,
        headers: &http::HeaderMap,
        extensions: Arc<http::Extensions>,
        body: &[u8],
    ) -> Response<ResponseBody> {
        let request: GetPromptRequest<serde_json::Value> = match serde_json::from_slice(body) {
            Ok(r) => r,
            Err(err) => {
                tracing::error!(?err, "Failed to parse GetPromptRequest");
                let error_response =
                    JsonRpcErrorResponse::invalid_params(req_id, format!("Invalid params: {err}"));
                return json_response(&error_response);
            }
        };

        let decoded_header_name = header_name.map(crate::utils::decode_sentinel_header);
        let prompt_name = match resolve_prompt_name(
            decoded_header_name.as_deref(),
            request.params.as_ref().map(|p| p.name.as_str()),
            false,
            "prompt name",
        ) {
            Ok(name) => name,
            Err(mut err) => {
                err.id = req_id.or(Some(request.id));
                let status =
                    crate::types::mcp::mcp_error_code_to_http_status(err.error.code.code());
                return crate::body::json_response_with_status(status, &err);
            }
        };

        if let Some(handler) = self.prompt_handlers.get(prompt_name) {
            let (prompt_ttl, prompt_scope) = self
                .prompt_cache_settings
                .get(prompt_name)
                .cloned()
                .unwrap_or((None, None));
            let req_id = request.id.clone();
            let meta = request.params.as_ref().and_then(|p| p.meta.clone());
            let ctx = RequestContext::new(meta, headers.clone(), extensions);
            let raw_args = request.params.and_then(|p| p.arguments);
            match handler.call(ctx, raw_args).await {
                Ok(result) => {
                    let response = GetPromptResultResponse::new(req_id, result);
                    return json_response_with_caching(
                        &response,
                        prompt_ttl,
                        prompt_scope.as_ref(),
                    );
                }
                Err(PromptError::InvalidParams(err)) => {
                    let error_response = JsonRpcErrorResponse::invalid_params(
                        Some(req_id),
                        format!("Invalid params: {err}"),
                    );
                    return json_response(&error_response);
                }
                Err(PromptError::Internal(err)) => {
                    let error_response = JsonRpcErrorResponse::internal_error(
                        Some(req_id),
                        format!("Prompt execution failed: {err}"),
                    );
                    return json_response(&error_response);
                }
            }
        }

        tracing::debug!(prompt_name, "Prompt not found");
        let error_response = JsonRpcErrorResponse::invalid_params(
            Some(request.id),
            format!("Invalid params: prompt '{prompt_name}' not found"),
        );
        json_response(&error_response)
    }
}
