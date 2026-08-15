// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Multi Round-Trip Request (MRTR) extractors for handler functions.
//!
//! Provides [`RequestState`] and [`InputResponses`] extractors to allow handlers
//! to resume multi round-trip workflows when clients retry requests.

use std::collections::HashMap;
use std::fmt;
use std::ops::Deref;

use serde::de::DeserializeOwned;

use crate::extract::context::RequestContext;
use crate::extract::error::ExtractionError;
use crate::extract::traits::FromRequestContext;
use crate::types::mcp::InputResponse;

/// Extractor for opaque request state in Multi Round-Trip Requests (MRTR).
///
/// Injected when the incoming request's `params` contains a `requestState` string.
///
/// # Example
///
/// ```rust,ignore
/// async fn my_handler(
///     state: Option<RequestState>,
/// ) -> Result<CallToolResult, ToolError> {
///     if let Some(state) = state {
///         // Resume multi round-trip workflow using saved state
///     } else {
///         // First request: initiate multi round-trip workflow
///     }
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RequestState(pub String);

impl RequestState {
    /// Creates a new [`RequestState`].
    pub fn new(state: impl Into<String>) -> Self {
        Self(state.into())
    }

    /// Returns the request state as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the wrapper and returns the inner string.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Deref for RequestState {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for RequestState {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RequestState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for RequestState {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for RequestState {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl FromRequestContext for RequestState {
    type Error = ExtractionError;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        ctx.extensions()
            .get::<RequestState>()
            .cloned()
            .ok_or_else(|| ExtractionError("Missing required 'requestState' in request".to_string()))
    }
}

impl FromRequestContext for Option<RequestState> {
    type Error = ExtractionError;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        Ok(ctx.extensions().get::<RequestState>().cloned())
    }
}

/// Extractor for client responses to server-initiated input requests in Multi Round-Trip Requests (MRTR).
///
/// Injected when the incoming request's `params` contains an `inputResponses` map.
///
/// # Example
///
/// ```rust,ignore
/// async fn my_handler(
///     responses: Option<InputResponses>,
/// ) -> Result<CallToolResult, ToolError> {
///     if let Some(responses) = responses {
///         if let Some(resp) = responses.get("confirm_action") {
///             // Process client response
///         }
///     }
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct InputResponses(pub HashMap<String, InputResponse>);

impl InputResponses {
    /// Creates a new [`InputResponses`] from a map of responses.
    pub fn new(responses: HashMap<String, InputResponse>) -> Self {
        Self(responses)
    }

    /// Retrieves an input response by its key.
    pub fn get(&self, key: &str) -> Option<&InputResponse> {
        self.0.get(key)
    }

    /// Deserializes the result payload for the given input request key.
    pub fn get_result<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, serde_json::Error> {
        match self.0.get(key) {
            Some(resp) => resp.get_result(),
            None => Ok(None),
        }
    }

    /// Consumes the wrapper and returns the inner map.
    pub fn into_inner(self) -> HashMap<String, InputResponse> {
        self.0
    }
}

impl Deref for InputResponses {
    type Target = HashMap<String, InputResponse>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<HashMap<String, InputResponse>> for InputResponses {
    fn from(map: HashMap<String, InputResponse>) -> Self {
        Self(map)
    }
}

impl FromRequestContext for InputResponses {
    type Error = ExtractionError;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        ctx.extensions()
            .get::<InputResponses>()
            .cloned()
            .ok_or_else(|| ExtractionError("Missing required 'inputResponses' in request".to_string()))
    }
}

impl FromRequestContext for Option<InputResponses> {
    type Error = ExtractionError;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        Ok(ctx.extensions().get::<InputResponses>().cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use http::HeaderMap;
    use serde_json::json;

    #[test]
    fn test_request_state_extractor() {
        let mut ext = http::Extensions::new();
        ext.insert(RequestState::new("state_abc_123"));

        let ctx = RequestContext::new(None, None, HeaderMap::new(), Arc::new(ext));

        let required = RequestState::from_request_context(&ctx).unwrap();
        assert_eq!(required.as_str(), "state_abc_123");
        assert_eq!(&*required, "state_abc_123");

        let optional = Option::<RequestState>::from_request_context(&ctx).unwrap();
        assert_eq!(optional.as_deref(), Some("state_abc_123"));

        let empty_ctx = RequestContext::new(None, None, HeaderMap::new(), Arc::new(http::Extensions::new()));
        assert!(RequestState::from_request_context(&empty_ctx).is_err());
        assert_eq!(
            Option::<RequestState>::from_request_context(&empty_ctx).unwrap(),
            None
        );
    }

    #[test]
    fn test_input_responses_extractor() {
        let mut responses_map = HashMap::new();
        responses_map.insert(
            "prompt_user".to_string(),
            InputResponse::result(&json!({"approved": true})).unwrap(),
        );

        let mut ext = http::Extensions::new();
        ext.insert(InputResponses::new(responses_map));

        let ctx = RequestContext::new(None, None, HeaderMap::new(), Arc::new(ext));

        let extracted = InputResponses::from_request_context(&ctx).unwrap();
        assert!(extracted.get("prompt_user").is_some());
        let val: Option<serde_json::Value> = extracted.get_result("prompt_user").unwrap();
        assert_eq!(val.unwrap()["approved"], true);

        let opt = Option::<InputResponses>::from_request_context(&ctx).unwrap();
        assert!(opt.is_some());

        let empty_ctx = RequestContext::new(None, None, HeaderMap::new(), Arc::new(http::Extensions::new()));
        assert!(InputResponses::from_request_context(&empty_ctx).is_err());
        assert!(Option::<InputResponses>::from_request_context(&empty_ctx).unwrap().is_none());
    }
}
