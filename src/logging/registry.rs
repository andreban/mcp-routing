// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, RwLock};

use http::Response;

use crate::body::{ResponseBody, json_response, json_response_with_caching};
use crate::extract::{RequestContext, SessionId};
use crate::logging::{IntoSetLevelHandler, LoggingError, SetLevelHandler};
use crate::router::{DispatchOutcome, MethodContext};
use crate::types::jsonrpc::{JsonRpcErrorResponse, JsonRpcRequestId};
use crate::types::mcp::{
    CacheScope, LoggingLevel,
    logging::{SetLevelParams, SetLevelRequest, SetLevelResult, SetLevelResultResponse},
};

/// Registry managing logging level state and dynamic `logging/setLevel` handlers.
#[derive(Clone)]
pub struct LoggingRegistry {
    pub(crate) current_level: Arc<RwLock<LoggingLevel>>,
    pub(crate) handler: Option<Arc<dyn SetLevelHandler>>,
    pub(crate) cache_ttl_ms: Option<u64>,
    pub(crate) cache_scope: Option<CacheScope>,
}

impl Default for LoggingRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl LoggingRegistry {
    /// Creates a new [`LoggingRegistry`] with default [`LoggingLevel::Info`] severity.
    pub fn new() -> Self {
        Self {
            current_level: Arc::new(RwLock::new(LoggingLevel::Info)),
            handler: None,
            cache_ttl_ms: None,
            cache_scope: None,
        }
    }

    /// Creates a new [`LoggingRegistry`] with the specified initial logging level.
    pub fn with_level(level: LoggingLevel) -> Self {
        Self {
            current_level: Arc::new(RwLock::new(level)),
            handler: None,
            cache_ttl_ms: None,
            cache_scope: None,
        }
    }

    /// Returns the server's current dynamic logging level.
    pub fn current_level(&self) -> LoggingLevel {
        *self.current_level.read().unwrap()
    }

    /// Updates the server's current dynamic logging level.
    pub fn set_current_level(&self, level: LoggingLevel) {
        *self.current_level.write().unwrap() = level;
    }

    /// Sets a custom asynchronous handler for `logging/setLevel` requests.
    pub fn set_handler<H, T>(&mut self, handler: H)
    where
        H: IntoSetLevelHandler<T>,
        T: 'static,
    {
        self.handler = Some(handler.into_set_level_handler());
    }

    /// Returns `true` if a custom `logging/setLevel` handler is registered.
    pub fn has_custom_handler(&self) -> bool {
        self.handler.is_some()
    }

    /// Sets caching directives (`ttl_ms` and `cache_scope`) for `logging/setLevel` responses.
    pub fn set_cache(&mut self, ttl_ms: Option<u64>, cache_scope: Option<CacheScope>) {
        self.cache_ttl_ms = ttl_ms;
        self.cache_scope = cache_scope;
    }

