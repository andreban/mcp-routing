// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Request dispatching implementations for `prompts/list` and `prompts/get`.

use std::collections::HashMap;
use std::sync::Arc;

use crate::extract::RequestContext;
use crate::prompts::PromptError;
use crate::prompts::registry::PromptRegistry;
use crate::router::{DispatchOutcome, MethodContext};
use crate::types::jsonrpc::JsonRpcErrorResponse;
use crate::types::mcp::prompts::{
    get::{GetPromptParams, GetPromptResultResponse},
    list::{ListPromptsParams, ListPromptsRequest, ListPromptsResultResponse},
};
use crate::utils::resolve_prompt_name;

impl PromptRegistry {
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
}
