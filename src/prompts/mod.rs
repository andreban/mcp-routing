// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::types::mcp::{
    ContentBlock,
    prompts::{
        PromptMessage,
        get::{GetPromptRequest, GetPromptResult},
    },
};

pub mod list;

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

/// An erased prompt handler trait for executing a prompt retrieval request.
pub trait PromptHandler: Send + Sync {
    fn call(
        &self,
        req: GetPromptRequest<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<GetPromptResult, PromptError>> + Send>>;
}

/// Trait for converting handler functions into a boxed [`PromptHandler`].
pub trait IntoPromptHandler<T>: Send + Sync + 'static {
    fn into_prompt_handler(self) -> Arc<dyn PromptHandler>;
}

struct NoArgsPromptHandler<F>(F);

impl<F, Fut, Res> PromptHandler for NoArgsPromptHandler<F>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoPromptResult,
{
    fn call(
        &self,
        _req: GetPromptRequest<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<GetPromptResult, PromptError>> + Send>> {
        let fut = (self.0)();
        Box::pin(async move { fut.await.into_prompt_result() })
    }
}

impl<F, Fut, Res> IntoPromptHandler<()> for F
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoPromptResult,
{
    fn into_prompt_handler(self) -> Arc<dyn PromptHandler> {
        Arc::new(NoArgsPromptHandler(self))
    }
}

struct ArgsPromptHandler<F, Args>(F, std::marker::PhantomData<fn(Args)>);

impl<F, Fut, Args, Res> PromptHandler for ArgsPromptHandler<F, Args>
where
    Args: DeserializeOwned + Send + 'static,
    F: Fn(Args) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoPromptResult,
{
    fn call(
        &self,
        req: GetPromptRequest<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<GetPromptResult, PromptError>> + Send>> {
        let raw_args = req.params.and_then(|p| p.arguments).unwrap_or(Value::Null);
        match serde_json::from_value::<Args>(raw_args) {
            Ok(args) => {
                let fut = (self.0)(args);
                Box::pin(async move { fut.await.into_prompt_result() })
            }
            Err(err) => {
                Box::pin(async move {
                    Err(PromptError::InvalidParams(format!(
                        "Invalid arguments: {err}"
                    )))
                })
            }
        }
    }
}

impl<F, Fut, Args, Res> IntoPromptHandler<(Args,)> for F
where
    Args: DeserializeOwned + Send + 'static,
    F: Fn(Args) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoPromptResult,
{
    fn into_prompt_handler(self) -> Arc<dyn PromptHandler> {
        Arc::new(ArgsPromptHandler(self, std::marker::PhantomData))
    }
}

#[cfg(test)]
mod tests {
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
