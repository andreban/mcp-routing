use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcNotification<P = Value> {
    #[serde(default = "super::default_jsonrpc")]
    pub jsonrpc: Cow<'static, str>,
    pub method: Cow<'static, str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<P>,
}

impl<P> JsonRpcNotification<P> {
    pub fn new(method: impl Into<Cow<'static, str>>, params: Option<P>) -> Self {
        Self {
            jsonrpc: Cow::Borrowed("2.0"),
            method: method.into(),
            params,
        }
    }
}
