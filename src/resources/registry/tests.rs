// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Unit tests for ResourceRegistry dispatch and execution.

use super::*;
use crate::router::MethodContext;
use crate::types::jsonrpc::JsonRpcRequestId;
use std::sync::Arc;

/// Tests that dispatching `resources/read` for an unknown resource returns an invalid params error.
#[tokio::test]
async fn test_resource_registry_dispatch_read_unknown_resource_returns_invalid_params() {
    let registry = ResourceRegistry::new();
    let headers = http::HeaderMap::new();
    let extensions = Arc::new(http::Extensions::new());
    let ctx = MethodContext {
        req_id: Some(JsonRpcRequestId::Number(42)),
        header_name: Some("file:///non_existent.txt".into()),
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
