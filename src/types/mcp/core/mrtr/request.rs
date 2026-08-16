// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Multi Round-Trip Request (MRTR) request types per MCP 2026-07-28 specification (SEP-2322).

use std::collections::HashMap;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::mcp::ResultMetaObject;
use crate::types::mcp::core::mrtr::types::RESULT_TYPE_INPUT_REQUIRED;

/// A server-initiated input request that the client must fulfill before retrying the original request.
///
/// In MCP 2026-07-28 (SEP-2322), this typically represents requests such as:
/// - `sampling/createMessage`: Sampling completions from an LLM connected to the client.
/// - `roots/list`: Requesting the list of root URIs / boundaries from the client.
/// - `elicitation/create`: Requesting structured user confirmation or input.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#inputrequest>
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InputRequest {
    /// The RPC method name to be invoked on the client (e.g. `"sampling/createMessage"`, `"roots/list"`, `"elicitation/create"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// Request parameters payload for the client invocation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    /// Additional unrecognized or custom properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extras: HashMap<String, Value>,
}

impl InputRequest {
    /// Creates a new [`InputRequest`] with the given method name.
    pub fn new(method: impl Into<String>) -> Self {
        Self {
            method: Some(method.into()),
            params: None,
            extras: HashMap::new(),
        }
    }

    /// Creates a new [`InputRequest`] with a method name and serialized parameters.
    pub fn with_params<T: Serialize>(
        method: impl Into<String>,
        params: &T,
    ) -> Result<Self, serde_json::Error> {
        let params_val = serde_json::to_value(params)?;
        Ok(Self {
            method: Some(method.into()),
            params: Some(params_val),
            extras: HashMap::new(),
        })
    }

    /// Creates a sampling input request (`"sampling/createMessage"`).
    pub fn sampling<T: Serialize>(params: &T) -> Result<Self, serde_json::Error> {
        Self::with_params("sampling/createMessage", params)
    }

    /// Creates a roots list input request (`"roots/list"`).
    pub fn roots() -> Self {
        Self::new("roots/list")
    }

    /// Creates an elicitation input request (`"elicitation/create"`).
    pub fn elicitation<T: Serialize>(params: &T) -> Result<Self, serde_json::Error> {
        Self::with_params("elicitation/create", params)
    }

    /// Creates an [`InputRequest`] from arbitrary JSON value.
    pub fn from_value(value: Value) -> Self {
        serde_json::from_value(value).unwrap_or_default()
    }

    /// Returns the method name of the input request, if present.
    pub fn method(&self) -> Option<&str> {
        self.method.as_deref()
    }

    /// Returns the request parameters, if present.
    pub fn params(&self) -> Option<&Value> {
        self.params.as_ref()
    }

    /// Deserializes the request parameters into a typed struct.
    pub fn get_params<T: DeserializeOwned>(&self) -> Result<Option<T>, serde_json::Error> {
        match &self.params {
            Some(v) => serde_json::from_value(v.clone()).map(Some),
            None => Ok(None),
        }
    }
}

/// Map of server-initiated input requests that the client must fulfill.
///
/// Keys are server-assigned identifiers; values are the request objects.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#inputrequests>
pub type InputRequests = HashMap<String, InputRequest>;

/// An `InputRequiredResult` sent by the server to indicate that additional input is needed
/// before the request can be completed.
///
/// In Multi Round-Trip Requests (MRTR - SEP-2322), the server returns this result
/// with `resultType: "input_required"`, one or more `inputRequests`, and/or an opaque `requestState`.
/// The client fulfills the input requests and retries the original request with `inputResponses`
/// and `requestState`.
///
/// At least one of `input_requests` or `request_state` MUST be present.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#inputrequiredresult>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputRequiredResult {
    /// Protocol-level response metadata.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<ResultMetaObject>,
    /// Result type discriminator, which is always `"input_required"`.
    pub result_type: String,
    /// Requests issued by the server that must be completed before the client can retry the original request.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub input_requests: InputRequests,
    /// Request state to be passed back to the server when the client retries the original request.
    /// The client MUST treat this as an opaque string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_state: Option<String>,
    /// Additional unrecognized or custom metadata properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extras: HashMap<String, Value>,
}

impl Default for InputRequiredResult {
    fn default() -> Self {
        Self {
            meta: None,
            result_type: RESULT_TYPE_INPUT_REQUIRED.to_string(),
            input_requests: HashMap::new(),
            request_state: None,
            extras: HashMap::new(),
        }
    }
}

impl InputRequiredResult {
    /// Creates a new [`InputRequiredResult`] with `resultType: "input_required"`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an [`InputRequiredResult`] for load shedding with only an opaque request state.
    pub fn load_shed(request_state: impl Into<String>) -> Self {
        Self::new().with_request_state(request_state)
    }

    /// Sets the opaque request state string.
    pub fn with_request_state(mut self, state: impl Into<String>) -> Self {
        self.request_state = Some(state.into());
        self
    }

    /// Sets an optional request state string.
    pub fn with_optional_request_state(mut self, state: Option<String>) -> Self {
        self.request_state = state;
        self
    }

    /// Sets the map of server input requests.
    pub fn with_input_requests(mut self, requests: InputRequests) -> Self {
        self.input_requests = requests;
        self
    }

    /// Adds a single input request with the given identifier.
    pub fn with_input_request(
        mut self,
        id: impl Into<String>,
        request: impl Into<InputRequest>,
    ) -> Self {
        self.input_requests.insert(id.into(), request.into());
        self
    }

    /// Sets response metadata on this result.
    pub fn with_meta(mut self, meta: ResultMetaObject) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Returns `true` if this result satisfies the MCP specification requirement
    /// that at least one of `input_requests` or `request_state` is present.
    pub fn is_valid(&self) -> bool {
        !self.input_requests.is_empty() || self.request_state.is_some()
    }

    /// Returns the opaque request state string, if present.
    pub fn request_state(&self) -> Option<&str> {
        self.request_state.as_deref()
    }

    /// Returns the input requests map.
    pub fn input_requests(&self) -> &InputRequests {
        &self.input_requests
    }

    /// Retrieves an input request by its identifier.
    pub fn get_input_request(&self, id: &str) -> Option<&InputRequest> {
        self.input_requests.get(id)
    }
}
