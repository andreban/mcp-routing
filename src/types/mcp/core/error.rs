// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Cow;

use http::StatusCode;
use serde::{Deserialize, Serialize};

use super::capabilities::ClientCapabilities;
use crate::types::jsonrpc::{
    INVALID_REQUEST_CODE, JsonRpcError, JsonRpcErrorCode, JsonRpcErrorResponse, JsonRpcRequestId,
    METHOD_NOT_FOUND_CODE, PARSE_ERROR_CODE,
};

/// Error code returned when the HTTP headers of a request do not match the corresponding
/// values in the request body, or required headers are missing or malformed (-32020).
///
/// For HTTP transport, the response status code MUST be `400 Bad Request`.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#headermismatcherror>
pub const HEADER_MISMATCH: i32 = -32020;

/// Error code returned when a server requires a client capability that was not declared
/// in the request's `clientCapabilities` (-32021).
///
/// For HTTP transport, the response status code MUST be `400 Bad Request`.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#missingrequiredclientcapabilityerror>
pub const MISSING_REQUIRED_CLIENT_CAPABILITY: i32 = -32021;

/// Error code returned when the request's protocol version is not supported by the server (-32022).
///
/// For HTTP transport, the response status code MUST be `400 Bad Request`.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#unsupportedprotocolversionerror>
pub const UNSUPPORTED_PROTOCOL_VERSION: i32 = -32022;

/// Data payload carried in an [`UnsupportedProtocolVersionError`].
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#unsupportedprotocolversionerror>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnsupportedProtocolVersionData {
    /// Protocol versions the server supports. The client should choose a mutually supported
    /// version from this list and retry.
    pub supported: Vec<String>,
    /// The protocol version that was requested by the client.
    pub requested: String,
}

impl UnsupportedProtocolVersionData {
    /// Creates a new [`UnsupportedProtocolVersionData`] payload.
    pub fn new(supported: Vec<String>, requested: impl Into<String>) -> Self {
        Self {
            supported,
            requested: requested.into(),
        }
    }

    /// Converts this data payload into a standard untyped [`JsonRpcError`].
    pub fn into_json_rpc_error(
        self,
        message: impl Into<Cow<'static, str>>,
    ) -> JsonRpcError<serde_json::Value> {
        let data = serde_json::to_value(self).ok();
        JsonRpcError::new(
            JsonRpcErrorCode::ServerError(UNSUPPORTED_PROTOCOL_VERSION),
            message,
            data,
        )
    }

    /// Converts this data payload into a typed [`JsonRpcError`].
    pub fn into_typed_json_rpc_error(
        self,
        message: impl Into<Cow<'static, str>>,
    ) -> JsonRpcError<Self> {
        JsonRpcError::new(
            JsonRpcErrorCode::ServerError(UNSUPPORTED_PROTOCOL_VERSION),
            message,
            Some(self),
        )
    }

    /// Converts this data payload into a standard untyped [`JsonRpcErrorResponse`].
    pub fn into_error_response(
        self,
        id: Option<JsonRpcRequestId>,
        message: impl Into<Cow<'static, str>>,
    ) -> JsonRpcErrorResponse {
        JsonRpcErrorResponse::new(id, self.into_json_rpc_error(message))
    }

    /// Converts this data payload into a typed [`JsonRpcErrorResponse`].
    pub fn into_typed_error_response(
        self,
        id: Option<JsonRpcRequestId>,
        message: impl Into<Cow<'static, str>>,
    ) -> JsonRpcErrorResponse<JsonRpcError<Self>> {
        JsonRpcErrorResponse::new(id, self.into_typed_json_rpc_error(message))
    }
}

/// Data payload carried in a [`MissingRequiredClientCapabilityError`].
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#missingrequiredclientcapabilityerror>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingRequiredClientCapabilityData {
    /// The capabilities the server requires from the client to process this request.
    pub required_capabilities: ClientCapabilities,
}

