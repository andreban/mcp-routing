// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::sync::Arc;

use http::Response;

use crate::body::{ResponseBody, json_response, json_response_with_caching};
use crate::extract::RequestContext;
use crate::prompts::{
    IntoPromptHandler, IntoPromptsListHandler, PromptError, PromptHandler, PromptsListHandler,
};
use crate::router::{DispatchOutcome, MethodContext};
use crate::types::jsonrpc::{JsonRpcErrorResponse, JsonRpcRequestId};
use crate::types::mcp::{
    CacheScope,
    prompts::{
        Prompt,
        get::{GetPromptParams, GetPromptRequest, GetPromptResultResponse},
        list::{ListPromptsParams, ListPromptsRequest, ListPromptsResultResponse},
    },
};
use crate::utils::resolve_prompt_name;

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

    /// Dispatches an incoming `prompts/list` JSON-RPC request.
    pub(crate) async fn dispatch_list(
        &self,
        ctx: MethodContext<'_>,
        params_val: Option<serde_json::Value>,
    ) -> DispatchOutcome {
        if ctx.is_notification {
            return DispatchOutcome::notification();
        }

        let params: ListPromptsParams = match params_val {
            Some(pv) => match serde_json::from_value(pv) {
                Ok(p) => p,
                Err(err) => {
                    return DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                        ctx.req_id,
                        format!("Invalid params: {err}"),
                    ));
                }
            },
            None => ListPromptsParams {
                meta: None,
                cursor: None,
                extras: HashMap::new(),
            },
        };

        if let Some(ref handler) = self.list_handler {
            let mut extensions = (*ctx.extensions).clone();
            extensions.insert(crate::extract::RegisteredPrompts((*self.prompts).clone()));
            let request_ctx = RequestContext::new(
                params.meta.clone(),
                ctx.headers.clone(),
                Arc::new(extensions),
            );
            match handler
                .call(
                    request_ctx,
                    params.cursor,
                    self.list_ttl_ms,
                    self.list_cache_scope.clone(),
                )
                .await
            {
                Ok(res) => {
                    let ttl_ms = res.ttl_ms;
                    let cache_scope = res.cache_scope.clone();
                    let response = ListPromptsResultResponse::new(
                        ctx.req_id.unwrap_or_else(|| "".into()),
                        res,
                    );
                    match serde_json::to_value(response) {
                        Ok(v) => DispatchOutcome::response_with_cache(v, ttl_ms, cache_scope),
                        Err(err) => DispatchOutcome::error(JsonRpcErrorResponse::internal_error(
                            None,
                            format!("Failed to serialize response: {err}"),
                        )),
                    }
                }
                Err(PromptError::InvalidParams(err)) => {
                    DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                        ctx.req_id,
                        format!("Invalid params: {err}"),
                    ))
                }
                Err(PromptError::Internal(err)) => {
                    DispatchOutcome::error(JsonRpcErrorResponse::internal_error(
                        ctx.req_id,
                        format!("Failed to list prompts: {err}"),
                    ))
                }
            }
        } else {
            let req = ListPromptsRequest::new(
                ctx.req_id.clone().unwrap_or_else(|| "".into()),
                "prompts/list",
                Some(params),
            );

            let res = crate::prompts::list::handle_list_prompts(
                req,
                (*self.prompts).clone(),
                self.list_ttl_ms,
                self.list_cache_scope.clone(),
            );

            let ttl_ms = res.result.ttl_ms;
            let cache_scope = res.result.cache_scope.clone();
            match serde_json::to_value(res) {
                Ok(v) => DispatchOutcome::response_with_cache(v, ttl_ms, cache_scope),
                Err(err) => DispatchOutcome::error(JsonRpcErrorResponse::internal_error(
                    ctx.req_id,
                    format!("Failed to serialize response: {err}"),
                )),
            }
        }
    }

    /// Dispatches an incoming `prompts/get` JSON-RPC request to a registered typed prompt handler.
    pub(crate) async fn dispatch_get(
        &self,
        ctx: MethodContext<'_>,
        params_val: Option<serde_json::Value>,
    ) -> DispatchOutcome {
        let params: Option<GetPromptParams<serde_json::Value>> = match params_val {
            Some(pv) => match serde_json::from_value(pv) {
                Ok(p) => Some(p),
                Err(err) => {
                    return if ctx.is_notification {
                        DispatchOutcome::notification()
                    } else {
                        DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                            ctx.req_id,
                            format!("Invalid params: {err}"),
                        ))
                    };
                }
            },
            None => None,
        };

        let (meta, arguments, params_name) = match params {
            Some(p) => (p.meta, p.arguments, Some(p.name)),
            None => (None, None, None),
        };

        let prompt_name = match resolve_prompt_name(
            ctx.header_name.as_deref(),
            params_name.as_deref(),
            ctx.is_batch,
            "prompt name",
        ) {
            Ok(name) => name,
            Err(mut err) => {
                err.id = ctx.req_id;
                return if ctx.is_notification {
                    DispatchOutcome::notification()
                } else {
                    DispatchOutcome::error(err)
                };
            }
        };

        let Some(handler) = self.prompt_handlers.get(prompt_name) else {
            tracing::debug!(prompt_name, "Prompt not found");
            return if ctx.is_notification {
                DispatchOutcome::notification()
            } else {
                DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                    ctx.req_id,
                    format!("Invalid params: prompt '{prompt_name}' not found"),
                ))
            };
        };

        let (prompt_ttl, prompt_scope) = self
            .prompt_cache_settings
            .get(prompt_name)
            .cloned()
            .unwrap_or((None, None));
        let request_ctx =
            RequestContext::new(meta, ctx.headers.clone(), ctx.extensions);
        let result = handler.call(request_ctx, arguments).await;
        if ctx.is_notification {
            DispatchOutcome::notification()
        } else {
            match result {
                Ok(res) => {
                    let response = GetPromptResultResponse::new(
                        ctx.req_id.clone().unwrap_or_else(|| "".into()),
                        res,
                    );
                    match serde_json::to_value(response) {
                        Ok(v) => DispatchOutcome::response_with_cache(v, prompt_ttl, prompt_scope),
                        Err(err) => DispatchOutcome::error(JsonRpcErrorResponse::internal_error(
                            ctx.req_id,
                            format!("Failed to serialize response: {err}"),
                        )),
                    }
                }
                Err(PromptError::InvalidParams(err)) => {
                    DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                        ctx.req_id,
                        format!("Invalid params: {err}"),
                    ))
                }
                Err(PromptError::Internal(err)) => {
                    DispatchOutcome::error(JsonRpcErrorResponse::internal_error(
                        ctx.req_id,
                        format!("Prompt execution failed: {err}"),
                    ))
                }
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;

    #[tokio::test]
    async fn test_prompt_registry_dispatch_get_unknown_prompt_returns_invalid_params() {
        let registry = PromptRegistry::new();
        let headers = http::HeaderMap::new();
        let extensions = Arc::new(http::Extensions::new());
        let ctx = MethodContext {
            req_id: Some(JsonRpcRequestId::Number(42.0)),
            is_notification: false,
            is_batch: false,
            header_name: Some(std::borrow::Cow::Borrowed("non_existent_prompt")),
            headers: &headers,
            extensions,
        };

        let params = serde_json::json!({
            "name": "non_existent_prompt"
        });

        let outcome = registry.dispatch_get(ctx, Some(params)).await;
        let resp = outcome.response.expect("expected error response");
        assert_eq!(
            resp["error"]["code"],
            crate::types::jsonrpc::INVALID_PARAMS_CODE
        );
        assert!(
            resp["error"]["message"]
                .as_str()
                .unwrap()
                .contains("prompt 'non_existent_prompt' not found")
        );
    }

    #[tokio::test]
    async fn test_prompt_registry_handle_get_unknown_prompt_returns_invalid_params() {
        let registry = PromptRegistry::new();
        let headers = http::HeaderMap::new();
        let extensions = Arc::new(http::Extensions::new());
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "prompts/get",
            "params": {
                "name": "non_existent_prompt"
            }
        });
        let body_bytes = serde_json::to_vec(&body).unwrap();

        let response = registry
            .handle_get(
                Some(JsonRpcRequestId::Number(1.0)),
                Some("non_existent_prompt"),
                &headers,
                extensions,
                &body_bytes,
            )
            .await;

        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let err_resp: JsonRpcErrorResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            err_resp.error.code.code(),
            crate::types::jsonrpc::INVALID_PARAMS_CODE
        );
        assert!(
            err_resp
                .error
                .message
                .contains("prompt 'non_existent_prompt' not found")
        );
    }
}
