// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Cow;
use std::sync::Arc;

use http::StatusCode;

use crate::body::ResponseBody;
use crate::types::jsonrpc::{JsonRpcErrorResponse, JsonRpcRequestId};
use crate::types::mcp::{CacheScope, mcp_error_code_to_http_status};

/// Represents the internal outcome of dispatching a JSON-RPC method.
#[derive(Debug)]
pub(crate) struct DispatchOutcome {
    pub(crate) response: Option<serde_json::Value>,
    pub(crate) stream_body: Option<ResponseBody>,
    pub(crate) ttl_ms: Option<u64>,
    pub(crate) cache_scope: Option<CacheScope>,
    pub(crate) has_cache_headers: bool,
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
            status_code: StatusCode::OK,
        }
    }
}

/// Context passed to capability dispatchers containing request correlation and metadata.
pub(crate) struct MethodContext<'a> {
    pub(crate) req_id: Option<JsonRpcRequestId>,
    pub(crate) is_notification: bool,
    pub(crate) is_batch: bool,
    pub(crate) header_name: Option<Cow<'a, str>>,
    pub(crate) headers: &'a http::HeaderMap,
    pub(crate) extensions: Arc<http::Extensions>,
}
