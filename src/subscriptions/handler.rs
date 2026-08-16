// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Subscriptions Listen Handler Traits
//!
//! Provides handler traits and conversion machinery for `subscriptions/listen` requests.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::body::ResponseBody;
use crate::extract::{FromRequestContext, RequestContext};
use crate::types::jsonrpc::{JsonRpcErrorResponse, JsonRpcRequestId};
use crate::types::mcp::subscriptions::{
    NotificationSubscriptions, SubscriptionsAcknowledgedParams, SubscriptionsListenParams,
};

/// Error type encountered during subscription listener execution or parameter validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscriptionError {
    /// Invalid parameters or request state.
    InvalidParams(String),
    /// Internal execution or business logic error.
    Internal(String),
}

impl std::fmt::Display for SubscriptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubscriptionError::InvalidParams(msg) => write!(f, "Invalid params: {msg}"),
            SubscriptionError::Internal(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for SubscriptionError {}

impl SubscriptionError {
    /// Converts this error into a standard JSON-RPC error response.
    pub fn into_error_response(self, id: Option<JsonRpcRequestId>) -> JsonRpcErrorResponse {
        match self {
            SubscriptionError::InvalidParams(err) => {
                JsonRpcErrorResponse::invalid_params(id, format!("Invalid params: {err}"))
            }
            SubscriptionError::Internal(err) => JsonRpcErrorResponse::internal_error(id, err),
        }
    }
}

/// The outcome of evaluating a `subscriptions/listen` request.
pub struct SubscriptionsListenOutcome {
    /// Acknowledged subscription filter parameters.
    pub acknowledged: SubscriptionsAcknowledgedParams,
    /// Optional streaming body producing subsequent notification frames.
    pub stream_body: Option<ResponseBody>,
}

/// Trait for types that can be converted into acknowledged subscription parameters and streams.
pub trait IntoSubscriptionsListenResult: Send {
    fn apply_to_outcome(
        self,
        base: SubscriptionsAcknowledgedParams,
    ) -> Result<SubscriptionsListenOutcome, SubscriptionError>;
}

impl IntoSubscriptionsListenResult for SubscriptionsAcknowledgedParams {
    fn apply_to_outcome(
        self,
        _base: SubscriptionsAcknowledgedParams,
    ) -> Result<SubscriptionsListenOutcome, SubscriptionError> {
        Ok(SubscriptionsListenOutcome {
            acknowledged: self,
            stream_body: None,
        })
    }
}

impl IntoSubscriptionsListenResult for NotificationSubscriptions {
    fn apply_to_outcome(
        self,
        mut base: SubscriptionsAcknowledgedParams,
    ) -> Result<SubscriptionsListenOutcome, SubscriptionError> {
        base.notifications = self;
        Ok(SubscriptionsListenOutcome {
            acknowledged: base,
            stream_body: None,
        })
    }
}

impl IntoSubscriptionsListenResult for () {
    fn apply_to_outcome(
        self,
        base: SubscriptionsAcknowledgedParams,
    ) -> Result<SubscriptionsListenOutcome, SubscriptionError> {
        Ok(SubscriptionsListenOutcome {
            acknowledged: base,
            stream_body: None,
        })
    }
}

impl IntoSubscriptionsListenResult for (NotificationSubscriptions, ResponseBody) {
    fn apply_to_outcome(
        self,
        mut base: SubscriptionsAcknowledgedParams,
    ) -> Result<SubscriptionsListenOutcome, SubscriptionError> {
        base.notifications = self.0;
        Ok(SubscriptionsListenOutcome {
            acknowledged: base,
            stream_body: Some(self.1),
        })
    }
}

impl IntoSubscriptionsListenResult for (SubscriptionsAcknowledgedParams, ResponseBody) {
    fn apply_to_outcome(
        self,
        _base: SubscriptionsAcknowledgedParams,
    ) -> Result<SubscriptionsListenOutcome, SubscriptionError> {
        Ok(SubscriptionsListenOutcome {
            acknowledged: self.0,
            stream_body: Some(self.1),
        })
    }
}

impl IntoSubscriptionsListenResult for ResponseBody {
    fn apply_to_outcome(
        self,
        base: SubscriptionsAcknowledgedParams,
    ) -> Result<SubscriptionsListenOutcome, SubscriptionError> {
        Ok(SubscriptionsListenOutcome {
            acknowledged: base,
            stream_body: Some(self),
        })
    }
}

impl<T, E> IntoSubscriptionsListenResult for Result<T, E>
where
    T: IntoSubscriptionsListenResult,
    E: std::fmt::Display + Send,
{
    fn apply_to_outcome(
        self,
        base: SubscriptionsAcknowledgedParams,
    ) -> Result<SubscriptionsListenOutcome, SubscriptionError> {
        match self {
            Ok(val) => val.apply_to_outcome(base),
            Err(err) => Err(SubscriptionError::Internal(err.to_string())),
        }
    }
}

/// An erased handler trait for handling `subscriptions/listen` requests.
pub trait SubscriptionsListenHandler: Send + Sync {
    fn call(
        &self,
        ctx: RequestContext,
        params: SubscriptionsListenParams,
        base_ack: SubscriptionsAcknowledgedParams,
    ) -> Pin<Box<dyn Future<Output = Result<SubscriptionsListenOutcome, SubscriptionError>> + Send>>;
}

/// Trait for converting handler functions into a boxed [`SubscriptionsListenHandler`].
pub trait IntoSubscriptionsListenHandler<T>: Send + Sync + 'static {
    fn into_subscriptions_listen_handler(self) -> Arc<dyn SubscriptionsListenHandler>;
}

