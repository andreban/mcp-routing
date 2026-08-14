use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::jsonrpc::default_jsonrpc;

use super::JsonRpcRequestId;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcRequest<P = Value> {
    #[serde(default = "super::default_jsonrpc")]
    pub jsonrpc: Cow<'static, str>,
    pub id: JsonRpcRequestId,
    pub method: Cow<'static, str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<P>,
}

impl<P> JsonRpcRequest<P> {
    pub fn new(
        id: JsonRpcRequestId,
        method: impl Into<Cow<'static, str>>,
        params: Option<P>,
    ) -> Self {
        Self {
            jsonrpc: default_jsonrpc(),
            id,
            method: method.into(),
            params,
        }
    }
}
