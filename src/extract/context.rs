// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Request context containing headers, session ID, extensions, and metadata.

use std::sync::Arc;

use http::HeaderMap;

use crate::extract::session::SessionId;
use crate::extract::traits::FromRequestContext;
use crate::types::mcp::{Implementation, LoggingLevel, ProgressToken, RequestMetaObject};

/// Context extracted from the incoming HTTP request and JSON-RPC envelope.
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// Session ID extracted from the `Mcp-Session-Id` header, if present.
    pub session_id: Option<SessionId>,
    /// Protocol-level metadata extracted from `params._meta`, if present.
    pub meta: Option<RequestMetaObject>,
    /// HTTP headers from the incoming request.
    pub headers: HeaderMap,
    /// HTTP extensions attached to the incoming request.
    pub extensions: Arc<http::Extensions>,
}

impl RequestContext {
    /// Creates a new [`RequestContext`].
    pub fn new(
        session_id: Option<SessionId>,
        meta: Option<RequestMetaObject>,
        headers: HeaderMap,
        extensions: Arc<http::Extensions>,
    ) -> Self {
        Self {
            session_id,
            meta,
            headers,
            extensions,
        }
    }

    /// Returns the session ID, if present.
    pub fn session_id(&self) -> Option<&SessionId> {
        self.session_id.as_ref()
    }

    /// Returns the session ID as a string slice, if present.
    pub fn session_id_str(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Returns the request metadata, if present.
    pub fn meta(&self) -> Option<&RequestMetaObject> {
        self.meta.as_ref()
    }

    /// Returns the HTTP headers of the request.
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Returns the HTTP extensions of the request.
    pub fn extensions(&self) -> &http::Extensions {
        &self.extensions
    }

    /// Retrieves a cloned value of a type stored in the request extensions.
    pub fn extension<T: Clone + Send + Sync + 'static>(&self) -> Option<T> {
        self.extensions.get::<T>().cloned()
    }

    /// Retrieves a cloned state or extension value from the request context.
    pub fn state<T: Clone + Send + Sync + 'static>(&self) -> Option<T> {
        self.extension::<T>()
    }

    /// Returns the client implementation info from request metadata, if present.
    pub fn client_info(&self) -> Option<&Implementation> {
        self.meta.as_ref().and_then(|m| m.client_info.as_ref())
    }

    /// Returns the progress token from request metadata, if present.
    pub fn progress_token(&self) -> Option<&ProgressToken> {
        self.meta.as_ref().and_then(|m| m.progress_token.as_ref())
    }

    /// Returns the client protocol version from request metadata, if present.
    pub fn protocol_version(&self) -> Option<&str> {
        self.meta
            .as_ref()
            .and_then(|m| m.protocol_version.as_deref())
    }

    /// Returns the client log level from request metadata, if present.
    pub fn log_level(&self) -> Option<&LoggingLevel> {
        self.meta.as_ref().and_then(|m| m.log_level.as_ref())
    }

    /// Returns the raw HTTP `Authorization` header value, if present.
    pub fn authorization(&self) -> Option<&str> {
        self.headers
            .get(http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
    }

    /// Extracts the Bearer token from the `Authorization` header, if present and valid.
    pub fn bearer_token(&self) -> Option<&str> {
        let auth = self.authorization()?;
        let trimmed = auth.trim();
        if trimmed.len() >= 7 && trimmed[..7].eq_ignore_ascii_case("bearer ") {
            Some(trimmed[7..].trim())
        } else {
            None
        }
    }
}

impl FromRequestContext for RequestContext {
    type Error = std::convert::Infallible;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        Ok(ctx.clone())
    }
}

impl FromRequestContext for HeaderMap {
    type Error = std::convert::Infallible;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        Ok(ctx.headers.clone())
    }
}
