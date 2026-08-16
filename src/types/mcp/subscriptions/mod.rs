// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Subscriptions Protocol Types (SEP-2575)
//!
//! Types for the stateless `subscriptions/listen` notification stream introduced in MCP `2026-07-28`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::jsonrpc::{JsonRpcNotification, JsonRpcRequest, JsonRpcResultResponse};
use crate::types::mcp::{RequestMetaObject, ResultMetaObject};

/// Notification subscription filter options indicating which server-initiated events
/// the client wishes to receive on a subscription stream.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#notificationsubscriptions>
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NotificationSubscriptions {
    /// Indicates whether to subscribe to tool list change notifications (`notifications/tools/list_changed`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools_list_changed: Option<bool>,
    /// Indicates whether to subscribe to prompt list change notifications (`notifications/prompts/list_changed`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts_list_changed: Option<bool>,
    /// Indicates whether to subscribe to resource list change notifications (`notifications/resources/list_changed`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources_list_changed: Option<bool>,
    /// Enumerates resource URIs for which the client wishes to receive resource update notifications (`notifications/resources/updated`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_subscriptions: Option<Vec<String>>,
    /// Additional unrecognized or custom metadata properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, Value>,
}

impl NotificationSubscriptions {
    /// Creates a new empty [`NotificationSubscriptions`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets `tools_list_changed` opt-in.
    pub fn with_tools_list_changed(mut self, enabled: bool) -> Self {
        self.tools_list_changed = Some(enabled);
        self
    }

    /// Sets `prompts_list_changed` opt-in.
    pub fn with_prompts_list_changed(mut self, enabled: bool) -> Self {
        self.prompts_list_changed = Some(enabled);
        self
    }

    /// Sets `resources_list_changed` opt-in.
    pub fn with_resources_list_changed(mut self, enabled: bool) -> Self {
        self.resources_list_changed = Some(enabled);
        self
    }

    /// Sets `resource_subscriptions` URIs.
    pub fn with_resource_subscriptions(mut self, uris: Vec<String>) -> Self {
        self.resource_subscriptions = Some(uris);
        self
    }
}

/// Parameters for a `subscriptions/listen` request.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#subscriptionslistenparams>
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionsListenParams {
    /// Protocol metadata for the request.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<RequestMetaObject>,
    /// Declarations of which notification types the client wants to receive.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notifications: Option<NotificationSubscriptions>,
    /// Additional unrecognized or custom properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, Value>,
}

pub type SubscriptionsListenRequest = JsonRpcRequest<SubscriptionsListenParams>;
pub type SubscriptionsListenResultResponse = JsonRpcResultResponse<SubscriptionsListenResult>;

/// Parameters for the `notifications/subscriptions/acknowledged` notification sent as the first
/// message on an established subscription stream.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#subscriptionsacknowledgedparams>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionsAcknowledgedParams {
    /// Protocol metadata including the correlated `subscription_id`.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<RequestMetaObject>,
    /// The subset of requested notifications that the server has agreed to honor.
    pub notifications: NotificationSubscriptions,
    /// Additional unrecognized or custom properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, Value>,
}

impl SubscriptionsAcknowledgedParams {
    /// Creates a new [`SubscriptionsAcknowledgedParams`] with the honored notifications filter.
    pub fn new(notifications: NotificationSubscriptions) -> Self {
        Self {
            meta: None,
            notifications,
            extra: HashMap::new(),
        }
    }

    /// Attaches metadata to the acknowledgment.
    pub fn with_meta(mut self, meta: RequestMetaObject) -> Self {
        self.meta = Some(meta);
        self
    }
}

/// Parameters for a `notifications/resources/updated` notification.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#resourceupdatedparams>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceUpdatedParams {
    /// The URI of the resource that was updated.
    pub uri: String,
    /// Protocol metadata including the correlated `subscription_id`.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<RequestMetaObject>,
    /// Additional unrecognized or custom properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, Value>,
}

impl ResourceUpdatedParams {
    /// Creates a new [`ResourceUpdatedParams`] for the given resource URI.
    pub fn new(uri: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            meta: None,
            extra: HashMap::new(),
        }
    }

    /// Attaches metadata to the notification.
    pub fn with_meta(mut self, meta: RequestMetaObject) -> Self {
        self.meta = Some(meta);
        self
    }
}

/// Parameters for list changed notifications (`notifications/tools/list_changed`,
/// `notifications/prompts/list_changed`, `notifications/resources/list_changed`).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListChangedParams {
    /// Protocol metadata including the correlated `subscription_id`.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<RequestMetaObject>,
    /// Additional unrecognized or custom properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, Value>,
}

impl ListChangedParams {
    /// Creates a new empty [`ListChangedParams`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Attaches metadata to the notification.
    pub fn with_meta(mut self, meta: RequestMetaObject) -> Self {
        self.meta = Some(meta);
        self
    }
}

/// Terminal result for a `subscriptions/listen` request, sent when the subscription terminates gracefully.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#subscriptionslistenresult>
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionsListenResult {
    /// Protocol metadata for the result.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<ResultMetaObject>,
    /// Additional unrecognized or custom properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, Value>,
}

