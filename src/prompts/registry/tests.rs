// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Unit tests for PromptRegistry dispatch and execution.

use std::sync::Arc;
use super::*;
use crate::router::MethodContext;
use crate::types::jsonrpc::JsonRpcRequestId;
use http_body_util::BodyExt;

/// Tests that dispatching `prompts/get` for an unknown prompt returns an invalid params error.
#[tokio::test]
async fn test_prompt_registry_dispatch_get_unknown_prompt_returns_invalid_params() {
    let registry = PromptRegistry::new();
    let headers = http::HeaderMap::new();
    let extensions = Arc::new(http::Extensions::new());
    let ctx = MethodContext {
        req_id: Some(JsonRpcRequestId::Number(42.0)),
        is_notification: false,
        is_batch: false,
        header_name: Some(std::borrow::Cow::Borrowed("non_existent_prompt")),
        headers: &headers,
        extensions,
    };

    let params = serde_json::json!({
        "name": "non_existent_prompt"
    });

    let outcome = registry.dispatch_get(ctx, Some(params)).await;
    let resp = outcome.response.expect("expected error response");
    assert_eq!(
        resp["error"]["code"],
        crate::types::jsonrpc::INVALID_PARAMS_CODE
    );
    assert!(
        resp["error"]["message"]
            .as_str()
            .unwrap()
            .contains("prompt 'non_existent_prompt' not found")
    );
}

/// Tests that `handle_get` for an unknown prompt returns an invalid params error.
#[tokio::test]
async fn test_prompt_registry_handle_get_unknown_prompt_returns_invalid_params() {
    let registry = PromptRegistry::new();
    let headers = http::HeaderMap::new();
    let extensions = Arc::new(http::Extensions::new());
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "prompts/get",
        "params": {
            "name": "non_existent_prompt"
        }
    });
    let body_bytes = serde_json::to_vec(&body).unwrap();

    let response = registry
        .handle_get(
            Some(JsonRpcRequestId::Number(1.0)),
            Some("non_existent_prompt"),
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
            .contains("prompt 'non_existent_prompt' not found")
    );
}
