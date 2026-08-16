// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Request dispatching implementations for `tools/list` and `tools/call`.

use std::collections::HashMap;
use std::sync::Arc;

use crate::extract::RequestContext;
use crate::router::{DispatchOutcome, MethodContext};
use crate::tools::registry::ToolRegistry;
use crate::tools::registry::validation::validate_tool_arguments;
use crate::tools::ToolError;
use crate::types::jsonrpc::JsonRpcErrorResponse;
use crate::types::mcp::tools::{
    call::{CallToolParams, CallToolResult, CallToolResultResponse},
    list::{ListToolsParams, ListToolsRequest, ListToolsResultResponse},
};
use crate::utils::resolve_tool_name;

impl ToolRegistry {
    /// Dispatches an incoming `tools/list` JSON-RPC request.
    pub(crate) async fn dispatch_list(
        &self,
        ctx: MethodContext<'_>,
        params_val: Option<serde_json::Value>,
    ) -> DispatchOutcome {
        if ctx.is_notification {
            return DispatchOutcome::notification();
        }

        let params: ListToolsParams = match params_val {
            Some(pv) => match serde_json::from_value(pv) {
                Ok(p) => p,
                Err(err) => {
                    return DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                        ctx.req_id,
                        format!("Invalid params: {err}"),
                    ));
                }
            },
            None => ListToolsParams {
                meta: None,
                cursor: None,
                extras: HashMap::new(),
            },
        };

        if let Some(ref handler) = self.list_handler {
            let mut extensions = (*ctx.extensions).clone();
            extensions.insert(crate::extract::RegisteredTools((*self.tools).clone()));
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
                    let response =
                        ListToolsResultResponse::new(ctx.req_id.unwrap_or_else(|| "".into()), res);
                    match serde_json::to_value(response) {
                        Ok(v) => DispatchOutcome::response_with_cache(v, ttl_ms, cache_scope),
                        Err(err) => DispatchOutcome::error(JsonRpcErrorResponse::internal_error(
                            None,
                            format!("Failed to serialize response: {err}"),
                        )),
                    }
                }
                Err(ToolError::InvalidParams(err)) => {
                    DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                        ctx.req_id,
                        format!("Invalid params: {err}"),
                    ))
                }
                Err(ToolError::Internal(err)) => {
                    DispatchOutcome::error(JsonRpcErrorResponse::internal_error(
                        ctx.req_id,
                        format!("Failed to list tools: {err}"),
                    ))
                }
            }
        } else {
            let req = ListToolsRequest::new(
                ctx.req_id.clone().unwrap_or_else(|| "".into()),
                "tools/list",
                Some(params),
            );

            let res = crate::tools::list::handle_list_tools(
                req,
                (*self.tools).clone(),
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

    /// Dispatches an incoming `tools/call` JSON-RPC request to a registered typed tool handler.
    pub(crate) async fn dispatch_call(
        &self,
        ctx: MethodContext<'_>,
        params_val: Option<serde_json::Value>,
    ) -> DispatchOutcome {
        let params: Option<CallToolParams<serde_json::Value>> = match params_val {
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

        let tool_name = match resolve_tool_name(
            ctx.header_name.as_deref(),
            params_name.as_deref(),
            ctx.is_batch,
            "tool name",
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

        let Some(handler) = self.tool_handlers.get(tool_name) else {
            tracing::debug!(tool_name, "Tool not found");
            return if ctx.is_notification {
                DispatchOutcome::notification()
            } else {
                DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                    ctx.req_id,
                    format!("Invalid params: tool '{tool_name}' not found"),
                ))
            };
        };

        let (tool_ttl, tool_scope) = self
            .tool_cache_settings
            .get(tool_name)
            .cloned()
            .unwrap_or((None, None));

        let empty_header_params = Vec::new();
        let header_params = self
            .tool_header_params
            .get(tool_name)
            .unwrap_or(&empty_header_params);

        if let Err(mut err) = crate::utils::validate_tool_header_params(
            ctx.req_id.clone(),
            header_params,
            arguments.as_ref(),
            ctx.headers,
            ctx.is_batch,
        ) {
            err.id = ctx.req_id;
            return if ctx.is_notification {
                DispatchOutcome::notification()
            } else {
                DispatchOutcome::error(err)
            };
        }

        if let Some(validator) = self.tool_validators.get(tool_name)
            && let Err(err_msg) = validate_tool_arguments(validator, arguments.as_ref())
        {
            if ctx.is_notification {
                return DispatchOutcome::notification();
            } else {
                let response = CallToolResultResponse::new(
                    ctx.req_id.clone().unwrap_or_else(|| "".into()),
                    CallToolResult::<serde_json::Value>::error(err_msg),
                );
                return match serde_json::to_value(response) {
                    Ok(v) => DispatchOutcome::response_with_cache(v, tool_ttl, tool_scope),
                    Err(err) => DispatchOutcome::error(JsonRpcErrorResponse::internal_error(
                        ctx.req_id,
                        format!("Failed to serialize response: {err}"),
                    )),
                };
            }
        }

        let request_ctx = RequestContext::new(
            meta,
            ctx.headers.clone(),
            Arc::clone(&ctx.extensions),
        );
        let result = handler.call(request_ctx, arguments).await;
        if ctx.is_notification {
            DispatchOutcome::notification()
        } else {
            let response = CallToolResultResponse::new(
                ctx.req_id.clone().unwrap_or_else(|| "".into()),
                result,
            );
            match serde_json::to_value(response) {
                Ok(v) => DispatchOutcome::response_with_cache(v, tool_ttl, tool_scope),
                Err(err) => DispatchOutcome::error(JsonRpcErrorResponse::internal_error(
                    ctx.req_id,
                    format!("Failed to serialize response: {err}"),
                )),
            }
        }
    }
}
