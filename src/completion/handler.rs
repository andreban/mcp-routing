// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Handler traits and adapter implementations for MCP completion.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::extract::{FromRequestContext, RequestContext};
use crate::types::mcp::completion::{
    CompleteArgument, CompleteContext, CompleteParams, CompleteResult, Reference,
};

use super::{CompletionError, IntoCompletionResult};

/// An erased completion handler trait for executing autocompletion requests.
pub trait CompletionHandler: Send + Sync {
    fn call(
        &self,
        ctx: RequestContext,
        params: CompleteParams,
    ) -> Pin<Box<dyn Future<Output = Result<CompleteResult, CompletionError>> + Send>>;
}

/// Trait for converting asynchronous functions into a boxed [`CompletionHandler`].
pub trait IntoCompletionHandler<T>: Send + Sync + 'static {
    fn into_completion_handler(self) -> Arc<dyn CompletionHandler>;
}

// 0 Extractors, 0 Args
struct NoArgsCompletionHandler<F>(F);

impl<F, Fut, Res> CompletionHandler for NoArgsCompletionHandler<F>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoCompletionResult + 'static,
{
    fn call(
        &self,
        _ctx: RequestContext,
        _params: CompleteParams,
    ) -> Pin<Box<dyn Future<Output = Result<CompleteResult, CompletionError>> + Send>> {
        let fut = (self.0)();
        Box::pin(async move { fut.await.into_completion_result() })
    }
}

impl<F, Fut, Res> IntoCompletionHandler<()> for F
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoCompletionResult + 'static,
{
    fn into_completion_handler(self) -> Arc<dyn CompletionHandler> {
        Arc::new(NoArgsCompletionHandler(self))
    }
}

// 0 Extractors, CompleteParams
struct ParamsCompletionHandler<F>(F);

impl<F, Fut, Res> CompletionHandler for ParamsCompletionHandler<F>
where
    F: Fn(CompleteParams) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoCompletionResult + 'static,
{
    fn call(
        &self,
        _ctx: RequestContext,
        params: CompleteParams,
    ) -> Pin<Box<dyn Future<Output = Result<CompleteResult, CompletionError>> + Send>> {
        let fut = (self.0)(params);
        Box::pin(async move { fut.await.into_completion_result() })
    }
}

impl<F, Fut, Res> IntoCompletionHandler<(CompleteParams,)> for F
where
    F: Fn(CompleteParams) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoCompletionResult + 'static,
{
    fn into_completion_handler(self) -> Arc<dyn CompletionHandler> {
        Arc::new(ParamsCompletionHandler(self))
    }
}

// 0 Extractors, CompleteArgument
struct ArgCompletionHandler<F>(F);

impl<F, Fut, Res> CompletionHandler for ArgCompletionHandler<F>
where
    F: Fn(CompleteArgument) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoCompletionResult + 'static,
{
    fn call(
        &self,
        _ctx: RequestContext,
        params: CompleteParams,
    ) -> Pin<Box<dyn Future<Output = Result<CompleteResult, CompletionError>> + Send>> {
        let fut = (self.0)(params.argument);
        Box::pin(async move { fut.await.into_completion_result() })
    }
}

impl<F, Fut, Res> IntoCompletionHandler<(CompleteArgument,)> for F
where
    F: Fn(CompleteArgument) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoCompletionResult + 'static,
{
    fn into_completion_handler(self) -> Arc<dyn CompletionHandler> {
        Arc::new(ArgCompletionHandler(self))
    }
}

// 0 Extractors, CompleteArgument + Option<CompleteContext>
struct ArgAndContextCompletionHandler<F>(F);

