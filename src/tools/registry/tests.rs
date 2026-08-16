// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Unit tests for ToolRegistry dispatch and execution.

use std::sync::Arc;
use super::*;
use crate::router::MethodContext;
use crate::types::jsonrpc::JsonRpcRequestId;
use http_body_util::BodyExt;

/// Tests that dispatching `tools/call` with an unknown tool returns an invalid params error.
#[tokio::test]
async fn test_tool_registry_dispatch_call_unknown_tool_returns_invalid_params() {
    let registry = ToolRegistry::new();
    let headers = http::HeaderMap::new();
    let extensions = Arc::new(http::Extensions::new());
    let ctx = MethodContext {
        req_id: Some(JsonRpcRequestId::Number(42.0)),
        is_notification: false,
        is_batch: false,
        header_name: Some(std::borrow::Cow::Borrowed("non_existent_tool")),
        headers: &headers,
        extensions,
    };

    let params = serde_json::json!({
        "name": "non_existent_tool"
    });

    let outcome = registry.dispatch_call(ctx, Some(params)).await;
    let resp = outcome.response.expect("expected error response");
    assert_eq!(
        resp["error"]["code"],
        crate::types::jsonrpc::INVALID_PARAMS_CODE
    );
    assert!(
        resp["error"]["message"]
            .as_str()
            .unwrap()
            .contains("tool 'non_existent_tool' not found")
    );
}

/// Tests that `handle_call` with an unknown tool returns an invalid params error.
#[tokio::test]
async fn test_tool_registry_handle_call_unknown_tool_returns_invalid_params() {
    let registry = ToolRegistry::new();
    let headers = http::HeaderMap::new();
    let extensions = Arc::new(http::Extensions::new());
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "non_existent_tool"
        }
    });
    let body_bytes = serde_json::to_vec(&body).unwrap();

    let response = registry
        .handle_call(
            Some(JsonRpcRequestId::Number(1.0)),
            Some("non_existent_tool"),
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
            .contains("tool 'non_existent_tool' not found")
    );
}
