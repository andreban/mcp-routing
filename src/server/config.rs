// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::sync::Arc;

use http::Response;

use crate::body::{ResponseBody, json_response, json_response_with_caching};
use crate::extract::{RequestContext, SessionId};
use crate::router::{DispatchOutcome, MethodContext};
use crate::server::discover::validate_protocol_version;
use crate::server::provider::{DiscoveryError, ServerDiscoveryHandler};
use crate::types::jsonrpc::{JsonRpcErrorResponse, JsonRpcRequestId};
use crate::types::mcp::{
    CacheScope, Implementation, ResultMetaObject, ServerCapabilities, ToolsCapability,
    server::discover::{
        ServerDiscoverParams, ServerDiscoverRequest, ServerDiscoverResult,
        ServerDiscoverResultResponse,
    },
};

/// Configuration and metadata for an MCP server instance.
#[derive(Clone)]
pub struct ServerConfig {
    pub(crate) server_info: Implementation,
    pub(crate) instructions: Option<String>,
    pub(crate) capabilities: ServerCapabilities,
    pub(crate) supported_versions: Vec<String>,
    pub(crate) validate_protocol_version: bool,
    pub(crate) discover_ttl_ms: Option<u64>,
    pub(crate) discover_cache_scope: Option<CacheScope>,
    pub(crate) discovery_provider: Option<Arc<dyn ServerDiscoveryHandler>>,
}

