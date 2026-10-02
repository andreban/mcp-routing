// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Cow;
use std::sync::Arc;

use http::StatusCode;

use crate::body::ResponseBody;
use crate::types::jsonrpc::{JsonRpcErrorResponse, JsonRpcRequestId};
use crate::types::mcp::{
    CacheScope, ClientCapabilities, Implementation, mcp_error_code_to_http_status,
    missing_required_client_capability_error,
};
use crate::utils::missing_input_capabilities;

const SERVER_INFO_KEY: &str = "io.modelcontextprotocol/serverInfo";

/// Represents the internal outcome of dispatching a JSON-RPC method.
#[derive(Debug)]
pub(crate) struct DispatchOutcome {
    pub(crate) response: Option<serde_json::Value>,
    pub(crate) stream_body: Option<ResponseBody>,
    pub(crate) ttl_ms: Option<u64>,
    pub(crate) cache_scope: Option<CacheScope>,
    pub(crate) has_cache_headers: bool,
    pub(crate) no_store: bool,
    pub(crate) status_code: StatusCode,
}

impl DispatchOutcome {
    pub(crate) fn response_with_cache(
        val: serde_json::Value,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) -> Self {
        Self {
            response: Some(val),
            stream_body: None,
            ttl_ms,
            cache_scope,
            has_cache_headers: true,
            no_store: false,
            status_code: StatusCode::OK,
        }
    }

    pub(crate) fn error(err: JsonRpcErrorResponse) -> Self {
        let status_code = mcp_error_code_to_http_status(err.error.code.code());
        Self {
            response: serde_json::to_value(err).ok(),
            stream_body: None,
            ttl_ms: None,
            cache_scope: None,
            has_cache_headers: false,
            no_store: false,
            status_code,
        }
    }

    pub(crate) fn notification() -> Self {
        Self {
            response: None,
            stream_body: None,
            ttl_ms: None,
            cache_scope: None,
            has_cache_headers: false,
            no_store: false,
            status_code: StatusCode::ACCEPTED,
        }
    }

    pub(crate) fn sse_stream(body: ResponseBody) -> Self {
        Self {
            response: None,
            stream_body: Some(body),
            ttl_ms: None,
            cache_scope: None,
            has_cache_headers: false,
            no_store: false,
            status_code: StatusCode::OK,
        }
    }

    /// Replaces an `input_required` result whose input requests need client capabilities the
    /// client did not declare with a `MissingRequiredClientCapability` (`-32021`) error.
    pub(crate) fn require_client_capabilities(
        &mut self,
        req_id: Option<JsonRpcRequestId>,
        client_capabilities: Option<&ClientCapabilities>,
    ) {
        let Some(result) = self.response.as_ref().and_then(|r| r.get("result")) else {
            return;
        };
        if result.get("resultType").and_then(|v| v.as_str()) != Some("input_required") {
            return;
        }
        if let Some(required) = missing_input_capabilities(result, client_capabilities) {
            tracing::debug!("Input request needs a client capability the client did not declare");
            *self = Self::error(missing_required_client_capability_error(
                req_id,
                "Missing required client capability for the requested input",
                required,
            ));
        }
    }

    /// Adds `io.modelcontextprotocol/serverInfo` to the result's `_meta`, unless the result
    /// already carries it, so the server identifies itself on every result.
    pub(crate) fn add_server_info(&mut self, server_info: &Implementation) {
        let Some(result) = self
            .response
            .as_mut()
            .and_then(|r| r.get_mut("result"))
            .and_then(|r| r.as_object_mut())
        else {
            return;
        };
        let Some(meta) = result
            .entry("_meta")
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()))
            .as_object_mut()
        else {
            return;
        };
        if !meta.contains_key(SERVER_INFO_KEY)
            && let Ok(info) = serde_json::to_value(server_info)
        {
            meta.insert(SERVER_INFO_KEY.to_string(), info);
        }
    }

    /// Applies the MCP caching rules to a successful result.
    ///
    /// - `input_required` results are not cacheable: their `ttlMs` and `cacheScope` hints are
    ///   removed and the response is marked `no-store`.
    /// - Results of multi round-trip retries (requests carrying `requestState` or
    ///   `inputResponses`) must not be cached: `ttlMs` is set to `0` and the response is marked
    ///   `no-store`.
    /// - Otherwise, HTTP caching directives are taken from the result's own `ttlMs` and
    ///   `cacheScope` hints when present, so the body and headers never disagree.
    pub(crate) fn apply_cache_policy(&mut self, is_retry: bool) {
        let Some(result) = self
            .response
            .as_mut()
            .and_then(|r| r.get_mut("result"))
            .and_then(|r| r.as_object_mut())
        else {
            return;
        };

        if result.get("resultType").and_then(|v| v.as_str()) == Some("input_required") {
            result.remove("ttlMs");
            result.remove("cacheScope");
            self.no_store = true;
        } else if is_retry {
            if result.contains_key("ttlMs") {
                result.insert("ttlMs".to_string(), 0.into());
            }
            self.no_store = true;
        } else if self.has_cache_headers {
            if let Some(ttl_ms) = result.get("ttlMs").and_then(|v| v.as_u64()) {
                self.ttl_ms = Some(ttl_ms);
            }
            if let Some(scope) = result
                .get("cacheScope")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
            {
                self.cache_scope = Some(scope);
            }
        }
    }
}

/// Context passed to capability dispatchers containing request correlation and metadata.
pub(crate) struct MethodContext<'a> {
    pub(crate) req_id: Option<JsonRpcRequestId>,
    pub(crate) header_name: Option<Cow<'a, str>>,
    pub(crate) headers: &'a http::HeaderMap,
    pub(crate) extensions: Arc<http::Extensions>,
}
