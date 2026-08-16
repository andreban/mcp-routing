// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Registry managing direct resources, URI resource templates, typed handlers, and cache configurations.

use std::collections::HashMap;
use std::sync::Arc;

use http::Response;

use crate::body::{ResponseBody, json_response, json_response_with_caching};
use crate::extract::RequestContext;
use crate::resources::{
    IntoResourceHandler, IntoResourceTemplatesListHandler, IntoResourcesListHandler, ResourceError,
    ResourceHandler, ResourceTemplatesListHandler, ResourcesListHandler,
};
use crate::types::jsonrpc::{JsonRpcErrorResponse, JsonRpcRequestId};
use crate::types::mcp::{
    CacheScope,
    resources::{
        Resource, ResourceTemplate,
        list::ListResourcesRequest,
        read::{ReadResourceRequest, ReadResourceResultResponse},
        templates::ListResourceTemplatesRequest,
    },
};
use crate::utils::resolve_resource_uri;

pub mod dispatch;
pub mod template;

#[cfg(test)]
mod tests;

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

        let matched_handler = self.find_handler(resource_uri);

        if let Some((handler, res_ttl, res_scope)) = matched_handler {
            let req_id = request.id.clone();
            let meta = request.params.as_ref().and_then(|p| p.meta.clone());
            let ctx = RequestContext::new(meta, headers.clone(), extensions);
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
