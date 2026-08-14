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
    BoxError, ResponseBody, bad_request, empty_response, json_response, json_response_with_caching,
    method_not_allowed, unsupported_media_type,
};
use crate::router::{McpRouter, McpRouterInner};
use crate::types::jsonrpc::JsonRpcErrorResponse;
use crate::utils::{extract_session_id, is_json_content_type};

impl McpRouterInner {
    /// Dispatches an incoming HTTP request into JSON-RPC handling.
    pub(crate) async fn dispatch<B>(&self, req: Request<B>) -> Response<ResponseBody>
    where
        B: http_body::Body<Data = Bytes> + Send + 'static,
        B::Error: Into<BoxError>,
    {
        let session_id = extract_session_id(req.headers());

        let attach_session = |mut resp: Response<ResponseBody>| {
            if let Some(ref sid) = session_id
                && let Ok(header_val) = http::HeaderValue::from_str(sid.as_str())
            {
                resp.headers_mut().insert(
                    http::header::HeaderName::from_static("mcp-session-id"),
                    header_val,
                );
            }
            resp
        };

        if req.method() != http::Method::POST {
            tracing::debug!(method = %req.method(), "HTTP method not allowed, only POST is supported");
            return attach_session(method_not_allowed());
        }

        if !is_json_content_type(req.headers()) {
            tracing::debug!("Missing or unsupported Content-Type header");
            return attach_session(unsupported_media_type());
        }

        let (mut parts, body) = req.into_parts();
        if parts
            .extensions
            .get::<crate::extract::CurrentLoggingLevel>()
            .is_none()
        {
            parts
                .extensions
                .insert(crate::extract::CurrentLoggingLevel(self.logging.current_level()));
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
                return attach_session(bad_request());
            }
        };

        let raw_json: serde_json::Value = match serde_json::from_slice(&body_bytes) {
            Ok(val) => val,
            Err(err) => {
                tracing::debug!(?err, "Failed to parse JSON body");
                let error_response =
                    JsonRpcErrorResponse::parse_error(format!("Parse error: {err}"));
                return attach_session(json_response(&error_response));
            }
        };

        let response = match raw_json {
            serde_json::Value::Array(items) => {
                if items.is_empty() {
                    let error_response = JsonRpcErrorResponse::invalid_request(
                        None,
                        "Invalid Request: empty batch array",
                    );
                    json_response(&error_response)
                } else {
                    let mut responses: Vec<serde_json::Value> = Vec::with_capacity(items.len());
                    for item in items {
                        if let Some(resp) = self
                            .dispatch_item(
                                item,
                                &parts.headers,
                                session_id.clone(),
                                Arc::clone(&extensions),
                            )
                            .await
                        {
                            responses.push(resp);
                        }
                    }

                    if responses.is_empty() {
                        empty_response(StatusCode::NO_CONTENT)
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
                        session_id.clone(),
                        Arc::clone(&extensions),
                    )
                    .await;

                match outcome.response {
                    None => empty_response(StatusCode::NO_CONTENT),
                    Some(val) => {
                        if outcome.has_cache_headers {
                            json_response_with_caching(
                                &val,
                                outcome.ttl_ms,
                                outcome.cache_scope.as_ref(),
                            )
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
                json_response(&error_response)
            }
        };

        attach_session(response)
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
