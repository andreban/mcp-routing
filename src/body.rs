// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use http::{Response, StatusCode, header};
use http_body_util::combinators::UnsyncBoxBody;
use http_body_util::{BodyExt, Empty, Full};

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// HTTP response body type produced by [`McpRouter`](crate::McpRouter).
pub struct ResponseBody(UnsyncBoxBody<Bytes, BoxError>);

impl fmt::Debug for ResponseBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResponseBody").finish()
    }
}

impl ResponseBody {
    /// Creates an empty response body.
    pub fn empty() -> Self {
        Self(Empty::new().map_err(Into::into).boxed_unsync())
    }

    /// Creates a response body containing the given bytes.
    pub fn from_bytes(bytes: Bytes) -> Self {
        Self(Full::new(bytes).map_err(Into::into).boxed_unsync())
    }

    /// Wraps any [`http_body::Body`] into a [`ResponseBody`].
    pub fn new<B>(body: B) -> Self
    where
        B: http_body::Body<Data = Bytes> + Send + 'static,
        B::Error: Into<BoxError>,
    {
        Self(body.map_err(Into::into).boxed_unsync())
    }
}

impl http_body::Body for ResponseBody {
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        Pin::new(&mut self.0).poll_frame(cx)
    }

    fn is_end_stream(&self) -> bool {
        self.0.is_end_stream()
    }

    fn size_hint(&self) -> http_body::SizeHint {
        self.0.size_hint()
    }
}

impl From<Bytes> for ResponseBody {
    fn from(bytes: Bytes) -> Self {
        Self::from_bytes(bytes)
    }
}

impl From<Vec<u8>> for ResponseBody {
    fn from(vec: Vec<u8>) -> Self {
        Self::from_bytes(Bytes::from(vec))
    }
}

impl From<String> for ResponseBody {
    fn from(s: String) -> Self {
        Self::from_bytes(Bytes::from(s))
    }
}

impl From<&'static str> for ResponseBody {
    fn from(s: &'static str) -> Self {
        Self::from_bytes(Bytes::from_static(s.as_bytes()))
    }
}

impl From<Full<Bytes>> for ResponseBody {
    fn from(body: Full<Bytes>) -> Self {
        Self::new(body)
    }
}

impl From<Empty<Bytes>> for ResponseBody {
    fn from(body: Empty<Bytes>) -> Self {
        Self::new(body)
    }
}

impl From<UnsyncBoxBody<Bytes, BoxError>> for ResponseBody {
    fn from(body: UnsyncBoxBody<Bytes, BoxError>) -> Self {
        Self(body)
    }
}

impl Default for ResponseBody {
    fn default() -> Self {
        Self::empty()
    }
}

/// Helper function to construct an empty response with the specified status code.
pub(crate) fn empty_response(status: StatusCode) -> Response<ResponseBody> {
    Response::builder()
        .status(status)
        .body(ResponseBody::empty())
        .unwrap()
}

/// Helper function to construct an empty 400 Bad Request response.
pub(crate) fn bad_request() -> Response<ResponseBody> {
    empty_response(StatusCode::BAD_REQUEST)
}

/// Helper function to construct an empty 403 Forbidden response.
pub(crate) fn forbidden() -> Response<ResponseBody> {
    empty_response(StatusCode::FORBIDDEN)
}

/// Helper function to construct an empty 405 Method Not Allowed response with `Allow: POST`.
pub(crate) fn method_not_allowed() -> Response<ResponseBody> {
    Response::builder()
        .status(StatusCode::METHOD_NOT_ALLOWED)
        .header(header::ALLOW, "POST")
        .body(ResponseBody::empty())
        .unwrap()
}

/// Helper function to construct an empty 415 Unsupported Media Type response.
pub(crate) fn unsupported_media_type() -> Response<ResponseBody> {
    empty_response(StatusCode::UNSUPPORTED_MEDIA_TYPE)
}

