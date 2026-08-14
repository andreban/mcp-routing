// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::extract::{FromRequestContext, RequestContext};
use crate::types::mcp::{ContentBlock, tools::call::CallToolResult};

pub mod list;
pub mod registry;

pub use crate::extract::Json;
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

/// An erased tool handler trait for executing a tool call with request context.
pub trait ToolHandler: Send + Sync {
    fn call(
        &self,
        ctx: RequestContext,
        raw_args: Option<Value>,
    ) -> Pin<Box<dyn Future<Output = CallToolResult> + Send>>;
}

/// Trait for converting handler functions into a boxed [`ToolHandler`].
pub trait IntoToolHandler<T>: Send + Sync + 'static {
    fn into_tool_handler(self) -> Arc<dyn ToolHandler>;
}

// 0 Extractors, 0 Args
struct NoArgsToolHandler<F>(F);

impl<F, Fut, Res> ToolHandler for NoArgsToolHandler<F>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoToolResult + 'static,
{
    fn call(
        &self,
        _ctx: RequestContext,
        _raw_args: Option<Value>,
    ) -> Pin<Box<dyn Future<Output = CallToolResult> + Send>> {
        let fut = (self.0)();
        Box::pin(async move { fut.await.into_tool_result() })
    }
}

impl<F, Fut, Res> IntoToolHandler<()> for F
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoToolResult + 'static,
{
    fn into_tool_handler(self) -> Arc<dyn ToolHandler> {
        Arc::new(NoArgsToolHandler(self))
    }
}

// 0 Extractors, 1 Args
struct ArgsToolHandler<F, Args>(F, std::marker::PhantomData<fn(Args)>);

impl<F, Fut, Args, Res> ToolHandler for ArgsToolHandler<F, Args>
where
    Args: DeserializeOwned + Send + 'static,
    F: Fn(Args) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoToolResult + 'static,
{
    fn call(
        &self,
        _ctx: RequestContext,
        raw_args: Option<Value>,
    ) -> Pin<Box<dyn Future<Output = CallToolResult> + Send>> {
        let raw = raw_args.unwrap_or(Value::Null);
        match serde_json::from_value::<Args>(raw) {
            Ok(args) => {
                let fut = (self.0)(args);
                Box::pin(async move { fut.await.into_tool_result() })
            }
            Err(err) => {
                Box::pin(async move { CallToolResult::error(format!("Invalid arguments: {err}")) })
            }
        }
    }
}

impl<F, Fut, Args, Res> IntoToolHandler<(Args,)> for F
where
    Args: DeserializeOwned + Send + 'static,
    F: Fn(Args) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoToolResult + 'static,
{
    fn into_tool_handler(self) -> Arc<dyn ToolHandler> {
        Arc::new(ArgsToolHandler(self, std::marker::PhantomData))
    }
}

