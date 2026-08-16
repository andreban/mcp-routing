// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Tools subsystem for defining and handling MCP tool invocations.

use crate::types::jsonrpc::{JsonRpcErrorResponse, JsonRpcRequestId};

pub mod handler;
pub mod list;
pub mod registry;
pub mod result;

pub use crate::extract::Json;
pub use handler::{IntoToolHandler, ToolHandler};
pub use list::{IntoToolsListHandler, IntoToolsListResult, ToolsListHandler};
pub use registry::ToolRegistry;
pub use result::IntoToolResult;

/// Error type encountered during tool execution or tool listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolError {
    /// Invalid arguments provided to the tool or listing handler.
    InvalidParams(String),
    /// Internal execution or business logic error.
    Internal(String),
}

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolError::InvalidParams(msg) => write!(f, "Invalid params: {msg}"),
            ToolError::Internal(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for ToolError {}

impl ToolError {
    /// Converts this error into a standard JSON-RPC error response.
    pub fn into_error_response(self, id: Option<JsonRpcRequestId>) -> JsonRpcErrorResponse {
        match self {
            ToolError::InvalidParams(err) => {
                JsonRpcErrorResponse::invalid_params(id, format!("Invalid params: {err}"))
            }
            ToolError::Internal(err) => JsonRpcErrorResponse::internal_error(id, err),
        }
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for tool error conversions and handler adaptations.

    use std::sync::Arc;
    use super::*;
    use crate::extract::{Extension, Meta, RequestContext};
    use crate::types::mcp::{ContentBlock, Implementation};

    /// Tests tool handler invocation with context extractors and deserialized arguments.
    #[tokio::test]
    async fn test_tool_handler_with_extractors_and_args() {
        #[derive(serde::Deserialize)]
        struct EchoParams {
            message: String,
        }

        #[derive(Clone)]
        struct AppState {
            prefix: String,
        }

        async fn echo_handler(
            Extension(state): Extension<AppState>,
            Meta(meta): Meta,
            params: EchoParams,
        ) -> Result<String, String> {
            let client = meta
                .client_info
                .as_ref()
                .map(|c| c.name.as_str())
                .unwrap_or("unknown");
            Ok(format!(
                "{}: {client} -> {}",
                state.prefix, params.message
            ))
        }

        let handler = echo_handler.into_tool_handler();

        let mut ext = http::Extensions::new();
        ext.insert(AppState {
            prefix: "APP".to_string(),
        });

        let ctx = RequestContext::new(
            Some(crate::types::mcp::RequestMetaObject {
                client_info: Some(Implementation::new("test-client", "1.0.0")),
                client_capabilities: None,
                protocol_version: None,
                progress_token: None,
                log_level: None,
                subscription_id: None,
                extra: std::collections::HashMap::new(),
            }),
            http::HeaderMap::new(),
            Arc::new(ext),
        );

        let args = serde_json::json!({ "message": "ping" });
        let result = handler.call(ctx, Some(args)).await;

        assert_eq!(result.is_error, Some(false));
        if let ContentBlock::Text(ref t) = result.content[0] {
            assert_eq!(t.text, "APP: test-client -> ping");
        } else {
            panic!("Expected text block");
        }
    }

    /// Tests conversion of `ToolError` variants into `JsonRpcErrorResponse`.
    #[test]
    fn test_tool_error_into_error_response() {
        let req_id = Some(JsonRpcRequestId::Number(3.0));

        let err_invalid = ToolError::InvalidParams("schema validation failed".to_string());
        let resp_invalid = err_invalid.into_error_response(req_id.clone());
        assert_eq!(resp_invalid.id, req_id);
        assert_eq!(
            resp_invalid.error.code,
            crate::types::jsonrpc::JsonRpcErrorCode::InvalidParams
        );
        assert_eq!(
            resp_invalid.error.message,
            "Invalid params: schema validation failed"
        );

        let err_internal = ToolError::Internal("execution panicked".to_string());
        let resp_internal = err_internal.into_error_response(req_id.clone());
        assert_eq!(resp_internal.id, req_id);
        assert_eq!(
            resp_internal.error.code,
            crate::types::jsonrpc::JsonRpcErrorCode::InternalError
        );
        assert_eq!(resp_internal.error.message, "execution panicked");
    }
}
