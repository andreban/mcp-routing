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
        ResourceTemplate,
        templates::{
            ListResourceTemplatesRequest, ListResourceTemplatesResult,
            ListResourceTemplatesResultResponse,
        },
    },
};

/// Trait for types that can be converted into a [`ListResourceTemplatesResult`].
pub trait IntoResourceTemplatesListResult: Send {
    fn into_resource_templates_list_result(
        self,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ListResourceTemplatesResult, ResourceError>;
}

impl IntoResourceTemplatesListResult for ListResourceTemplatesResult {
    fn into_resource_templates_list_result(
        self,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ListResourceTemplatesResult, ResourceError> {
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

impl IntoResourceTemplatesListResult for Vec<ResourceTemplate> {
    fn into_resource_templates_list_result(
        self,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ListResourceTemplatesResult, ResourceError> {
        Ok(ListResourceTemplatesResult {
            meta: None,
            result_type: Some("complete".to_string()),
            next_cursor: None,
            ttl_ms: base_ttl_ms,
            cache_scope: base_cache_scope,
            resource_templates: self,
            extras: HashMap::new(),
        })
    }
}

impl<T, E> IntoResourceTemplatesListResult for Result<T, E>
where
    T: IntoResourceTemplatesListResult,
    E: std::fmt::Display + Send,
{
    fn into_resource_templates_list_result(
        self,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ListResourceTemplatesResult, ResourceError> {
        match self {
            Ok(val) => val.into_resource_templates_list_result(base_ttl_ms, base_cache_scope),
            Err(err) => Err(ResourceError::Internal(err.to_string())),
        }
    }
}

/// An erased handler trait for generating the list of available resource templates with request context.
pub trait ResourceTemplatesListHandler: Send + Sync {
    fn call(
        &self,
        ctx: RequestContext,
        cursor: Option<String>,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Pin<Box<dyn Future<Output = Result<ListResourceTemplatesResult, ResourceError>> + Send>>;
}

/// Trait for converting handler functions into a boxed [`ResourceTemplatesListHandler`].
pub trait IntoResourceTemplatesListHandler<T>: Send + Sync + 'static {
    fn into_resource_templates_list_handler(self) -> Arc<dyn ResourceTemplatesListHandler>;
}

// 0 Extractors, 0 Args
struct NoArgsResourceTemplatesListHandler<F>(F);

impl<F, Fut, Res> ResourceTemplatesListHandler for NoArgsResourceTemplatesListHandler<F>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoResourceTemplatesListResult + 'static,
{
    fn call(
        &self,
        _ctx: RequestContext,
        _cursor: Option<String>,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Pin<Box<dyn Future<Output = Result<ListResourceTemplatesResult, ResourceError>> + Send>> {
        let fut = (self.0)();
        Box::pin(async move {
            fut.await
                .into_resource_templates_list_result(base_ttl_ms, base_cache_scope)
        })
    }
}

impl<F, Fut, Res> IntoResourceTemplatesListHandler<()> for F
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoResourceTemplatesListResult + 'static,
{
    fn into_resource_templates_list_handler(self) -> Arc<dyn ResourceTemplatesListHandler> {
        Arc::new(NoArgsResourceTemplatesListHandler(self))
    }
}

// 0 Extractors, 1 Arg (cursor)
struct CursorResourceTemplatesListHandler<F>(F);

impl<F, Fut, Res> ResourceTemplatesListHandler for CursorResourceTemplatesListHandler<F>
where
    F: Fn(Option<String>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoResourceTemplatesListResult + 'static,
{
    fn call(
        &self,
        _ctx: RequestContext,
        cursor: Option<String>,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Pin<Box<dyn Future<Output = Result<ListResourceTemplatesResult, ResourceError>> + Send>> {
        let fut = (self.0)(cursor);
        Box::pin(async move {
            fut.await
                .into_resource_templates_list_result(base_ttl_ms, base_cache_scope)
        })
    }
}

impl<F, Fut, Res> IntoResourceTemplatesListHandler<(Option<String>,)> for F
where
    F: Fn(Option<String>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoResourceTemplatesListResult + 'static,
{
    fn into_resource_templates_list_handler(self) -> Arc<dyn ResourceTemplatesListHandler> {
        Arc::new(CursorResourceTemplatesListHandler(self))
    }
}

macro_rules! impl_into_resource_templates_list_handler {
    ($($E:ident),+) => {
        #[allow(non_snake_case)]
        impl<F, Fut, $($E,)+ Res> IntoResourceTemplatesListHandler<($($E,)+ ())> for F
        where
            $($E: FromRequestContext + Send + 'static,)+
            F: Fn($($E),+) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Res> + Send + 'static,
            Res: IntoResourceTemplatesListResult + 'static,
        {
            fn into_resource_templates_list_handler(self) -> Arc<dyn ResourceTemplatesListHandler> {
                struct Handler<F, M>(F, std::marker::PhantomData<fn() -> M>);

                impl<F, Fut, $($E,)+ Res> ResourceTemplatesListHandler for Handler<F, (Fut, $($E,)+ Res)>
                where
                    $($E: FromRequestContext + Send + 'static,)+
                    F: Fn($($E),+) -> Fut + Send + Sync + 'static,
                    Fut: Future<Output = Res> + Send + 'static,
                    Res: IntoResourceTemplatesListResult + 'static,
                {
                    fn call(
                        &self,
                        ctx: RequestContext,
                        _cursor: Option<String>,
                        base_ttl_ms: Option<u64>,
                        base_cache_scope: Option<CacheScope>,
                    ) -> Pin<Box<dyn Future<Output = Result<ListResourceTemplatesResult, ResourceError>> + Send>> {
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
                        Box::pin(async move { fut.await.into_resource_templates_list_result(base_ttl_ms, base_cache_scope) })
                    }
                }
                Arc::new(Handler(self, std::marker::PhantomData))
            }
        }

        #[allow(non_snake_case)]
        impl<F, Fut, $($E,)+ Res> IntoResourceTemplatesListHandler<($($E,)+ (Option<String>,))> for F
        where
            $($E: FromRequestContext + Send + 'static,)+
            F: Fn($($E,)+ Option<String>) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Res> + Send + 'static,
            Res: IntoResourceTemplatesListResult + 'static,
        {
            fn into_resource_templates_list_handler(self) -> Arc<dyn ResourceTemplatesListHandler> {
                struct Handler<F, M>(F, std::marker::PhantomData<fn() -> M>);

                impl<F, Fut, $($E,)+ Res> ResourceTemplatesListHandler for Handler<F, (Fut, $($E,)+ Option<String>, Res)>
                where
                    $($E: FromRequestContext + Send + 'static,)+
                    F: Fn($($E,)+ Option<String>) -> Fut + Send + Sync + 'static,
                    Fut: Future<Output = Res> + Send + 'static,
                    Res: IntoResourceTemplatesListResult + 'static,
                {
                    fn call(
                        &self,
                        ctx: RequestContext,
                        cursor: Option<String>,
                        base_ttl_ms: Option<u64>,
                        base_cache_scope: Option<CacheScope>,
                    ) -> Pin<Box<dyn Future<Output = Result<ListResourceTemplatesResult, ResourceError>> + Send>> {
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
                        Box::pin(async move { fut.await.into_resource_templates_list_result(base_ttl_ms, base_cache_scope) })
                    }
                }
                Arc::new(Handler(self, std::marker::PhantomData))
            }
        }
    };
}

impl_into_resource_templates_list_handler!(E1);
impl_into_resource_templates_list_handler!(E1, E2);
impl_into_resource_templates_list_handler!(E1, E2, E3);
impl_into_resource_templates_list_handler!(E1, E2, E3, E4);
impl_into_resource_templates_list_handler!(E1, E2, E3, E4, E5);

/// Handles an MCP `resources/templates/list` request by constructing a [`ListResourceTemplatesResultResponse`] with the registered resource templates.
pub fn handle_list_resource_templates(
    req: ListResourceTemplatesRequest,
    resource_templates: Vec<ResourceTemplate>,
    ttl_ms: Option<u64>,
    cache_scope: Option<CacheScope>,
) -> ListResourceTemplatesResultResponse {
    ListResourceTemplatesResultResponse::new(
        req.id,
        ListResourceTemplatesResult {
            meta: None,
            result_type: Some("complete".to_string()),
            next_cursor: None,
            ttl_ms,
            cache_scope,
            resource_templates,
            extras: HashMap::new(),
        },
    )
}
