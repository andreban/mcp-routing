// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Handler traits and adapter implementations for MCP resource reading.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::extract::{FromRequestContext, RequestContext};
use crate::types::mcp::{CacheScope, resources::read::ReadResourceResult};

use super::{IntoResourceResult, ResourceError};

/// An erased resource handler trait for executing a resource read request with request context and target URI.
pub trait ResourceHandler: Send + Sync {
    fn call(
        &self,
        ctx: RequestContext,
        uri: String,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Pin<Box<dyn Future<Output = Result<ReadResourceResult, ResourceError>> + Send>>;
}

/// Trait for converting handler functions into a boxed [`ResourceHandler`].
pub trait IntoResourceHandler<T>: Send + Sync + 'static {
    fn into_resource_handler(self) -> Arc<dyn ResourceHandler>;
}

// 0 Extractors, 0 Args
struct NoArgsResourceHandler<F>(F);

impl<F, Fut, Res> ResourceHandler for NoArgsResourceHandler<F>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoResourceResult + 'static,
{
    fn call(
        &self,
        _ctx: RequestContext,
        uri: String,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Pin<Box<dyn Future<Output = Result<ReadResourceResult, ResourceError>> + Send>> {
        let fut = (self.0)();
        Box::pin(async move {
            fut.await
                .into_resource_result(&uri, base_ttl_ms, base_cache_scope)
        })
    }
}

impl<F, Fut, Res> IntoResourceHandler<()> for F
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoResourceResult + 'static,
{
    fn into_resource_handler(self) -> Arc<dyn ResourceHandler> {
        Arc::new(NoArgsResourceHandler(self))
    }
}

// 0 Extractors, 1 Arg (uri: String)
struct UriResourceHandler<F>(F);

impl<F, Fut, Res> ResourceHandler for UriResourceHandler<F>
where
    F: Fn(String) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoResourceResult + 'static,
{
    fn call(
        &self,
        _ctx: RequestContext,
        uri: String,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Pin<Box<dyn Future<Output = Result<ReadResourceResult, ResourceError>> + Send>> {
        let fut = (self.0)(uri.clone());
        Box::pin(async move {
            fut.await
                .into_resource_result(&uri, base_ttl_ms, base_cache_scope)
        })
    }
}

impl<F, Fut, Res> IntoResourceHandler<(String,)> for F
where
    F: Fn(String) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoResourceResult + 'static,
{
    fn into_resource_handler(self) -> Arc<dyn ResourceHandler> {
        Arc::new(UriResourceHandler(self))
    }
}

macro_rules! impl_into_resource_handler {
    ($($E:ident),+) => {
        #[allow(non_snake_case)]
        impl<F, Fut, $($E,)+ Res> IntoResourceHandler<($($E,)+ ())> for F
        where
            $($E: FromRequestContext + Send + 'static,)+
            F: Fn($($E),+) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Res> + Send + 'static,
            Res: IntoResourceResult + 'static,
        {
            fn into_resource_handler(self) -> Arc<dyn ResourceHandler> {
                struct Handler<F, M>(F, std::marker::PhantomData<fn() -> M>);

                impl<F, Fut, $($E,)+ Res> ResourceHandler for Handler<F, (Fut, $($E,)+ Res)>
                where
                    $($E: FromRequestContext + Send + 'static,)+
                    F: Fn($($E),+) -> Fut + Send + Sync + 'static,
                    Fut: Future<Output = Res> + Send + 'static,
                    Res: IntoResourceResult + 'static,
                {
                    fn call(
                        &self,
                        ctx: RequestContext,
                        uri: String,
                        base_ttl_ms: Option<u64>,
                        base_cache_scope: Option<CacheScope>,
                    ) -> Pin<Box<dyn Future<Output = Result<ReadResourceResult, ResourceError>> + Send>> {
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
                        Box::pin(async move { fut.await.into_resource_result(&uri, base_ttl_ms, base_cache_scope) })
                    }
                }
                Arc::new(Handler(self, std::marker::PhantomData))
            }
        }

        #[allow(non_snake_case)]
        impl<F, Fut, $($E,)+ Res> IntoResourceHandler<($($E,)+ (String,))> for F
        where
            $($E: FromRequestContext + Send + 'static,)+
            F: Fn($($E,)+ String) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Res> + Send + 'static,
            Res: IntoResourceResult + 'static,
        {
            fn into_resource_handler(self) -> Arc<dyn ResourceHandler> {
                struct Handler<F, M>(F, std::marker::PhantomData<fn() -> M>);

                impl<F, Fut, $($E,)+ Res> ResourceHandler for Handler<F, (Fut, $($E,)+ Res)>
                where
                    $($E: FromRequestContext + Send + 'static,)+
                    F: Fn($($E,)+ String) -> Fut + Send + Sync + 'static,
                    Fut: Future<Output = Res> + Send + 'static,
                    Res: IntoResourceResult + 'static,
                {
                    fn call(
                        &self,
                        ctx: RequestContext,
                        uri: String,
                        base_ttl_ms: Option<u64>,
                        base_cache_scope: Option<CacheScope>,
                    ) -> Pin<Box<dyn Future<Output = Result<ReadResourceResult, ResourceError>> + Send>> {
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
                        let fut = (self.0)($($E,)+ uri.clone());
                        Box::pin(async move { fut.await.into_resource_result(&uri, base_ttl_ms, base_cache_scope) })
                    }
                }
                Arc::new(Handler(self, std::marker::PhantomData))
            }
        }
    };
}

impl_into_resource_handler!(E1);
impl_into_resource_handler!(E1, E2);
impl_into_resource_handler!(E1, E2, E3);
impl_into_resource_handler!(E1, E2, E3, E4);
impl_into_resource_handler!(E1, E2, E3, E4, E5);

#[cfg(test)]
mod tests {
    //! Unit tests for resource handler invocation with extractors.

    use super::*;
    use crate::extract::Extension;
    use crate::types::mcp::resources::ResourceContents;

    /// Tests invoking resource handlers with extractors (`Extension`) and target URI parameter.
    #[tokio::test]
    async fn test_resource_handler_with_extractors_and_uri() {
        #[derive(Clone)]
        struct RootDir(String);

        async fn read_file(
            Extension(root): Extension<RootDir>,
            uri: String,
        ) -> Result<String, String> {
            Ok(format!("Content of {uri} under {}", root.0))
        }

        let handler = read_file.into_resource_handler();

        let mut ext = http::Extensions::new();
        ext.insert(RootDir("/var/data".to_string()));

        let ctx = RequestContext::new(None, http::HeaderMap::new(), Arc::new(ext));

        let result = handler
            .call(ctx, "file:///logs/app.log".to_string(), None, None)
            .await
            .unwrap();

        assert_eq!(result.contents.len(), 1);
        assert_eq!(result.ttl_ms, Some(0));
        assert_eq!(result.cache_scope, Some(CacheScope::Public));
        if let ResourceContents::Text(ref t) = result.contents[0] {
            assert_eq!(t.text, "Content of file:///logs/app.log under /var/data");
        } else {
            panic!("Expected text resource");
        }
    }
}
