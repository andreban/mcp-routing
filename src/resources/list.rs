// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::extract::{FromRequestContext, RequestContext};
use crate::resources::ResourceError;
use crate::types::mcp::{
    CacheScope,
    resources::{
        Resource,
        list::{ListResourcesRequest, ListResourcesResult, ListResourcesResultResponse},
    },
};

/// Trait for types that can be converted into a [`ListResourcesResult`].
pub trait IntoResourcesListResult: Send {
    fn into_resources_list_result(
        self,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ListResourcesResult, ResourceError>;
}

impl IntoResourcesListResult for ListResourcesResult {
    fn into_resources_list_result(
        self,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ListResourcesResult, ResourceError> {
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

impl IntoResourcesListResult for Vec<Resource> {
    fn into_resources_list_result(
        self,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ListResourcesResult, ResourceError> {
        Ok(ListResourcesResult {
            meta: None,
            result_type: Some("complete".to_string()),
            next_cursor: None,
            ttl_ms: base_ttl_ms,
            cache_scope: base_cache_scope,
            resources: self,
            extras: HashMap::new(),
        })
    }
}

impl<T, E> IntoResourcesListResult for Result<T, E>
where
    T: IntoResourcesListResult,
    E: std::fmt::Display + Send,
{
    fn into_resources_list_result(
        self,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ListResourcesResult, ResourceError> {
        match self {
            Ok(val) => val.into_resources_list_result(base_ttl_ms, base_cache_scope),
            Err(err) => Err(ResourceError::Internal(err.to_string())),
        }
    }
}

/// An erased handler trait for generating the list of available direct resources with request context.
pub trait ResourcesListHandler: Send + Sync {
    fn call(
        &self,
        ctx: RequestContext,
        cursor: Option<String>,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Pin<Box<dyn Future<Output = Result<ListResourcesResult, ResourceError>> + Send>>;
}

/// Trait for converting handler functions into a boxed [`ResourcesListHandler`].
pub trait IntoResourcesListHandler<T>: Send + Sync + 'static {
    fn into_resources_list_handler(self) -> Arc<dyn ResourcesListHandler>;
}

// 0 Extractors, 0 Args
struct NoArgsResourcesListHandler<F>(F);

impl<F, Fut, Res> ResourcesListHandler for NoArgsResourcesListHandler<F>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoResourcesListResult + 'static,
{
    fn call(
        &self,
        _ctx: RequestContext,
        _cursor: Option<String>,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Pin<Box<dyn Future<Output = Result<ListResourcesResult, ResourceError>> + Send>> {
        let fut = (self.0)();
        Box::pin(async move {
            fut.await
                .into_resources_list_result(base_ttl_ms, base_cache_scope)
        })
    }
}

impl<F, Fut, Res> IntoResourcesListHandler<()> for F
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoResourcesListResult + 'static,
{
    fn into_resources_list_handler(self) -> Arc<dyn ResourcesListHandler> {
        Arc::new(NoArgsResourcesListHandler(self))
    }
}

// 0 Extractors, 1 Arg (cursor)
struct CursorResourcesListHandler<F>(F);

impl<F, Fut, Res> ResourcesListHandler for CursorResourcesListHandler<F>
where
    F: Fn(Option<String>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoResourcesListResult + 'static,
{
    fn call(
        &self,
        _ctx: RequestContext,
        cursor: Option<String>,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Pin<Box<dyn Future<Output = Result<ListResourcesResult, ResourceError>> + Send>> {
        let fut = (self.0)(cursor);
        Box::pin(async move {
            fut.await
                .into_resources_list_result(base_ttl_ms, base_cache_scope)
        })
    }
}

impl<F, Fut, Res> IntoResourcesListHandler<(Option<String>,)> for F
where
    F: Fn(Option<String>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoResourcesListResult + 'static,
{
    fn into_resources_list_handler(self) -> Arc<dyn ResourcesListHandler> {
        Arc::new(CursorResourcesListHandler(self))
    }
}

macro_rules! impl_into_resources_list_handler {
    ($($E:ident),+) => {
        #[allow(non_snake_case)]
        impl<F, Fut, $($E,)+ Res> IntoResourcesListHandler<($($E,)+ ())> for F
        where
            $($E: FromRequestContext + Send + 'static,)+
            F: Fn($($E),+) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Res> + Send + 'static,
            Res: IntoResourcesListResult + 'static,
        {
            fn into_resources_list_handler(self) -> Arc<dyn ResourcesListHandler> {
                struct Handler<F, M>(F, std::marker::PhantomData<fn() -> M>);

                impl<F, Fut, $($E,)+ Res> ResourcesListHandler for Handler<F, (Fut, $($E,)+ Res)>
                where
                    $($E: FromRequestContext + Send + 'static,)+
                    F: Fn($($E),+) -> Fut + Send + Sync + 'static,
                    Fut: Future<Output = Res> + Send + 'static,
                    Res: IntoResourcesListResult + 'static,
                {
                    fn call(
                        &self,
                        ctx: RequestContext,
                        _cursor: Option<String>,
                        base_ttl_ms: Option<u64>,
                        base_cache_scope: Option<CacheScope>,
                    ) -> Pin<Box<dyn Future<Output = Result<ListResourcesResult, ResourceError>> + Send>> {
                        $(
                            let $E = match $E::from_request_context(&ctx) {
                                Ok(val) => val,
                                Err(err) => {
                                    return Box::pin(async move {
                                        Err(ResourceError::InvalidParams(format!("Extraction error: {err}")))
                                    });
                                }
                            };
                        )+
                        let fut = (self.0)($($E),+);
                        Box::pin(async move { fut.await.into_resources_list_result(base_ttl_ms, base_cache_scope) })
                    }
                }
                Arc::new(Handler(self, std::marker::PhantomData))
            }
        }

        #[allow(non_snake_case)]
        impl<F, Fut, $($E,)+ Res> IntoResourcesListHandler<($($E,)+ (Option<String>,))> for F
        where
            $($E: FromRequestContext + Send + 'static,)+
            F: Fn($($E,)+ Option<String>) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Res> + Send + 'static,
            Res: IntoResourcesListResult + 'static,
        {
            fn into_resources_list_handler(self) -> Arc<dyn ResourcesListHandler> {
                struct Handler<F, M>(F, std::marker::PhantomData<fn() -> M>);

                impl<F, Fut, $($E,)+ Res> ResourcesListHandler for Handler<F, (Fut, $($E,)+ Option<String>, Res)>
                where
                    $($E: FromRequestContext + Send + 'static,)+
                    F: Fn($($E,)+ Option<String>) -> Fut + Send + Sync + 'static,
                    Fut: Future<Output = Res> + Send + 'static,
                    Res: IntoResourcesListResult + 'static,
                {
                    fn call(
                        &self,
                        ctx: RequestContext,
                        cursor: Option<String>,
                        base_ttl_ms: Option<u64>,
                        base_cache_scope: Option<CacheScope>,
                    ) -> Pin<Box<dyn Future<Output = Result<ListResourcesResult, ResourceError>> + Send>> {
                        $(
                            let $E = match $E::from_request_context(&ctx) {
                                Ok(val) => val,
                                Err(err) => {
                                    return Box::pin(async move {
                                        Err(ResourceError::InvalidParams(format!("Extraction error: {err}")))
                                    });
                                }
                            };
                        )+
                        let fut = (self.0)($($E,)+ cursor);
                        Box::pin(async move { fut.await.into_resources_list_result(base_ttl_ms, base_cache_scope) })
                    }
                }
                Arc::new(Handler(self, std::marker::PhantomData))
            }
        }
    };
}

impl_into_resources_list_handler!(E1);
impl_into_resources_list_handler!(E1, E2);
impl_into_resources_list_handler!(E1, E2, E3);
impl_into_resources_list_handler!(E1, E2, E3, E4);
impl_into_resources_list_handler!(E1, E2, E3, E4, E5);

/// Handles an MCP `resources/list` request by constructing a [`ListResourcesResultResponse`] with the registered resources.
pub fn handle_list_resources(
    req: ListResourcesRequest,
    resources: Vec<Resource>,
    ttl_ms: Option<u64>,
    cache_scope: Option<CacheScope>,
) -> ListResourcesResultResponse {
    ListResourcesResultResponse::new(
        req.id,
        ListResourcesResult {
            meta: None,
            result_type: Some("complete".to_string()),
            next_cursor: None,
            ttl_ms,
            cache_scope,
            resources,
            extras: HashMap::new(),
        },
    )
}
