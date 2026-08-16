// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Subscriptions Registry and Dispatching
//!
//! Manages subscription stream listeners and dispatches `subscriptions/listen` requests.

use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll};

use bytes::Bytes;

use crate::body::{BoxError, ResponseBody};
use crate::extract::RequestContext;
use crate::router::{DispatchOutcome, MethodContext};
use crate::subscriptions::handler::{
    IntoSubscriptionsListenHandler, SubscriptionError, SubscriptionsListenHandler,
    SubscriptionsListenOutcome,
};
use crate::types::jsonrpc::JsonRpcErrorResponse;
use crate::types::mcp::{
    RequestMetaObject,
    subscriptions::{
        NotificationSubscriptions, SubscriptionsAcknowledgedParams, SubscriptionsListenParams,
        subscriptions_acknowledged_notification,
    },
};
use crate::utils::format_sse_message;

static SUBSCRIPTION_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Generates a unique subscription ID string.
fn next_subscription_id() -> String {
    let id = SUBSCRIPTION_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("sub-{id}")
}

struct ChainedBody {
    first: Option<Bytes>,
    second: ResponseBody,
}

impl http_body::Body for ChainedBody {
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        if let Some(first_bytes) = self.first.take() {
            return Poll::Ready(Some(Ok(http_body::Frame::data(first_bytes))));
        }
        Pin::new(&mut self.second).poll_frame(cx)
    }

    fn is_end_stream(&self) -> bool {
        self.first.is_none() && self.second.is_end_stream()
    }

    fn size_hint(&self) -> http_body::SizeHint {
        self.second.size_hint()
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
    pub(crate) async fn dispatch_listen(
        &self,
        ctx: MethodContext<'_>,
        params_val: Option<serde_json::Value>,
        tools_list_changed: bool,
        prompts_list_changed: bool,
        resources_list_changed: bool,
        known_resources: &[String],
    ) -> DispatchOutcome {
        if ctx.is_notification {
            return DispatchOutcome::notification();
        }

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

        let sub_id = params
            .meta
            .as_ref()
            .and_then(|m| m.subscription_id.clone())
            .unwrap_or_else(next_subscription_id);

        let mut ack_notifications = NotificationSubscriptions::default();
        if let Some(ref req_subs) = params.notifications {
            if req_subs.tools_list_changed == Some(true) && tools_list_changed {
                ack_notifications.tools_list_changed = Some(true);
            }
            if req_subs.prompts_list_changed == Some(true) && prompts_list_changed {
                ack_notifications.prompts_list_changed = Some(true);
            }
            if req_subs.resources_list_changed == Some(true) && resources_list_changed {
                ack_notifications.resources_list_changed = Some(true);
            }
            if let Some(ref uris) = req_subs.resource_subscriptions {
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
        ack_meta.subscription_id = Some(sub_id);
        let base_ack = SubscriptionsAcknowledgedParams::new(ack_notifications).with_meta(ack_meta);

        let outcome = if let Some(ref handler) = self.listen_handler {
            let request_ctx = RequestContext::new(
                params.meta.clone(),
                ctx.headers.clone(),
                ctx.extensions,
            );
            match handler.call(request_ctx, params, base_ack).await {
                Ok(res) => res,
                Err(SubscriptionError::InvalidParams(err)) => {
                    return DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                        ctx.req_id,
                        format!("Invalid params: {err}"),
                    ));
                }
                Err(SubscriptionError::Internal(err)) => {
                    return DispatchOutcome::error(JsonRpcErrorResponse::internal_error(
                        ctx.req_id,
                        format!("Subscription failed: {err}"),
                    ));
                }
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

        let body = if let Some(stream_body) = outcome.stream_body {
            ResponseBody::new(ChainedBody {
                first: Some(sse_bytes),
                second: stream_body,
            })
        } else {
            ResponseBody::from_bytes(sse_bytes)
        };

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