/// Formats an HTTP `Cache-Control` header value based on optional `ttl_ms` and `cache_scope`.
///
/// Directives:
/// - `CacheScope::Public` -> `"public"`
/// - `CacheScope::Private` -> `"private"`
/// - `ttl_ms` -> `format!("max-age={}", ttl_ms / 1000)`
///
/// Returns `None` if neither `ttl_ms` nor `cache_scope` is provided.
pub fn format_cache_control(
    ttl_ms: Option<u64>,
    cache_scope: Option<&crate::types::mcp::CacheScope>,
) -> Option<String> {
    let mut directives = Vec::new();

    if let Some(scope) = cache_scope {
        match scope {
            crate::types::mcp::CacheScope::Public => directives.push("public".to_string()),
            crate::types::mcp::CacheScope::Private => directives.push("private".to_string()),
        }
    }

    if let Some(ttl) = ttl_ms {
        directives.push(format!("max-age={}", ttl / 1000));
    }

    if directives.is_empty() {
        None
    } else {
        Some(directives.join(", "))
    }
}

/// Computes an entity tag (ETag) string for the given response body bytes.
///
/// Uses standard FNV-1a 64-bit hashing and formats the result as a quoted entity tag (e.g. `"\"0123456789abcdef\""`).
pub fn compute_etag(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("\"{:016x}\"", hash)
}

/// Helper function to construct a JSON response with status 200 OK.
pub(crate) fn json_response<T: serde::Serialize>(val: &T) -> Response<ResponseBody> {
    json_response_with_status(StatusCode::OK, val)
}

