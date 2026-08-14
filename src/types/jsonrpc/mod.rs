// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

mod batch;
mod error;
mod notification;
mod request;
mod response;

use std::{borrow::Cow, fmt::Debug};

use serde::{Deserialize, Serialize};

pub use batch::{JsonRpcBatchRequest, JsonRpcBatchResponse, JsonRpcPayload};
pub use error::{
    INTERNAL_ERROR_CODE, INVALID_PARAMS_CODE, INVALID_REQUEST_CODE, METHOD_NOT_FOUND_CODE,
    PARSE_ERROR_CODE, JsonRpcError, JsonRpcErrorCode,
};
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
pub enum JsonRpcMessage<P = (), N = (), R = (), E = JsonRpcError> {
    Request(JsonRpcRequest<P>),
    Notification(JsonRpcNotification<N>),
    Response(JsonRpcResponse<R, E>),
}

pub fn default_jsonrpc() -> Cow<'static, str> {
    Cow::Borrowed("2.0")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests conversion from various string and numeric primitive types into [`JsonRpcRequestId`].
    #[test]
    fn test_jsonrpc_request_id_conversions() {
        let id_str: JsonRpcRequestId = "abc".into();
        assert_eq!(id_str, JsonRpcRequestId::String("abc".to_string()));

        let id_string: JsonRpcRequestId = "hello".to_string().into();
        assert_eq!(id_string, JsonRpcRequestId::String("hello".to_string()));

        let id_i32: JsonRpcRequestId = 42_i32.into();
        assert_eq!(id_i32, JsonRpcRequestId::Number(42.0));

        let id_i64: JsonRpcRequestId = 1000_i64.into();
        assert_eq!(id_i64, JsonRpcRequestId::Number(1000.0));

        let id_u64: JsonRpcRequestId = 2000_u64.into();
        assert_eq!(id_u64, JsonRpcRequestId::Number(2000.0));

        let id_f64: JsonRpcRequestId = 3.5_f64.into();
        assert_eq!(id_f64, JsonRpcRequestId::Number(3.5));

        let serialized = serde_json::to_string(&id_str).unwrap();
        assert_eq!(serialized, "\"abc\"");

        let serialized_num = serde_json::to_string(&id_i32).unwrap();
        assert_eq!(serialized_num, "42.0");
    }

    /// Tests JSON-RPC 2.0 message envelope structures ([`JsonRpcRequest`], [`JsonRpcResultResponse`], [`JsonRpcErrorResponse`], [`JsonRpcNotification`], [`JsonRpcMessage`]).
    #[test]
    fn test_jsonrpc_messages_serde() {
        // Request
        let req = JsonRpcRequest::new(1.into(), "tools/list", Some(()));
        let req_json = serde_json::to_value(&req).unwrap();
        assert_eq!(req_json["jsonrpc"], "2.0");
        assert_eq!(req_json["method"], "tools/list");
        assert_eq!(req_json["id"], 1.0);

        // Result Response
        let res = JsonRpcResultResponse::new(1.into(), "success_result");
        let res_json = serde_json::to_value(&res).unwrap();
        assert_eq!(res_json["jsonrpc"], "2.0");
        assert_eq!(res_json["id"], 1.0);
        assert_eq!(res_json["result"], "success_result");

        // Error Response with numeric ID
        let err_resp = JsonRpcErrorResponse::with_id(
            1.into(),
            serde_json::json!({
                "code": -32601,
                "message": "Method not found"
            }),
        );
        let err_json = serde_json::to_value(&err_resp).unwrap();
        assert_eq!(err_json["jsonrpc"], "2.0");
        assert_eq!(err_json["id"], 1.0);
        assert_eq!(err_json["error"]["code"], -32601);

        // Error Response with null ID (e.g. Parse Error)
        let parse_err_resp = JsonRpcErrorResponse::parse_error("Invalid syntax");
        let parse_err_json = serde_json::to_value(&parse_err_resp).unwrap();
        assert_eq!(parse_err_json["jsonrpc"], "2.0");
        assert_eq!(parse_err_json["id"], serde_json::Value::Null);
        assert_eq!(parse_err_json["error"]["code"], -32700);

        // Notification
        let notif = JsonRpcNotification::new(
            "notifications/message",
            Some(serde_json::json!({"text": "hi"})),
        );
        let notif_json = serde_json::to_value(&notif).unwrap();
        assert_eq!(notif_json["jsonrpc"], "2.0");
        assert_eq!(notif_json["method"], "notifications/message");
        assert!(notif_json.get("id").is_none());

        // Untagged JsonRpcMessage
        let msg_req: JsonRpcMessage<serde_json::Value, (), (), JsonRpcError> =
            serde_json::from_value(req_json).unwrap();
        assert!(matches!(msg_req, JsonRpcMessage::Request(_)));

        let msg_notif: JsonRpcMessage<(), serde_json::Value, (), JsonRpcError> =
            serde_json::from_value(notif_json).unwrap();
        assert!(matches!(msg_notif, JsonRpcMessage::Notification(_)));

        let msg_resp: JsonRpcMessage<(), (), String, serde_json::Value> =
            serde_json::from_value(res_json).unwrap();
        assert!(matches!(msg_resp, JsonRpcMessage::Response(_)));

        let msg_err: JsonRpcMessage<(), (), (), JsonRpcError> =
            serde_json::from_value(parse_err_json).unwrap();
        assert!(matches!(msg_err, JsonRpcMessage::Response(_)));
    }

    /// Tests the [`default_jsonrpc`] helper constant.
    #[test]
    fn test_default_jsonrpc_helper() {
        assert_eq!(default_jsonrpc(), Cow::Borrowed("2.0"));
    }
}
