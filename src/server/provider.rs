// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::extract::{FromRequestContext, RequestContext};
use crate::types::mcp::{ServerCapabilities, server::discover::ServerDiscoverResult};

/// Error type encountered during dynamic discovery execution or parameter validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryError {
    /// Invalid parameters or request state.
    InvalidParams(String),
    /// Internal execution or business logic error.
    Internal(String),
}

impl std::fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscoveryError::InvalidParams(msg) => write!(f, "Invalid params: {msg}"),
            DiscoveryError::Internal(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for DiscoveryError {}

/// Trait for types that can be converted into dynamic discovery output to populate [`ServerDiscoverResult`].
pub trait IntoServerDiscoveryResult: Send {
    fn apply_to_discover_result(
        self,
        base_result: ServerDiscoverResult,
    ) -> Result<ServerDiscoverResult, DiscoveryError>;
}

impl IntoServerDiscoveryResult for ServerDiscoverResult {
    fn apply_to_discover_result(
        self,
        base_result: ServerDiscoverResult,
    ) -> Result<ServerDiscoverResult, DiscoveryError> {
        let mut res = self;
        if res
            .meta
            .as_ref()
            .and_then(|m| m.server_info.as_ref())
            .is_none()
            && let Some(base_meta) = base_result.meta
        {
            if let Some(res_meta) = res.meta.as_mut() {
                res_meta.server_info = base_meta.server_info;
            } else {
                res.meta = Some(base_meta);
            }
        }
        if res.supported_versions.is_empty() {
            res.supported_versions = base_result.supported_versions;
        }
        if res.result_type.is_none() {
            res.result_type = base_result.result_type;
        }
        if res.ttl_ms.is_none() {
            res.ttl_ms = base_result.ttl_ms;
        }
        if res.cache_scope.is_none() {
            res.cache_scope = base_result.cache_scope;
        }
        Ok(res)
    }
}

impl IntoServerDiscoveryResult for crate::types::mcp::InputRequiredResult {
    fn apply_to_discover_result(
        self,
        base_result: ServerDiscoverResult,
    ) -> Result<ServerDiscoverResult, DiscoveryError> {
        let (meta, result_type, extras) = self.into_parts();
        Ok(ServerDiscoverResult {
            meta: meta.or(base_result.meta),
            result_type: Some(result_type),
            supported_versions: base_result.supported_versions,
            capabilities: base_result.capabilities,
            instructions: None,
            ttl_ms: base_result.ttl_ms,
            cache_scope: base_result.cache_scope,
            extras,
        })
    }
}

impl IntoServerDiscoveryResult for ServerCapabilities {
    fn apply_to_discover_result(
        self,
        mut base_result: ServerDiscoverResult,
    ) -> Result<ServerDiscoverResult, DiscoveryError> {
        base_result.capabilities = self;
        Ok(base_result)
    }
}

impl IntoServerDiscoveryResult for (ServerCapabilities, String) {
    fn apply_to_discover_result(
        self,
        mut base_result: ServerDiscoverResult,
    ) -> Result<ServerDiscoverResult, DiscoveryError> {
        base_result.capabilities = self.0;
        base_result.instructions = Some(self.1);
        Ok(base_result)
    }
}

impl IntoServerDiscoveryResult for (ServerCapabilities, Option<String>) {
    fn apply_to_discover_result(
        self,
        mut base_result: ServerDiscoverResult,
    ) -> Result<ServerDiscoverResult, DiscoveryError> {
        base_result.capabilities = self.0;
        base_result.instructions = self.1;
        Ok(base_result)
    }
}

impl IntoServerDiscoveryResult for String {
    fn apply_to_discover_result(
        self,
        mut base_result: ServerDiscoverResult,
    ) -> Result<ServerDiscoverResult, DiscoveryError> {
        base_result.instructions = Some(self);
        Ok(base_result)
    }
}

impl IntoServerDiscoveryResult for &str {
    fn apply_to_discover_result(
        self,
        mut base_result: ServerDiscoverResult,
    ) -> Result<ServerDiscoverResult, DiscoveryError> {
        base_result.instructions = Some(self.to_string());
        Ok(base_result)
    }
}

impl IntoServerDiscoveryResult for Option<String> {
    fn apply_to_discover_result(
        self,
        mut base_result: ServerDiscoverResult,
    ) -> Result<ServerDiscoverResult, DiscoveryError> {
        base_result.instructions = self;
        Ok(base_result)
    }
}

impl<T, E> IntoServerDiscoveryResult for Result<T, E>
where
    T: IntoServerDiscoveryResult,
    E: std::fmt::Display + Send,
{
    fn apply_to_discover_result(
        self,
        base_result: ServerDiscoverResult,
    ) -> Result<ServerDiscoverResult, DiscoveryError> {
        match self {
            Ok(val) => val.apply_to_discover_result(base_result),
            Err(err) => Err(DiscoveryError::Internal(err.to_string())),
        }
    }
}

/// An erased discovery handler trait for dynamically generating server discovery metadata.
pub trait ServerDiscoveryHandler: Send + Sync {
    fn call(
        &self,
        ctx: RequestContext,
        base_result: ServerDiscoverResult,
    ) -> Pin<Box<dyn Future<Output = Result<ServerDiscoverResult, DiscoveryError>> + Send>>;
}

/// Trait for converting handler functions into a boxed [`ServerDiscoveryHandler`].
pub trait IntoServerDiscoveryHandler<T>: Send + Sync + 'static {
    fn into_discovery_handler(self) -> Arc<dyn ServerDiscoveryHandler>;
}