macro_rules! impl_into_tool_handler {
    ($($E:ident),+) => {
        #[allow(non_snake_case)]
        impl<F, Fut, $($E,)+ Res> IntoToolHandler<($($E,)+ ())> for F
        where
            $($E: FromRequestContext + Send + 'static,)+
            F: Fn($($E),+) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Res> + Send + 'static,
            Res: IntoToolResult + 'static,
        {
            fn into_tool_handler(self) -> Arc<dyn ToolHandler> {
                struct Handler<F, M>(F, std::marker::PhantomData<fn() -> M>);

                impl<F, Fut, $($E,)+ Res> ToolHandler for Handler<F, (Fut, $($E,)+ Res)>
                where
                    $($E: FromRequestContext + Send + 'static,)+
                    F: Fn($($E),+) -> Fut + Send + Sync + 'static,
                    Fut: Future<Output = Res> + Send + 'static,
                    Res: IntoToolResult + 'static,
                {
                    fn call(
                        &self,
                        ctx: RequestContext,
                        _raw_args: Option<Value>,
                    ) -> Pin<Box<dyn Future<Output = CallToolResult> + Send>> {
                        $(
                            let $E = match $E::from_request_context(&ctx) {
                                Ok(val) => val,
                                Err(err) => {
                                    return Box::pin(async move {
                                        CallToolResult::error(format!("Extraction error: {err}"))
                                    });
                                }
                            };
                        )+
                        let fut = (self.0)($($E),+);
                        Box::pin(async move { fut.await.into_tool_result() })
                    }
                }
                Arc::new(Handler(self, std::marker::PhantomData))
            }
        }

        #[allow(non_snake_case)]
        impl<F, Fut, $($E,)+ Args, Res> IntoToolHandler<($($E,)+ (Args,))> for F
        where
            $($E: FromRequestContext + Send + 'static,)+
            Args: DeserializeOwned + Send + 'static,
            F: Fn($($E,)+ Args) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Res> + Send + 'static,
            Res: IntoToolResult + 'static,
        {
            fn into_tool_handler(self) -> Arc<dyn ToolHandler> {
                struct Handler<F, M>(F, std::marker::PhantomData<fn() -> M>);

                impl<F, Fut, $($E,)+ Args, Res> ToolHandler for Handler<F, (Fut, $($E,)+ Args, Res)>
                where
                    $($E: FromRequestContext + Send + 'static,)+
                    Args: DeserializeOwned + Send + 'static,
                    F: Fn($($E,)+ Args) -> Fut + Send + Sync + 'static,
                    Fut: Future<Output = Res> + Send + 'static,
                    Res: IntoToolResult + 'static,
                {
                    fn call(
                        &self,
                        ctx: RequestContext,
                        raw_args: Option<Value>,
                    ) -> Pin<Box<dyn Future<Output = CallToolResult> + Send>> {
                        $(
                            let $E = match $E::from_request_context(&ctx) {
                                Ok(val) => val,
                                Err(err) => {
                                    return Box::pin(async move {
                                        CallToolResult::error(format!("Extraction error: {err}"))
                                    });
                                }
                            };
                        )+
                        let raw = raw_args.unwrap_or(Value::Null);
                        let args = match serde_json::from_value::<Args>(raw) {
                            Ok(a) => a,
                            Err(err) => {
                                return Box::pin(async move {
                                    CallToolResult::error(format!("Invalid arguments: {err}"))
                                });
                            }
                        };
                        let fut = (self.0)($($E,)+ args);
                        Box::pin(async move { fut.await.into_tool_result() })
                    }
                }
                Arc::new(Handler(self, std::marker::PhantomData))
            }
        }
    };
}

impl_into_tool_handler!(E1);
impl_into_tool_handler!(E1, E2);
impl_into_tool_handler!(E1, E2, E3);
impl_into_tool_handler!(E1, E2, E3, E4);
impl_into_tool_handler!(E1, E2, E3, E4, E5);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::{Extension, Meta, SessionId};
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
            session: SessionId,
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
                "{}: [{session}] {client} -> {}",
                state.prefix, params.message
            ))
        }

        let handler = echo_handler.into_tool_handler();

        let mut ext = http::Extensions::new();
        ext.insert(AppState {
            prefix: "APP".to_string(),
        });

        let ctx = RequestContext::new(
            Some(SessionId::new("session-42")),
            Some(crate::types::mcp::RequestMetaObject {
                client_info: Some(Implementation::new("test-client", "1.0.0")),
                client_capabilities: None,
                protocol_version: None,
                progress_token: None,
                log_level: None,
                extra: std::collections::HashMap::new(),
            }),
            http::HeaderMap::new(),
            Arc::new(ext),
        );

        let result = handler
            .call(
                ctx,
                Some(serde_json::json!({
                    "message": "hello world"
                })),
            )
            .await;

        assert_eq!(result.is_error, Some(false));
        if let ContentBlock::Text(ref t) = result.content[0] {
            assert_eq!(t.text, "APP: [session-42] test-client -> hello world");
        } else {
            panic!("Expected text block");
        }
    }
}
