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

/// Data payload carried in a [`MissingRequiredClientCapabilityError`].
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#missingrequiredclientcapabilityerror>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingRequiredClientCapabilityData {
    /// The capabilities the server requires from the client to process this request.
    pub required_capabilities: ClientCapabilities,
}

/// Constructs a standard MCP Header Mismatch error (-32020) response.
pub fn header_mismatch_error(
    id: Option<JsonRpcRequestId>,
    message: impl Into<Cow<'static, str>>,
) -> JsonRpcErrorResponse {
    JsonRpcErrorResponse::new(
        id,
        JsonRpcError::new(JsonRpcErrorCode::ServerError(HEADER_MISMATCH), message, None),
    )
}

/// Constructs a standard MCP Unsupported Protocol Version error (-32022) response.
pub fn unsupported_protocol_version_error(
    id: Option<JsonRpcRequestId>,
    message: impl Into<Cow<'static, str>>,
    supported: Vec<String>,
    requested: impl Into<String>,
) -> JsonRpcErrorResponse {
    let data = serde_json::to_value(UnsupportedProtocolVersionData {
        supported,
        requested: requested.into(),
    })
    .ok();

    JsonRpcErrorResponse::new(
        id,
        JsonRpcError::new(
            JsonRpcErrorCode::ServerError(UNSUPPORTED_PROTOCOL_VERSION),
            message,
            data,
        ),
    )
}

/// Constructs a standard MCP Missing Required Client Capability error (-32021) response.
pub fn missing_required_client_capability_error(
    id: Option<JsonRpcRequestId>,
    message: impl Into<Cow<'static, str>>,
    required_capabilities: ClientCapabilities,
) -> JsonRpcErrorResponse {
    let data = serde_json::to_value(MissingRequiredClientCapabilityData {
        required_capabilities,
    })
    .ok();

    JsonRpcErrorResponse::new(
        id,
        JsonRpcError::new(
            JsonRpcErrorCode::ServerError(MISSING_REQUIRED_CLIENT_CAPABILITY),
            message,
            data,
        ),
    )
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
    use super::*;
    use crate::types::jsonrpc::{INTERNAL_ERROR_CODE, INVALID_PARAMS_CODE};

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

    #[test]
    fn test_mcp_error_constructors_and_serde() {
        let mismatch = header_mismatch_error(Some("req-1".into()), "Header mismatch");
        assert_eq!(mismatch.error.code.code(), HEADER_MISMATCH);
        assert_eq!(mismatch.error.message, "Header mismatch");

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
    }
}
