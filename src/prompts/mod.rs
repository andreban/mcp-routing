// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! MCP prompts subsystem for prompt retrieval, listing, and templating.

use crate::types::mcp::{
    ContentBlock,
    prompts::{PromptMessage, get::GetPromptResult},
};

pub mod handler;
pub mod list;
pub mod registry;

pub use handler::{IntoPromptHandler, PromptHandler};
pub use list::{IntoPromptsListHandler, IntoPromptsListResult, PromptsListHandler};
pub use registry::PromptRegistry;

/// Error type encountered during prompt execution or argument deserialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptError {
    /// Invalid arguments provided to the prompt handler.
    InvalidParams(String),
    /// Internal execution or business logic error.
    Internal(String),
}

impl std::fmt::Display for PromptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PromptError::InvalidParams(msg) => write!(f, "Invalid params: {msg}"),
            PromptError::Internal(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for PromptError {}

/// Trait for types that can be converted into a [`GetPromptResult`].
pub trait IntoPromptResult: Send {
    fn into_prompt_result(self) -> Result<GetPromptResult, PromptError>;
}

impl IntoPromptResult for GetPromptResult {
    fn into_prompt_result(self) -> Result<GetPromptResult, PromptError> {
        Ok(self)
    }
}

impl IntoPromptResult for PromptMessage {
    fn into_prompt_result(self) -> Result<GetPromptResult, PromptError> {
        Ok(GetPromptResult::new(vec![self]))
    }
}

impl IntoPromptResult for Vec<PromptMessage> {
    fn into_prompt_result(self) -> Result<GetPromptResult, PromptError> {
        Ok(GetPromptResult::new(self))
    }
}

impl IntoPromptResult for String {
    fn into_prompt_result(self) -> Result<GetPromptResult, PromptError> {
        Ok(GetPromptResult::user(self))
    }
}

impl IntoPromptResult for &str {
    fn into_prompt_result(self) -> Result<GetPromptResult, PromptError> {
        Ok(GetPromptResult::user(self))
    }
}

impl IntoPromptResult for ContentBlock {
    fn into_prompt_result(self) -> Result<GetPromptResult, PromptError> {
        Ok(GetPromptResult::new(vec![PromptMessage::user(self)]))
    }
}

impl IntoPromptResult for Vec<ContentBlock> {
    fn into_prompt_result(self) -> Result<GetPromptResult, PromptError> {
        Ok(GetPromptResult::new(
            self.into_iter().map(PromptMessage::user).collect(),
        ))
    }
}

impl IntoPromptResult for crate::types::mcp::InputRequiredResult {
    fn into_prompt_result(self) -> Result<GetPromptResult, PromptError> {
        let mut extras = self.extras;
        if let Some(state) = self.request_state {
            extras.insert("requestState".to_string(), serde_json::Value::String(state));
        }
        if !self.input_requests.is_empty()
            && let Ok(reqs) = serde_json::to_value(&self.input_requests)
        {
            extras.insert("inputRequests".to_string(), reqs);
        }
        Ok(GetPromptResult {
            meta: self.meta,
            result_type: Some(self.result_type),
            description: None,
            messages: Vec::new(),
            extras,
        })
    }
}

impl<T, E> IntoPromptResult for Result<T, E>
where
    T: IntoPromptResult,
    E: std::fmt::Display + Send,
{
    fn into_prompt_result(self) -> Result<GetPromptResult, PromptError> {
        match self {
            Ok(val) => val.into_prompt_result(),
            Err(err) => Err(PromptError::Internal(err.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for `IntoPromptResult` conversions.

    use super::*;
    use crate::types::mcp::{Role, TextContent};

    /// Tests `IntoPromptResult` implementations across various types.
    #[test]
    fn test_into_prompt_result() {
        // String
        let res_str = "hello".into_prompt_result().unwrap();
        assert_eq!(res_str.messages.len(), 1);
        assert!(matches!(res_str.messages[0].role, Role::User));
        if let ContentBlock::Text(ref t) = res_str.messages[0].content {
            assert_eq!(t.text, "hello");
        }

        // Owned String
        let res_owned = "world".to_string().into_prompt_result().unwrap();
        assert_eq!(res_owned.messages.len(), 1);

        // Result::Ok
        let res_ok: Result<&str, &str> = Ok("success");
        let res = res_ok.into_prompt_result().unwrap();
        assert_eq!(res.messages.len(), 1);

        // Result::Err
        let res_err: Result<&str, &str> = Err("failure");
        let err = res_err.into_prompt_result().unwrap_err();
        assert!(matches!(err, PromptError::Internal(ref s) if s == "failure"));

        // PromptMessage
        let msg = PromptMessage::assistant_text("assistant answer");
        let res_msg = msg.into_prompt_result().unwrap();
        assert_eq!(res_msg.messages.len(), 1);
        assert!(matches!(res_msg.messages[0].role, Role::Assistant));

        // ContentBlock
        let block = ContentBlock::Text(TextContent {
            text: "content block".to_string(),
            annotations: None,
            meta: None,
        });
        let res_block = block.into_prompt_result().unwrap();
        assert_eq!(res_block.messages.len(), 1);
    }
}
