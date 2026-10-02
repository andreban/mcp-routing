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
/// In MCP 2026-07-28 (SEP-2322), this is the client's result for the requested input itself:
/// a `CreateMessageResult`, `ListRootsResult`, or `ElicitResult`. It is not wrapped in any envelope.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#inputresponse>
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct InputResponse(Value);

impl InputResponse {
    /// Creates an [`InputResponse`] by serializing the client's result.
    pub fn result<T: Serialize>(value: &T) -> Result<Self, serde_json::Error> {
        serde_json::to_value(value).map(Self)
    }

    /// Creates an [`InputResponse`] from an arbitrary JSON value.
    pub fn from_value(value: Value) -> Self {
        Self(value)
    }

    /// Deserializes the client's result into a typed struct.
    pub fn get_result<T: DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_value(self.0.clone())
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
