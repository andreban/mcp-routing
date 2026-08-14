// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::types::mcp::{
    ContentBlock,
    tools::call::{CallToolRequest, CallToolResult},
};

pub mod list;

/// Trait for types that can be converted into a [`CallToolResult`].
pub trait IntoToolResult: Send {
    fn into_tool_result(self) -> CallToolResult;
}

impl IntoToolResult for CallToolResult {
    fn into_tool_result(self) -> CallToolResult {
        self
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

/// An erased tool handler trait for executing a tool call.
pub trait ToolHandler: Send + Sync {
    fn call(
        &self,
        req: CallToolRequest<Value>,
    ) -> Pin<Box<dyn Future<Output = CallToolResult> + Send>>;
}

/// Trait for converting handler functions into a boxed [`ToolHandler`].
pub trait IntoToolHandler<T>: Send + Sync + 'static {
    fn into_tool_handler(self) -> Arc<dyn ToolHandler>;
}

struct NoArgsToolHandler<F>(F);

impl<F, Fut, Res> ToolHandler for NoArgsToolHandler<F>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoToolResult,
{
    fn call(
        &self,
        _req: CallToolRequest<Value>,
    ) -> Pin<Box<dyn Future<Output = CallToolResult> + Send>> {
        let fut = (self.0)();
        Box::pin(async move { fut.await.into_tool_result() })
    }
}

impl<F, Fut, Res> IntoToolHandler<()> for F
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoToolResult,
{
    fn into_tool_handler(self) -> Arc<dyn ToolHandler> {
        Arc::new(NoArgsToolHandler(self))
    }
}

struct ArgsToolHandler<F, Args>(F, std::marker::PhantomData<fn(Args)>);

impl<F, Fut, Args, Res> ToolHandler for ArgsToolHandler<F, Args>
where
    Args: DeserializeOwned + Send + 'static,
    F: Fn(Args) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoToolResult,
{
    fn call(
        &self,
        req: CallToolRequest<Value>,
    ) -> Pin<Box<dyn Future<Output = CallToolResult> + Send>> {
        let raw_args = req.params.and_then(|p| p.arguments).unwrap_or(Value::Null);
        match serde_json::from_value::<Args>(raw_args) {
            Ok(args) => {
                let fut = (self.0)(args);
                Box::pin(async move { fut.await.into_tool_result() })
            }
            Err(err) => {
                Box::pin(async move {
                    CallToolResult::error(format!("Invalid arguments: {err}"))
                })
            }
        }
    }
}

impl<F, Fut, Args, Res> IntoToolHandler<(Args,)> for F
where
    Args: DeserializeOwned + Send + 'static,
    F: Fn(Args) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoToolResult,
{
    fn into_tool_handler(self) -> Arc<dyn ToolHandler> {
        Arc::new(ArgsToolHandler(self, std::marker::PhantomData))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::mcp::TextContent;

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
    }
}
