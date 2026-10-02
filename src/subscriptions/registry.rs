// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Subscriptions Registry and Dispatching
//!
//! Manages subscription stream listeners and dispatches `subscriptions/listen` requests.

use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use bytes::Bytes;

use crate::body::{BoxError, ResponseBody};
use crate::extract::{RequestContext, Subscription};
use crate::router::{DispatchOutcome, MethodContext};
use crate::subscriptions::handler::{
    IntoSubscriptionsListenHandler, SubscriptionsListenHandler, SubscriptionsListenOutcome,
};
use crate::types::jsonrpc::{JsonRpcErrorResponse, JsonRpcResultResponse};
use crate::types::mcp::{
    Implementation, RequestMetaObject, ResultMetaObject,
    subscriptions::{
        NotificationSubscriptions, SubscriptionsAcknowledgedParams, SubscriptionsListenParams,
        subscriptions_acknowledged_notification,
    },
};
use crate::utils::format_sse_message;

/// SSE body emitting the acknowledgment, then the optional notification stream, then the
/// graceful-closure response once the notification stream ends.
struct SubscriptionBody {
    acknowledgment: Option<Bytes>,
    notifications: Option<ResponseBody>,
    closure: Option<Bytes>,
}

impl http_body::Body for SubscriptionBody {
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        if let Some(bytes) = self.acknowledgment.take() {
            return Poll::Ready(Some(Ok(http_body::Frame::data(bytes))));
        }
        if let Some(notifications) = self.notifications.as_mut() {
            match Pin::new(notifications).poll_frame(cx) {
                Poll::Ready(None) => self.notifications = None,
                other => return other,
            }
        }
        Poll::Ready(
            self.closure
                .take()
                .map(|bytes| Ok(http_body::Frame::data(bytes))),
        )
    }

    fn is_end_stream(&self) -> bool {
        self.acknowledgment.is_none() && self.notifications.is_none() && self.closure.is_none()
    }
}

/// Registry managing subscription listeners and SSE streaming for `subscriptions/listen`.
#[derive(Clone, Default)]
pub struct SubscriptionsRegistry {
    pub(crate) listen_handler: Option<Arc<dyn SubscriptionsListenHandler>>,
}

impl SubscriptionsRegistry {
    /// Creates a new empty [`SubscriptionsRegistry`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets a custom handler function for `subscriptions/listen` requests.
    pub fn set_listen_handler<H, T>(&mut self, handler: H)
    where
        H: IntoSubscriptionsListenHandler<T>,
        T: 'static,
    {
        self.listen_handler = Some(handler.into_subscriptions_listen_handler());
    }

    /// Dispatches an incoming `subscriptions/listen` JSON-RPC request.
    ///
    /// `supported` describes the notification types the server's capabilities back; the
    /// acknowledgment is the intersection of the client's request with it. `server_info`
    /// identifies the server in the graceful-closure result.
    pub(crate) async fn dispatch_listen(
        &self,
        ctx: MethodContext<'_>,
        params_val: Option<serde_json::Value>,
        supported: &NotificationSubscriptions,
        server_info: &Implementation,
    ) -> DispatchOutcome {
        let params: SubscriptionsListenParams = match params_val {
            Some(pv) => match serde_json::from_value(pv) {
                Ok(p) => p,
                Err(err) => {
                    return DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                        ctx.req_id,
                        format!("Invalid params: {err}"),
                    ));
                }
            },
            None => SubscriptionsListenParams::default(),
        };

        let Some(sub_id) = ctx.req_id.clone() else {
            return DispatchOutcome::error(JsonRpcErrorResponse::invalid_request(
                None,
                "Invalid Request: subscriptions/listen requires a request id",
            ));
        };

        let mut ack_notifications = NotificationSubscriptions::default();
        if let Some(ref req_subs) = params.notifications {
            if req_subs.tools_list_changed == Some(true)
                && supported.tools_list_changed == Some(true)
            {
                ack_notifications.tools_list_changed = Some(true);
            }
            if req_subs.prompts_list_changed == Some(true)
                && supported.prompts_list_changed == Some(true)
            {
                ack_notifications.prompts_list_changed = Some(true);
            }
            if req_subs.resources_list_changed == Some(true)
                && supported.resources_list_changed == Some(true)
            {
                ack_notifications.resources_list_changed = Some(true);
            }
            if let Some(ref uris) = req_subs.resource_subscriptions {
                let known_resources = supported.resource_subscriptions.as_deref().unwrap_or(&[]);
                let matched: Vec<String> = uris
                    .iter()
                    .filter(|u| known_resources.contains(u))
                    .cloned()
                    .collect();
                if !matched.is_empty() {
                    ack_notifications.resource_subscriptions = Some(matched);
                }
            }
        }

        let mut ack_meta = RequestMetaObject::empty();
        ack_meta.subscription_id = Some(sub_id.clone());
        let base_ack = SubscriptionsAcknowledgedParams::new(ack_notifications).with_meta(ack_meta);

        let outcome = if let Some(ref handler) = self.listen_handler {
            let mut extensions = (*ctx.extensions).clone();
            extensions.insert(Subscription {
                id: sub_id.clone(),
                notifications: base_ack.notifications.clone(),
            });
            let request_ctx = RequestContext::new(
                params.meta.clone(),
                ctx.headers.clone(),
                Arc::new(extensions),
            );
            match handler.call(request_ctx, params, base_ack).await {
                Ok(res) => res,
                Err(err) => return DispatchOutcome::error(err.into_error_response(ctx.req_id)),
            }
        } else {
            SubscriptionsListenOutcome {
                acknowledged: base_ack,
                stream_body: None,
            }
        };

        let notif = subscriptions_acknowledged_notification(outcome.acknowledged);
        let sse_bytes = match format_sse_message(&notif) {
            Ok(b) => b,
            Err(err) => {
                return DispatchOutcome::error(JsonRpcErrorResponse::internal_error(
                    ctx.req_id,
                    format!("Failed to serialize subscription acknowledgment: {err}"),
                ));
            }
        };

        let mut closure_meta = ResultMetaObject::new(Some(server_info.clone()));
        closure_meta.subscription_id = Some(sub_id.clone());
        let closure = JsonRpcResultResponse::new(
            sub_id,
            serde_json::json!({ "resultType": "complete", "_meta": closure_meta }),
        );
        let closure_bytes = match format_sse_message(&closure) {
            Ok(b) => b,
            Err(err) => {
                return DispatchOutcome::error(JsonRpcErrorResponse::internal_error(
                    ctx.req_id,
                    format!("Failed to serialize subscription closure: {err}"),
                ));
            }
        };

        let body = ResponseBody::new(SubscriptionBody {
            acknowledgment: Some(sse_bytes),
            notifications: outcome.stream_body,
            closure: Some(closure_bytes),
        });

        DispatchOutcome::sse_stream(body)
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for `SubscriptionsRegistry`.

    use super::*;

    /// Tests `SubscriptionsRegistry` creation and default state.
    #[test]
    fn test_subscriptions_registry_defaults() {
        let registry = SubscriptionsRegistry::new();
        assert!(registry.listen_handler.is_none());
    }
}