impl MissingRequiredClientCapabilityData {
    /// Creates a new [`MissingRequiredClientCapabilityData`] payload.
    pub fn new(required_capabilities: ClientCapabilities) -> Self {
        Self {
            required_capabilities,
        }
    }

    /// Converts this data payload into a standard untyped [`JsonRpcError`].
    pub fn into_json_rpc_error(
        self,
        message: impl Into<Cow<'static, str>>,
    ) -> JsonRpcError<serde_json::Value> {
        let data = serde_json::to_value(self).ok();
        JsonRpcError::new(
            JsonRpcErrorCode::ServerError(MISSING_REQUIRED_CLIENT_CAPABILITY),
            message,
            data,
        )
    }

    /// Converts this data payload into a typed [`JsonRpcError`].
    pub fn into_typed_json_rpc_error(
        self,
        message: impl Into<Cow<'static, str>>,
    ) -> JsonRpcError<Self> {
        JsonRpcError::new(
            JsonRpcErrorCode::ServerError(MISSING_REQUIRED_CLIENT_CAPABILITY),
            message,
            Some(self),
        )
    }

    /// Converts this data payload into a standard untyped [`JsonRpcErrorResponse`].
    pub fn into_error_response(
        self,
        id: Option<JsonRpcRequestId>,
        message: impl Into<Cow<'static, str>>,
    ) -> JsonRpcErrorResponse {
        JsonRpcErrorResponse::new(id, self.into_json_rpc_error(message))
    }

    /// Converts this data payload into a typed [`JsonRpcErrorResponse`].
    pub fn into_typed_error_response(
        self,
        id: Option<JsonRpcRequestId>,
        message: impl Into<Cow<'static, str>>,
    ) -> JsonRpcErrorResponse<JsonRpcError<Self>> {
        JsonRpcErrorResponse::new(id, self.into_typed_json_rpc_error(message))
    }
}

impl JsonRpcError<serde_json::Value> {
    /// Constructs a standard MCP Header Mismatch error (-32020).
    pub fn header_mismatch(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(
            JsonRpcErrorCode::ServerError(HEADER_MISMATCH),
            message,
            None,
        )
    }

    /// Constructs a standard MCP Unsupported Protocol Version error (-32022) with JSON-serialized data.
    pub fn unsupported_protocol_version(
        message: impl Into<Cow<'static, str>>,
        supported: Vec<String>,
        requested: impl Into<String>,
    ) -> Self {
        UnsupportedProtocolVersionData::new(supported, requested).into_json_rpc_error(message)
    }

    /// Constructs a standard MCP Missing Required Client Capability error (-32021) with JSON-serialized data.
    pub fn missing_required_client_capability(
        message: impl Into<Cow<'static, str>>,
        required_capabilities: ClientCapabilities,
    ) -> Self {
        MissingRequiredClientCapabilityData::new(required_capabilities).into_json_rpc_error(message)
    }
}

impl JsonRpcError<UnsupportedProtocolVersionData> {
    /// Constructs a typed MCP Unsupported Protocol Version error (-32022).
    pub fn unsupported_protocol_version_typed(
        message: impl Into<Cow<'static, str>>,
        supported: Vec<String>,
        requested: impl Into<String>,
    ) -> Self {
        UnsupportedProtocolVersionData::new(supported, requested).into_typed_json_rpc_error(message)
    }
}

impl JsonRpcError<MissingRequiredClientCapabilityData> {
    /// Constructs a typed MCP Missing Required Client Capability error (-32021).
    pub fn missing_required_client_capability_typed(
        message: impl Into<Cow<'static, str>>,
        required_capabilities: ClientCapabilities,
    ) -> Self {
        MissingRequiredClientCapabilityData::new(required_capabilities)
            .into_typed_json_rpc_error(message)
    }
}

impl JsonRpcErrorResponse<JsonRpcError<serde_json::Value>> {
    /// Constructs a standard MCP Header Mismatch error (-32020) response.
    pub fn mcp_header_mismatch(
        id: Option<JsonRpcRequestId>,
        message: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self::new(id, JsonRpcError::header_mismatch(message))
    }

