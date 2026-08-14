// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::extract::{FromRequestContext, RequestContext};
use crate::prompts::PromptError;
use crate::types::mcp::{
    CacheScope,
    prompts::{
        Prompt,
        list::{ListPromptsRequest, ListPromptsResult, ListPromptsResultResponse},
    },
};

/// Trait for types that can be converted into a [`ListPromptsResult`].
pub trait IntoPromptsListResult: Send {
    fn into_prompts_list_result(
        self,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ListPromptsResult, PromptError>;
}

impl IntoPromptsListResult for ListPromptsResult {
    fn into_prompts_list_result(
        self,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ListPromptsResult, PromptError> {
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

impl IntoPromptsListResult for Vec<Prompt> {
    fn into_prompts_list_result(
        self,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ListPromptsResult, PromptError> {
        Ok(ListPromptsResult {
            meta: None,
            result_type: Some("complete".to_string()),
            next_cursor: None,
            ttl_ms: base_ttl_ms,
            cache_scope: base_cache_scope,
            prompts: self,
            extras: HashMap::new(),
        })
    }
}

impl<T, E> IntoPromptsListResult for Result<T, E>
where
    T: IntoPromptsListResult,
    E: std::fmt::Display + Send,
{
    fn into_prompts_list_result(
        self,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ListPromptsResult, PromptError> {
        match self {
            Ok(val) => val.into_prompts_list_result(base_ttl_ms, base_cache_scope),
            Err(err) => Err(PromptError::Internal(err.to_string())),
        }
    }
}

/// An erased handler trait for generating the list of available prompts with request context.
pub trait PromptsListHandler: Send + Sync {
    fn call(
        &self,
        ctx: RequestContext,
        cursor: Option<String>,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Pin<Box<dyn Future<Output = Result<ListPromptsResult, PromptError>> + Send>>;
}

/// Trait for converting handler functions into a boxed [`PromptsListHandler`].
pub trait IntoPromptsListHandler<T>: Send + Sync + 'static {
    fn into_prompts_list_handler(self) -> Arc<dyn PromptsListHandler>;
}

// 0 Extractors, 0 Args
struct NoArgsPromptsListHandler<F>(F);

impl<F, Fut, Res> PromptsListHandler for NoArgsPromptsListHandler<F>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoPromptsListResult + 'static,
{
    fn call(
        &self,
        _ctx: RequestContext,
        _cursor: Option<String>,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Pin<Box<dyn Future<Output = Result<ListPromptsResult, PromptError>> + Send>> {
        let fut = (self.0)();
        Box::pin(async move {
            fut.await
                .into_prompts_list_result(base_ttl_ms, base_cache_scope)
        })
    }
}

impl<F, Fut, Res> IntoPromptsListHandler<()> for F
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoPromptsListResult + 'static,
{
    fn into_prompts_list_handler(self) -> Arc<dyn PromptsListHandler> {
        Arc::new(NoArgsPromptsListHandler(self))
    }
}

// 0 Extractors, 1 Arg (cursor)
struct CursorPromptsListHandler<F>(F);

impl<F, Fut, Res> PromptsListHandler for CursorPromptsListHandler<F>
where
    F: Fn(Option<String>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoPromptsListResult + 'static,
{
    fn call(
        &self,
        _ctx: RequestContext,
        cursor: Option<String>,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Pin<Box<dyn Future<Output = Result<ListPromptsResult, PromptError>> + Send>> {
        let fut = (self.0)(cursor);
        Box::pin(async move {
            fut.await
                .into_prompts_list_result(base_ttl_ms, base_cache_scope)
        })
    }
}

impl<F, Fut, Res> IntoPromptsListHandler<(Option<String>,)> for F
where
    F: Fn(Option<String>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoPromptsListResult + 'static,
{
    fn into_prompts_list_handler(self) -> Arc<dyn PromptsListHandler> {
        Arc::new(CursorPromptsListHandler(self))
    }
}

macro_rules! impl_into_prompts_list_handler {
    ($($E:ident),+) => {
        #[allow(non_snake_case)]
        impl<F, Fut, $($E,)+ Res> IntoPromptsListHandler<($($E,)+ ())> for F
        where
            $($E: FromRequestContext + Send + 'static,)+
            F: Fn($($E),+) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Res> + Send + 'static,
            Res: IntoPromptsListResult + 'static,
        {
            fn into_prompts_list_handler(self) -> Arc<dyn PromptsListHandler> {
                struct Handler<F, M>(F, std::marker::PhantomData<fn() -> M>);

                impl<F, Fut, $($E,)+ Res> PromptsListHandler for Handler<F, (Fut, $($E,)+ Res)>
                where
                    $($E: FromRequestContext + Send + 'static,)+
                    F: Fn($($E),+) -> Fut + Send + Sync + 'static,
                    Fut: Future<Output = Res> + Send + 'static,
                    Res: IntoPromptsListResult + 'static,
                {
                    fn call(
                        &self,
                        ctx: RequestContext,
                        _cursor: Option<String>,
                        base_ttl_ms: Option<u64>,
                        base_cache_scope: Option<CacheScope>,
                    ) -> Pin<Box<dyn Future<Output = Result<ListPromptsResult, PromptError>> + Send>> {
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
                        Box::pin(async move { fut.await.into_prompts_list_result(base_ttl_ms, base_cache_scope) })
                    }
                }
                Arc::new(Handler(self, std::marker::PhantomData))
            }
        }

        #[allow(non_snake_case)]
        impl<F, Fut, $($E,)+ Res> IntoPromptsListHandler<($($E,)+ (Option<String>,))> for F
        where
            $($E: FromRequestContext + Send + 'static,)+
            F: Fn($($E,)+ Option<String>) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Res> + Send + 'static,
            Res: IntoPromptsListResult + 'static,
        {
            fn into_prompts_list_handler(self) -> Arc<dyn PromptsListHandler> {
                struct Handler<F, M>(F, std::marker::PhantomData<fn() -> M>);

                impl<F, Fut, $($E,)+ Res> PromptsListHandler for Handler<F, (Fut, $($E,)+ Res)>
                where
                    $($E: FromRequestContext + Send + 'static,)+
                    F: Fn($($E,)+ Option<String>) -> Fut + Send + Sync + 'static,
                    Fut: Future<Output = Res> + Send + 'static,
                    Res: IntoPromptsListResult + 'static,
                {
                    fn call(
                        &self,
                        ctx: RequestContext,
                        cursor: Option<String>,
                        base_ttl_ms: Option<u64>,
                        base_cache_scope: Option<CacheScope>,
                    ) -> Pin<Box<dyn Future<Output = Result<ListPromptsResult, PromptError>> + Send>> {
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
                        let fut = (self.0)($($E,)+ cursor);
                        Box::pin(async move { fut.await.into_prompts_list_result(base_ttl_ms, base_cache_scope) })
                    }
                }
                Arc::new(Handler(self, std::marker::PhantomData))
            }
        }
    };
}

impl_into_prompts_list_handler!(E1);
impl_into_prompts_list_handler!(E1, E2);
impl_into_prompts_list_handler!(E1, E2, E3);
impl_into_prompts_list_handler!(E1, E2, E3, E4);
impl_into_prompts_list_handler!(E1, E2, E3, E4, E5);

/// Handles an MCP `prompts/list` request by constructing a [`ListPromptsResultResponse`] with the registered prompts.
pub fn handle_list_prompts(
    req: ListPromptsRequest,
    prompts: Vec<Prompt>,
    ttl_ms: Option<u64>,
    cache_scope: Option<CacheScope>,
) -> ListPromptsResultResponse {
    ListPromptsResultResponse::new(
        req.id,
        ListPromptsResult {
            meta: None,
            result_type: Some("complete".to_string()),
            next_cursor: None,
            ttl_ms,
            cache_scope,
            prompts,
            extras: HashMap::new(),
        },
    )
}
