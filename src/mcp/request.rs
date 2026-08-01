use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::mcp::JsonRpcId;

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcRequest<P = Value> {
    #[serde(default = "super::default_jsonrpc")]
    pub jsonrpc: Cow<'static, str>,
    pub id: JsonRpcId,
    pub method: Cow<'static, str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<P>,
}

impl<P> JsonRpcRequest<P> {
    pub fn new(id: JsonRpcId, method: impl Into<Cow<'static, str>>, params: Option<P>) -> Self {
        Self {
            jsonrpc: Cow::Borrowed("2.0"),
            id,
            method: method.into(),
            params,
        }
    }
}
