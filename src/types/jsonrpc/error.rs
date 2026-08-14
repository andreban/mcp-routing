// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Cow;
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Standard JSON-RPC 2.0 parse error code (-32700).
pub const PARSE_ERROR_CODE: i32 = -32700;

/// Standard JSON-RPC 2.0 invalid request code (-32600).
pub const INVALID_REQUEST_CODE: i32 = -32600;

/// Standard JSON-RPC 2.0 method not found code (-32601).
pub const METHOD_NOT_FOUND_CODE: i32 = -32601;

/// Standard JSON-RPC 2.0 invalid params code (-32602).
pub const INVALID_PARAMS_CODE: i32 = -32602;

/// Standard JSON-RPC 2.0 internal error code (-32603).
pub const INTERNAL_ERROR_CODE: i32 = -32603;

/// Standard JSON-RPC 2.0 error codes and implementation-defined error ranges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JsonRpcErrorCode {
    /// Invalid JSON was received by the server (-32700).
    ParseError,
    /// The JSON sent is not a valid Request object (-32600).
    InvalidRequest,
    /// The method does not exist or is not available (-32601).
    MethodNotFound,
    /// Invalid method parameter(s) (-32602).
    InvalidParams,
    /// Internal JSON-RPC error (-32603).
    InternalError,
    /// Reserved for implementation-defined server-errors (-32000 to -32099).
    ServerError(i32),
    /// Custom application error code outside reserved ranges.
    Custom(i32),
}

impl JsonRpcErrorCode {
    /// Returns the integer code value corresponding to this error variant.
    pub fn code(&self) -> i32 {
        match self {
            Self::ParseError => PARSE_ERROR_CODE,
            Self::InvalidRequest => INVALID_REQUEST_CODE,
            Self::MethodNotFound => METHOD_NOT_FOUND_CODE,
            Self::InvalidParams => INVALID_PARAMS_CODE,
            Self::InternalError => INTERNAL_ERROR_CODE,
            Self::ServerError(code) => *code,
            Self::Custom(code) => *code,
        }
    }

    /// Returns the standard specification message for this error code.
    pub fn default_message(&self) -> &'static str {
        match self {
            Self::ParseError => "Parse error",
            Self::InvalidRequest => "Invalid Request",
            Self::MethodNotFound => "Method not found",
            Self::InvalidParams => "Invalid params",
            Self::InternalError => "Internal error",
            Self::ServerError(_) => "Server error",
            Self::Custom(_) => "Custom error",
        }
    }
}

impl From<i32> for JsonRpcErrorCode {
    fn from(code: i32) -> Self {
        match code {
            PARSE_ERROR_CODE => Self::ParseError,
            INVALID_REQUEST_CODE => Self::InvalidRequest,
            METHOD_NOT_FOUND_CODE => Self::MethodNotFound,
            INVALID_PARAMS_CODE => Self::InvalidParams,
            INTERNAL_ERROR_CODE => Self::InternalError,
            c if (-32099..=-32000).contains(&c) => Self::ServerError(c),
            c => Self::Custom(c),
        }
    }
}

impl From<JsonRpcErrorCode> for i32 {
    fn from(code: JsonRpcErrorCode) -> Self {
        code.code()
    }
}

impl Serialize for JsonRpcErrorCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_i32(self.code())
    }
}

impl<'de> Deserialize<'de> for JsonRpcErrorCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let code = i32::deserialize(deserializer)?;
        Ok(Self::from(code))
    }
}

impl fmt::Display for JsonRpcErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.default_message(), self.code())
    }
}

/// A structured JSON-RPC 2.0 error object.
///
/// Contains an integer error code, a short string message, and optional error data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcError<D = Value> {
    /// A number indicating the error type that occurred.
    pub code: JsonRpcErrorCode,
    /// A short description of the error.
    pub message: Cow<'static, str>,
    /// Additional information about the error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<D>,
}

impl<D> JsonRpcError<D> {
    /// Creates a new [`JsonRpcError`] with the given code, message, and optional data.
    pub fn new(
        code: impl Into<JsonRpcErrorCode>,
        message: impl Into<Cow<'static, str>>,
        data: Option<D>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            data,
        }
    }
}

impl JsonRpcError<Value> {
    /// Constructs a standard Parse Error (-32700).
    pub fn parse_error(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(JsonRpcErrorCode::ParseError, message, None)
    }

    /// Constructs a standard Invalid Request error (-32600).
    pub fn invalid_request(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(JsonRpcErrorCode::InvalidRequest, message, None)
    }

    /// Constructs a standard Method Not Found error (-32601).
    pub fn method_not_found(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(JsonRpcErrorCode::MethodNotFound, message, None)
    }

    /// Constructs a standard Invalid Params error (-32602).
    pub fn invalid_params(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(JsonRpcErrorCode::InvalidParams, message, None)
    }

    /// Constructs a standard Internal Error (-32603).
    pub fn internal_error(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(JsonRpcErrorCode::InternalError, message, None)
    }
}

impl<D: fmt::Debug> fmt::Display for JsonRpcError<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JSON-RPC error {}: {}", self.code.code(), self.message)
    }
}

impl<D: fmt::Debug> std::error::Error for JsonRpcError<D> {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests serialization and deserialization of standard JSON-RPC error codes.
    #[test]
    fn test_error_code_conversions_and_serde() {
        let codes = vec![
            (JsonRpcErrorCode::ParseError, -32700, "Parse error"),
            (JsonRpcErrorCode::InvalidRequest, -32600, "Invalid Request"),
            (JsonRpcErrorCode::MethodNotFound, -32601, "Method not found"),
            (JsonRpcErrorCode::InvalidParams, -32602, "Invalid params"),
            (JsonRpcErrorCode::InternalError, -32603, "Internal error"),
            (
                JsonRpcErrorCode::ServerError(-32050),
                -32050,
                "Server error",
            ),
            (JsonRpcErrorCode::Custom(1001), 1001, "Custom error"),
        ];

        for (variant, int_val, default_msg) in codes {
            assert_eq!(variant.code(), int_val);
            assert_eq!(variant.default_message(), default_msg);
            assert_eq!(JsonRpcErrorCode::from(int_val), variant);
            assert_eq!(i32::from(variant), int_val);

            let json_str = serde_json::to_string(&variant).unwrap();
            assert_eq!(json_str, int_val.to_string());

            let deserialized: JsonRpcErrorCode = serde_json::from_str(&json_str).unwrap();
            assert_eq!(deserialized, variant);
        }
    }

    /// Tests [`JsonRpcError`] constructors and JSON serialization.
    #[test]
    fn test_jsonrpc_error_constructors_and_serde() {
        let err = JsonRpcError::method_not_found("Method unknown");
        let val = serde_json::to_value(&err).unwrap();
        assert_eq!(val["code"], -32601);
        assert_eq!(val["message"], "Method unknown");
        assert!(val.get("data").is_none());

        let err_with_data = JsonRpcError::new(
            JsonRpcErrorCode::InvalidParams,
            "Bad argument",
            Some(serde_json::json!({"field": "name"})),
        );
        let val_with_data = serde_json::to_value(&err_with_data).unwrap();
        assert_eq!(val_with_data["code"], -32602);
        assert_eq!(val_with_data["message"], "Bad argument");
        assert_eq!(val_with_data["data"]["field"], "name");

        let deserialized: JsonRpcError = serde_json::from_value(val_with_data).unwrap();
        assert_eq!(deserialized.code, JsonRpcErrorCode::InvalidParams);
        assert_eq!(deserialized.message, "Bad argument");
        assert_eq!(deserialized.data.unwrap()["field"], "name");
    }
}
