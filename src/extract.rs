// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Request Extractors and Context
//!
//! Provides extractors for handler functions in `mcp-routing`, including [`RequestContext`],
//! [`SessionId`], [`Extension`], and [`Meta`].

use std::sync::Arc;

use http::HeaderMap;

use crate::types::mcp::{Implementation, LoggingLevel, ProgressToken, RequestMetaObject};

/// Error encountered during request extraction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionError(pub String);

impl std::fmt::Display for ExtractionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ExtractionError {}

/// An identifier for an MCP session, extracted from the `Mcp-Session-Id` HTTP header.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SessionId(pub String);

impl SessionId {
    /// Creates a new [`SessionId`].
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the session ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::ops::Deref for SessionId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for SessionId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for SessionId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for SessionId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Extractor for protocol-level request metadata ([`RequestMetaObject`]).
#[derive(Debug, Clone)]
pub struct Meta(pub RequestMetaObject);

impl std::ops::Deref for Meta {
    type Target = RequestMetaObject;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Meta {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<RequestMetaObject> for Meta {
    fn from(meta: RequestMetaObject) -> Self {
        Self(meta)
    }
}

/// Extractor for request extensions provided by Tower middleware or web frameworks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Extension<T>(pub T);

impl<T> std::ops::Deref for Extension<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for Extension<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> From<T> for Extension<T> {
    fn from(inner: T) -> Self {
        Self(inner)
    }
}

/// Extractor for application state passed via `with_state` or request extensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct State<T>(pub T);

impl<T> std::ops::Deref for State<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for State<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> From<T> for State<T> {
    fn from(inner: T) -> Self {
        Self(inner)
    }
}

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
        self.meta.as_ref().and_then(|m| m.protocol_version.as_deref())
    }

    /// Returns the client log level from request metadata, if present.
    pub fn log_level(&self) -> Option<&LoggingLevel> {
        self.meta.as_ref().and_then(|m| m.log_level.as_ref())
    }
}

/// Trait for types that can be extracted from a [`RequestContext`].
pub trait FromRequestContext: Sized {
    type Error: std::fmt::Display + Send;
    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error>;
}

impl FromRequestContext for RequestContext {
    type Error = std::convert::Infallible;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        Ok(ctx.clone())
    }
}

impl FromRequestContext for SessionId {
    type Error = ExtractionError;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        ctx.session_id
            .clone()
            .ok_or_else(|| ExtractionError("Missing required Mcp-Session-Id header".to_string()))
    }
}

impl FromRequestContext for Option<SessionId> {
    type Error = std::convert::Infallible;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        Ok(ctx.session_id.clone())
    }
}

impl FromRequestContext for Meta {
    type Error = ExtractionError;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        ctx.meta
            .clone()
            .map(Meta)
            .ok_or_else(|| ExtractionError("Missing required _meta in request parameters".to_string()))
    }
}

impl FromRequestContext for Option<Meta> {
    type Error = std::convert::Infallible;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        Ok(ctx.meta.clone().map(Meta))
    }
}

impl<T> FromRequestContext for Extension<T>
where
    T: Clone + Send + Sync + 'static,
{
    type Error = ExtractionError;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        ctx.extensions
            .get::<T>()
            .cloned()
            .map(Extension)
            .ok_or_else(|| {
                ExtractionError(format!(
                    "Missing request extension: {}",
                    std::any::type_name::<T>()
                ))
            })
    }
}

impl<T> FromRequestContext for Option<Extension<T>>
where
    T: Clone + Send + Sync + 'static,
{
    type Error = std::convert::Infallible;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        Ok(ctx.extensions.get::<T>().cloned().map(Extension))
    }
}

impl<T> FromRequestContext for State<T>
where
    T: Clone + Send + Sync + 'static,
{
    type Error = ExtractionError;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        ctx.extensions
            .get::<T>()
            .cloned()
            .map(State)
            .ok_or_else(|| {
                ExtractionError(format!(
                    "Missing state: {}",
                    std::any::type_name::<T>()
                ))
            })
    }
}

impl<T> FromRequestContext for Option<State<T>>
where
    T: Clone + Send + Sync + 'static,
{
    type Error = std::convert::Infallible;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        Ok(ctx.extensions.get::<T>().cloned().map(State))
    }
}

