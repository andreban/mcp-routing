// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! `subscriptions/listen` extractor for subscription handler functions.
//!
//! Provides the [`Subscription`] extractor so handlers can filter the notifications they stream
//! and tag each one with the subscription ID.

use crate::extract::context::RequestContext;
use crate::extract::error::ExtractionError;
use crate::extract::traits::FromRequestContext;
use crate::types::jsonrpc::JsonRpcRequestId;
use crate::types::mcp::{RequestMetaObject, subscriptions::NotificationSubscriptions};

/// Extractor for the `subscriptions/listen` stream being served.
///
/// Available only in `subscriptions/listen` handlers.
///
/// # Example
///
/// ```rust,ignore
/// async fn handle_listen(subscription: Subscription) -> ResponseBody {
///     if subscription.notifications.tools_list_changed == Some(true) {
///         let notif = tools_list_changed_notification(Some(
///             ListChangedParams::new().with_meta(subscription.meta()),
///         ));
///         // ... stream `notif` on the returned body
///     }
///     # unimplemented!()
/// }
/// ```
#[derive(Clone)]
pub struct Subscription {
    /// The subscription ID: the JSON-RPC ID of the `subscriptions/listen` request.
    ///
    /// Every notification delivered on the stream MUST carry it in
    /// `_meta["io.modelcontextprotocol/subscriptionId"]` (see [`Subscription::meta`]).
    pub id: JsonRpcRequestId,
    /// The acknowledged notification filter: the types the client requested, narrowed to those
    /// supported by the server's declared capabilities. The server MUST NOT send other types.
    pub notifications: NotificationSubscriptions,
}

impl Subscription {
    /// Returns notification `_meta` carrying this subscription's ID.
    pub fn meta(&self) -> RequestMetaObject {
        let mut meta = RequestMetaObject::empty();
        meta.subscription_id = Some(self.id.clone());
        meta
    }
}

impl FromRequestContext for Subscription {
    type Error = ExtractionError;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        ctx.extensions()
            .get::<Subscription>()
            .cloned()
            .ok_or_else(|| {
                ExtractionError(
                    "Subscription is only available in subscriptions/listen".to_string(),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for the [`Subscription`] extractor.

    use super::*;
    use http::HeaderMap;
    use std::sync::Arc;

    /// Tests extracting a [`Subscription`] and building its notification `_meta`.
    #[test]
    fn test_subscription_extractor() {
        let mut ext = http::Extensions::new();
        ext.insert(Subscription {
            id: 7.into(),
            notifications: NotificationSubscriptions::new().with_tools_list_changed(true),
        });
        let ctx = RequestContext::new(None, HeaderMap::new(), Arc::new(ext));

        let subscription = Subscription::from_request_context(&ctx).unwrap();
        assert_eq!(subscription.id, JsonRpcRequestId::Number(7));
        assert_eq!(subscription.notifications.tools_list_changed, Some(true));

        let meta = serde_json::to_value(subscription.meta()).unwrap();
        assert_eq!(meta["io.modelcontextprotocol/subscriptionId"], 7);

        let empty_ctx =
            RequestContext::new(None, HeaderMap::new(), Arc::new(http::Extensions::new()));
        assert!(Subscription::from_request_context(&empty_ctx).is_err());
    }
}
