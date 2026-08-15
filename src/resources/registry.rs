// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::sync::Arc;

use http::Response;

use crate::body::{ResponseBody, json_response, json_response_with_caching};
use crate::extract::{RequestContext, SessionId};
use crate::resources::{
    IntoResourceHandler, IntoResourceTemplatesListHandler, IntoResourcesListHandler, ResourceError,
    ResourceHandler, ResourceTemplatesListHandler, ResourcesListHandler,
};
use crate::router::{DispatchOutcome, MethodContext};
use crate::types::jsonrpc::{JsonRpcErrorResponse, JsonRpcRequestId};
use crate::types::mcp::{
    CacheScope,
    resources::{
        Resource, ResourceTemplate,
        list::{ListResourcesParams, ListResourcesRequest, ListResourcesResultResponse},
        read::{ReadResourceParams, ReadResourceRequest, ReadResourceResultResponse},
        templates::{
            ListResourceTemplatesParams, ListResourceTemplatesRequest,
            ListResourceTemplatesResultResponse,
        },
    },
};
use crate::utils::{extract_header_uri, match_uri_template, resolve_resource_uri};

type MatchedResourceHandler = (Arc<dyn ResourceHandler>, Option<u64>, Option<CacheScope>);

/// Registry managing direct resources, URI resource templates, typed handlers, and cache configurations.
#[derive(Clone)]
pub struct ResourceRegistry {
    pub(crate) resources: Arc<Vec<Resource>>,
    pub(crate) resource_templates: Arc<Vec<ResourceTemplate>>,
    pub(crate) resource_handlers: HashMap<String, Arc<dyn ResourceHandler>>,
    pub(crate) template_handlers: Vec<(ResourceTemplate, Arc<dyn ResourceHandler>)>,
    pub(crate) resource_cache_settings: HashMap<String, (Option<u64>, Option<CacheScope>)>,
    pub(crate) list_ttl_ms: Option<u64>,
    pub(crate) list_cache_scope: Option<CacheScope>,
    pub(crate) list_handler: Option<Arc<dyn ResourcesListHandler>>,
    pub(crate) templates_list_ttl_ms: Option<u64>,
    pub(crate) templates_list_cache_scope: Option<CacheScope>,
    pub(crate) templates_list_handler: Option<Arc<dyn ResourceTemplatesListHandler>>,
}

impl Default for ResourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceRegistry {
    /// Creates a new empty [`ResourceRegistry`].
    pub fn new() -> Self {
        Self {
            resources: Arc::new(Vec::new()),
            resource_templates: Arc::new(Vec::new()),
            resource_handlers: HashMap::new(),
            template_handlers: Vec::new(),
            resource_cache_settings: HashMap::new(),
            list_ttl_ms: Some(0),
            list_cache_scope: Some(CacheScope::Public),
            list_handler: None,
            templates_list_ttl_ms: Some(0),
            templates_list_cache_scope: Some(CacheScope::Public),
            templates_list_handler: None,
        }
    }

    /// Sets a custom handler for `resources/list` requests.
    pub fn set_list_handler<H, T>(&mut self, handler: H)
    where
        H: IntoResourcesListHandler<T>,
        T: 'static,
    {
        self.list_handler = Some(handler.into_resources_list_handler());
    }

    /// Sets a custom handler for `resources/templates/list` requests.
    pub fn set_templates_list_handler<H, T>(&mut self, handler: H)
    where
        H: IntoResourceTemplatesListHandler<T>,
        T: 'static,
    {
        self.templates_list_handler = Some(handler.into_resource_templates_list_handler());
    }

    /// Registers a direct resource definition alongside a typed asynchronous handler.
    pub fn register<TResource, H, T>(&mut self, resource: TResource, handler: H)
    where
        TResource: Into<Resource>,
        H: IntoResourceHandler<T>,
        T: 'static,
    {
        let resource = resource.into();
        let uri = resource.uri.clone();
        self.resource_handlers
            .insert(uri, handler.into_resource_handler());
        Arc::make_mut(&mut self.resources).push(resource);
    }

    /// Registers a direct resource definition alongside a typed asynchronous handler and caching directives.
    pub fn register_with_cache<TResource, H, T>(
        &mut self,
        resource: TResource,
        handler: H,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) where
        TResource: Into<Resource>,
        H: IntoResourceHandler<T>,
        T: 'static,
    {
        let resource = resource.into();
        let uri = resource.uri.clone();
        self.resource_handlers
            .insert(uri.clone(), handler.into_resource_handler());
        self.resource_cache_settings
            .insert(uri, (ttl_ms, cache_scope));
        Arc::make_mut(&mut self.resources).push(resource);
    }

    /// Registers a resource template definition alongside a typed asynchronous handler.
    pub fn register_template<TTemplate, H, T>(&mut self, template: TTemplate, handler: H)
    where
        TTemplate: Into<ResourceTemplate>,
        H: IntoResourceHandler<T>,
        T: 'static,
    {
        let template = template.into();
        self.template_handlers
            .push((template.clone(), handler.into_resource_handler()));
        Arc::make_mut(&mut self.resource_templates).push(template);
    }