    /// Constructs a standard MCP Unsupported Protocol Version error (-32022) response.
    pub fn mcp_unsupported_protocol_version(
        id: Option<JsonRpcRequestId>,
        message: impl Into<Cow<'static, str>>,
        supported: Vec<String>,
        requested: impl Into<String>,
    ) -> Self {
        UnsupportedProtocolVersionData::new(supported, requested).into_error_response(id, message)
    }

    /// Constructs a standard MCP Missing Required Client Capability error (-32021) response.
    pub fn mcp_missing_required_client_capability(
        id: Option<JsonRpcRequestId>,
        message: impl Into<Cow<'static, str>>,
        required_capabilities: ClientCapabilities,
    ) -> Self {
        MissingRequiredClientCapabilityData::new(required_capabilities)
            .into_error_response(id, message)
    }
}

impl JsonRpcErrorResponse<JsonRpcError<UnsupportedProtocolVersionData>> {
    /// Constructs a typed MCP Unsupported Protocol Version error (-32022) response.
    pub fn unsupported_protocol_version_typed(
        id: Option<JsonRpcRequestId>,
        message: impl Into<Cow<'static, str>>,
        supported: Vec<String>,
        requested: impl Into<String>,
    ) -> Self {
        UnsupportedProtocolVersionData::new(supported, requested)
            .into_typed_error_response(id, message)
    }
}

impl JsonRpcErrorResponse<JsonRpcError<MissingRequiredClientCapabilityData>> {
    /// Constructs a typed MCP Missing Required Client Capability error (-32021) response.
    pub fn missing_required_client_capability_typed(
        id: Option<JsonRpcRequestId>,
        message: impl Into<Cow<'static, str>>,
        required_capabilities: ClientCapabilities,
    ) -> Self {
        MissingRequiredClientCapabilityData::new(required_capabilities)
            .into_typed_error_response(id, message)
    }
}

/// Constructs a standard MCP Header Mismatch error (-32020) response.
pub fn header_mismatch_error(
    id: Option<JsonRpcRequestId>,
    message: impl Into<Cow<'static, str>>,
) -> JsonRpcErrorResponse {
    JsonRpcErrorResponse::mcp_header_mismatch(id, message)
}

/// Constructs a standard MCP Unsupported Protocol Version error (-32022) response.
pub fn unsupported_protocol_version_error(
    id: Option<JsonRpcRequestId>,
    message: impl Into<Cow<'static, str>>,
    supported: Vec<String>,
    requested: impl Into<String>,
) -> JsonRpcErrorResponse {
    JsonRpcErrorResponse::mcp_unsupported_protocol_version(id, message, supported, requested)
}

/// Constructs a standard MCP Missing Required Client Capability error (-32021) response.
pub fn missing_required_client_capability_error(
    id: Option<JsonRpcRequestId>,
    message: impl Into<Cow<'static, str>>,
    required_capabilities: ClientCapabilities,
) -> JsonRpcErrorResponse {
    JsonRpcErrorResponse::mcp_missing_required_client_capability(id, message, required_capabilities)
}

