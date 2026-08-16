// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Handler traits and adapter implementations for MCP tools.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::extract::{FromRequestContext, RequestContext};
use crate::tools::IntoToolResult;
use crate::types::mcp::tools::call::CallToolResult;

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
