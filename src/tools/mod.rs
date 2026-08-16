// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Tools subsystem for defining and handling MCP tool invocations.

use serde_json::Value;

use crate::types::jsonrpc::{JsonRpcErrorResponse, JsonRpcRequestId};
use crate::types::mcp::{ContentBlock, tools::call::CallToolResult};

pub mod handler;
pub mod list;
pub mod registry;

pub use crate::extract::Json;
pub use handler::{IntoToolHandler, ToolHandler};
pub use list::{IntoToolsListHandler, IntoToolsListResult, ToolsListHandler};
pub use registry::ToolRegistry;

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

/// Trait for types that can be converted into a [`CallToolResult`].
pub trait IntoToolResult: Send {
    fn into_tool_result(self) -> CallToolResult;
}

impl<S> IntoToolResult for CallToolResult<S>
where
    S: serde::Serialize + Send,
{
    fn into_tool_result(self) -> CallToolResult {
        let structured_content = match self.structured_content {
            Some(s) => match serde_json::to_value(s) {
                Ok(v) => Some(v),
                Err(err) => {
                    return CallToolResult::error(format!(
                        "Failed to serialize structured output: {err}"
                    ));
                }
            },
            None => None,
        };

        CallToolResult {
            meta: self.meta,
            result_type: self.result_type,
            content: self.content,
            is_error: self.is_error,
            structured_content,
            extras: self.extras,
        }
    }
}

impl IntoToolResult for crate::types::mcp::InputRequiredResult {
    fn into_tool_result(self) -> CallToolResult {
        let (meta, result_type, extras) = self.into_parts();
        CallToolResult {
            meta,
            result_type: Some(result_type),
            content: Vec::new(),
            is_error: None,
            structured_content: None,
            extras,
        }
    }
}

impl IntoToolResult for String {
    fn into_tool_result(self) -> CallToolResult {
        CallToolResult::text(self)
    }
}

impl IntoToolResult for &str {
    fn into_tool_result(self) -> CallToolResult {
        CallToolResult::text(self)
    }
}

impl IntoToolResult for ContentBlock {
    fn into_tool_result(self) -> CallToolResult {
        CallToolResult::with_content(vec![self])
    }
}

impl IntoToolResult for Vec<ContentBlock> {
    fn into_tool_result(self) -> CallToolResult {
        CallToolResult::with_content(self)
    }
}

impl IntoToolResult for Value {
    fn into_tool_result(self) -> CallToolResult {
        CallToolResult::structured(self)
    }
}

impl<T> IntoToolResult for Json<T>
where
    T: serde::Serialize + Send,
{
    fn into_tool_result(self) -> CallToolResult {
        match serde_json::to_value(&self.0) {
            Ok(val) => CallToolResult::structured(val),
            Err(err) => {
                CallToolResult::error(format!("Failed to serialize structured output: {err}"))
            }
        }
    }
}

// Tuple conversions with String / &str text
impl IntoToolResult for (Value, String) {
    fn into_tool_result(self) -> CallToolResult {
        CallToolResult::structured_with_text(self.0, self.1)
    }
}

impl IntoToolResult for (Value, &'static str) {
    fn into_tool_result(self) -> CallToolResult {
        CallToolResult::structured_with_text(self.0, self.1)
    }
}

impl IntoToolResult for (String, Value) {
    fn into_tool_result(self) -> CallToolResult {
        CallToolResult::structured_with_text(self.1, self.0)
    }
}

impl IntoToolResult for (&'static str, Value) {
    fn into_tool_result(self) -> CallToolResult {
        CallToolResult::structured_with_text(self.1, self.0)
    }
}

impl<T> IntoToolResult for (Json<T>, String)
where
    T: serde::Serialize + Send,
{
    fn into_tool_result(self) -> CallToolResult {
        match serde_json::to_value(&self.0.0) {
            Ok(val) => CallToolResult::structured_with_text(val, self.1),
            Err(err) => {
                CallToolResult::error(format!("Failed to serialize structured output: {err}"))
            }
        }
    }
}

