// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::extract::{FromRequestContext, RequestContext};
use crate::types::mcp::{
    CacheScope,
    resources::{
        BlobResourceContents, ResourceContents, TextResourceContents,
        read::ReadResourceResult,
    },
};

pub mod list;
pub mod read;
pub mod registry;
pub mod templates;

pub use list::{IntoResourcesListHandler, IntoResourcesListResult, ResourcesListHandler};
pub use read::handle_read_resource;
pub use registry::ResourceRegistry;
pub use templates::{
    IntoResourceTemplatesListHandler, IntoResourceTemplatesListResult,
    ResourceTemplatesListHandler,
};

/// Error type encountered during resource reading or listing operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceError {
    /// Invalid arguments provided to the resource handler.
    InvalidParams(String),
    /// The requested resource was not found.
    NotFound(String),
    /// Internal execution or business logic error.
    Internal(String),
}

impl std::fmt::Display for ResourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceError::InvalidParams(msg) => write!(f, "Invalid params: {msg}"),
            ResourceError::NotFound(msg) => write!(f, "Resource not found: {msg}"),
            ResourceError::Internal(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for ResourceError {}

/// Trait for types that can be converted into a [`ReadResourceResult`].
pub trait IntoResourceResult: Send {
    fn into_resource_result(
        self,
        uri: &str,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ReadResourceResult, ResourceError>;
}

impl IntoResourceResult for ReadResourceResult {
    fn into_resource_result(
        self,
        _uri: &str,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ReadResourceResult, ResourceError> {
        let mut res = self;
        if res.ttl_ms.is_none() {
            res.ttl_ms = base_ttl_ms.or(Some(0));
        }
        if res.cache_scope.is_none() {
            res.cache_scope = base_cache_scope.or(Some(CacheScope::Public));
        }
        if res.result_type.is_none() {
            res.result_type = Some("complete".to_string());
        }
        Ok(res)
    }
}

impl IntoResourceResult for ResourceContents {
    fn into_resource_result(
        self,
        _uri: &str,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ReadResourceResult, ResourceError> {
        Ok(ReadResourceResult::new(vec![self])
            .with_cache(base_ttl_ms.or(Some(0)), base_cache_scope.or(Some(CacheScope::Public))))
    }
}

impl IntoResourceResult for Vec<ResourceContents> {
    fn into_resource_result(
        self,
        _uri: &str,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ReadResourceResult, ResourceError> {
        Ok(ReadResourceResult::new(self)
            .with_cache(base_ttl_ms.or(Some(0)), base_cache_scope.or(Some(CacheScope::Public))))
    }
}

impl IntoResourceResult for TextResourceContents {
    fn into_resource_result(
        self,
        _uri: &str,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ReadResourceResult, ResourceError> {
        Ok(ReadResourceResult::new(vec![ResourceContents::Text(self)])
            .with_cache(base_ttl_ms.or(Some(0)), base_cache_scope.or(Some(CacheScope::Public))))
    }
}

impl IntoResourceResult for BlobResourceContents {
    fn into_resource_result(
        self,
        _uri: &str,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ReadResourceResult, ResourceError> {
        Ok(ReadResourceResult::new(vec![ResourceContents::Blob(self)])
            .with_cache(base_ttl_ms.or(Some(0)), base_cache_scope.or(Some(CacheScope::Public))))
    }
}

impl IntoResourceResult for String {
    fn into_resource_result(
        self,
        uri: &str,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ReadResourceResult, ResourceError> {
        Ok(ReadResourceResult::text(uri, self, None::<String>)
            .with_cache(base_ttl_ms.or(Some(0)), base_cache_scope.or(Some(CacheScope::Public))))
    }
}

impl IntoResourceResult for &str {
    fn into_resource_result(
        self,
        uri: &str,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ReadResourceResult, ResourceError> {
        Ok(ReadResourceResult::text(uri, self.to_string(), None::<String>)
            .with_cache(base_ttl_ms.or(Some(0)), base_cache_scope.or(Some(CacheScope::Public))))
    }
}

impl<T, E> IntoResourceResult for Result<T, E>
where
    T: IntoResourceResult,
    E: std::fmt::Display + Send,
{
    fn into_resource_result(
        self,
        uri: &str,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ReadResourceResult, ResourceError> {
        match self {
            Ok(val) => val.into_resource_result(uri, base_ttl_ms, base_cache_scope),
            Err(err) => Err(ResourceError::Internal(err.to_string())),
        }
    }
}

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
        Box::pin(async move { fut.await.into_resource_result(&uri, base_ttl_ms, base_cache_scope) })
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
        Box::pin(async move { fut.await.into_resource_result(&uri, base_ttl_ms, base_cache_scope) })
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
    use super::*;
    use crate::extract::{Extension, SessionId};

    #[test]
    fn test_into_resource_result() {
        // String
        let res_str = "resource text".into_resource_result("file:///a.txt", None, None).unwrap();
        assert_eq!(res_str.contents.len(), 1);
        assert_eq!(res_str.ttl_ms, Some(0));
        assert_eq!(res_str.cache_scope, Some(CacheScope::Public));
        if let ResourceContents::Text(ref t) = res_str.contents[0] {
            assert_eq!(t.uri, "file:///a.txt");
            assert_eq!(t.text, "resource text");
        } else {
            panic!("Expected text resource");
        }

        // Result::Ok
        let res_ok: Result<&str, &str> = Ok("success content");
        let res = res_ok.into_resource_result("memo://1", Some(60000), Some(CacheScope::Private)).unwrap();
        assert_eq!(res.contents.len(), 1);
        assert_eq!(res.ttl_ms, Some(60000));
        assert_eq!(res.cache_scope, Some(CacheScope::Private));

        // Result::Err
        let res_err: Result<&str, &str> = Err("file not found");
        let err = res_err.into_resource_result("memo://1", None, None).unwrap_err();
        assert!(matches!(err, ResourceError::Internal(ref s) if s == "file not found"));
    }

    #[tokio::test]
    async fn test_resource_handler_with_extractors_and_uri() {
        #[derive(Clone)]
        struct RootDir(String);

        async fn read_file(
            session: SessionId,
            Extension(root): Extension<RootDir>,
            uri: String,
        ) -> Result<String, String> {
            Ok(format!("[{session}] Content of {uri} under {}", root.0))
        }

        let handler = read_file.into_resource_handler();

        let mut ext = http::Extensions::new();
        ext.insert(RootDir("/var/data".to_string()));

        let ctx = RequestContext::new(
            Some(SessionId::new("sess-res-1")),
            None,
            http::HeaderMap::new(),
            Arc::new(ext),
        );

        let result = handler
            .call(ctx, "file:///logs/app.log".to_string(), None, None)
            .await
            .unwrap();

        assert_eq!(result.contents.len(), 1);
        assert_eq!(result.ttl_ms, Some(0));
        assert_eq!(result.cache_scope, Some(CacheScope::Public));
        if let ResourceContents::Text(ref t) = result.contents[0] {
            assert_eq!(
                t.text,
                "[sess-res-1] Content of file:///logs/app.log under /var/data"
            );
        } else {
            panic!("Expected text resource");
        }
    }
}
