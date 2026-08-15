// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::extract::{FromRequestContext, RequestContext};
use crate::types::mcp::completion::{
    CompleteArgument, CompleteContext, CompleteParams, CompleteResult, CompletionValues, Reference,
};

pub mod registry;

pub use registry::CompletionRegistry;

/// Error type encountered during argument completion operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionError {
    /// Invalid parameters or arguments provided to the completion handler.
    InvalidParams(String),
    /// Target prompt, resource, or argument was not found.
    NotFound(String),
    /// Internal execution or business logic error.
    Internal(String),
}

impl std::fmt::Display for CompletionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompletionError::InvalidParams(msg) => write!(f, "Invalid params: {msg}"),
            CompletionError::NotFound(msg) => write!(f, "Not found: {msg}"),
            CompletionError::Internal(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for CompletionError {}

/// Trait for types that can be converted into a [`CompleteResult`].
pub trait IntoCompletionResult: Send {
    fn into_completion_result(self) -> Result<CompleteResult, CompletionError>;
}

impl IntoCompletionResult for CompleteResult {
    fn into_completion_result(self) -> Result<CompleteResult, CompletionError> {
        Ok(self)
    }
}

impl IntoCompletionResult for CompletionValues {
    fn into_completion_result(self) -> Result<CompleteResult, CompletionError> {
        Ok(CompleteResult::with_completion(self))
    }
}

impl IntoCompletionResult for Vec<String> {
    fn into_completion_result(self) -> Result<CompleteResult, CompletionError> {
        Ok(CompleteResult::new(self))
    }
}

impl IntoCompletionResult for Vec<&str> {
    fn into_completion_result(self) -> Result<CompleteResult, CompletionError> {
        Ok(CompleteResult::new(
            self.into_iter().map(String::from).collect::<Vec<_>>(),
        ))
    }
}

impl IntoCompletionResult for &[&str] {
    fn into_completion_result(self) -> Result<CompleteResult, CompletionError> {
        Ok(CompleteResult::new(
            self.iter().map(|&s| s.to_string()).collect::<Vec<_>>(),
        ))
    }
}

impl IntoCompletionResult for &[String] {
    fn into_completion_result(self) -> Result<CompleteResult, CompletionError> {
        Ok(CompleteResult::new(self.to_vec()))
    }
}

impl IntoCompletionResult for crate::types::mcp::InputRequiredResult {
    fn into_completion_result(self) -> Result<CompleteResult, CompletionError> {
        let mut extras = self.extras;
        if let Some(state) = self.request_state {
            extras.insert("requestState".to_string(), serde_json::Value::String(state));
        }
        if !self.input_requests.is_empty() {
            if let Ok(reqs) = serde_json::to_value(&self.input_requests) {
                extras.insert("inputRequests".to_string(), reqs);
            }
        }
        Ok(CompleteResult {
            meta: self.meta,
            result_type: Some(self.result_type),
            completion: CompletionValues::empty(),
            extras,
        })
    }
}

impl<T, E> IntoCompletionResult for Result<T, E>
where
    T: IntoCompletionResult,
    E: std::fmt::Display + Send,
{
    fn into_completion_result(self) -> Result<CompleteResult, CompletionError> {
        match self {
            Ok(val) => val.into_completion_result(),
            Err(err) => Err(CompletionError::Internal(err.to_string())),
        }
    }
}

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
    use super::*;
    use crate::extract::SessionId;

    #[test]
    fn test_into_completion_result() {
        let res1: CompleteResult = vec!["a".to_string(), "b".to_string()]
            .into_completion_result()
            .unwrap();
        assert_eq!(res1.completion.values, vec!["a", "b"]);

        let res2: CompleteResult = vec!["x", "y"].into_completion_result().unwrap();
        assert_eq!(res2.completion.values, vec!["x", "y"]);

        let res3: CompleteResult = (&["m", "n"][..]).into_completion_result().unwrap();
        assert_eq!(res3.completion.values, vec!["m", "n"]);

        let res_err: Result<CompleteResult, CompletionError> =
            Result::<Vec<String>, &str>::Err("failed").into_completion_result();
        assert!(matches!(res_err, Err(CompletionError::Internal(msg)) if msg == "failed"));
    }

    #[tokio::test]
    async fn test_completion_handlers_invocation() {
        // No args
        let h1 = (|| async { vec!["opt1", "opt2"] }).into_completion_handler();
        let ctx = RequestContext::new(
            None,
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
        let h3 = (|SessionId(sid): SessionId, arg: CompleteArgument| async move {
            vec![format!("{sid}:{}", arg.name)]
        })
        .into_completion_handler();

        let ctx_with_sess = RequestContext::new(
            Some(SessionId::new("session-42")),
            None,
            http::HeaderMap::new(),
            Arc::new(http::Extensions::new()),
        );
        let res = h3.call(ctx_with_sess, params).await.unwrap();
        assert_eq!(res.completion.values, vec!["session-42:arg"]);
    }
}
