// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Multi Round-Trip Request (MRTR) types per MCP 2026-07-28 specification (SEP-2322).
//!
//! MRTR enables stateless multi round-trip interactions (such as model sampling,
//! user confirmation / elicitation, and filesystem root selection) between client
//! and server without requiring persistent connections.
//!
//! See <https://modelcontextprotocol.io/specification/2026-07-28/schema#inputrequiredresult>

use std::collections::HashMap;
use std::fmt;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::mcp::{RequestMetaObject, ResultMetaObject};

/// Result type discriminator constant for complete results.
pub const RESULT_TYPE_COMPLETE: &str = "complete";

/// Result type discriminator constant for results requiring additional client input.
pub const RESULT_TYPE_INPUT_REQUIRED: &str = "input_required";

/// Indicates the type of a `Result` object, allowing the client to determine how to parse the response.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#resulttype>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultType {
    /// The request completed successfully and contains the final result.
    #[serde(rename = "complete")]
    Complete,
    /// The request requires additional input before it can be completed.
    #[serde(rename = "input_required")]
    InputRequired,
    /// Custom or forward-compatible result type discriminator.
    #[serde(untagged)]
    Custom(String),
}

impl ResultType {
    /// Returns the string representation of the result type.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Complete => RESULT_TYPE_COMPLETE,
            Self::InputRequired => RESULT_TYPE_INPUT_REQUIRED,
            Self::Custom(s) => s.as_str(),
        }
    }

    /// Returns `true` if this is [`ResultType::Complete`].
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Complete)
    }

    /// Returns `true` if this is [`ResultType::InputRequired`].
    pub fn is_input_required(&self) -> bool {
        matches!(self, Self::InputRequired)
    }
}

impl fmt::Display for ResultType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl AsRef<str> for ResultType {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<&str> for ResultType {
    fn from(s: &str) -> Self {
        match s {
            RESULT_TYPE_COMPLETE => Self::Complete,
            RESULT_TYPE_INPUT_REQUIRED => Self::InputRequired,
            other => Self::Custom(other.to_string()),
        }
    }
}

impl From<String> for ResultType {
    fn from(s: String) -> Self {
        match s.as_str() {
            RESULT_TYPE_COMPLETE => Self::Complete,
            RESULT_TYPE_INPUT_REQUIRED => Self::InputRequired,
            _ => Self::Custom(s),
        }
    }
}

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

/// A client response to a server-initiated [`InputRequest`].
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

/// Map of server-initiated input requests that the client must fulfill.
///
/// Keys are server-assigned identifiers; values are the request objects.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#inputrequests>
pub type InputRequests = HashMap<String, InputRequest>;

/// Map of client responses to server-initiated input requests.
///
/// Keys correspond to the identifiers in the [`InputRequests`] map;
/// values are the client's results for each request.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#inputresponses>
pub type InputResponses = HashMap<String, InputResponse>;

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

/// Request parameter type that includes input responses and request state.
///
/// These parameters may be included in any client-initiated request when retrying
/// after receiving an [`InputRequiredResult`].
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_result_type_serde() {
        let rt_complete = ResultType::Complete;
        let serialized = serde_json::to_string(&rt_complete).unwrap();
        assert_eq!(serialized, "\"complete\"");
        let deserialized: ResultType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, ResultType::Complete);
        assert!(deserialized.is_complete());
        assert!(!deserialized.is_input_required());

        let rt_input_req = ResultType::InputRequired;
        let serialized = serde_json::to_string(&rt_input_req).unwrap();
        assert_eq!(serialized, "\"input_required\"");
        let deserialized: ResultType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, ResultType::InputRequired);
        assert!(!deserialized.is_complete());
        assert!(deserialized.is_input_required());

        let rt_custom = ResultType::from("custom_type");
        assert_eq!(rt_custom.as_str(), "custom_type");
    }

    #[test]
    fn test_input_required_result_serde() {
        let sampling_req = InputRequest::sampling(&json!({
            "messages": [{"role": "user", "content": {"type": "text", "text": "Hello"}}]
        }))
        .unwrap();

        let roots_req = InputRequest::roots();

        let result = InputRequiredResult::new()
            .with_request_state("opaque_state_12345")
            .with_input_request("sampling_1", sampling_req)
            .with_input_request("roots_1", roots_req);

        assert!(result.is_valid());
        assert_eq!(result.request_state(), Some("opaque_state_12345"));
        assert_eq!(result.input_requests.len(), 2);

        let json_val = serde_json::to_value(&result).unwrap();
        assert_eq!(json_val["resultType"], "input_required");
        assert_eq!(json_val["requestState"], "opaque_state_12345");
        assert_eq!(
            json_val["inputRequests"]["sampling_1"]["method"],
            "sampling/createMessage"
        );
        assert_eq!(
            json_val["inputRequests"]["roots_1"]["method"],
            "roots/list"
        );

        let deserialized: InputRequiredResult = serde_json::from_value(json_val).unwrap();
        assert_eq!(deserialized.result_type, "input_required");
        assert_eq!(
            deserialized.request_state.as_deref(),
            Some("opaque_state_12345")
        );
        assert_eq!(
            deserialized
                .get_input_request("roots_1")
                .unwrap()
                .method(),
            Some("roots/list")
        );
    }

    #[test]
    fn test_input_required_load_shed_serde() {
        let result = InputRequiredResult::load_shed("state_shed_999");
        assert!(result.is_valid());
        assert_eq!(result.request_state(), Some("state_shed_999"));
        assert!(result.input_requests().is_empty());

        let json_val = serde_json::to_value(&result).unwrap();
        assert_eq!(json_val["resultType"], "input_required");
        assert_eq!(json_val["requestState"], "state_shed_999");
        assert!(json_val.get("inputRequests").is_none());

        let deserialized: InputRequiredResult = serde_json::from_value(json_val).unwrap();
        assert_eq!(deserialized.result_type, "input_required");
        assert_eq!(
            deserialized.request_state.as_deref(),
            Some("state_shed_999")
        );
    }

    #[test]
    fn test_input_response_request_params_serde() {
        let sampling_resp = InputResponse::result(&json!({
            "model": "gemini-2.5-flash",
            "content": {"type": "text", "text": "World"}
        }))
        .unwrap();

        let roots_resp = InputResponse::result(&json!({
            "roots": [{"uri": "file:///workspace", "name": "Workspace"}]
        }))
        .unwrap();

        let params = InputResponseRequestParams::new()
            .with_request_state("opaque_state_12345")
            .with_input_response("sampling_1", sampling_resp)
            .with_input_response("roots_1", roots_resp);

        assert_eq!(params.request_state(), Some("opaque_state_12345"));
        assert_eq!(params.input_responses.len(), 2);

        let json_val = serde_json::to_value(&params).unwrap();
        assert_eq!(json_val["requestState"], "opaque_state_12345");
        assert_eq!(
            json_val["inputResponses"]["sampling_1"]["result"]["model"],
            "gemini-2.5-flash"
        );

        let deserialized: InputResponseRequestParams = serde_json::from_value(json_val).unwrap();
        assert_eq!(
            deserialized.request_state.as_deref(),
            Some("opaque_state_12345")
        );

        let resp = deserialized.get_response("sampling_1").unwrap();
        assert!(!resp.is_error());
        let res_json: Option<Value> = resp.get_result().unwrap();
        assert_eq!(res_json.unwrap()["model"], "gemini-2.5-flash");
    }
}
