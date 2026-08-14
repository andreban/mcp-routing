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

/// Helper function to construct a JSON response with status 200 OK.
pub(crate) fn json_response<T: serde::Serialize>(val: &T) -> Response<ResponseBody> {
    match serde_json::to_vec(val) {
        Ok(bytes) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(ResponseBody::from_bytes(Bytes::from(bytes)))
            .unwrap(),
        Err(err) => {
            tracing::error!(?err, "Failed to serialize JSON response");
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(ResponseBody::empty())
                .unwrap()
        }
    }
}
