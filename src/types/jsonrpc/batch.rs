// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};

/// A JSON-RPC 2.0 batch request payload containing multiple requests.
pub type JsonRpcBatchRequest<P = Value> = Vec<JsonRpcRequest<P>>;

/// A JSON-RPC 2.0 batch response payload containing multiple responses.
pub type JsonRpcBatchResponse<R = Value, E = JsonRpcError> = Vec<JsonRpcResponse<R, E>>;

/// Represents either a single JSON-RPC element or a batch of elements.
///
/// In JSON-RPC 2.0, requests and responses can arrive or be returned either as a single
/// JSON-RPC object or as a batch array of JSON-RPC objects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcPayload<T> {
    Single(T),
    Batch(Vec<T>),
}

impl<T> JsonRpcPayload<T> {
    /// Creates a single-element payload.
    pub fn single(item: T) -> Self {
        Self::Single(item)
    }

    /// Creates a batch payload.
    pub fn batch(items: Vec<T>) -> Self {
        Self::Batch(items)
    }

    /// Returns `true` if the payload is a batch.
    pub fn is_batch(&self) -> bool {
        matches!(self, Self::Batch(_))
    }

    /// Returns `true` if the payload is a single item.
    pub fn is_single(&self) -> bool {
        matches!(self, Self::Single(_))
    }

    /// Returns the number of items in the payload.
    pub fn len(&self) -> usize {
        match self {
            Self::Single(_) => 1,
            Self::Batch(items) => items.len(),
        }
    }

    /// Returns `true` if the payload contains no items (i.e. an empty batch).
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Single(_) => false,
            Self::Batch(items) => items.is_empty(),
        }
    }

    /// Converts the payload into a vector of items.
    pub fn into_vec(self) -> Vec<T> {
        match self {
            Self::Single(item) => vec![item],
            Self::Batch(items) => items,
        }
    }
}

impl<T> From<Vec<T>> for JsonRpcPayload<T> {
    fn from(items: Vec<T>) -> Self {
        Self::Batch(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::jsonrpc::JsonRpcResultResponse;

    /// Tests serialization and deserialization of single and batch JSON-RPC payloads.
    #[test]
    fn test_jsonrpc_payload_serde() {
        // Single Request
        let single_req = JsonRpcRequest::new(
            1.into(),
            "tools/list",
            Some(serde_json::json!({"cursor": "p1"})),
        );
        let single_payload = JsonRpcPayload::Single(single_req.clone());
        assert!(single_payload.is_single());
        assert!(!single_payload.is_batch());
        assert_eq!(single_payload.len(), 1);
        assert!(!single_payload.is_empty());

        let json = serde_json::to_value(&single_payload).unwrap();
        assert!(json.is_object());
        assert_eq!(json["method"], "tools/list");

        let deserialized: JsonRpcPayload<JsonRpcRequest<serde_json::Value>> =
            serde_json::from_value(json).unwrap();
        assert_eq!(deserialized, single_payload);

        // Batch Request
        let req2 = JsonRpcRequest::new(2.into(), "prompts/list", Some(serde_json::json!({})));
        let batch_payload = JsonRpcPayload::Batch(vec![single_req.clone(), req2.clone()]);
        assert!(batch_payload.is_batch());
        assert!(!batch_payload.is_single());
        assert_eq!(batch_payload.len(), 2);

        let batch_json = serde_json::to_value(&batch_payload).unwrap();
        assert!(batch_json.is_array());
        assert_eq!(batch_json.as_array().unwrap().len(), 2);

        let deserialized_batch: JsonRpcPayload<JsonRpcRequest<serde_json::Value>> =
            serde_json::from_value(batch_json).unwrap();
        assert_eq!(deserialized_batch, batch_payload);
        assert_eq!(deserialized_batch.into_vec().len(), 2);

        // Single and Batch Responses
        let res1: JsonRpcResponse<String, JsonRpcError> =
            JsonRpcResponse::Result(JsonRpcResultResponse::new(1.into(), "res1".to_string()));
        let res2: JsonRpcResponse<String, JsonRpcError> =
            JsonRpcResponse::Result(JsonRpcResultResponse::new(2.into(), "res2".to_string()));

        let res_payload = JsonRpcPayload::batch(vec![res1, res2]);
        let res_json = serde_json::to_value(&res_payload).unwrap();
        assert!(res_json.is_array());
        assert_eq!(res_json[0]["result"], "res1");
        assert_eq!(res_json[1]["result"], "res2");
    }
}
