mod notification;
mod request;
mod response;

use std::{borrow::Cow, fmt::Debug};

use serde::{Deserialize, Serialize};

pub use notification::JsonRpcNotification;
pub use request::JsonRpcRequest;
pub use response::{JsonRpcErrorResponse, JsonRpcResponse, JsonRpcResultResponse};

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcRequestId {
    String(String),
    Number(f64),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcMessage<P = (), N = (), R = (), E = ()> {
    Request(JsonRpcRequest<P>),
    Notification(JsonRpcNotification<N>),
    Response(JsonRpcResponse<R, E>),
}

pub fn default_jsonrpc() -> Cow<'static, str> {
    Cow::Borrowed("2.0")
}
