// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::extract::{FromRequestContext, RequestContext};
pub mod registry;

pub use crate::types::mcp::logging::{SetLevelParams, SetLevelResult};
pub use registry::LoggingRegistry;

/// Error type encountered during logging level changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoggingError {
    /// Invalid parameters provided to the logging level handler.
    InvalidParams(String),
    /// Internal execution or configuration error.
    Internal(String),
}

impl std::fmt::Display for LoggingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoggingError::InvalidParams(msg) => write!(f, "Invalid params: {msg}"),
            LoggingError::Internal(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for LoggingError {}

/// Trait for types that can be converted into a [`LoggingError`].
pub trait IntoLoggingError: Send {
    fn into_logging_error(self) -> LoggingError;
}

impl IntoLoggingError for LoggingError {
    fn into_logging_error(self) -> LoggingError {
        self
    }
}

impl IntoLoggingError for String {
    fn into_logging_error(self) -> LoggingError {
        LoggingError::Internal(self)
    }
}

impl IntoLoggingError for &str {
    fn into_logging_error(self) -> LoggingError {
        LoggingError::Internal(self.to_string())
    }
}

/// Trait for types that can be converted into a [`SetLevelResult`].
pub trait IntoSetLevelResult: Send {
    fn into_set_level_result(self) -> Result<SetLevelResult, LoggingError>;
}

impl IntoSetLevelResult for SetLevelResult {
    fn into_set_level_result(self) -> Result<SetLevelResult, LoggingError> {
        Ok(self)
    }
}

impl IntoSetLevelResult for () {
    fn into_set_level_result(self) -> Result<SetLevelResult, LoggingError> {
        Ok(SetLevelResult::default())
    }
}

impl<T, E> IntoSetLevelResult for Result<T, E>
where
    T: IntoSetLevelResult,
    E: IntoLoggingError + Send,
{
    fn into_set_level_result(self) -> Result<SetLevelResult, LoggingError> {
        match self {
            Ok(val) => val.into_set_level_result(),
            Err(err) => Err(err.into_logging_error()),
        }
    }
}

/// An erased handler trait for handling `logging/setLevel` requests.
pub trait SetLevelHandler: Send + Sync {
    fn call(
        &self,
        ctx: RequestContext,
        params: SetLevelParams,
    ) -> Pin<Box<dyn Future<Output = Result<SetLevelResult, LoggingError>> + Send>>;
}

/// Trait for converting asynchronous functions into a boxed [`SetLevelHandler`].
pub trait IntoSetLevelHandler<T>: Send + Sync + 'static {
    fn into_set_level_handler(self) -> Arc<dyn SetLevelHandler>;
}

// 0 Extractors, 0 Args
struct NoArgsSetLevelHandler<F>(F);

impl<F, Fut, Res> SetLevelHandler for NoArgsSetLevelHandler<F>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoSetLevelResult + 'static,
{
    fn call(
        &self,
        _ctx: RequestContext,
        _params: SetLevelParams,
    ) -> Pin<Box<dyn Future<Output = Result<SetLevelResult, LoggingError>> + Send>> {
        let fut = (self.0)();
        Box::pin(async move { fut.await.into_set_level_result() })
    }
}

impl<F, Fut, Res> IntoSetLevelHandler<()> for F
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoSetLevelResult + 'static,
{
    fn into_set_level_handler(self) -> Arc<dyn SetLevelHandler> {
        Arc::new(NoArgsSetLevelHandler(self))
    }
}

// 0 Extractors, SetLevelParams
struct ParamsSetLevelHandler<F>(F);

impl<F, Fut, Res> SetLevelHandler for ParamsSetLevelHandler<F>
where
    F: Fn(SetLevelParams) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoSetLevelResult + 'static,
{
    fn call(
        &self,
        _ctx: RequestContext,
        params: SetLevelParams,
    ) -> Pin<Box<dyn Future<Output = Result<SetLevelResult, LoggingError>> + Send>> {
        let fut = (self.0)(params);
        Box::pin(async move { fut.await.into_set_level_result() })
    }
}

impl<F, Fut, Res> IntoSetLevelHandler<(SetLevelParams,)> for F
where
    F: Fn(SetLevelParams) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoSetLevelResult + 'static,
{
    fn into_set_level_handler(self) -> Arc<dyn SetLevelHandler> {
        Arc::new(ParamsSetLevelHandler(self))
    }
}

