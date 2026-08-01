use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::mcp::JsonRpcId;

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcResponse<R = Value, E = Value> {
    Result(JsonRpcResultResponse<R>),
    Error(JsonRpcErrorResponse<E>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcResultResponse<R = Value> {
    pub jsonrpc: Cow<'static, str>,
    pub id: JsonRpcId,
    pub result: R,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcErrorResponse<E = Value> {
    pub jsonrpc: Cow<'static, str>,
    pub id: JsonRpcId,
    pub error: E,
}