impl FromRequestContext for HeaderMap {
    type Error = std::convert::Infallible;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        Ok(ctx.headers.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_id_deref_and_display() {
        let sid = SessionId::new("sess-12345");
        assert_eq!(sid.as_str(), "sess-12345");
        assert_eq!(&*sid, "sess-12345");
        assert_eq!(sid.as_ref(), "sess-12345");
        assert_eq!(format!("{sid}"), "sess-12345");
        assert_eq!(SessionId::from("abc"), SessionId::new("abc"));
        assert_eq!(SessionId::from("def".to_string()), SessionId::new("def"));
    }

    #[test]
    fn test_extractors_from_context() {
        #[derive(Clone, Debug, PartialEq, Eq)]
        struct TestState(String);

        let mut headers = HeaderMap::new();
        headers.insert("Custom-Header", "custom-val".parse().unwrap());

        let mut ext = http::Extensions::new();
        ext.insert(TestState("state-data".to_string()));

        let meta = RequestMetaObject {
            progress_token: Some(ProgressToken::String("prog-1".to_string())),
            client_info: Some(Implementation::new("client-a", "1.0.0")),
            client_capabilities: None,
            protocol_version: Some("2026-07-28".to_string()),
            log_level: Some(LoggingLevel::Debug),
            extra: std::collections::HashMap::new(),
        };

        let ctx = RequestContext::new(
            Some(SessionId::new("sess-999")),
            Some(meta.clone()),
            headers,
            Arc::new(ext),
        );

        // RequestContext extractor
        let extracted_ctx = RequestContext::from_request_context(&ctx).unwrap();
        assert_eq!(extracted_ctx.session_id_str(), Some("sess-999"));
        assert_eq!(
            extracted_ctx.client_info().unwrap().name,
            "client-a"
        );
        assert_eq!(
            extracted_ctx.protocol_version(),
            Some("2026-07-28")
        );
        assert_eq!(
            extracted_ctx.log_level(),
            Some(&LoggingLevel::Debug)
        );
        assert!(matches!(
            extracted_ctx.progress_token(),
            Some(ProgressToken::String(s)) if s == "prog-1"
        ));

        // SessionId extractor
        let sid = SessionId::from_request_context(&ctx).unwrap();
        assert_eq!(sid.as_str(), "sess-999");

        let opt_sid = Option::<SessionId>::from_request_context(&ctx).unwrap();
        assert_eq!(opt_sid.as_deref(), Some("sess-999"));

        // Meta extractor
        let extracted_meta = Meta::from_request_context(&ctx).unwrap();
        assert_eq!(
            extracted_meta.protocol_version.as_deref(),
            Some("2026-07-28")
        );

        let opt_meta = Option::<Meta>::from_request_context(&ctx).unwrap();
        assert!(opt_meta.is_some());

        // Extension extractor
        let state = Extension::<TestState>::from_request_context(&ctx).unwrap();
        assert_eq!(state.0, TestState("state-data".to_string()));

        let opt_state = Option::<Extension<TestState>>::from_request_context(&ctx).unwrap();
        assert_eq!(opt_state.unwrap().0, TestState("state-data".to_string()));

        // State extractor
        let direct_state = State::<TestState>::from_request_context(&ctx).unwrap();
        assert_eq!(direct_state.0, TestState("state-data".to_string()));

        let opt_direct_state = Option::<State<TestState>>::from_request_context(&ctx).unwrap();
        assert_eq!(opt_direct_state.unwrap().0, TestState("state-data".to_string()));
        assert_eq!(ctx.state::<TestState>().unwrap(), TestState("state-data".to_string()));

        // HeaderMap extractor
        let extracted_headers = HeaderMap::from_request_context(&ctx).unwrap();
        assert_eq!(
            extracted_headers.get("Custom-Header").unwrap().to_str().unwrap(),
            "custom-val"
        );
    }

    #[test]
    fn test_extractors_missing_data_errors() {
        let ctx = RequestContext::new(
            None,
            None,
            HeaderMap::new(),
            Arc::new(http::Extensions::new()),
        );

        // Missing SessionId
        let sid_err = SessionId::from_request_context(&ctx).unwrap_err();
        assert!(sid_err.0.contains("Missing required Mcp-Session-Id header"));

        let opt_sid = Option::<SessionId>::from_request_context(&ctx).unwrap();
        assert_eq!(opt_sid, None);

        // Missing Meta
        let meta_err = Meta::from_request_context(&ctx).unwrap_err();
        assert!(meta_err.0.contains("Missing required _meta"));

        let opt_meta = Option::<Meta>::from_request_context(&ctx).unwrap();
        assert!(opt_meta.is_none());

        // Missing Extension
        #[derive(Clone, Debug)]
        struct MissingState;
        let ext_err = Extension::<MissingState>::from_request_context(&ctx).unwrap_err();
        assert!(ext_err.0.contains("Missing request extension"));

        let opt_ext = Option::<Extension<MissingState>>::from_request_context(&ctx).unwrap();
        assert!(opt_ext.is_none());

        // Missing State
        let state_err = State::<MissingState>::from_request_context(&ctx).unwrap_err();
        assert!(state_err.0.contains("Missing state"));

        let opt_state_missing = Option::<State<MissingState>>::from_request_context(&ctx).unwrap();
        assert!(opt_state_missing.is_none());
        assert!(ctx.state::<MissingState>().is_none());
    }
}
