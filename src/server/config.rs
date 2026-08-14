// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use http::Response;

use crate::body::{ResponseBody, json_response, json_response_with_caching};
use crate::router::DispatchOutcome;
use crate::server::discover::handle_server_discover;
use crate::types::jsonrpc::{JsonRpcErrorResponse, JsonRpcRequestId};
use crate::types::mcp::{
    CacheScope, Implementation, ServerCapabilities, ToolsCapability,
    server::discover::{ServerDiscoverParams, ServerDiscoverRequest},
};

/// Configuration and metadata for an MCP server instance.
#[derive(Clone)]
pub struct ServerConfig {
    pub(crate) server_info: Implementation,
    pub(crate) instructions: Option<String>,
    pub(crate) capabilities: ServerCapabilities,
    pub(crate) supported_versions: Vec<String>,
    pub(crate) discover_ttl_ms: Option<u64>,
    pub(crate) discover_cache_scope: Option<CacheScope>,
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
            discover_ttl_ms: Some(0),
            discover_cache_scope: Some(CacheScope::Public),
        }
    }

    /// Dispatches an incoming `server/discover` JSON-RPC request to the discovery handler.
    pub(crate) fn dispatch_discover(
        &self,
        req_id: Option<JsonRpcRequestId>,
        is_notification: bool,
        params_val: Option<serde_json::Value>,
    ) -> DispatchOutcome {
        if is_notification {
            return DispatchOutcome::notification();
        }

        let params: ServerDiscoverParams = match params_val {
            Some(pv) => match serde_json::from_value(pv) {
                Ok(p) => p,
                Err(err) => {
                    return DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                        req_id,
                        format!("Invalid params: {err}"),
                    ));
                }
            },
            None => ServerDiscoverParams {
                meta: None,
                extras: HashMap::new(),
            },
        };

        let req = ServerDiscoverRequest::new(
            req_id.clone().unwrap_or_else(|| "".into()),
            "server/discover",
            Some(params),
        );

        let res = handle_server_discover(
            req,
            self.server_info.clone(),
            self.instructions.clone(),
            self.capabilities.clone(),
            self.supported_versions.clone(),
            self.discover_ttl_ms,
            self.discover_cache_scope.clone(),
        );

        let ttl_ms = res.result.ttl_ms;
        let cache_scope = res.result.cache_scope.clone();
        match serde_json::to_value(res) {
            Ok(v) => DispatchOutcome::response_with_cache(v, ttl_ms, cache_scope),
            Err(err) => DispatchOutcome::error(JsonRpcErrorResponse::internal_error(
                req_id,
                format!("Failed to serialize response: {err}"),
            )),
        }
    }

    /// Handles an incoming `server/discover` JSON-RPC request.
    pub fn handle_discover(
        &self,
        req_id: Option<JsonRpcRequestId>,
        body: &[u8],
    ) -> Response<ResponseBody> {
        let request: ServerDiscoverRequest = match serde_json::from_slice(body) {
            Ok(r) => r,
            Err(err) => {
                tracing::error!(?err, "Failed to parse ServerDiscoverRequest");
                let error_response = JsonRpcErrorResponse::invalid_params(
                    req_id,
                    format!("Invalid params: {err}"),
                );
                return json_response(&error_response);
            }
        };

        let response = handle_server_discover(
            request,
            self.server_info.clone(),
            self.instructions.clone(),
            self.capabilities.clone(),
            self.supported_versions.clone(),
            self.discover_ttl_ms,
            self.discover_cache_scope.clone(),
        );

        json_response_with_caching(
            &response,
            response.result.ttl_ms,
            response.result.cache_scope.as_ref(),
        )
    }
}