/// Helper function to construct a JSON response with the specified status code.
pub(crate) fn json_response_with_status<T: serde::Serialize>(
    status: StatusCode,
    val: &T,
) -> Response<ResponseBody> {
    match serde_json::to_vec(val) {
        Ok(bytes) => Response::builder()
            .status(status)
            .header(header::CONTENT_TYPE, "application/json")
            .body(ResponseBody::from_bytes(Bytes::from(bytes)))
            .unwrap(),
        Err(err) => {
            tracing::error!(?err, "Failed to serialize JSON response");
            empty_response(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Helper function to construct a JSON response with status 200 OK, ETag, and optional Cache-Control headers.
pub(crate) fn json_response_with_caching<T: serde::Serialize>(
    val: &T,
    ttl_ms: Option<u64>,
    cache_scope: Option<&crate::types::mcp::CacheScope>,
) -> Response<ResponseBody> {
    match serde_json::to_vec(val) {
        Ok(bytes) => {
            let etag = compute_etag(&bytes);
            let mut builder = Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::ETAG, etag);

            if let Some(cache_control) = format_cache_control(ttl_ms, cache_scope) {
                builder = builder.header(header::CACHE_CONTROL, cache_control);
            }

            builder
                .body(ResponseBody::from_bytes(Bytes::from(bytes)))
                .unwrap()
        }
        Err(err) => {
            tracing::error!(?err, "Failed to serialize JSON response");
            empty_response(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Helper function to construct a Server-Sent Events (SSE) streaming response with status 200 OK.
pub(crate) fn sse_response(body: ResponseBody) -> Response<ResponseBody> {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache")
        .header(header::CONNECTION, "keep-alive")
        .header("X-Accel-Buffering", "no")
        .body(body)
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;

    /// Tests [`ResponseBody`] constructors, conversions, size hints, and async frame collection.
    #[tokio::test]
    async fn test_response_body_conversions_and_streaming() {
        // Test ResponseBody::empty()
        let empty_body = ResponseBody::empty();
        assert_eq!(http_body::Body::size_hint(&empty_body).exact(), Some(0));
        let collected_empty = empty_body.collect().await.unwrap().to_bytes();
        assert!(collected_empty.is_empty());

        // Test ResponseBody::from_bytes()
        let from_bytes_body = ResponseBody::from_bytes(Bytes::from_static(b"hello world"));
        let collected = from_bytes_body.collect().await.unwrap().to_bytes();
        assert_eq!(collected.as_ref(), b"hello world");

        // Test From<Vec<u8>>
        let from_vec: ResponseBody = vec![1, 2, 3, 4].into();
        let collected_vec = from_vec.collect().await.unwrap().to_bytes();
        assert_eq!(collected_vec.as_ref(), &[1, 2, 3, 4]);

        // Test From<String>
        let from_string: ResponseBody = String::from("test string").into();
        let collected_string = from_string.collect().await.unwrap().to_bytes();
        assert_eq!(collected_string.as_ref(), b"test string");

        // Test From<&'static str>
        let from_str: ResponseBody = "static str".into();
        let collected_str = from_str.collect().await.unwrap().to_bytes();
        assert_eq!(collected_str.as_ref(), b"static str");

        // Test Default
        let default_body = ResponseBody::default();
        let collected_default = default_body.collect().await.unwrap().to_bytes();
        assert!(collected_default.is_empty());
    }

    /// Tests helper functions for constructing standard HTTP responses.
    #[tokio::test]
    async fn test_response_helpers() {
        let resp_bad = bad_request();
        assert_eq!(resp_bad.status(), StatusCode::BAD_REQUEST);

        let resp_not_allowed = method_not_allowed();
        assert_eq!(resp_not_allowed.status(), StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(
            resp_not_allowed.headers().get(header::ALLOW).unwrap(),
            "POST"
        );

        let resp_unsupported = unsupported_media_type();
        assert_eq!(
            resp_unsupported.status(),
            StatusCode::UNSUPPORTED_MEDIA_TYPE
        );

        let resp_json = json_response(&serde_json::json!({"ok": true}));
        assert_eq!(resp_json.status(), StatusCode::OK);
        assert_eq!(
            resp_json.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
        let bytes = resp_json.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(bytes.as_ref(), b"{\"ok\":true}");
    }

    /// Tests `format_cache_control` directive combinations and edge cases.
    #[test]
    fn test_format_cache_control() {
        use crate::types::mcp::CacheScope;

        // Both public and ttl_ms
        assert_eq!(
            format_cache_control(Some(0), Some(&CacheScope::Public)),
            Some("public, max-age=0".to_string())
        );
        assert_eq!(
            format_cache_control(Some(60000), Some(&CacheScope::Public)),
            Some("public, max-age=60".to_string())
        );

        // Private and ttl_ms
        assert_eq!(
            format_cache_control(Some(3600000), Some(&CacheScope::Private)),
            Some("private, max-age=3600".to_string())
        );

        // Scope only
        assert_eq!(
            format_cache_control(None, Some(&CacheScope::Public)),
            Some("public".to_string())
        );
        assert_eq!(
            format_cache_control(None, Some(&CacheScope::Private)),
            Some("private".to_string())
        );

        // TTL only
        assert_eq!(
            format_cache_control(Some(5000), None),
            Some("max-age=5".to_string())
        );

        // Neither
        assert_eq!(format_cache_control(None, None), None);
    }

    /// Tests deterministic `compute_etag` output and quoting format.
    #[test]
    fn test_compute_etag() {
        let etag1 = compute_etag(b"{\"hello\":\"world\"}");
        let etag2 = compute_etag(b"{\"hello\":\"world\"}");
        let etag_different = compute_etag(b"{\"hello\":\"other\"}");

        // Must be quoted
        assert!(etag1.starts_with('"') && etag1.ends_with('"'));
        // Deterministic
        assert_eq!(etag1, etag2);
        // Different payloads produce different ETags
        assert_ne!(etag1, etag_different);
    }

    /// Tests `json_response_with_caching` headers propagation.
    #[tokio::test]
    async fn test_json_response_with_caching() {
        use crate::types::mcp::CacheScope;

        let data = serde_json::json!({"result": "cached_data"});
        let resp = json_response_with_caching(&data, Some(10000), Some(&CacheScope::Public));

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
        assert_eq!(
            resp.headers().get(header::CACHE_CONTROL).unwrap(),
            "public, max-age=10"
        );
        assert!(resp.headers().contains_key(header::ETAG));

        let expected_etag = compute_etag(serde_json::to_string(&data).unwrap().as_bytes());
        assert_eq!(
            resp.headers().get(header::ETAG).unwrap().to_str().unwrap(),
            expected_etag
        );
    }

    /// Tests `forbidden` response helper produces empty body with 403 Forbidden status.
    #[tokio::test]
    async fn test_forbidden_response() {
        let resp = forbidden();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        let collected = resp.into_body().collect().await.unwrap().to_bytes();
        assert!(collected.is_empty());
    }

    /// Tests `sse_response` helper sets 200 OK, `text/event-stream`, `no-cache`, and passes body frames.
    #[tokio::test]
    async fn test_sse_response() {
        let resp = sse_response(ResponseBody::from_bytes(Bytes::from_static(
            b"event: message\ndata: 1\n\n",
        )));
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/event-stream"
        );
        assert_eq!(
            resp.headers().get(header::CACHE_CONTROL).unwrap(),
            "no-cache"
        );
        let collected = resp.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(collected.as_ref(), b"event: message\ndata: 1\n\n");
    }
}
