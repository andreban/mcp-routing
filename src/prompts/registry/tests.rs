// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Unit tests for PromptRegistry dispatch and execution.

use super::*;
use crate::router::MethodContext;
use crate::types::jsonrpc::JsonRpcRequestId;
use std::sync::Arc;

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
