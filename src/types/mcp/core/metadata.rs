// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::capabilities::ClientCapabilities;
use super::info::Implementation;
use crate::types::jsonrpc::JsonRpcRequestId;

/// A progress token, used to associate progress notifications with the original request.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#progresstoken>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProgressToken {
    /// A numeric progress token.
    Number(f32),
    /// A string progress token.
    String(String),
}

/// Additional metadata associated with a request or entity.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#metaobject>
pub type MetaObject = HashMap<String, Value>;

/// An object containing metadata for a result.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#resultmetaobject>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultMetaObject {
    /// Identifies the server software producing the response.
    /// Servers SHOULD include this field on every response unless specifically configured not to do so.
    #[serde(
        rename = "io.modelcontextprotocol/serverInfo",
        skip_serializing_if = "Option::is_none"
    )]
    pub server_info: Option<Implementation>,
    /// The JSON-RPC ID of the `subscriptions/listen` request a notification belongs to.
    #[serde(
        rename = "io.modelcontextprotocol/subscriptionId",
        skip_serializing_if = "Option::is_none"
    )]
    pub subscription_id: Option<JsonRpcRequestId>,
    /// Additional metadata properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, Value>,
}

impl ResultMetaObject {
    /// Creates a new [`ResultMetaObject`] with optional server info.
    pub fn new(server_info: Option<Implementation>) -> Self {
        Self {
            server_info,
            subscription_id: None,
            extra: HashMap::new(),
        }
    }
}

/// An object containing metadata for a request.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#requestmetaobject>
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RequestMetaObject {
    /// A progress token used to associate progress notifications with the original request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress_token: Option<ProgressToken>,
    /// Identifies the client software producing the request.
    #[serde(
        rename = "io.modelcontextprotocol/clientInfo",
        skip_serializing_if = "Option::is_none"
    )]
    pub client_info: Option<Implementation>,
    /// Capabilities supported by the client for this request.
    #[serde(
        rename = "io.modelcontextprotocol/clientCapabilities",
        skip_serializing_if = "Option::is_none"
    )]
    pub client_capabilities: Option<ClientCapabilities>,
    /// Specifies the MCP protocol version being used for the request.
    #[serde(
        rename = "io.modelcontextprotocol/protocolVersion",
        skip_serializing_if = "Option::is_none"
    )]
    pub protocol_version: Option<String>,
    /// The JSON-RPC ID of the `subscriptions/listen` request a notification belongs to.
    #[serde(
        rename = "io.modelcontextprotocol/subscriptionId",
        skip_serializing_if = "Option::is_none"
    )]
    pub subscription_id: Option<JsonRpcRequestId>,
    /// Additional metadata properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, Value>,
}

impl RequestMetaObject {
    /// Creates a new empty [`RequestMetaObject`].
    pub fn empty() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests serialization and deserialization of [`ResultMetaObject`].
    #[test]
    fn test_result_meta_object_serde() {
        let json_data = serde_json::json!({
            "io.modelcontextprotocol/serverInfo": {
                "name": "test-server",
                "version": "1.0.0"
            },
            "custom/meta": "value"
        });

        let meta: ResultMetaObject = serde_json::from_value(json_data).unwrap();
        assert_eq!(meta.server_info.as_ref().unwrap().name, "test-server");
        assert_eq!(meta.extra.get("custom/meta").unwrap(), "value");

        let reserialized = serde_json::to_value(&meta).unwrap();
        assert_eq!(
            reserialized["io.modelcontextprotocol/serverInfo"]["name"],
            "test-server"
        );
        assert_eq!(reserialized["custom/meta"], "value");
    }

    /// Tests numeric and string [`ProgressToken`] parsing.
    #[test]
    fn test_progress_token_serde() {
        let num_token: ProgressToken = serde_json::from_value(serde_json::json!(42.0)).unwrap();
        assert!(matches!(num_token, ProgressToken::Number(n) if (n - 42.0).abs() < f32::EPSILON));

        let str_token: ProgressToken =
            serde_json::from_value(serde_json::json!("tok-123")).unwrap();
        assert!(matches!(str_token, ProgressToken::String(s) if s == "tok-123"));
    }
}