/// Constructs a `notifications/subscriptions/acknowledged` JSON-RPC notification.
pub fn subscriptions_acknowledged_notification(
    params: SubscriptionsAcknowledgedParams,
) -> JsonRpcNotification<SubscriptionsAcknowledgedParams> {
    JsonRpcNotification::new("notifications/subscriptions/acknowledged", Some(params))
}

/// Constructs a `notifications/tools/list_changed` JSON-RPC notification.
pub fn tools_list_changed_notification(
    params: Option<ListChangedParams>,
) -> JsonRpcNotification<ListChangedParams> {
    JsonRpcNotification::new("notifications/tools/list_changed", params)
}

/// Constructs a `notifications/prompts/list_changed` JSON-RPC notification.
pub fn prompts_list_changed_notification(
    params: Option<ListChangedParams>,
) -> JsonRpcNotification<ListChangedParams> {
    JsonRpcNotification::new("notifications/prompts/list_changed", params)
}

/// Constructs a `notifications/resources/list_changed` JSON-RPC notification.
pub fn resources_list_changed_notification(
    params: Option<ListChangedParams>,
) -> JsonRpcNotification<ListChangedParams> {
    JsonRpcNotification::new("notifications/resources/list_changed", params)
}

/// Constructs a `notifications/resources/updated` JSON-RPC notification.
pub fn resource_updated_notification(
    params: ResourceUpdatedParams,
) -> JsonRpcNotification<ResourceUpdatedParams> {
    JsonRpcNotification::new("notifications/resources/updated", Some(params))
}

#[cfg(test)]
mod tests {
    //! Unit tests for MCP subscriptions protocol types and notification constructors.

    use super::*;

    /// Tests serialization and deserialization of [`NotificationSubscriptions`].
    #[test]
    fn test_notification_subscriptions_serde() {
        let subs = NotificationSubscriptions::new()
            .with_tools_list_changed(true)
            .with_prompts_list_changed(false)
            .with_resources_list_changed(true)
            .with_resource_subscriptions(vec!["file:///test.txt".to_string()]);

        let json = serde_json::to_value(&subs).unwrap();
        assert_eq!(json["toolsListChanged"], true);
        assert_eq!(json["promptsListChanged"], false);
        assert_eq!(json["resourcesListChanged"], true);
        assert_eq!(json["resourceSubscriptions"][0], "file:///test.txt");

        let deserialized: NotificationSubscriptions = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized.tools_list_changed, Some(true));
        assert_eq!(deserialized.prompts_list_changed, Some(false));
        assert_eq!(deserialized.resources_list_changed, Some(true));
        assert_eq!(
            deserialized.resource_subscriptions,
            Some(vec!["file:///test.txt".to_string()])
        );
    }

    /// Tests serialization and deserialization of [`SubscriptionsAcknowledgedParams`] and notification construction.
    #[test]
    fn test_subscriptions_acknowledged_notification() {
        let mut meta = RequestMetaObject::empty();
        meta.subscription_id = Some("sub-12345".to_string());

        let params = SubscriptionsAcknowledgedParams::new(
            NotificationSubscriptions::new().with_tools_list_changed(true),
        )
        .with_meta(meta);

        let notif = subscriptions_acknowledged_notification(params);
        assert_eq!(notif.method, "notifications/subscriptions/acknowledged");
        let json = serde_json::to_value(&notif).unwrap();
        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["method"], "notifications/subscriptions/acknowledged");
        assert_eq!(
            json["params"]["_meta"]["io.modelcontextprotocol/subscriptionId"],
            "sub-12345"
        );
        assert_eq!(json["params"]["notifications"]["toolsListChanged"], true);
    }

    /// Tests [`ResourceUpdatedParams`] and notification construction.
    #[test]
    fn test_resource_updated_notification() {
        let mut meta = RequestMetaObject::empty();
        meta.subscription_id = Some("sub-abc".to_string());

        let params = ResourceUpdatedParams::new("sqlite://data.db/tables").with_meta(meta);
        let notif = resource_updated_notification(params);
        assert_eq!(notif.method, "notifications/resources/updated");

        let json = serde_json::to_value(&notif).unwrap();
        assert_eq!(json["params"]["uri"], "sqlite://data.db/tables");
        assert_eq!(
            json["params"]["_meta"]["io.modelcontextprotocol/subscriptionId"],
            "sub-abc"
        );
    }

    /// Tests list changed notifications constructors.
    #[test]
    fn test_list_changed_notifications() {
        let mut meta = RequestMetaObject::empty();
        meta.subscription_id = Some("sub-999".to_string());

        let tools_notif =
            tools_list_changed_notification(Some(ListChangedParams::new().with_meta(meta.clone())));
        assert_eq!(tools_notif.method, "notifications/tools/list_changed");

        let prompts_notif = prompts_list_changed_notification(Some(
            ListChangedParams::new().with_meta(meta.clone()),
        ));
        assert_eq!(prompts_notif.method, "notifications/prompts/list_changed");

        let res_notif =
            resources_list_changed_notification(Some(ListChangedParams::new().with_meta(meta)));
        assert_eq!(res_notif.method, "notifications/resources/list_changed");
    }
}
