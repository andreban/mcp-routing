// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::extract::{FromRequestContext, RequestContext};
use crate::tools::ToolError;
use crate::types::mcp::{
    CacheScope,
    tools::{
        Tool,
        list::{ListToolsRequest, ListToolsResult, ListToolsResultResponse},
    },
};

/// Trait for types that can be converted into a [`ListToolsResult`].
pub trait IntoToolsListResult: Send {
    fn into_tools_list_result(
        self,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ListToolsResult, ToolError>;
}

impl IntoToolsListResult for ListToolsResult {
    fn into_tools_list_result(
        self,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ListToolsResult, ToolError> {
        let mut res = self;
        if res.ttl_ms.is_none() {
            res.ttl_ms = base_ttl_ms;
        }
        if res.cache_scope.is_none() {
            res.cache_scope = base_cache_scope;
        }
        if res.result_type.is_none() {
            res.result_type = Some("complete".to_string());
        }
        Ok(res)
    }
}

impl IntoToolsListResult for Vec<Tool> {
    fn into_tools_list_result(
        self,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ListToolsResult, ToolError> {
        Ok(ListToolsResult {
            meta: None,
            result_type: Some("complete".to_string()),
            next_cursor: None,
            ttl_ms: base_ttl_ms,
            cache_scope: base_cache_scope,
            tools: self,
            extras: HashMap::new(),
        })
    }
}

impl<T, E> IntoToolsListResult for Result<T, E>
where
    T: IntoToolsListResult,
    E: std::fmt::Display + Send,
{
    fn into_tools_list_result(
        self,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ListToolsResult, ToolError> {
        match self {
            Ok(val) => val.into_tools_list_result(base_ttl_ms, base_cache_scope),
            Err(err) => Err(ToolError::Internal(err.to_string())),
        }
    }
}

/// An erased handler trait for generating the list of available tools with request context.
pub trait ToolsListHandler: Send + Sync {
    fn call(
        &self,
        ctx: RequestContext,
        cursor: Option<String>,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Pin<Box<dyn Future<Output = Result<ListToolsResult, ToolError>> + Send>>;
}

/// Trait for converting handler functions into a boxed [`ToolsListHandler`].
pub trait IntoToolsListHandler<T>: Send + Sync + 'static {
    fn into_tools_list_handler(self) -> Arc<dyn ToolsListHandler>;
}

// 0 Extractors, 0 Args
struct NoArgsToolsListHandler<F>(F);

impl<F, Fut, Res> ToolsListHandler for NoArgsToolsListHandler<F>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoToolsListResult + 'static,
{
    fn call(
        &self,
        _ctx: RequestContext,
        _cursor: Option<String>,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Pin<Box<dyn Future<Output = Result<ListToolsResult, ToolError>> + Send>> {
        let fut = (self.0)();
        Box::pin(async move {
            fut.await
                .into_tools_list_result(base_ttl_ms, base_cache_scope)
        })
    }
}

impl<F, Fut, Res> IntoToolsListHandler<()> for F
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoToolsListResult + 'static,
{
    fn into_tools_list_handler(self) -> Arc<dyn ToolsListHandler> {
        Arc::new(NoArgsToolsListHandler(self))
    }
}

// 0 Extractors, 1 Arg (cursor)
struct CursorToolsListHandler<F>(F);

impl<F, Fut, Res> ToolsListHandler for CursorToolsListHandler<F>
where
    F: Fn(Option<String>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoToolsListResult + 'static,
{
    fn call(
        &self,
        _ctx: RequestContext,
        cursor: Option<String>,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Pin<Box<dyn Future<Output = Result<ListToolsResult, ToolError>> + Send>> {
        let fut = (self.0)(cursor);
        Box::pin(async move {
            fut.await
                .into_tools_list_result(base_ttl_ms, base_cache_scope)
        })
    }
}

impl<F, Fut, Res> IntoToolsListHandler<(Option<String>,)> for F
where
    F: Fn(Option<String>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoToolsListResult + 'static,
{
    fn into_tools_list_handler(self) -> Arc<dyn ToolsListHandler> {
        Arc::new(CursorToolsListHandler(self))
    }
}

macro_rules! impl_into_tools_list_handler {
    ($($E:ident),+) => {
        #[allow(non_snake_case)]
        impl<F, Fut, $($E,)+ Res> IntoToolsListHandler<($($E,)+ ())> for F
        where
            $($E: FromRequestContext + Send + 'static,)+
            F: Fn($($E),+) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Res> + Send + 'static,
            Res: IntoToolsListResult + 'static,
        {
            fn into_tools_list_handler(self) -> Arc<dyn ToolsListHandler> {
                struct Handler<F, M>(F, std::marker::PhantomData<fn() -> M>);

                impl<F, Fut, $($E,)+ Res> ToolsListHandler for Handler<F, (Fut, $($E,)+ Res)>
                where
                    $($E: FromRequestContext + Send + 'static,)+
                    F: Fn($($E),+) -> Fut + Send + Sync + 'static,
                    Fut: Future<Output = Res> + Send + 'static,
                    Res: IntoToolsListResult + 'static,
                {
                    fn call(
                        &self,
                        ctx: RequestContext,
                        _cursor: Option<String>,
                        base_ttl_ms: Option<u64>,
                        base_cache_scope: Option<CacheScope>,
                    ) -> Pin<Box<dyn Future<Output = Result<ListToolsResult, ToolError>> + Send>> {
                        $(
                            let $E = match $E::from_request_context(&ctx) {
                                Ok(val) => val,
                                Err(err) => {
                                    return Box::pin(async move {
                                        Err(ToolError::InvalidParams(format!("Extraction error: {err}")))
                                    });
                                }
                            };
                        )+
                        let fut = (self.0)($($E),+);
                        Box::pin(async move { fut.await.into_tools_list_result(base_ttl_ms, base_cache_scope) })
                    }
                }
                Arc::new(Handler(self, std::marker::PhantomData))
            }
        }

        #[allow(non_snake_case)]
        impl<F, Fut, $($E,)+ Res> IntoToolsListHandler<($($E,)+ (Option<String>,))> for F
        where
            $($E: FromRequestContext + Send + 'static,)+
            F: Fn($($E,)+ Option<String>) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Res> + Send + 'static,
            Res: IntoToolsListResult + 'static,
        {
            fn into_tools_list_handler(self) -> Arc<dyn ToolsListHandler> {
                struct Handler<F, M>(F, std::marker::PhantomData<fn() -> M>);

                impl<F, Fut, $($E,)+ Res> ToolsListHandler for Handler<F, (Fut, $($E,)+ Res)>
                where
                    $($E: FromRequestContext + Send + 'static,)+
                    F: Fn($($E,)+ Option<String>) -> Fut + Send + Sync + 'static,
                    Fut: Future<Output = Res> + Send + 'static,
                    Res: IntoToolsListResult + 'static,
                {
                    fn call(
                        &self,
                        ctx: RequestContext,
                        cursor: Option<String>,
                        base_ttl_ms: Option<u64>,
                        base_cache_scope: Option<CacheScope>,
                    ) -> Pin<Box<dyn Future<Output = Result<ListToolsResult, ToolError>> + Send>> {
                        $(
                            let $E = match $E::from_request_context(&ctx) {
                                Ok(val) => val,
                                Err(err) => {
                                    return Box::pin(async move {
                                        Err(ToolError::InvalidParams(format!("Extraction error: {err}")))
                                    });
                                }
                            };
                        )+
                        let fut = (self.0)($($E,)+ cursor);
                        Box::pin(async move { fut.await.into_tools_list_result(base_ttl_ms, base_cache_scope) })
                    }
                }
                Arc::new(Handler(self, std::marker::PhantomData))
            }
        }
    };
}

impl_into_tools_list_handler!(E1);
impl_into_tools_list_handler!(E1, E2);
impl_into_tools_list_handler!(E1, E2, E3);
impl_into_tools_list_handler!(E1, E2, E3, E4);
impl_into_tools_list_handler!(E1, E2, E3, E4, E5);

/// Handles an MCP `tools/list` request by constructing a [`ListToolsResultResponse`] with the registered tools.
pub fn handle_list_tools(
    req: ListToolsRequest,
    tools: Vec<Tool>,
    ttl_ms: Option<u64>,
    cache_scope: Option<CacheScope>,
) -> ListToolsResultResponse {
    ListToolsResultResponse::new(
        req.id,
        ListToolsResult {
            meta: None,
            result_type: Some("complete".to_string()),
            next_cursor: None,
            ttl_ms,
            cache_scope,
            tools,
            extras: HashMap::new(),
        },
    )
}