impl ServerConfig {
    /// Creates a new [`ServerConfig`] initialized with the given server [`Implementation`] metadata.
    pub fn new(server_info: Implementation) -> Self {
        Self {
            server_info,
            instructions: None,
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability { list_changed: None }),
                resources: None,
                prompts: None,
                completions: None,
                experimental: None,
            },
            supported_versions: vec!["2026-07-28".to_string()],
            validate_protocol_version: true,
            discover_ttl_ms: Some(0),
            discover_cache_scope: Some(CacheScope::Public),
            discovery_provider: None,
        }
    }

    /// Sets whether client protocol version must be validated against `supported_versions`.
    pub fn set_validate_protocol_version(&mut self, validate: bool) {
        self.validate_protocol_version = validate;
    }

    /// Sets a dynamic async server discovery provider.
    pub fn set_discovery_provider(&mut self, provider: Arc<dyn ServerDiscoveryHandler>) {
        self.discovery_provider = Some(provider);
    }

    /// Dispatches an incoming `server/discover` JSON-RPC request to the discovery handler.
    pub(crate) async fn dispatch_discover(
        &self,
        ctx: MethodContext<'_>,
        params_val: Option<serde_json::Value>,
    ) -> DispatchOutcome {
        if ctx.is_notification {
            return DispatchOutcome::notification();
        }

        let params: ServerDiscoverParams = match params_val {
            Some(pv) => match serde_json::from_value(pv) {
                Ok(p) => p,
                Err(err) => {
                    return DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                        ctx.req_id,
                        format!("Invalid params: {err}"),
                    ));
                }
            },
            None => ServerDiscoverParams {
                meta: None,
                extras: HashMap::new(),
            },
        };

        if self.validate_protocol_version {
            let client_version = params
                .meta
                .as_ref()
                .and_then(|m| m.protocol_version.as_deref());
            if let Err(err_msg) =
                validate_protocol_version(client_version, &self.supported_versions)
            {
                return DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                    ctx.req_id, err_msg,
                ));
            }
        }

        let base_result = ServerDiscoverResult {
            meta: Some(ResultMetaObject {
                server_info: Some(self.server_info.clone()),
                extra: HashMap::new(),
            }),
            result_type: Some("complete".to_string()),
            supported_versions: self.supported_versions.clone(),
            capabilities: self.capabilities.clone(),
            instructions: self.instructions.clone(),
            ttl_ms: self.discover_ttl_ms,
            cache_scope: self.discover_cache_scope.clone(),
            extras: HashMap::new(),
        };

        let result = if let Some(ref provider) = self.discovery_provider {
            let request_ctx = RequestContext::new(
                ctx.session_id,
                params.meta.clone(),
                ctx.headers.clone(),
                ctx.extensions,
            );
            match provider.call(request_ctx, base_result).await {
                Ok(res) => res,
                Err(DiscoveryError::InvalidParams(err)) => {
                    return DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                        ctx.req_id,
                        format!("Invalid params: {err}"),
                    ));
                }
                Err(DiscoveryError::Internal(err)) => {
                    return DispatchOutcome::error(JsonRpcErrorResponse::internal_error(
                        ctx.req_id,
                        format!("Discovery failed: {err}"),
                    ));
                }
            }
        } else {
            base_result
        };

        let ttl_ms = result.ttl_ms;
        let cache_scope = result.cache_scope.clone();
        let response = ServerDiscoverResultResponse::new(
            ctx.req_id.clone().unwrap_or_else(|| "".into()),
            result,
        );

        match serde_json::to_value(response) {
            Ok(v) => DispatchOutcome::response_with_cache(v, ttl_ms, cache_scope),
            Err(err) => DispatchOutcome::error(JsonRpcErrorResponse::internal_error(
                ctx.req_id,
                format!("Failed to serialize response: {err}"),
            )),
        }
    }

    /// Handles an incoming `server/discover` JSON-RPC request.
    pub async fn handle_discover(
        &self,
        req_id: Option<JsonRpcRequestId>,
        session_id: Option<SessionId>,
        headers: &http::HeaderMap,
        extensions: Arc<http::Extensions>,
        body: &[u8],
    ) -> Response<ResponseBody> {
        let request: ServerDiscoverRequest = match serde_json::from_slice(body) {
            Ok(r) => r,
            Err(err) => {
                tracing::error!(?err, "Failed to parse ServerDiscoverRequest");
                let error_response =
                    JsonRpcErrorResponse::invalid_params(req_id, format!("Invalid params: {err}"));
                return json_response(&error_response);
            }
        };

        if self.validate_protocol_version {
            let client_version = request
                .params
                .as_ref()
                .and_then(|p| p.meta.as_ref())
                .and_then(|m| m.protocol_version.as_deref());
            if let Err(err_msg) =
                validate_protocol_version(client_version, &self.supported_versions)
            {
                let error_response =
                    JsonRpcErrorResponse::invalid_params(Some(request.id), err_msg);
                return json_response(&error_response);
            }
        }

        let base_result = ServerDiscoverResult {
            meta: Some(ResultMetaObject {
                server_info: Some(self.server_info.clone()),
                extra: HashMap::new(),
            }),
            result_type: Some("complete".to_string()),
            supported_versions: self.supported_versions.clone(),
            capabilities: self.capabilities.clone(),
            instructions: self.instructions.clone(),
            ttl_ms: self.discover_ttl_ms,
            cache_scope: self.discover_cache_scope.clone(),
            extras: HashMap::new(),
        };

        let result = if let Some(ref provider) = self.discovery_provider {
            let meta = request.params.as_ref().and_then(|p| p.meta.clone());
            let request_ctx = RequestContext::new(session_id, meta, headers.clone(), extensions);
            match provider.call(request_ctx, base_result).await {
                Ok(res) => res,
                Err(DiscoveryError::InvalidParams(err)) => {
                    let error_response = JsonRpcErrorResponse::invalid_params(
                        Some(request.id),
                        format!("Invalid params: {err}"),
                    );
                    return json_response(&error_response);
                }
                Err(DiscoveryError::Internal(err)) => {
                    let error_response = JsonRpcErrorResponse::internal_error(
                        Some(request.id),
                        format!("Discovery failed: {err}"),
                    );
                    return json_response(&error_response);
                }
            }
        } else {
            base_result
        };

        let response = ServerDiscoverResultResponse::new(request.id, result);
        json_response_with_caching(
            &response,
            response.result.ttl_ms,
            response.result.cache_scope.as_ref(),
        )
    }
}
