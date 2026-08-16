// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Multi Round-Trip Request (MRTR) response types per MCP 2026-07-28 specification (SEP-2322).

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::types::mcp::RequestMetaObject;

/// A client response to a server-initiated [`InputRequest`](crate::types::mcp::core::mrtr::InputRequest).
///
/// In MCP 2026-07-28 (SEP-2322), this contains the result of the client fulfilling
/// a requested input (e.g. LLM completion, roots list, or user elicitation).
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#inputresponse>
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InputResponse {
    /// Result payload returned by the client.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Error payload if the client failed to fulfill the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Value>,
    /// Additional unrecognized or custom metadata properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extras: HashMap<String, Value>,
}

impl InputResponse {
    /// Creates a successful [`InputResponse`] containing a serialized result payload.
    pub fn result<T: Serialize>(value: &T) -> Result<Self, serde_json::Error> {
        let val = serde_json::to_value(value)?;
        Ok(Self {
            result: Some(val),
            error: None,
            extras: HashMap::new(),
        })
    }

    /// Creates an error [`InputResponse`] containing a serialized error payload.
    pub fn error<T: Serialize>(error: &T) -> Result<Self, serde_json::Error> {
        let err_val = serde_json::to_value(error)?;
        Ok(Self {
            result: None,
            error: Some(err_val),
            extras: HashMap::new(),
        })
    }

    /// Creates an [`InputResponse`] from an arbitrary JSON value.
    pub fn from_value(value: Value) -> Self {
        serde_json::from_value(value).unwrap_or_default()
    }

    /// Returns `true` if this input response represents an error.
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }

    /// Deserializes the result payload into a typed struct.
    pub fn get_result<T: DeserializeOwned>(&self) -> Result<Option<T>, serde_json::Error> {
        match &self.result {
            Some(v) => serde_json::from_value(v.clone()).map(Some),
            None => Ok(None),
        }
    }

    /// Deserializes the error payload into a typed struct.
    pub fn get_error<T: DeserializeOwned>(&self) -> Result<Option<T>, serde_json::Error> {
        match &self.error {
            Some(v) => serde_json::from_value(v.clone()).map(Some),
            None => Ok(None),
        }
    }
}

/// Map of client responses to server-initiated input requests.
///
/// Keys correspond to the identifiers in the [`InputRequests`](crate::types::mcp::core::mrtr::InputRequests) map;
/// values are the client's results for each request.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#inputresponses>
pub type InputResponses = HashMap<String, InputResponse>;

/// Request parameter type that includes input responses and request state.
///
/// These parameters may be included in any client-initiated request when retrying
/// after receiving an [`InputRequiredResult`](crate::types::mcp::core::mrtr::InputRequiredResult).
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#inputresponserequestparams>
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InputResponseRequestParams {
    /// Protocol-level request metadata.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<RequestMetaObject>,
    /// Responses for the server's input requests from the previous `InputRequiredResult`.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub input_responses: InputResponses,
    /// Request state passed back to the server from the client.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_state: Option<String>,
    /// Additional unrecognized or custom metadata properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extras: HashMap<String, Value>,
}

impl InputResponseRequestParams {
    /// Creates a new empty [`InputResponseRequestParams`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the opaque request state string.
    pub fn with_request_state(mut self, state: impl Into<String>) -> Self {
        self.request_state = Some(state.into());
        self
    }

    /// Sets the map of input responses.
    pub fn with_input_responses(mut self, responses: InputResponses) -> Self {
        self.input_responses = responses;
        self
    }

    /// Adds a single input response with the given identifier.
    pub fn with_input_response(
        mut self,
        id: impl Into<String>,
        response: impl Into<InputResponse>,
    ) -> Self {
        self.input_responses.insert(id.into(), response.into());
        self
    }

    /// Sets request metadata.
    pub fn with_meta(mut self, meta: RequestMetaObject) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Returns the request state string, if present.
    pub fn request_state(&self) -> Option<&str> {
        self.request_state.as_deref()
    }

    /// Returns the input responses map.
    pub fn input_responses(&self) -> &InputResponses {
        &self.input_responses
    }

    /// Retrieves an input response by its identifier.
    pub fn get_response(&self, id: &str) -> Option<&InputResponse> {
        self.input_responses.get(id)
    }
}