macro_rules! impl_into_set_level_handler {
    ($($E:ident),+) => {
        // Extractors with 0 extra args
        #[allow(non_snake_case)]
        impl<F, Fut, $($E,)+ Res> IntoSetLevelHandler<($($E,)+)> for F
        where
            $($E: FromRequestContext + Send + 'static,)+
            F: Fn($($E),+) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Res> + Send + 'static,
            Res: IntoSetLevelResult + 'static,
        {
            fn into_set_level_handler(self) -> Arc<dyn SetLevelHandler> {
                struct Handler<F, M>(F, std::marker::PhantomData<fn() -> M>);

                impl<F, Fut, $($E,)+ Res> SetLevelHandler for Handler<F, (Fut, $($E,)+ Res)>
                where
                    $($E: FromRequestContext + Send + 'static,)+
                    F: Fn($($E),+) -> Fut + Send + Sync + 'static,
                    Fut: Future<Output = Res> + Send + 'static,
                    Res: IntoSetLevelResult + 'static,
                {
                    fn call(
                        &self,
                        ctx: RequestContext,
                        _params: SetLevelParams,
                    ) -> Pin<Box<dyn Future<Output = Result<SetLevelResult, LoggingError>> + Send>> {
                        $(
                            let $E = match $E::from_request_context(&ctx) {
                                Ok(val) => val,
                                Err(err) => {
                                    return Box::pin(async move {
                                        Err(LoggingError::InvalidParams(format!("Extraction error: {err}")))
                                    });
                                }
                            };
                        )+
                        let fut = (self.0)($($E),+);
                        Box::pin(async move { fut.await.into_set_level_result() })
                    }
                }
                Arc::new(Handler(self, std::marker::PhantomData))
            }
        }

        // Extractors with SetLevelParams
        #[allow(non_snake_case)]
        impl<F, Fut, $($E,)+ Res> IntoSetLevelHandler<($($E,)+ (SetLevelParams,))> for F
        where
            $($E: FromRequestContext + Send + 'static,)+
            F: Fn($($E,)+ SetLevelParams) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Res> + Send + 'static,
            Res: IntoSetLevelResult + 'static,
        {
            fn into_set_level_handler(self) -> Arc<dyn SetLevelHandler> {
                struct Handler<F, M>(F, std::marker::PhantomData<fn() -> M>);

                impl<F, Fut, $($E,)+ Res> SetLevelHandler for Handler<F, (Fut, $($E,)+ Res)>
                where
                    $($E: FromRequestContext + Send + 'static,)+
                    F: Fn($($E,)+ SetLevelParams) -> Fut + Send + Sync + 'static,
                    Fut: Future<Output = Res> + Send + 'static,
                    Res: IntoSetLevelResult + 'static,
                {
                    fn call(
                        &self,
                        ctx: RequestContext,
                        params: SetLevelParams,
                    ) -> Pin<Box<dyn Future<Output = Result<SetLevelResult, LoggingError>> + Send>> {
                        $(
                            let $E = match $E::from_request_context(&ctx) {
                                Ok(val) => val,
                                Err(err) => {
                                    return Box::pin(async move {
                                        Err(LoggingError::InvalidParams(format!("Extraction error: {err}")))
                                    });
                                }
                            };
                        )+
                        let fut = (self.0)($($E,)+ params);
                        Box::pin(async move { fut.await.into_set_level_result() })
                    }
                }
                Arc::new(Handler(self, std::marker::PhantomData))
            }
        }
    };
}

impl_into_set_level_handler!(E1);
impl_into_set_level_handler!(E1, E2);
impl_into_set_level_handler!(E1, E2, E3);
impl_into_set_level_handler!(E1, E2, E3, E4);
impl_into_set_level_handler!(E1, E2, E3, E4, E5);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::SessionId;
    use crate::types::mcp::{LoggingLevel, RequestMetaObject};

    #[test]
    fn test_into_set_level_result() {
        let res1: SetLevelResult = ().into_set_level_result().unwrap();
        assert!(res1.meta.is_none());

        let res2: SetLevelResult = SetLevelResult::new().into_set_level_result().unwrap();
        assert!(res2.meta.is_none());

        let res_err: Result<SetLevelResult, LoggingError> =
            Result::<(), &str>::Err("failure").into_set_level_result();
        assert!(matches!(res_err, Err(LoggingError::Internal(msg)) if msg == "failure"));
    }

    #[tokio::test]
    async fn test_set_level_handlers_invocation() {
        let h1 = (|| async { Ok::<(), &str>(()) }).into_set_level_handler();
        let ctx = RequestContext::new(
            None,
            None,
            http::HeaderMap::new(),
            Arc::new(http::Extensions::new()),
        );
        let params = SetLevelParams::new(LoggingLevel::Debug);
        assert!(h1.call(ctx.clone(), params.clone()).await.is_ok());

        let h2 = (|level: LoggingLevel| async move {
            assert_eq!(level, LoggingLevel::Debug);
            Ok::<(), &str>(())
        })
        .into_set_level_handler();

        let mut meta = RequestMetaObject {
            progress_token: None,
            client_info: None,
            client_capabilities: None,
            protocol_version: None,
            log_level: Some(LoggingLevel::Debug),
            extra: std::collections::HashMap::new(),
        };

        let ctx_with_level = RequestContext::new(
            None,
            Some(meta.clone()),
            http::HeaderMap::new(),
            Arc::new(http::Extensions::new()),
        );
        assert!(h2.call(ctx_with_level, params.clone()).await.is_ok());

        let h3 = (|SessionId(sid): SessionId, level: LoggingLevel| async move {
            assert_eq!(sid, "sess-123");
            assert_eq!(level, LoggingLevel::Debug);
            Ok::<(), &str>(())
        })
        .into_set_level_handler();

        meta.log_level = Some(LoggingLevel::Debug);
        let ctx_with_sess = RequestContext::new(
            Some(SessionId::new("sess-123")),
            Some(meta),
            http::HeaderMap::new(),
            Arc::new(http::Extensions::new()),
        );
        assert!(h3.call(ctx_with_sess, params).await.is_ok());
    }
}
