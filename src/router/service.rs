// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use bytes::Bytes;
use http::{Request, Response, StatusCode};
use http_body_util::BodyExt;
use tower::Service;

use crate::body::{
    bad_request, empty_response, forbidden, json_response, json_response_with_caching,
    json_response_with_status, method_not_allowed, unsupported_media_type, BoxError, ResponseBody,
};
use crate::router::{McpRouter, McpRouterInner};
use crate::types::jsonrpc::JsonRpcErrorResponse;
use crate::types::mcp::{header_mismatch_error, unsupported_protocol_version_error};
use crate::utils::{
    extract_protocol_version, is_json_content_type, is_origin_header_allowed,
};

impl McpRouterInner {
    /// Dispatches an incoming HTTP request into JSON-RPC handling.
    pub(crate) async fn dispatch<B>(&self, req: Request<B>) -> Response<ResponseBody>
    where
        B: http_body::Body<Data = Bytes> + Send + 'static,
        B::Error: Into<BoxError>,
    {
        if req.method() != http::Method::POST {
            tracing::debug!(method = %req.method(), "HTTP method not allowed, only POST is supported");
            return method_not_allowed();
        }

        if !is_json_content_type(req.headers()) {
            tracing::debug!("Missing or unsupported Content-Type header");
            return unsupported_media_type();
        }

        if let Some(ref allowed) = self.server.allowed_origins
            && !is_origin_header_allowed(req.headers(), allowed)
        {
            tracing::debug!("Rejected untrusted Origin header with 403 Forbidden");
            return forbidden();
        }

        if self.server.validate_protocol_version {
            match extract_protocol_version(req.headers()) {
                None => {
                    tracing::debug!("Missing required MCP-Protocol-Version header");
                    let error_response = header_mismatch_error(
                        None,
                        "Header mismatch: missing required MCP-Protocol-Version header",
                    );
                    return json_response_with_status(
                        StatusCode::BAD_REQUEST,
                        &error_response,
                    );
                }
                Some(req_ver) => {
                    if !self.server.supported_versions.iter().any(|v| v == req_ver) {
                        tracing::debug!(%req_ver, "Unsupported MCP-Protocol-Version header");
                        let error_response = unsupported_protocol_version_error(
                            None,
                            format!("Unsupported protocol version '{req_ver}'"),
                            self.server.supported_versions.clone(),
                            req_ver,
                        );
                        return json_response_with_status(
                            StatusCode::BAD_REQUEST,
                            &error_response,
                        );
                    }
                }
            }
        }

        let (mut parts, body) = req.into_parts();
        if parts
            .extensions
            .get::<crate::extract::CurrentLoggingLevel>()
            .is_none()
        {
            parts
                .extensions
                .insert(crate::extract::CurrentLoggingLevel(self.logging_level));
        }
        for injector in &self.state_injectors {
            injector(&mut parts.extensions);
        }
        let extensions = Arc::new(parts.extensions);

        let body_bytes = match body.collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(err) => {
                let err = err.into();
                tracing::error!(?err, "Failed to read request body");
                return bad_request();
            }
        };

        let raw_json: serde_json::Value = match serde_json::from_slice(&body_bytes) {
            Ok(val) => val,
            Err(err) => {
                tracing::debug!(?err, "Failed to parse JSON body");
                let error_response =
                    JsonRpcErrorResponse::parse_error(format!("Parse error: {err}"));
                return json_response_with_status(
                    StatusCode::BAD_REQUEST,
                    &error_response,
                );
            }
        };

        match raw_json {
            serde_json::Value::Array(items) => {
                if items.is_empty() {
                    let error_response = JsonRpcErrorResponse::invalid_request(
                        None,
                        "Invalid Request: empty batch array",
                    );
                    json_response_with_status(StatusCode::BAD_REQUEST, &error_response)
                } else {
                    let mut responses: Vec<serde_json::Value> = Vec::with_capacity(items.len());
                    for item in items {
                        if let Some(resp) = self
                            .dispatch_item(
                                item,
                                &parts.headers,
                                Arc::clone(&extensions),
                            )
                            .await
                        {
                            responses.push(resp);
                        }
                    }

                    if responses.is_empty() {
                        empty_response(StatusCode::ACCEPTED)
                    } else {
                        json_response(&responses)
                    }
                }
            }
            serde_json::Value::Object(map) => {
                let outcome = self
                    .dispatch_object(
                        map,
                        &parts.headers,
                        Arc::clone(&extensions),
                        false,
                    )
                    .await;

                match outcome.response {
                    None => empty_response(StatusCode::ACCEPTED),
                    Some(val) => {
                        if outcome.has_cache_headers {
                            json_response_with_caching(
                                &val,
                                outcome.ttl_ms,
                                outcome.cache_scope.as_ref(),
                            )
                        } else if outcome.status_code != StatusCode::OK {
                            json_response_with_status(outcome.status_code, &val)
                        } else {
                            json_response(&val)
                        }
                    }
                }
            }
            _ => {
                let error_response = JsonRpcErrorResponse::invalid_request(
                    None,
                    "Invalid Request: expected object or batch array",
                );
                json_response_with_status(StatusCode::BAD_REQUEST, &error_response)
            }
        }
    }
}

impl<B> Service<Request<B>> for McpRouter
where
    B: http_body::Body<Data = Bytes> + Send + 'static,
    B::Error: Into<BoxError>,
{
    type Response = Response<ResponseBody>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let this = Arc::clone(&self.inner);
        Box::pin(async move { Ok(this.dispatch(req).await) })
    }
}