// 0 Extractors
struct NoArgsSubscriptionHandler<F>(F);

impl<F, Fut, Res> SubscriptionsListenHandler for NoArgsSubscriptionHandler<F>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoSubscriptionsListenResult + 'static,
{
    fn call(
        &self,
        _ctx: RequestContext,
        _params: SubscriptionsListenParams,
        base_ack: SubscriptionsAcknowledgedParams,
    ) -> Pin<Box<dyn Future<Output = Result<SubscriptionsListenOutcome, SubscriptionError>> + Send>> {
        let fut = (self.0)();
        Box::pin(async move { fut.await.apply_to_outcome(base_ack) })
    }
}

impl<F, Fut, Res> IntoSubscriptionsListenHandler<()> for F
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoSubscriptionsListenResult + 'static,
{
    fn into_subscriptions_listen_handler(self) -> Arc<dyn SubscriptionsListenHandler> {
        Arc::new(NoArgsSubscriptionHandler(self))
    }
}

macro_rules! impl_into_subscription_handler {
    ($($E:ident),+) => {
        #[allow(non_snake_case)]
        impl<F, Fut, $($E,)+ Res> IntoSubscriptionsListenHandler<($($E,)+)> for F
        where
            $($E: FromRequestContext + Send + 'static,)+
            F: Fn($($E),+) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Res> + Send + 'static,
            Res: IntoSubscriptionsListenResult + 'static,
        {
            fn into_subscriptions_listen_handler(self) -> Arc<dyn SubscriptionsListenHandler> {
                struct Handler<F, M>(F, std::marker::PhantomData<fn() -> M>);

                impl<F, Fut, $($E,)+ Res> SubscriptionsListenHandler for Handler<F, (Fut, $($E,)+ Res)>
                where
                    $($E: FromRequestContext + Send + 'static,)+
                    F: Fn($($E),+) -> Fut + Send + Sync + 'static,
                    Fut: Future<Output = Res> + Send + 'static,
                    Res: IntoSubscriptionsListenResult + 'static,
                {
                    fn call(
                        &self,
                        ctx: RequestContext,
                        _params: SubscriptionsListenParams,
                        base_ack: SubscriptionsAcknowledgedParams,
                    ) -> Pin<Box<dyn Future<Output = Result<SubscriptionsListenOutcome, SubscriptionError>> + Send>> {
                        $(
                            let $E = match $E::from_request_context(&ctx) {
                                Ok(val) => val,
                                Err(err) => return Box::pin(async move {
                                    Err(SubscriptionError::InvalidParams(err.to_string()))
                                }),
                            };
                        )+
                        let fut = (self.0)($($E),+);
                        Box::pin(async move { fut.await.apply_to_outcome(base_ack) })
                    }
                }

                Arc::new(Handler(self, std::marker::PhantomData))
            }
        }
    };
}

impl_into_subscription_handler!(E1);
impl_into_subscription_handler!(E1, E2);
impl_into_subscription_handler!(E1, E2, E3);
impl_into_subscription_handler!(E1, E2, E3, E4);
impl_into_subscription_handler!(E1, E2, E3, E4, E5);
impl_into_subscription_handler!(E1, E2, E3, E4, E5, E6);
impl_into_subscription_handler!(E1, E2, E3, E4, E5, E6, E7);
impl_into_subscription_handler!(E1, E2, E3, E4, E5, E6, E7, E8);

#[cfg(test)]
mod tests {
    //! Unit tests for subscription handlers and result conversions.

    use super::*;

    /// Tests `IntoSubscriptionsListenResult` conversions.
    #[tokio::test]
    async fn test_into_subscriptions_listen_result() {
        let base = SubscriptionsAcknowledgedParams::new(NotificationSubscriptions::default());
        let res = NotificationSubscriptions::new().with_tools_list_changed(true);

        let outcome = res.apply_to_outcome(base).unwrap();
        assert_eq!(outcome.acknowledged.notifications.tools_list_changed, Some(true));
        assert!(outcome.stream_body.is_none());
    }

    /// Tests `SubscriptionError` Display formatting.
    #[test]
    fn test_subscription_error_display() {
        let err_invalid = SubscriptionError::InvalidParams("bad param".to_string());
        assert_eq!(err_invalid.to_string(), "Invalid params: bad param");

        let err_internal = SubscriptionError::Internal("db fail".to_string());
        assert_eq!(err_internal.to_string(), "db fail");
    }

    /// Tests conversion of `SubscriptionError` variants into `JsonRpcErrorResponse`.
    #[test]
    fn test_subscription_error_into_error_response() {
        let req_id = Some(JsonRpcRequestId::Number(9.0));

        let err_invalid = SubscriptionError::InvalidParams("invalid subscription uri".to_string());
        let resp_invalid = err_invalid.into_error_response(req_id.clone());
        assert_eq!(resp_invalid.id, req_id);
        assert_eq!(
            resp_invalid.error.code,
            crate::types::jsonrpc::JsonRpcErrorCode::InvalidParams
        );
        assert_eq!(
            resp_invalid.error.message,
            "Invalid params: invalid subscription uri"
        );

        let err_internal = SubscriptionError::Internal("sse channel closed".to_string());
        let resp_internal = err_internal.into_error_response(req_id.clone());
        assert_eq!(resp_internal.id, req_id);
        assert_eq!(
            resp_internal.error.code,
            crate::types::jsonrpc::JsonRpcErrorCode::InternalError
        );
        assert_eq!(resp_internal.error.message, "sse channel closed");
    }
}
