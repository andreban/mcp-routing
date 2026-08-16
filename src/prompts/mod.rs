// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::extract::{FromRequestContext, RequestContext};
use crate::types::mcp::{
    ContentBlock,
    prompts::{PromptMessage, get::GetPromptResult},
};

pub mod list;
pub mod registry;

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

/// An erased prompt handler trait for executing a prompt retrieval request with request context.
pub trait PromptHandler: Send + Sync {
    fn call(
        &self,
        ctx: RequestContext,
        raw_args: Option<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<GetPromptResult, PromptError>> + Send>>;
}

/// Trait for converting handler functions into a boxed [`PromptHandler`].
pub trait IntoPromptHandler<T>: Send + Sync + 'static {
    fn into_prompt_handler(self) -> Arc<dyn PromptHandler>;
}

// 0 Extractors, 0 Args
struct NoArgsPromptHandler<F>(F);

impl<F, Fut, Res> PromptHandler for NoArgsPromptHandler<F>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoPromptResult + 'static,
{
    fn call(
        &self,
        _ctx: RequestContext,
        _raw_args: Option<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<GetPromptResult, PromptError>> + Send>> {
        let fut = (self.0)();
        Box::pin(async move { fut.await.into_prompt_result() })
    }
}

impl<F, Fut, Res> IntoPromptHandler<()> for F
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoPromptResult + 'static,
{
    fn into_prompt_handler(self) -> Arc<dyn PromptHandler> {
        Arc::new(NoArgsPromptHandler(self))
    }
}

// 0 Extractors, 1 Args
struct ArgsPromptHandler<F, Args>(F, std::marker::PhantomData<fn(Args)>);

impl<F, Fut, Args, Res> PromptHandler for ArgsPromptHandler<F, Args>
where
    Args: DeserializeOwned + Send + 'static,
    F: Fn(Args) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoPromptResult + 'static,
{
    fn call(
        &self,
        _ctx: RequestContext,
        raw_args: Option<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<GetPromptResult, PromptError>> + Send>> {
        let raw = raw_args.unwrap_or(Value::Null);
        match serde_json::from_value::<Args>(raw) {
            Ok(args) => {
                let fut = (self.0)(args);
                Box::pin(async move { fut.await.into_prompt_result() })
            }
            Err(err) => Box::pin(async move {
                Err(PromptError::InvalidParams(format!(
                    "Invalid arguments: {err}"
                )))
            }),
        }
    }
}

impl<F, Fut, Args, Res> IntoPromptHandler<(Args,)> for F
where
    Args: DeserializeOwned + Send + 'static,
    F: Fn(Args) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoPromptResult + 'static,
{
    fn into_prompt_handler(self) -> Arc<dyn PromptHandler> {
        Arc::new(ArgsPromptHandler(self, std::marker::PhantomData))
    }
}

macro_rules! impl_into_prompt_handler {
    ($($E:ident),+) => {
        #[allow(non_snake_case)]
        impl<F, Fut, $($E,)+ Res> IntoPromptHandler<($($E,)+ ())> for F
        where
            $($E: FromRequestContext + Send + 'static,)+
            F: Fn($($E),+) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Res> + Send + 'static,
            Res: IntoPromptResult + 'static,
        {
            fn into_prompt_handler(self) -> Arc<dyn PromptHandler> {
                struct Handler<F, M>(F, std::marker::PhantomData<fn() -> M>);

                impl<F, Fut, $($E,)+ Res> PromptHandler for Handler<F, (Fut, $($E,)+ Res)>
                where
                    $($E: FromRequestContext + Send + 'static,)+
                    F: Fn($($E),+) -> Fut + Send + Sync + 'static,
                    Fut: Future<Output = Res> + Send + 'static,
                    Res: IntoPromptResult + 'static,
                {
                    fn call(
                        &self,
                        ctx: RequestContext,
                        _raw_args: Option<Value>,
                    ) -> Pin<Box<dyn Future<Output = Result<GetPromptResult, PromptError>> + Send>> {
                        $(
                            let $E = match $E::from_request_context(&ctx) {
                                Ok(val) => val,
                                Err(err) => {
                                    return Box::pin(async move {
                                        Err(PromptError::InvalidParams(format!("Extraction error: {err}")))
                                    });
                                }
                            };
                        )+
                        let fut = (self.0)($($E),+);
                        Box::pin(async move { fut.await.into_prompt_result() })
                    }
                }
                Arc::new(Handler(self, std::marker::PhantomData))
            }
        }

        #[allow(non_snake_case)]
        impl<F, Fut, $($E,)+ Args, Res> IntoPromptHandler<($($E,)+ (Args,))> for F
        where
            $($E: FromRequestContext + Send + 'static,)+
            Args: DeserializeOwned + Send + 'static,
            F: Fn($($E,)+ Args) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Res> + Send + 'static,
            Res: IntoPromptResult + 'static,
        {
            fn into_prompt_handler(self) -> Arc<dyn PromptHandler> {
                struct Handler<F, M>(F, std::marker::PhantomData<fn() -> M>);

                impl<F, Fut, $($E,)+ Args, Res> PromptHandler for Handler<F, (Fut, $($E,)+ Args, Res)>
                where
                    $($E: FromRequestContext + Send + 'static,)+
                    Args: DeserializeOwned + Send + 'static,
                    F: Fn($($E,)+ Args) -> Fut + Send + Sync + 'static,
                    Fut: Future<Output = Res> + Send + 'static,
                    Res: IntoPromptResult + 'static,
                {
                    fn call(
                        &self,
                        ctx: RequestContext,
                        raw_args: Option<Value>,
                    ) -> Pin<Box<dyn Future<Output = Result<GetPromptResult, PromptError>> + Send>> {
                        $(
                            let $E = match $E::from_request_context(&ctx) {
                                Ok(val) => val,
                                Err(err) => {
                                    return Box::pin(async move {
                                        Err(PromptError::InvalidParams(format!("Extraction error: {err}")))
                                    });
                                }
                            };
                        )+
                        let raw = raw_args.unwrap_or(Value::Null);
                        let args = match serde_json::from_value::<Args>(raw) {
                            Ok(a) => a,
                            Err(err) => {
                                return Box::pin(async move {
                                    Err(PromptError::InvalidParams(format!("Invalid arguments: {err}")))
                                });
                            }
                        };
                        let fut = (self.0)($($E,)+ args);
                        Box::pin(async move { fut.await.into_prompt_result() })
                    }
                }
                Arc::new(Handler(self, std::marker::PhantomData))
            }
        }
    };
}

impl_into_prompt_handler!(E1);
impl_into_prompt_handler!(E1, E2);
impl_into_prompt_handler!(E1, E2, E3);
impl_into_prompt_handler!(E1, E2, E3, E4);
impl_into_prompt_handler!(E1, E2, E3, E4, E5);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::{Extension, Meta};
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

    /// Tests invoking prompt handlers with extractors (`Extension`, `Meta`) and typed arguments.
    #[tokio::test]
    async fn test_prompt_handler_with_extractors_and_args() {
        #[derive(serde::Deserialize)]
        struct SummarizeArgs {
            text: String,
        }

        #[derive(Clone)]
        struct AppConfig {
            tone: String,
        }

        async fn summarize_prompt(
            Extension(config): Extension<AppConfig>,
            Meta(meta): Meta,
            args: SummarizeArgs,
        ) -> Result<String, String> {
            let ver = meta.protocol_version.as_deref().unwrap_or("v1");
            Ok(format!(
                "[{ver}] Summarize in {} tone: {}",
                config.tone, args.text
            ))
        }

        let handler = summarize_prompt.into_prompt_handler();

        let mut ext = http::Extensions::new();
        ext.insert(AppConfig {
            tone: "formal".to_string(),
        });

        let ctx = RequestContext::new(
            Some(crate::types::mcp::RequestMetaObject {
                client_info: None,
                client_capabilities: None,
                protocol_version: Some("2026-07-28".to_string()),
                progress_token: None,
                log_level: None,
                subscription_id: None,
                extra: std::collections::HashMap::new(),
            }),
            http::HeaderMap::new(),
            Arc::new(ext),
        );

        let result = handler
            .call(
                ctx,
                Some(serde_json::json!({
                    "text": "Antigravity codebase"
                })),
            )
            .await
            .unwrap();

        assert_eq!(result.messages.len(), 1);
        if let ContentBlock::Text(ref t) = result.messages[0].content {
            assert_eq!(
                t.text,
                "[2026-07-28] Summarize in formal tone: Antigravity codebase"
            );
        } else {
            panic!("Expected text block");
        }
    }
}