impl<T> IntoToolResult for (Json<T>, &'static str)
where
    T: serde::Serialize + Send,
{
    fn into_tool_result(self) -> CallToolResult {
        match serde_json::to_value(&self.0.0) {
            Ok(val) => CallToolResult::structured_with_text(val, self.1),
            Err(err) => {
                CallToolResult::error(format!("Failed to serialize structured output: {err}"))
            }
        }
    }
}

impl<T> IntoToolResult for (String, Json<T>)
where
    T: serde::Serialize + Send,
{
    fn into_tool_result(self) -> CallToolResult {
        match serde_json::to_value(&self.1.0) {
            Ok(val) => CallToolResult::structured_with_text(val, self.0),
            Err(err) => {
                CallToolResult::error(format!("Failed to serialize structured output: {err}"))
            }
        }
    }
}

impl<T> IntoToolResult for (&'static str, Json<T>)
where
    T: serde::Serialize + Send,
{
    fn into_tool_result(self) -> CallToolResult {
        match serde_json::to_value(&self.1.0) {
            Ok(val) => CallToolResult::structured_with_text(val, self.0),
            Err(err) => {
                CallToolResult::error(format!("Failed to serialize structured output: {err}"))
            }
        }
    }
}

// Tuple conversions with ContentBlock / Vec<ContentBlock>
impl IntoToolResult for (Value, ContentBlock) {
    fn into_tool_result(self) -> CallToolResult {
        CallToolResult::structured_with_content(self.0, vec![self.1])
    }
}

impl IntoToolResult for (ContentBlock, Value) {
    fn into_tool_result(self) -> CallToolResult {
        CallToolResult::structured_with_content(self.1, vec![self.0])
    }
}

impl IntoToolResult for (Value, Vec<ContentBlock>) {
    fn into_tool_result(self) -> CallToolResult {
        CallToolResult::structured_with_content(self.0, self.1)
    }
}

impl IntoToolResult for (Vec<ContentBlock>, Value) {
    fn into_tool_result(self) -> CallToolResult {
        CallToolResult::structured_with_content(self.1, self.0)
    }
}

impl<T> IntoToolResult for (Json<T>, ContentBlock)
where
    T: serde::Serialize + Send,
{
    fn into_tool_result(self) -> CallToolResult {
        match serde_json::to_value(&self.0.0) {
            Ok(val) => CallToolResult::structured_with_content(val, vec![self.1]),
            Err(err) => {
                CallToolResult::error(format!("Failed to serialize structured output: {err}"))
            }
        }
    }
}

impl<T> IntoToolResult for (ContentBlock, Json<T>)
where
    T: serde::Serialize + Send,
{
    fn into_tool_result(self) -> CallToolResult {
        match serde_json::to_value(&self.1.0) {
            Ok(val) => CallToolResult::structured_with_content(val, vec![self.0]),
            Err(err) => {
                CallToolResult::error(format!("Failed to serialize structured output: {err}"))
            }
        }
    }
}

impl<T> IntoToolResult for (Json<T>, Vec<ContentBlock>)
where
    T: serde::Serialize + Send,
{
    fn into_tool_result(self) -> CallToolResult {
        match serde_json::to_value(&self.0.0) {
            Ok(val) => CallToolResult::structured_with_content(val, self.1),
            Err(err) => {
                CallToolResult::error(format!("Failed to serialize structured output: {err}"))
            }
        }
    }
}

impl<T> IntoToolResult for (Vec<ContentBlock>, Json<T>)
where
    T: serde::Serialize + Send,
{
    fn into_tool_result(self) -> CallToolResult {
        match serde_json::to_value(&self.1.0) {
            Ok(val) => CallToolResult::structured_with_content(val, self.0),
            Err(err) => {
                CallToolResult::error(format!("Failed to serialize structured output: {err}"))
            }
        }
    }
}

impl<T, E> IntoToolResult for Result<T, E>
where
    T: IntoToolResult,
    E: std::fmt::Display + Send,
{
    fn into_tool_result(self) -> CallToolResult {
        match self {
            Ok(val) => val.into_tool_result(),
            Err(err) => CallToolResult::error(err.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for `IntoToolResult` conversions and handler adaptations.

    use std::sync::Arc;
    use super::*;
    use crate::extract::{Extension, Meta, RequestContext};
    use crate::types::mcp::{Implementation, TextContent};

    /// Tests `IntoToolResult` implementations across primitive and complex return types.
    #[test]
    fn test_into_tool_result() {
        // String
        let res_str = "hello".into_tool_result();
        assert_eq!(res_str.is_error, Some(false));
        if let ContentBlock::Text(ref t) = res_str.content[0] {
            assert_eq!(t.text, "hello");
        }

        // Owned String
        let res_owned = "world".to_string().into_tool_result();
        assert_eq!(res_owned.is_error, Some(false));

        // Result::Ok
        let res_ok: Result<&str, &str> = Ok("success");
        let res = res_ok.into_tool_result();
        assert_eq!(res.is_error, Some(false));

        // Result::Err
        let res_err: Result<&str, &str> = Err("failure");
        let res = res_err.into_tool_result();
        assert_eq!(res.is_error, Some(true));
        if let ContentBlock::Text(ref t) = res.content[0] {
            assert_eq!(t.text, "failure");
        }

        // ContentBlock
        let block = ContentBlock::Text(TextContent {
            text: "block".to_string(),
            annotations: None,
            meta: None,
        });
        let res_block = block.into_tool_result();
        assert_eq!(res_block.content.len(), 1);

        // Value
        let val = serde_json::json!({ "answer": 42 });
        let res_val = val.into_tool_result();
        assert_eq!(res_val.structured_content.unwrap()["answer"], 42);

        // Json<T>
        #[derive(serde::Serialize)]
        struct Output {
            count: usize,
        }
        let res_json = Json(Output { count: 5 }).into_tool_result();
        assert_eq!(res_json.structured_content.unwrap()["count"], 5);

        // (Json<T>, &str) tuple
        let res_tuple_str = (Json(Output { count: 10 }), "Summary text").into_tool_result();
        assert_eq!(res_tuple_str.structured_content.unwrap()["count"], 10);
        if let ContentBlock::Text(ref t) = res_tuple_str.content[0] {
            assert_eq!(t.text, "Summary text");
        }

        // (Value, String) tuple
        let res_val_str =
            (serde_json::json!({ "ok": true }), "All good".to_string()).into_tool_result();
        assert_eq!(res_val_str.structured_content.unwrap()["ok"], true);
        if let ContentBlock::Text(ref t) = res_val_str.content[0] {
            assert_eq!(t.text, "All good");
        }

        // Generic CallToolResult<Output>
        let custom_res = CallToolResult::structured(Output { count: 99 }).with_text("Count report");
        let res_custom = custom_res.into_tool_result();
        assert_eq!(res_custom.structured_content.unwrap()["count"], 99);
        if let ContentBlock::Text(ref t) = res_custom.content[0] {
            assert_eq!(t.text, "Count report");
        }
    }

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