impl<F, Fut, Res> CompletionHandler for ArgAndContextCompletionHandler<F>
where
    F: Fn(CompleteArgument, Option<CompleteContext>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoCompletionResult + 'static,
{
    fn call(
        &self,
        _ctx: RequestContext,
        params: CompleteParams,
    ) -> Pin<Box<dyn Future<Output = Result<CompleteResult, CompletionError>> + Send>> {
        let fut = (self.0)(params.argument, params.context);
        Box::pin(async move { fut.await.into_completion_result() })
    }
}

impl<F, Fut, Res> IntoCompletionHandler<(CompleteArgument, Option<CompleteContext>)> for F
where
    F: Fn(CompleteArgument, Option<CompleteContext>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoCompletionResult + 'static,
{
    fn into_completion_handler(self) -> Arc<dyn CompletionHandler> {
        Arc::new(ArgAndContextCompletionHandler(self))
    }
}

// 0 Extractors, Reference + CompleteArgument
struct RefAndArgCompletionHandler<F>(F);

impl<F, Fut, Res> CompletionHandler for RefAndArgCompletionHandler<F>
where
    F: Fn(Reference, CompleteArgument) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoCompletionResult + 'static,
{
    fn call(
        &self,
        _ctx: RequestContext,
        params: CompleteParams,
    ) -> Pin<Box<dyn Future<Output = Result<CompleteResult, CompletionError>> + Send>> {
        let fut = (self.0)(params.reference, params.argument);
        Box::pin(async move { fut.await.into_completion_result() })
    }
}

impl<F, Fut, Res> IntoCompletionHandler<(Reference, CompleteArgument)> for F
where
    F: Fn(Reference, CompleteArgument) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoCompletionResult + 'static,
{
    fn into_completion_handler(self) -> Arc<dyn CompletionHandler> {
        Arc::new(RefAndArgCompletionHandler(self))
    }
}

// 0 Extractors, Reference + CompleteArgument + Option<CompleteContext>
struct RefArgAndContextCompletionHandler<F>(F);

impl<F, Fut, Res> CompletionHandler for RefArgAndContextCompletionHandler<F>
where
    F: Fn(Reference, CompleteArgument, Option<CompleteContext>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoCompletionResult + 'static,
{
    fn call(
        &self,
        _ctx: RequestContext,
        params: CompleteParams,
    ) -> Pin<Box<dyn Future<Output = Result<CompleteResult, CompletionError>> + Send>> {
        let fut = (self.0)(params.reference, params.argument, params.context);
        Box::pin(async move { fut.await.into_completion_result() })
    }
}

impl<F, Fut, Res> IntoCompletionHandler<(Reference, CompleteArgument, Option<CompleteContext>)>
    for F
where
    F: Fn(Reference, CompleteArgument, Option<CompleteContext>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoCompletionResult + 'static,
{
    fn into_completion_handler(self) -> Arc<dyn CompletionHandler> {
        Arc::new(RefArgAndContextCompletionHandler(self))
    }
}

macro_rules! impl_into_completion_handler {
    ($($E:ident),+) => {
        // Extractors with 0 args
        #[allow(non_snake_case)]
        impl<F, Fut, $($E,)+ Res> IntoCompletionHandler<($($E,)+ ())> for F
        where
            $($E: FromRequestContext + Send + 'static,)+
            F: Fn($($E),+) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Res> + Send + 'static,
            Res: IntoCompletionResult + 'static,
        {
            fn into_completion_handler(self) -> Arc<dyn CompletionHandler> {
                struct Handler<F, M>(F, std::marker::PhantomData<fn() -> M>);

                impl<F, Fut, $($E,)+ Res> CompletionHandler for Handler<F, (Fut, $($E,)+ Res)>
                where
                    $($E: FromRequestContext + Send + 'static,)+
                    F: Fn($($E),+) -> Fut + Send + Sync + 'static,
                    Fut: Future<Output = Res> + Send + 'static,
                    Res: IntoCompletionResult + 'static,
                {
                    fn call(
                        &self,
                        ctx: RequestContext,
                        _params: CompleteParams,
                    ) -> Pin<Box<dyn Future<Output = Result<CompleteResult, CompletionError>> + Send>> {
                        $(
                            let $E = match $E::from_request_context(&ctx) {
                                Ok(val) => val,
                                Err(err) => {
                                    return Box::pin(async move {
                                        Err(CompletionError::InvalidParams(format!("Extraction error: {err}")))
                                    });
                                }
                            };
                        )+
                        let fut = (self.0)($($E),+);
                        Box::pin(async move { fut.await.into_completion_result() })
                    }
                }
                Arc::new(Handler(self, std::marker::PhantomData))
            }
        }

        // Extractors with CompleteParams
        #[allow(non_snake_case)]
        impl<F, Fut, $($E,)+ Res> IntoCompletionHandler<($($E,)+ (CompleteParams,))> for F
        where
            $($E: FromRequestContext + Send + 'static,)+
            F: Fn($($E,)+ CompleteParams) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Res> + Send + 'static,
            Res: IntoCompletionResult + 'static,
        {
            fn into_completion_handler(self) -> Arc<dyn CompletionHandler> {
                struct Handler<F, M>(F, std::marker::PhantomData<fn() -> M>);

                impl<F, Fut, $($E,)+ Res> CompletionHandler for Handler<F, (Fut, $($E,)+ Res)>
                where
                    $($E: FromRequestContext + Send + 'static,)+
                    F: Fn($($E,)+ CompleteParams) -> Fut + Send + Sync + 'static,
                    Fut: Future<Output = Res> + Send + 'static,
                    Res: IntoCompletionResult + 'static,
                {
                    fn call(
                        &self,
                        ctx: RequestContext,
                        params: CompleteParams,
                    ) -> Pin<Box<dyn Future<Output = Result<CompleteResult, CompletionError>> + Send>> {
                        $(
                            let $E = match $E::from_request_context(&ctx) {
                                Ok(val) => val,
                                Err(err) => {
                                    return Box::pin(async move {
                                        Err(CompletionError::InvalidParams(format!("Extraction error: {err}")))
                                    });
                                }
                            };
                        )+
                        let fut = (self.0)($($E,)+ params);
                        Box::pin(async move { fut.await.into_completion_result() })
                    }
                }
                Arc::new(Handler(self, std::marker::PhantomData))
            }
        }

        // Extractors with CompleteArgument
        #[allow(non_snake_case)]
        impl<F, Fut, $($E,)+ Res> IntoCompletionHandler<($($E,)+ (CompleteArgument,))> for F
        where
            $($E: FromRequestContext + Send + 'static,)+
            F: Fn($($E,)+ CompleteArgument) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Res> + Send + 'static,
            Res: IntoCompletionResult + 'static,
        {
            fn into_completion_handler(self) -> Arc<dyn CompletionHandler> {
                struct Handler<F, M>(F, std::marker::PhantomData<fn() -> M>);

                impl<F, Fut, $($E,)+ Res> CompletionHandler for Handler<F, (Fut, $($E,)+ Res)>
                where
                    $($E: FromRequestContext + Send + 'static,)+
                    F: Fn($($E,)+ CompleteArgument) -> Fut + Send + Sync + 'static,
                    Fut: Future<Output = Res> + Send + 'static,
                    Res: IntoCompletionResult + 'static,
                {
                    fn call(
                        &self,
                        ctx: RequestContext,
                        params: CompleteParams,
                    ) -> Pin<Box<dyn Future<Output = Result<CompleteResult, CompletionError>> + Send>> {
                        $(
                            let $E = match $E::from_request_context(&ctx) {
                                Ok(val) => val,
                                Err(err) => {
                                    return Box::pin(async move {
                                        Err(CompletionError::InvalidParams(format!("Extraction error: {err}")))
                                    });
                                }
                            };
                        )+
                        let fut = (self.0)($($E,)+ params.argument);
                        Box::pin(async move { fut.await.into_completion_result() })
                    }
                }
                Arc::new(Handler(self, std::marker::PhantomData))
            }
        }

        // Extractors with CompleteArgument + Option<CompleteContext>
        #[allow(non_snake_case)]
        impl<F, Fut, $($E,)+ Res> IntoCompletionHandler<($($E,)+ (CompleteArgument, Option<CompleteContext>))> for F
        where
            $($E: FromRequestContext + Send + 'static,)+
            F: Fn($($E,)+ CompleteArgument, Option<CompleteContext>) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Res> + Send + 'static,
            Res: IntoCompletionResult + 'static,
        {
            fn into_completion_handler(self) -> Arc<dyn CompletionHandler> {
                struct Handler<F, M>(F, std::marker::PhantomData<fn() -> M>);

                impl<F, Fut, $($E,)+ Res> CompletionHandler for Handler<F, (Fut, $($E,)+ Res)>
                where
                    $($E: FromRequestContext + Send + 'static,)+
                    F: Fn($($E,)+ CompleteArgument, Option<CompleteContext>) -> Fut + Send + Sync + 'static,
                    Fut: Future<Output = Res> + Send + 'static,
                    Res: IntoCompletionResult + 'static,
                {
                    fn call(
                        &self,
                        ctx: RequestContext,
                        params: CompleteParams,
                    ) -> Pin<Box<dyn Future<Output = Result<CompleteResult, CompletionError>> + Send>> {
                        $(
                            let $E = match $E::from_request_context(&ctx) {
                                Ok(val) => val,
                                Err(err) => {
                                    return Box::pin(async move {
                                        Err(CompletionError::InvalidParams(format!("Extraction error: {err}")))
                                    });
                                }
                            };
                        )+
                        let fut = (self.0)($($E,)+ params.argument, params.context);
                        Box::pin(async move { fut.await.into_completion_result() })
                    }
                }
                Arc::new(Handler(self, std::marker::PhantomData))
            }
        }

        // Extractors with Reference + CompleteArgument
        #[allow(non_snake_case)]
        impl<F, Fut, $($E,)+ Res> IntoCompletionHandler<($($E,)+ (Reference, CompleteArgument))> for F
        where
            $($E: FromRequestContext + Send + 'static,)+
            F: Fn($($E,)+ Reference, CompleteArgument) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Res> + Send + 'static,
            Res: IntoCompletionResult + 'static,
        {
            fn into_completion_handler(self) -> Arc<dyn CompletionHandler> {
                struct Handler<F, M>(F, std::marker::PhantomData<fn() -> M>);

                impl<F, Fut, $($E,)+ Res> CompletionHandler for Handler<F, (Fut, $($E,)+ Res)>
                where
                    $($E: FromRequestContext + Send + 'static,)+
                    F: Fn($($E,)+ Reference, CompleteArgument) -> Fut + Send + Sync + 'static,
                    Fut: Future<Output = Res> + Send + 'static,
                    Res: IntoCompletionResult + 'static,
                {
                    fn call(
                        &self,
                        ctx: RequestContext,
                        params: CompleteParams,
                    ) -> Pin<Box<dyn Future<Output = Result<CompleteResult, CompletionError>> + Send>> {
                        $(
                            let $E = match $E::from_request_context(&ctx) {
                                Ok(val) => val,
                                Err(err) => {
                                    return Box::pin(async move {
                                        Err(CompletionError::InvalidParams(format!("Extraction error: {err}")))
                                    });
                                }
                            };
                        )+
                        let fut = (self.0)($($E,)+ params.reference, params.argument);
                        Box::pin(async move { fut.await.into_completion_result() })
                    }
                }
                Arc::new(Handler(self, std::marker::PhantomData))
            }
        }

        // Extractors with Reference + CompleteArgument + Option<CompleteContext>
        #[allow(non_snake_case)]
        impl<F, Fut, $($E,)+ Res> IntoCompletionHandler<($($E,)+ (Reference, CompleteArgument, Option<CompleteContext>))> for F
        where
            $($E: FromRequestContext + Send + 'static,)+
            F: Fn($($E,)+ Reference, CompleteArgument, Option<CompleteContext>) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Res> + Send + 'static,
            Res: IntoCompletionResult + 'static,
        {
            fn into_completion_handler(self) -> Arc<dyn CompletionHandler> {
                struct Handler<F, M>(F, std::marker::PhantomData<fn() -> M>);

                impl<F, Fut, $($E,)+ Res> CompletionHandler for Handler<F, (Fut, $($E,)+ Res)>
                where
                    $($E: FromRequestContext + Send + 'static,)+
                    F: Fn($($E,)+ Reference, CompleteArgument, Option<CompleteContext>) -> Fut + Send + Sync + 'static,
                    Fut: Future<Output = Res> + Send + 'static,
                    Res: IntoCompletionResult + 'static,
                {
                    fn call(
                        &self,
                        ctx: RequestContext,
                        params: CompleteParams,
                    ) -> Pin<Box<dyn Future<Output = Result<CompleteResult, CompletionError>> + Send>> {
                        $(
                            let $E = match $E::from_request_context(&ctx) {
                                Ok(val) => val,
                                Err(err) => {
                                    return Box::pin(async move {
                                        Err(CompletionError::InvalidParams(format!("Extraction error: {err}")))
                                    });
                                }
                            };
                        )+
                        let fut = (self.0)($($E,)+ params.reference, params.argument, params.context);
                        Box::pin(async move { fut.await.into_completion_result() })
                    }
                }
                Arc::new(Handler(self, std::marker::PhantomData))
            }
        }
    };
}

impl_into_completion_handler!(E1);
impl_into_completion_handler!(E1, E2);
impl_into_completion_handler!(E1, E2, E3);
impl_into_completion_handler!(E1, E2, E3, E4);
impl_into_completion_handler!(E1, E2, E3, E4, E5);

#[cfg(test)]
mod tests {
    //! Unit tests for completion handler traits and extractor integrations.

    use super::*;
    use crate::extract::Extension;

    /// Tests invoking completion handlers with no args, `CompleteArgument`, and extractors.
    #[tokio::test]
    async fn test_completion_handlers_invocation() {
        // No args
        let h1 = (|| async { vec!["opt1", "opt2"] }).into_completion_handler();
        let ctx = RequestContext::new(
            None,
            http::HeaderMap::new(),
            Arc::new(http::Extensions::new()),
        );
        let params = CompleteParams::prompt("test", "arg", "val");
        let res = h1.call(ctx.clone(), params.clone()).await.unwrap();
        assert_eq!(res.completion.values, vec!["opt1", "opt2"]);

        // CompleteArgument
        let h2 = (|arg: CompleteArgument| async move {
            vec![format!("{}_1", arg.value), format!("{}_2", arg.value)]
        })
        .into_completion_handler();
        let res = h2.call(ctx.clone(), params.clone()).await.unwrap();
        assert_eq!(res.completion.values, vec!["val_1", "val_2"]);

        // With extractors
        #[derive(Clone)]
        struct Prefix(String);

        let h3 = (|Extension(p): Extension<Prefix>, arg: CompleteArgument| async move {
            vec![format!("{}:{}", p.0, arg.name)]
        })
        .into_completion_handler();

        let mut ext = http::Extensions::new();
        ext.insert(Prefix("custom".to_string()));

        let ctx_with_ext = RequestContext::new(None, http::HeaderMap::new(), Arc::new(ext));
        let res = h3.call(ctx_with_ext, params).await.unwrap();
        assert_eq!(res.completion.values, vec!["custom:arg"]);
    }
}
