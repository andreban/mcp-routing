// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::jsonrpc::{JsonRpcError, default_jsonrpc};

use super::JsonRpcRequestId;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcResponse<R = Value, E = JsonRpcError> {
    Result(JsonRpcResultResponse<R>),
    Error(JsonRpcErrorResponse<E>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcResultResponse<R = Value> {
    pub jsonrpc: Cow<'static, str>,
    pub id: JsonRpcRequestId,
    pub result: R,
}

impl<R> JsonRpcResultResponse<R> {
    pub fn new(id: JsonRpcRequestId, result: R) -> Self {
        Self {
            jsonrpc: default_jsonrpc(),
            id,
            result,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcErrorResponse<E = JsonRpcError> {
    pub jsonrpc: Cow<'static, str>,
    pub id: Option<JsonRpcRequestId>,
    pub error: E,
}

impl<E> JsonRpcErrorResponse<E> {
    pub fn new(id: Option<JsonRpcRequestId>, error: E) -> Self {
        Self {
            error,
            id,
            jsonrpc: default_jsonrpc(),
        }
    }

    pub fn with_id(id: JsonRpcRequestId, error: E) -> Self {
        Self::new(Some(id), error)
    }
}

impl JsonRpcErrorResponse<JsonRpcError> {
    /// Constructs a standard Parse Error (-32700) response.
    pub fn parse_error(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(None, JsonRpcError::parse_error(message))
    }

    /// Constructs a standard Invalid Request error (-32600) response.
    pub fn invalid_request(
        id: Option<JsonRpcRequestId>,
        message: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self::new(id, JsonRpcError::invalid_request(message))
    }

    /// Constructs a standard Method Not Found error (-32601) response.
    pub fn method_not_found(
        id: Option<JsonRpcRequestId>,
        message: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self::new(id, JsonRpcError::method_not_found(message))
    }

    /// Constructs a standard Invalid Params error (-32602) response.
    pub fn invalid_params(
        id: Option<JsonRpcRequestId>,
        message: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self::new(id, JsonRpcError::invalid_params(message))
    }

    /// Constructs a standard Internal Error (-32603) response.
    pub fn internal_error(
        id: Option<JsonRpcRequestId>,
        message: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self::new(id, JsonRpcError::internal_error(message))
    }
}
