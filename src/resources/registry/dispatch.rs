// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Request dispatching implementations for `resources/list`, `resources/templates/list`, and `resources/read`.

use std::collections::HashMap;
use std::sync::Arc;

use crate::extract::RequestContext;
use crate::resources::ResourceError;
use crate::resources::registry::ResourceRegistry;
use crate::router::{DispatchOutcome, MethodContext};
use crate::types::jsonrpc::JsonRpcErrorResponse;
use crate::types::mcp::resources::{
    list::{ListResourcesParams, ListResourcesRequest, ListResourcesResultResponse},
    read::{ReadResourceParams, ReadResourceResultResponse},
    templates::{
        ListResourceTemplatesParams, ListResourceTemplatesRequest,
        ListResourceTemplatesResultResponse,
    },
};
use crate::utils::{extract_header_uri, resolve_resource_uri};

impl ResourceRegistry {
    /// Dispatches an incoming `resources/list` JSON-RPC request.
    pub(crate) async fn dispatch_list(
        &self,
        ctx: MethodContext<'_>,
        params_val: Option<serde_json::Value>,
    ) -> DispatchOutcome {
        if ctx.is_notification {
            return DispatchOutcome::notification();
        }

        let params: ListResourcesParams = match params_val {
            Some(pv) => match serde_json::from_value(pv) {
                Ok(p) => p,
                Err(err) => {
                    return DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                        ctx.req_id,
                        format!("Invalid params: {err}"),
                    ));
                }
            },
            None => ListResourcesParams {
                meta: None,
                cursor: None,
                extras: HashMap::new(),
            },
        };

        if let Some(ref handler) = self.list_handler {
            let mut extensions = (*ctx.extensions).clone();
            extensions.insert(crate::extract::RegisteredResources(
                (*self.resources).clone(),
            ));
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
                    let response = ListResourcesResultResponse::new(
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
                Err(ResourceError::InvalidParams(err)) => {
                    DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                        ctx.req_id,
                        format!("Invalid params: {err}"),
                    ))
                }
                Err(ResourceError::NotFound(err)) => {
                    DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                        ctx.req_id,
                        format!("Invalid params: {err}"),
                    ))
                }
                Err(ResourceError::Internal(err)) => {
                    DispatchOutcome::error(JsonRpcErrorResponse::internal_error(
                        ctx.req_id,
                        format!("Failed to list resources: {err}"),
                    ))
                }
            }
        } else {
            let req = ListResourcesRequest::new(
                ctx.req_id.clone().unwrap_or_else(|| "".into()),
                "resources/list",
                Some(params),
            );

            let res = crate::resources::list::handle_list_resources(
                req,
                (*self.resources).clone(),
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

    /// Dispatches an incoming `resources/templates/list` JSON-RPC request.
    pub(crate) async fn dispatch_templates_list(
        &self,
        ctx: MethodContext<'_>,
        params_val: Option<serde_json::Value>,
    ) -> DispatchOutcome {
        if ctx.is_notification {
            return DispatchOutcome::notification();
        }

        let params: ListResourceTemplatesParams = match params_val {
            Some(pv) => match serde_json::from_value(pv) {
                Ok(p) => p,
                Err(err) => {
                    return DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                        ctx.req_id,
                        format!("Invalid params: {err}"),
                    ));
                }
            },
            None => ListResourceTemplatesParams {
                meta: None,
                cursor: None,
                extras: HashMap::new(),
            },
        };

        if let Some(ref handler) = self.templates_list_handler {
            let mut extensions = (*ctx.extensions).clone();
            extensions.insert(crate::extract::RegisteredResourceTemplates(
                (*self.resource_templates).clone(),
            ));
            let request_ctx = RequestContext::new(
                params.meta.clone(),
                ctx.headers.clone(),
                Arc::new(extensions),
            );
            match handler
                .call(
                    request_ctx,
                    params.cursor,
                    self.templates_list_ttl_ms,
                    self.templates_list_cache_scope.clone(),
                )
                .await
            {
                Ok(res) => {
                    let ttl_ms = res.ttl_ms;
                    let cache_scope = res.cache_scope.clone();
                    let response = ListResourceTemplatesResultResponse::new(
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
                Err(ResourceError::InvalidParams(err)) => {
                    DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                        ctx.req_id,
                        format!("Invalid params: {err}"),
                    ))
                }
                Err(ResourceError::NotFound(err)) => {
                    DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                        ctx.req_id,
                        format!("Invalid params: {err}"),
                    ))
                }
                Err(ResourceError::Internal(err)) => {
                    DispatchOutcome::error(JsonRpcErrorResponse::internal_error(
                        ctx.req_id,
                        format!("Failed to list resource templates: {err}"),
                    ))
                }
            }
        } else {
            let req = ListResourceTemplatesRequest::new(
                ctx.req_id.clone().unwrap_or_else(|| "".into()),
                "resources/templates/list",
                Some(params),
            );

            let res = crate::resources::templates::handle_list_resource_templates(
                req,
                (*self.resource_templates).clone(),
                self.templates_list_ttl_ms,
                self.templates_list_cache_scope.clone(),
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

    /// Dispatches an incoming `resources/read` JSON-RPC request to a registered typed resource handler.
    pub(crate) async fn dispatch_read(
        &self,
        ctx: MethodContext<'_>,
        params_val: Option<serde_json::Value>,
    ) -> DispatchOutcome {
        let params: Option<ReadResourceParams> = match params_val {
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

        let (meta, params_uri) = match params {
            Some(p) => (p.meta, Some(p.uri)),
            None => (None, None),
        };

        let header_uri = extract_header_uri(ctx.headers);
        let resource_uri = match resolve_resource_uri(
            header_uri.as_deref(),
            params_uri.as_deref(),
            ctx.is_batch,
        ) {
            Ok(uri) => uri,
            Err(mut err) => {
                err.id = ctx.req_id;
                return if ctx.is_notification {
                    DispatchOutcome::notification()
                } else {
                    DispatchOutcome::error(err)
                };
            }
        };

        let Some((handler, res_ttl, res_scope)) = self.find_handler(resource_uri) else {
            tracing::debug!(resource_uri, "Resource not found");
            return if ctx.is_notification {
                DispatchOutcome::notification()
            } else {
                DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                    ctx.req_id,
                    format!("Invalid params: resource '{resource_uri}' not found"),
                ))
            };
        };

        let request_ctx =
            RequestContext::new(meta, ctx.headers.clone(), ctx.extensions);
        let result = handler
            .call(
                request_ctx,
                resource_uri.to_string(),
                res_ttl,
                res_scope.clone(),
            )
            .await;

        if ctx.is_notification {
            DispatchOutcome::notification()
        } else {
            match result {
                Ok(res) => {
                    let response = ReadResourceResultResponse::new(
                        ctx.req_id.clone().unwrap_or_else(|| "".into()),
                        res,
                    );
                    match serde_json::to_value(response) {
                        Ok(v) => DispatchOutcome::response_with_cache(v, res_ttl, res_scope),
                        Err(err) => DispatchOutcome::error(JsonRpcErrorResponse::internal_error(
                            ctx.req_id,
                            format!("Failed to serialize response: {err}"),
                        )),
                    }
                }
                Err(ResourceError::InvalidParams(err)) => {
                    DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                        ctx.req_id,
                        format!("Invalid params: {err}"),
                    ))
                }
                Err(ResourceError::NotFound(err)) => {
                    DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                        ctx.req_id,
                        format!("Invalid params: {err}"),
                    ))
                }
                Err(ResourceError::Internal(err)) => {
                    DispatchOutcome::error(JsonRpcErrorResponse::internal_error(
                        ctx.req_id,
                        format!("Resource read failed: {err}"),
                    ))
                }
            }
        }
    }
}
