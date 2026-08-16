// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! MCP argument autocompletion subsystem.
//!
//! Provides handlers and registries for prompt and resource argument autocompletion
//! per MCP 2026-07-28 specification.

pub mod handler;
pub mod registry;

pub use handler::{CompletionHandler, IntoCompletionHandler};
pub use registry::CompletionRegistry;

use crate::types::jsonrpc::{JsonRpcErrorResponse, JsonRpcRequestId};
use crate::types::mcp::completion::{CompleteResult, CompletionValues};

/// Error type encountered during argument completion operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionError {
    /// Invalid parameters or arguments provided to the completion handler.
    InvalidParams(String),
    /// Target prompt, resource, or argument was not found.
    NotFound(String),
    /// Internal execution or business logic error.
    Internal(String),
}

impl std::fmt::Display for CompletionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompletionError::InvalidParams(msg) => write!(f, "Invalid params: {msg}"),
            CompletionError::NotFound(msg) => write!(f, "Not found: {msg}"),
            CompletionError::Internal(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for CompletionError {}

impl CompletionError {
    /// Converts this error into a standard JSON-RPC error response.
    pub fn into_error_response(self, id: Option<JsonRpcRequestId>) -> JsonRpcErrorResponse {
        match self {
            CompletionError::InvalidParams(err) | CompletionError::NotFound(err) => {
                JsonRpcErrorResponse::invalid_params(id, format!("Invalid params: {err}"))
            }
            CompletionError::Internal(err) => JsonRpcErrorResponse::internal_error(id, err),
        }
    }
}

/// Trait for types that can be converted into a [`CompleteResult`].
pub trait IntoCompletionResult: Send {
    fn into_completion_result(self) -> Result<CompleteResult, CompletionError>;
}

impl IntoCompletionResult for CompleteResult {
    fn into_completion_result(self) -> Result<CompleteResult, CompletionError> {
        Ok(self)
    }
}

impl IntoCompletionResult for CompletionValues {
    fn into_completion_result(self) -> Result<CompleteResult, CompletionError> {
        Ok(CompleteResult::with_completion(self))
    }
}

impl IntoCompletionResult for Vec<String> {
    fn into_completion_result(self) -> Result<CompleteResult, CompletionError> {
        Ok(CompleteResult::new(self))
    }
}

impl IntoCompletionResult for Vec<&str> {
    fn into_completion_result(self) -> Result<CompleteResult, CompletionError> {
        Ok(CompleteResult::new(
            self.into_iter().map(String::from).collect::<Vec<_>>(),
        ))
    }
}

impl IntoCompletionResult for &[&str] {
    fn into_completion_result(self) -> Result<CompleteResult, CompletionError> {
        Ok(CompleteResult::new(
            self.iter().map(|&s| s.to_string()).collect::<Vec<_>>(),
        ))
    }
}

impl IntoCompletionResult for &[String] {
    fn into_completion_result(self) -> Result<CompleteResult, CompletionError> {
        Ok(CompleteResult::new(self.to_vec()))
    }
}

impl IntoCompletionResult for crate::types::mcp::InputRequiredResult {
    fn into_completion_result(self) -> Result<CompleteResult, CompletionError> {
        let (meta, result_type, extras) = self.into_parts();
        Ok(CompleteResult {
            meta,
            result_type: Some(result_type),
            completion: CompletionValues::empty(),
            extras,
        })
    }
}

impl<T, E> IntoCompletionResult for Result<T, E>
where
    T: IntoCompletionResult,
    E: std::fmt::Display + Send,
{
    fn into_completion_result(self) -> Result<CompleteResult, CompletionError> {
        match self {
            Ok(val) => val.into_completion_result(),
            Err(err) => Err(CompletionError::Internal(err.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for `IntoCompletionResult` conversions.

    use super::*;

    /// Tests conversion implementations of `IntoCompletionResult` for vectors, slices, and errors.
    #[test]
    fn test_into_completion_result() {
        let res1: CompleteResult = vec!["a".to_string(), "b".to_string()]
            .into_completion_result()
            .unwrap();
        assert_eq!(res1.completion.values, vec!["a", "b"]);

        let res2: CompleteResult = vec!["x", "y"].into_completion_result().unwrap();
        assert_eq!(res2.completion.values, vec!["x", "y"]);

        let res3: CompleteResult = (&["m", "n"][..]).into_completion_result().unwrap();
        assert_eq!(res3.completion.values, vec!["m", "n"]);

        let res_err: Result<CompleteResult, CompletionError> =
            Result::<Vec<String>, &str>::Err("failed").into_completion_result();
        assert!(matches!(res_err, Err(CompletionError::Internal(msg)) if msg == "failed"));
    }

    /// Tests conversion of `CompletionError` variants into `JsonRpcErrorResponse`.
    #[test]
    fn test_completion_error_into_error_response() {
        let req_id = Some(JsonRpcRequestId::Number(7.0));

        let err_invalid = CompletionError::InvalidParams("bad argument name".to_string());
        let resp_invalid = err_invalid.into_error_response(req_id.clone());
        assert_eq!(resp_invalid.id, req_id);
        assert_eq!(
            resp_invalid.error.code,
            crate::types::jsonrpc::JsonRpcErrorCode::InvalidParams
        );
        assert_eq!(
            resp_invalid.error.message,
            "Invalid params: bad argument name"
        );

        let err_not_found = CompletionError::NotFound("prompt 'review'".to_string());
        let resp_not_found = err_not_found.into_error_response(req_id.clone());
        assert_eq!(resp_not_found.id, req_id);
        assert_eq!(
            resp_not_found.error.code,
            crate::types::jsonrpc::JsonRpcErrorCode::InvalidParams
        );
        assert_eq!(
            resp_not_found.error.message,
            "Invalid params: prompt 'review'"
        );

        let err_internal = CompletionError::Internal("completion timeout".to_string());
        let resp_internal = err_internal.into_error_response(req_id.clone());
        assert_eq!(resp_internal.id, req_id);
        assert_eq!(
            resp_internal.error.code,
            crate::types::jsonrpc::JsonRpcErrorCode::InternalError
        );
        assert_eq!(resp_internal.error.message, "completion timeout");
    }
}