/// Maps a JSON-RPC or MCP error code to the corresponding HTTP status code
/// according to the MCP Streamable HTTP transport specification.
///
/// Status code mappings:
/// - Method Not Found (`-32601`) -> `404 Not Found`
/// - Parse Error (`-32700`), Invalid Request (`-32600`), Header Mismatch (`-32020`),
///   Missing Required Capability (`-32021`), Unsupported Protocol Version (`-32022`) -> `400 Bad Request`
/// - Application / standard JSON-RPC results and errors -> `200 OK`
pub fn mcp_error_code_to_http_status(code: i32) -> StatusCode {
    match code {
        METHOD_NOT_FOUND_CODE => StatusCode::NOT_FOUND,
        PARSE_ERROR_CODE
        | INVALID_REQUEST_CODE
        | HEADER_MISMATCH
        | MISSING_REQUIRED_CLIENT_CAPABILITY
        | UNSUPPORTED_PROTOCOL_VERSION => StatusCode::BAD_REQUEST,
        _ => StatusCode::OK,
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for standard MCP error codes, typed payloads, and helper constructors.

    use super::*;
    use crate::types::jsonrpc::{INTERNAL_ERROR_CODE, INVALID_PARAMS_CODE};

    /// Tests mapping MCP and JSON-RPC error codes to appropriate HTTP status codes.
    #[test]
    fn test_mcp_error_code_to_http_status() {
        assert_eq!(
            mcp_error_code_to_http_status(METHOD_NOT_FOUND_CODE),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            mcp_error_code_to_http_status(PARSE_ERROR_CODE),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            mcp_error_code_to_http_status(INVALID_REQUEST_CODE),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            mcp_error_code_to_http_status(HEADER_MISMATCH),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            mcp_error_code_to_http_status(MISSING_REQUIRED_CLIENT_CAPABILITY),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            mcp_error_code_to_http_status(UNSUPPORTED_PROTOCOL_VERSION),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            mcp_error_code_to_http_status(INVALID_PARAMS_CODE),
            StatusCode::OK
        );
        assert_eq!(
            mcp_error_code_to_http_status(INTERNAL_ERROR_CODE),
            StatusCode::OK
        );
    }

    /// Tests constructors and JSON serialization of untyped MCP error helpers.
    #[test]
    fn test_mcp_error_constructors_and_serde() {
        let mismatch = header_mismatch_error(Some("req-1".into()), "Header mismatch");
        assert_eq!(mismatch.error.code.code(), HEADER_MISMATCH);
        assert_eq!(mismatch.error.message, "Header mismatch");

        let err = JsonRpcError::header_mismatch("Custom mismatch");
        assert_eq!(err.code.code(), HEADER_MISMATCH);
        assert_eq!(err.message, "Custom mismatch");
        assert!(err.data.is_none());

        let unsupported = unsupported_protocol_version_error(
            Some(1.into()),
            "Unsupported protocol version",
            vec!["2026-07-28".to_string()],
            "2024-11-05",
        );
        assert_eq!(unsupported.error.code.code(), UNSUPPORTED_PROTOCOL_VERSION);
        let data = unsupported.error.data.unwrap();
        assert_eq!(data["supported"][0], "2026-07-28");
        assert_eq!(data["requested"], "2024-11-05");

        let missing_cap = missing_required_client_capability_error(
            Some("req-cap".into()),
            "Missing sampling capability",
            ClientCapabilities {
                experimental: None,
                sampling: Some(crate::types::mcp::SamplingCapability {}),
                elicitation: None,
                roots: None,
                extensions: None,
            },
        );
        assert_eq!(
            missing_cap.error.code.code(),
            MISSING_REQUIRED_CLIENT_CAPABILITY
        );
        let cap_data = missing_cap.error.data.unwrap();
        assert!(cap_data["requiredCapabilities"]["sampling"].is_object());
    }

    /// Tests typed error structures for unsupported protocol version and missing client capability.
    #[test]
    fn test_typed_mcp_error_constructors() {
        let typed_unsupported = JsonRpcError::unsupported_protocol_version_typed(
            "Version not supported",
            vec!["2026-07-28".to_string()],
            "2024-11-05",
        );
        assert_eq!(typed_unsupported.code.code(), UNSUPPORTED_PROTOCOL_VERSION);
        let payload = typed_unsupported.data.unwrap();
        assert_eq!(payload.supported, vec!["2026-07-28".to_string()]);
        assert_eq!(payload.requested, "2024-11-05");

        let typed_resp = JsonRpcErrorResponse::unsupported_protocol_version_typed(
            Some(42.into()),
            "Version not supported",
            vec!["2026-07-28".to_string()],
            "2024-11-05",
        );
        assert_eq!(typed_resp.id, Some(42.into()));
        assert_eq!(typed_resp.error.code.code(), UNSUPPORTED_PROTOCOL_VERSION);

        let typed_cap_resp = JsonRpcErrorResponse::missing_required_client_capability_typed(
            Some("cap-req".into()),
            "Missing capability",
            ClientCapabilities {
                experimental: None,
                sampling: None,
                elicitation: Some(crate::types::mcp::ElicitationCapability {}),
                roots: None,
                extensions: None,
            },
        );
        assert_eq!(
            typed_cap_resp.error.code.code(),
            MISSING_REQUIRED_CLIENT_CAPABILITY
        );
        let cap_payload = typed_cap_resp.error.data.unwrap();
        assert!(cap_payload.required_capabilities.elicitation.is_some());
    }

    /// Tests [`UnsupportedProtocolVersionData`] conversion methods to typed and untyped errors and responses.
    #[test]
    fn test_unsupported_protocol_version_data_helpers() {
        let data =
            UnsupportedProtocolVersionData::new(vec!["2026-07-28".to_string()], "2024-11-05");

        let err = data.clone().into_json_rpc_error("Unsupported version");
        assert_eq!(err.code.code(), UNSUPPORTED_PROTOCOL_VERSION);
        assert_eq!(err.message, "Unsupported version");
        let val = err.data.unwrap();
        assert_eq!(val["supported"][0], "2026-07-28");
        assert_eq!(val["requested"], "2024-11-05");

        let typed_err = data
            .clone()
            .into_typed_json_rpc_error("Unsupported version");
        assert_eq!(typed_err.code.code(), UNSUPPORTED_PROTOCOL_VERSION);
        let typed_data = typed_err.data.unwrap();
        assert_eq!(typed_data.supported, vec!["2026-07-28".to_string()]);
        assert_eq!(typed_data.requested, "2024-11-05");

        let resp = data
            .clone()
            .into_error_response(Some(10.into()), "Unsupported version");
        assert_eq!(resp.id, Some(10.into()));
        assert_eq!(resp.error.code.code(), UNSUPPORTED_PROTOCOL_VERSION);

        let typed_resp = data.into_typed_error_response(Some(11.into()), "Unsupported version");
        assert_eq!(typed_resp.id, Some(11.into()));
        assert_eq!(typed_resp.error.code.code(), UNSUPPORTED_PROTOCOL_VERSION);
    }

    /// Tests [`MissingRequiredClientCapabilityData`] conversion methods to typed and untyped errors and responses.
    #[test]
    fn test_missing_required_client_capability_data_helpers() {
        let data = MissingRequiredClientCapabilityData::new(ClientCapabilities {
            experimental: None,
            sampling: Some(crate::types::mcp::SamplingCapability {}),
            elicitation: None,
            roots: None,
            extensions: None,
        });

        let err = data.clone().into_json_rpc_error("Missing capability");
        assert_eq!(err.code.code(), MISSING_REQUIRED_CLIENT_CAPABILITY);
        let val = err.data.unwrap();
        assert!(val["requiredCapabilities"]["sampling"].is_object());

        let typed_err = data.clone().into_typed_json_rpc_error("Missing capability");
        assert_eq!(typed_err.code.code(), MISSING_REQUIRED_CLIENT_CAPABILITY);
        assert!(
            typed_err
                .data
                .unwrap()
                .required_capabilities
                .sampling
                .is_some()
        );

        let resp = data
            .clone()
            .into_error_response(Some("cap-id".into()), "Missing capability");
        assert_eq!(resp.id, Some("cap-id".into()));
        assert_eq!(resp.error.code.code(), MISSING_REQUIRED_CLIENT_CAPABILITY);

        let typed_resp =
            data.into_typed_error_response(Some("cap-id-2".into()), "Missing capability");
        assert_eq!(typed_resp.id, Some("cap-id-2".into()));
        assert_eq!(
            typed_resp.error.code.code(),
            MISSING_REQUIRED_CLIENT_CAPABILITY
        );
    }
}