    /// Registers a resource template definition alongside a typed asynchronous handler and caching directives.
    pub fn register_template_with_cache<TTemplate, H, T>(
        &mut self,
        template: TTemplate,
        handler: H,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) where
        TTemplate: Into<ResourceTemplate>,
        H: IntoResourceHandler<T>,
        T: 'static,
    {
        let template = template.into();
        let uri_template = template.uri_template.clone();
        self.template_handlers
            .push((template.clone(), handler.into_resource_handler()));
        self.resource_cache_settings
            .insert(uri_template, (ttl_ms, cache_scope));
        Arc::make_mut(&mut self.resource_templates).push(template);
    }

    /// Sets caching directives for a specific registered resource or template by URI.
    pub fn set_resource_cache(
        &mut self,
        uri: impl Into<String>,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) {
        self.resource_cache_settings
            .insert(uri.into(), (ttl_ms, cache_scope));
    }

    /// Sets caching directives for `resources/list` responses.
    pub fn set_list_cache(&mut self, ttl_ms: Option<u64>, cache_scope: Option<CacheScope>) {
        self.list_ttl_ms = ttl_ms;
        self.list_cache_scope = cache_scope;
    }

    /// Sets caching directives for `resources/templates/list` responses.
    pub fn set_templates_list_cache(
        &mut self,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) {
        self.templates_list_ttl_ms = ttl_ms;
        self.templates_list_cache_scope = cache_scope;
    }

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
                ctx.session_id,
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
                ctx.session_id,
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
        let resource_uri =
            match resolve_resource_uri(header_uri.as_deref(), params_uri.as_deref(), ctx.is_batch) {
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

        // 1. Check exact resource handler match
        let matched_handler: Option<MatchedResourceHandler> =
            if let Some(handler) = self.resource_handlers.get(resource_uri) {
                let (ttl, scope) = self
                    .resource_cache_settings
                    .get(resource_uri)
                    .cloned()
                    .unwrap_or((None, None));
                Some((handler.clone(), ttl, scope))
            } else {
                // 2. Check resource template match
                let mut found = None;
                for (template, handler) in &self.template_handlers {
                    if match_uri_template(&template.uri_template, resource_uri) {
                        let (ttl, scope) = self
                            .resource_cache_settings
                            .get(&template.uri_template)
                            .cloned()
                            .unwrap_or((None, None));
                        found = Some((handler.clone(), ttl, scope));
                        break;
                    }
                }
                found
            };

        let Some((handler, res_ttl, res_scope)) = matched_handler else {
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
            RequestContext::new(ctx.session_id, meta, ctx.headers.clone(), ctx.extensions);
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

    /// Handles an incoming `resources/list` JSON-RPC request.
    pub fn handle_list(
        &self,
        req_id: Option<JsonRpcRequestId>,
        body: &[u8],
    ) -> Response<ResponseBody> {
        let request: ListResourcesRequest = match serde_json::from_slice(body) {
            Ok(r) => r,
            Err(err) => {
                tracing::error!(?err, "Failed to parse ListResourcesRequest");
                let error_response =
                    JsonRpcErrorResponse::invalid_params(req_id, format!("Invalid params: {err}"));
                return json_response(&error_response);
            }
        };

        let response = crate::resources::list::handle_list_resources(
            request,
            (*self.resources).clone(),
            self.list_ttl_ms,
            self.list_cache_scope.clone(),
        );
        json_response_with_caching(
            &response,
            response.result.ttl_ms,
            response.result.cache_scope.as_ref(),
        )
    }

    /// Handles an incoming `resources/templates/list` JSON-RPC request.
    pub fn handle_templates_list(
        &self,
        req_id: Option<JsonRpcRequestId>,
        body: &[u8],
    ) -> Response<ResponseBody> {
        let request: ListResourceTemplatesRequest = match serde_json::from_slice(body) {
            Ok(r) => r,
            Err(err) => {
                tracing::error!(?err, "Failed to parse ListResourceTemplatesRequest");
                let error_response =
                    JsonRpcErrorResponse::invalid_params(req_id, format!("Invalid params: {err}"));
                return json_response(&error_response);
            }
        };

        let response = crate::resources::templates::handle_list_resource_templates(
            request,
            (*self.resource_templates).clone(),
            self.templates_list_ttl_ms,
            self.templates_list_cache_scope.clone(),
        );
        json_response_with_caching(
            &response,
            response.result.ttl_ms,
            response.result.cache_scope.as_ref(),
        )
    }

    /// Handles an incoming `resources/read` JSON-RPC request.
    pub async fn handle_read(
        &self,
        req_id: Option<JsonRpcRequestId>,
        header_uri: Option<&str>,
        session_id: Option<SessionId>,
        headers: &http::HeaderMap,
        extensions: Arc<http::Extensions>,
        body: &[u8],
    ) -> Response<ResponseBody> {
        let request: ReadResourceRequest = match serde_json::from_slice(body) {
            Ok(r) => r,
            Err(err) => {
                tracing::error!(?err, "Failed to parse ReadResourceRequest");
                let error_response =
                    JsonRpcErrorResponse::invalid_params(req_id, format!("Invalid params: {err}"));
                return json_response(&error_response);
            }
        };

        let decoded_header_uri = header_uri.map(crate::utils::decode_sentinel_header);
        let resource_uri = match resolve_resource_uri(
            decoded_header_uri.as_deref(),
            request.params.as_ref().map(|p| p.uri.as_str()),
            false,
        ) {
            Ok(uri) => uri,
            Err(mut err) => {
                err.id = req_id.or(Some(request.id));
                let status =
                    crate::types::mcp::mcp_error_code_to_http_status(err.error.code.code());
                return crate::body::json_response_with_status(status, &err);
            }
        };

        let matched_handler: Option<MatchedResourceHandler> =
            if let Some(handler) = self.resource_handlers.get(resource_uri) {
                let (ttl, scope) = self
                    .resource_cache_settings
                    .get(resource_uri)
                    .cloned()
                    .unwrap_or((None, None));
                Some((handler.clone(), ttl, scope))
            } else {
                let mut found = None;
                for (template, handler) in &self.template_handlers {
                    if match_uri_template(&template.uri_template, resource_uri) {
                        let (ttl, scope) = self
                            .resource_cache_settings
                            .get(&template.uri_template)
                            .cloned()
                            .unwrap_or((None, None));
                        found = Some((handler.clone(), ttl, scope));
                        break;
                    }
                }
                found
            };

        if let Some((handler, res_ttl, res_scope)) = matched_handler {
            let req_id = request.id.clone();
            let meta = request.params.as_ref().and_then(|p| p.meta.clone());
            let ctx = RequestContext::new(session_id, meta, headers.clone(), extensions);
            match handler
                .call(ctx, resource_uri.to_string(), res_ttl, res_scope.clone())
                .await
            {
                Ok(result) => {
                    let response = ReadResourceResultResponse::new(req_id, result);
                    return json_response_with_caching(&response, res_ttl, res_scope.as_ref());
                }
                Err(ResourceError::InvalidParams(err)) => {
                    let error_response = JsonRpcErrorResponse::invalid_params(
                        Some(req_id),
                        format!("Invalid params: {err}"),
                    );
                    return json_response(&error_response);
                }
                Err(ResourceError::NotFound(err)) => {
                    let error_response = JsonRpcErrorResponse::invalid_params(
                        Some(req_id),
                        format!("Invalid params: {err}"),
                    );
                    return json_response(&error_response);
                }
                Err(ResourceError::Internal(err)) => {
                    let error_response = JsonRpcErrorResponse::internal_error(
                        Some(req_id),
                        format!("Resource read failed: {err}"),
                    );
                    return json_response(&error_response);
                }
            }
        }

        tracing::debug!(resource_uri, "Resource not found");
        let error_response = JsonRpcErrorResponse::invalid_params(
            Some(request.id),
            format!("Invalid params: resource '{resource_uri}' not found"),
        );
        json_response(&error_response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;

    #[tokio::test]
    async fn test_resource_registry_dispatch_read_unknown_resource_returns_invalid_params() {
        let registry = ResourceRegistry::new();
        let mut headers = http::HeaderMap::new();
        headers.insert("mcp-uri", "file:///non_existent.txt".parse().unwrap());
        let extensions = Arc::new(http::Extensions::new());
        let ctx = MethodContext {
            req_id: Some(JsonRpcRequestId::Number(42.0)),
            is_notification: false,
            is_batch: false,
            header_name: None,
            session_id: None,
            headers: &headers,
            extensions,
        };

        let params = serde_json::json!({
            "uri": "file:///non_existent.txt"
        });

        let outcome = registry.dispatch_read(ctx, Some(params)).await;
        let resp = outcome.response.expect("expected error response");
        assert_eq!(
            resp["error"]["code"],
            crate::types::jsonrpc::INVALID_PARAMS_CODE
        );
        assert!(
            resp["error"]["message"]
                .as_str()
                .unwrap()
                .contains("resource 'file:///non_existent.txt' not found")
        );
    }

    #[tokio::test]
    async fn test_resource_registry_handle_read_unknown_resource_returns_invalid_params() {
        let registry = ResourceRegistry::new();
        let headers = http::HeaderMap::new();
        let extensions = Arc::new(http::Extensions::new());
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "resources/read",
            "params": {
                "uri": "file:///non_existent.txt"
            }
        });
        let body_bytes = serde_json::to_vec(&body).unwrap();

        let response = registry
            .handle_read(
                Some(JsonRpcRequestId::Number(1.0)),
                Some("file:///non_existent.txt"),
                None,
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
                .contains("resource 'file:///non_existent.txt' not found")
        );
    }
}

