// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Unit tests for ResourceRegistry dispatch and execution.

use std::sync::Arc;
use super::*;
use crate::router::MethodContext;
use crate::types::jsonrpc::JsonRpcRequestId;
use http_body_util::BodyExt;

/// Tests that dispatching `resources/read` for an unknown resource returns an invalid params error.
#[tokio::test]
async fn test_resource_registry_dispatch_read_unknown_resource_returns_invalid_params() {
    let registry = ResourceRegistry::new();
    let mut headers = http::HeaderMap::new();
    headers.insert("mcp-uri", "file:///non_existent.txt".parse().unwrap());
    let extensions = Arc::new(http::Extensions::new());
    let ctx = MethodContext {
        req_id: Some(JsonRpcRequestId::Number(42.0)),
        is_notification: false,
        is_batch: false,
        header_name: None,
        headers: &headers,
        extensions,
    };

    let params = serde_json::json!({
        "uri": "file:///non_existent.txt"
    });

    let outcome = registry.dispatch_read(ctx, Some(params)).await;
    let resp = outcome.response.expect("expected error response");
    assert_eq!(
        resp["error"]["code"],
        crate::types::jsonrpc::INVALID_PARAMS_CODE
    );
    assert!(
        resp["error"]["message"]
            .as_str()
            .unwrap()
            .contains("resource 'file:///non_existent.txt' not found")
    );
}

/// Tests that `handle_read` for an unknown resource returns an invalid params error.
#[tokio::test]
async fn test_resource_registry_handle_read_unknown_resource_returns_invalid_params() {
    let registry = ResourceRegistry::new();
    let headers = http::HeaderMap::new();
    let extensions = Arc::new(http::Extensions::new());
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "resources/read",
        "params": {
            "uri": "file:///non_existent.txt"
        }
    });
    let body_bytes = serde_json::to_vec(&body).unwrap();

    let response = registry
        .handle_read(
            Some(JsonRpcRequestId::Number(1.0)),
            Some("file:///non_existent.txt"),
            &headers,
            extensions,
            &body_bytes,
        )
        .await;

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let err_resp: JsonRpcErrorResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        err_resp.error.code.code(),
        crate::types::jsonrpc::INVALID_PARAMS_CODE
    );
    assert!(
        err_resp
            .error
            .message
            .contains("resource 'file:///non_existent.txt' not found")
    );
}