// 0 Extractors
struct NoArgsDiscoveryHandler<F>(F);

impl<F, Fut, Res> ServerDiscoveryHandler for NoArgsDiscoveryHandler<F>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoServerDiscoveryResult + 'static,
{
    fn call(
        &self,
        _ctx: RequestContext,
        base_result: ServerDiscoverResult,
    ) -> Pin<Box<dyn Future<Output = Result<ServerDiscoverResult, DiscoveryError>> + Send>> {
        let fut = (self.0)();
        Box::pin(async move { fut.await.apply_to_discover_result(base_result) })
    }
}

impl<F, Fut, Res> IntoServerDiscoveryHandler<()> for F
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoServerDiscoveryResult + 'static,
{
    fn into_discovery_handler(self) -> Arc<dyn ServerDiscoveryHandler> {
        Arc::new(NoArgsDiscoveryHandler(self))
    }
}

macro_rules! impl_into_discovery_handler {
    ($($E:ident),+) => {
        #[allow(non_snake_case)]
        impl<F, Fut, $($E,)+ Res> IntoServerDiscoveryHandler<($($E,)+)> for F
        where
            $($E: FromRequestContext + Send + 'static,)+
            F: Fn($($E),+) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Res> + Send + 'static,
            Res: IntoServerDiscoveryResult + 'static,
        {
            fn into_discovery_handler(self) -> Arc<dyn ServerDiscoveryHandler> {
                struct Handler<F, M>(F, std::marker::PhantomData<fn() -> M>);

                impl<F, Fut, $($E,)+ Res> ServerDiscoveryHandler for Handler<F, (Fut, $($E,)+ Res)>
                where
                    $($E: FromRequestContext + Send + 'static,)+
                    F: Fn($($E),+) -> Fut + Send + Sync + 'static,
                    Fut: Future<Output = Res> + Send + 'static,
                    Res: IntoServerDiscoveryResult + 'static,
                {
                    fn call(
                        &self,
                        ctx: RequestContext,
                        base_result: ServerDiscoverResult,
                    ) -> Pin<Box<dyn Future<Output = Result<ServerDiscoverResult, DiscoveryError>> + Send>> {
                        $(
                            let $E = match $E::from_request_context(&ctx) {
                                Ok(val) => val,
                                Err(err) => {
                                    return Box::pin(async move {
                                        Err(DiscoveryError::InvalidParams(format!("Extraction error: {err}")))
                                    });
                                }
                            };
                        )+
                        let fut = (self.0)($($E),+);
                        Box::pin(async move { fut.await.apply_to_discover_result(base_result) })
                    }
                }
                Arc::new(Handler(self, std::marker::PhantomData))
            }
        }
    };
}

impl_into_discovery_handler!(E1);
impl_into_discovery_handler!(E1, E2);
impl_into_discovery_handler!(E1, E2, E3);
impl_into_discovery_handler!(E1, E2, E3, E4);
impl_into_discovery_handler!(E1, E2, E3, E4, E5);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::{Extension, Meta};
    use crate::types::mcp::{CacheScope, Implementation, ToolsCapability};

    /// Tests invoking discovery provider handlers with extractors (`Extension`, `Meta`).
    #[tokio::test]
    async fn test_discovery_provider_with_extractors() {
        #[derive(Clone)]
        struct TenantContext {
            tenant_name: String,
        }

        async fn dynamic_provider(
            Extension(tenant): Extension<TenantContext>,
            Meta(meta): Meta,
        ) -> Result<ServerCapabilities, String> {
            let client = meta
                .client_info
                .as_ref()
                .map(|c| c.name.as_str())
                .unwrap_or("unknown");
            assert_eq!(tenant.tenant_name, "acme-corp");
            assert_eq!(client, "client-x");

            Ok(ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(true),
                }),
                resources: None,
                prompts: None,
                completions: None,
                logging: None,
                experimental: None,
                extensions: None,
            })
        }

        let handler = dynamic_provider.into_discovery_handler();

        let mut exts = http::Extensions::new();
        exts.insert(TenantContext {
            tenant_name: "acme-corp".to_string(),
        });

        let ctx = RequestContext::new(
            Some(crate::types::mcp::RequestMetaObject {
                client_info: Some(Implementation::new("client-x", "1.0")),
                client_capabilities: None,
                protocol_version: None,
                progress_token: None,
                log_level: None,
                subscription_id: None,
                extra: std::collections::HashMap::new(),
            }),
            http::HeaderMap::new(),
            Arc::new(exts),
        );

        let base =
            ServerDiscoverResult::new(ServerCapabilities::empty(), vec!["2026-07-28".to_string()])
                .with_cache(Some(5000), Some(CacheScope::Private));

        let res = handler.call(ctx, base).await.unwrap();
        assert_eq!(res.capabilities.tools.unwrap().list_changed, Some(true));
        assert_eq!(res.ttl_ms, Some(5000));
        assert!(matches!(res.cache_scope, Some(CacheScope::Private)));
    }
}
