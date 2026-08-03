use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::jsonrpc::default_jsonrpc;

use super::JsonRpcRequestId;

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcResponse<R = Value, E = Value> {
    Result(JsonRpcResultResponse<R>),
    Error(JsonRpcErrorResponse<E>),
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcErrorResponse<E = Value> {
    pub jsonrpc: Cow<'static, str>,
    pub id: JsonRpcRequestId,
    pub error: E,
}

impl<E> JsonRpcErrorResponse<E> {
    pub fn new(id: JsonRpcRequestId, error: E) -> Self {
        Self {
            error,
            id,
            jsonrpc: default_jsonrpc(),
        }
    }
}