    /// Dispatches an incoming `logging/setLevel` JSON-RPC request.
    pub(crate) async fn dispatch_set_level(
        &self,
        ctx: MethodContext<'_>,
        params_val: Option<serde_json::Value>,
    ) -> DispatchOutcome {
        let Some(pv) = params_val else {
            if ctx.is_notification {
                return DispatchOutcome::notification();
            }
            return DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                ctx.req_id,
                "Invalid params: missing parameters for logging/setLevel",
            ));
        };

        let params: SetLevelParams = match serde_json::from_value(pv) {
            Ok(p) => p,
            Err(err) => {
                if ctx.is_notification {
                    return DispatchOutcome::notification();
                }
                return DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                    ctx.req_id,
                    format!("Invalid params: {err}"),
                ));
            }
        };

        // Update server's dynamic logging level
        self.set_current_level(params.level);

        let request_ctx = RequestContext::new(
            ctx.session_id,
            params.meta.clone(),
            ctx.headers.clone(),
            ctx.extensions,
        );

        if let Some(ref handler) = self.handler {
            match handler.call(request_ctx, params).await {
                Ok(result) => {
                    if ctx.is_notification {
                        return DispatchOutcome::notification();
                    }
                    let response = SetLevelResultResponse::new(
                        ctx.req_id.unwrap_or_else(|| "".into()),
                        result,
                    );
                    match serde_json::to_value(response) {
                        Ok(v) => DispatchOutcome::response_with_cache(
                            v,
                            self.cache_ttl_ms,
                            self.cache_scope.clone(),
                        ),
                        Err(err) => DispatchOutcome::error(JsonRpcErrorResponse::internal_error(
                            None,
                            format!("Failed to serialize response: {err}"),
                        )),
                    }
                }
                Err(LoggingError::InvalidParams(err)) => {
                    if ctx.is_notification {
                        return DispatchOutcome::notification();
                    }
                    DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                        ctx.req_id,
                        format!("Invalid params: {err}"),
                    ))
                }
                Err(LoggingError::Internal(err)) => {
                    if ctx.is_notification {
                        return DispatchOutcome::notification();
                    }
                    DispatchOutcome::error(JsonRpcErrorResponse::internal_error(
                        ctx.req_id,
                        format!("Logging handler failed: {err}"),
                    ))
                }
            }
        } else {
            if ctx.is_notification {
                return DispatchOutcome::notification();
            }
            let response = SetLevelResultResponse::new(
                ctx.req_id.unwrap_or_else(|| "".into()),
                SetLevelResult::default(),
            );
            match serde_json::to_value(response) {
                Ok(v) => DispatchOutcome::response_with_cache(
                    v,
                    self.cache_ttl_ms,
                    self.cache_scope.clone(),
                ),
                Err(err) => DispatchOutcome::error(JsonRpcErrorResponse::internal_error(
                    None,
                    format!("Failed to serialize response: {err}"),
                )),
            }
        }
    }

    /// Handles an incoming `logging/setLevel` HTTP request directly.
    pub async fn handle_set_level(
        &self,
        req_id: Option<JsonRpcRequestId>,
        session_id: Option<SessionId>,
        headers: &http::HeaderMap,
        extensions: Arc<http::Extensions>,
        body: &[u8],
    ) -> Response<ResponseBody> {
        let request: SetLevelRequest = match serde_json::from_slice(body) {
            Ok(r) => r,
            Err(err) => {
                tracing::error!(?err, "Failed to parse SetLevelRequest");
                let error_response =
                    JsonRpcErrorResponse::invalid_params(req_id, format!("Invalid params: {err}"));
                return json_response(&error_response);
            }
        };

        let Some(params) = request.params else {
            let error_response = JsonRpcErrorResponse::invalid_params(
                Some(request.id),
                "Invalid params: missing parameters for logging/setLevel",
            );
            return json_response(&error_response);
        };

        self.set_current_level(params.level);

        let result = if let Some(ref handler) = self.handler {
            let meta = params.meta.clone();
            let request_ctx = RequestContext::new(session_id, meta, headers.clone(), extensions);
            match handler.call(request_ctx, params).await {
                Ok(res) => res,
                Err(LoggingError::InvalidParams(err)) => {
                    let error_response = JsonRpcErrorResponse::invalid_params(
                        Some(request.id),
                        format!("Invalid params: {err}"),
                    );
                    return json_response(&error_response);
                }
                Err(LoggingError::Internal(err)) => {
                    let error_response = JsonRpcErrorResponse::internal_error(
                        Some(request.id),
                        format!("Logging handler failed: {err}"),
                    );
                    return json_response(&error_response);
                }
            }
        } else {
            SetLevelResult::default()
        };

        let response = SetLevelResultResponse::new(request.id, result);
        json_response_with_caching(&response, self.cache_ttl_ms, self.cache_scope.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::SessionId;
    use crate::types::jsonrpc::JsonRpcRequestId;

    #[tokio::test]
    async fn test_logging_registry_default_dispatch() {
        let registry = LoggingRegistry::new();
        assert_eq!(registry.current_level(), LoggingLevel::Info);

        let headers = http::HeaderMap::new();
        let extensions = Arc::new(http::Extensions::new());
        let ctx = MethodContext {
            req_id: Some(JsonRpcRequestId::Number(1.0)),
            is_notification: false,
            is_batch: false,
            header_name: None,
            session_id: Some(SessionId::new("sess-1")),
            headers: &headers,
            extensions,
        };

        let params = serde_json::json!({
            "level": "debug"
        });

        let outcome = registry.dispatch_set_level(ctx, Some(params)).await;
        let resp = outcome.response.expect("expected response");
        assert_eq!(resp["result"], serde_json::json!({}));
        assert_eq!(registry.current_level(), LoggingLevel::Debug);
    }

    #[tokio::test]
    async fn test_logging_registry_custom_handler_dispatch() {
        let mut registry = LoggingRegistry::new();
        registry.set_handler(|params: SetLevelParams| async move {
            assert_eq!(params.level, LoggingLevel::Warning);
            Ok::<(), LoggingError>(())
        });

        let headers = http::HeaderMap::new();
        let extensions = Arc::new(http::Extensions::new());
        let ctx = MethodContext {
            req_id: Some(JsonRpcRequestId::String("id-2".into())),
            is_notification: false,
            is_batch: false,
            header_name: None,
            session_id: None,
            headers: &headers,
            extensions,
        };

        let params = serde_json::json!({
            "level": "warning"
        });

        let outcome = registry.dispatch_set_level(ctx, Some(params)).await;
        let resp = outcome.response.expect("expected response");
        assert_eq!(resp["result"], serde_json::json!({}));
        assert_eq!(registry.current_level(), LoggingLevel::Warning);
    }

    #[tokio::test]
    async fn test_logging_registry_invalid_params() {
        let registry = LoggingRegistry::new();

        let headers = http::HeaderMap::new();
        let extensions = Arc::new(http::Extensions::new());
        let ctx = MethodContext {
            req_id: Some(JsonRpcRequestId::Number(3.0)),
            is_notification: false,
            is_batch: false,
            header_name: None,
            session_id: None,
            headers: &headers,
            extensions,
        };

        let outcome = registry.dispatch_set_level(ctx, None).await;
        let resp = outcome.response.expect("expected error response");
        assert_eq!(resp["error"]["code"], -32602);
    }
}
