// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::sync::Arc;

use crate::extract::RequestContext;
use crate::router::{DispatchOutcome, MethodContext};
use crate::server::handler::ServerDiscoveryHandler;
use crate::types::jsonrpc::JsonRpcErrorResponse;
use crate::types::mcp::{
    CacheScope, Implementation, ResultMetaObject, ServerCapabilities, ToolsCapability,
    server::discover::{ServerDiscoverParams, ServerDiscoverResult, ServerDiscoverResultResponse},
    unsupported_protocol_version_error,
};

/// Configuration and metadata for an MCP server instance.
#[derive(Clone)]
pub struct ServerConfig {
    pub(crate) server_info: Implementation,
    pub(crate) instructions: Option<String>,
    pub(crate) capabilities: ServerCapabilities,
    pub(crate) supported_versions: Vec<String>,
    pub(crate) validate_protocol_version: bool,
    pub(crate) allowed_origins: Option<Vec<String>>,
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
                logging: None,
                experimental: None,
                extensions: None,
            },
            supported_versions: vec!["2026-07-28".to_string()],
            validate_protocol_version: true,
            allowed_origins: None,
            discover_ttl_ms: Some(0),
            discover_cache_scope: Some(CacheScope::Public),
            discovery_provider: None,
        }
    }

    /// Sets whether client protocol version must be validated against `supported_versions`.
    pub fn set_validate_protocol_version(&mut self, validate: bool) {
        self.validate_protocol_version = validate;
    }

    /// Sets the list of allowed origins for DNS rebinding protection.
    ///
    /// When configured, incoming requests with an `Origin` header that does not match
    /// any allowed origin will be rejected with HTTP 403 Forbidden.
    pub fn set_allowed_origins(&mut self, origins: impl IntoIterator<Item = impl Into<String>>) {
        self.allowed_origins = Some(origins.into_iter().map(Into::into).collect());
    }

    /// Returns the configured allowed origins, if any.
    pub fn allowed_origins(&self) -> Option<&[String]> {
        self.allowed_origins.as_deref()
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

        if self.validate_protocol_version
            && let Some(client_ver) = params
                .meta
                .as_ref()
                .and_then(|m| m.protocol_version.as_deref())
            && !self.supported_versions.iter().any(|v| v == client_ver)
        {
            return DispatchOutcome::error(unsupported_protocol_version_error(
                ctx.req_id,
                format!(
                    "Unsupported protocol version '{client_ver}'. Supported versions: {}",
                    self.supported_versions.join(", ")
                ),
                self.supported_versions.clone(),
                client_ver,
            ));
        }

        let base_result = ServerDiscoverResult {
            meta: Some(ResultMetaObject::new(Some(self.server_info.clone()))),
            result_type: Some("complete".to_string()),
            supported_versions: self.supported_versions.clone(),
            capabilities: self.capabilities.clone(),
            instructions: self.instructions.clone(),
            ttl_ms: self.discover_ttl_ms,
            cache_scope: self.discover_cache_scope.clone(),
            extras: HashMap::new(),
        };

        let result = if let Some(ref provider) = self.discovery_provider {
            let request_ctx =
                RequestContext::new(params.meta.clone(), ctx.headers.clone(), ctx.extensions);
            match provider.call(request_ctx, base_result).await {
                Ok(res) => res,
                Err(err) => return DispatchOutcome::error(err.into_error_response(ctx.req_id)),
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
}
