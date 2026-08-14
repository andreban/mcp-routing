mod notification;
mod request;
mod response;

use std::{borrow::Cow, fmt::Debug};

use serde::{Deserialize, Serialize};

pub use notification::JsonRpcNotification;
pub use request::JsonRpcRequest;
pub use response::{JsonRpcErrorResponse, JsonRpcResponse, JsonRpcResultResponse};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcRequestId {
    String(String),
    Number(f64),
}

impl From<String> for JsonRpcRequestId {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<&str> for JsonRpcRequestId {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<i32> for JsonRpcRequestId {
    fn from(n: i32) -> Self {
        Self::Number(n as f64)
    }
}

impl From<i64> for JsonRpcRequestId {
    fn from(n: i64) -> Self {
        Self::Number(n as f64)
    }
}

impl From<u64> for JsonRpcRequestId {
    fn from(n: u64) -> Self {
        Self::Number(n as f64)
    }
}

impl From<f64> for JsonRpcRequestId {
    fn from(n: f64) -> Self {
        Self::Number(n)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcMessage<P = (), N = (), R = (), E = ()> {
    Request(JsonRpcRequest<P>),
    Notification(JsonRpcNotification<N>),
    Response(JsonRpcResponse<R, E>),
}

pub fn default_jsonrpc() -> Cow<'static, str> {
    Cow::Borrowed("2.0")
}
