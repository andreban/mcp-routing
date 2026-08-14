// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Capabilities a client may support. Known capabilities are defined here, in this schema,
/// but this is not a closed set: any client can define its own, additional capabilities.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#clientcapabilities>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientCapabilities {
    /// Experimental, non-standard capabilities that the client supports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<HashMap<String, Value>>,
    /// Present if the client supports sampling LLM completions from the server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<SamplingCapability>,
    /// Present if the client supports server-driven elicitation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elicitation: Option<ElicitationCapability>,
}

/// Capability configuration for sampling LLM completions.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#clientcapabilities>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SamplingCapability {}

/// Capability configuration for server-driven elicitation.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#clientcapabilities>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElicitationCapability {}

/// Capabilities a server may support. Known capabilities are defined here, in this schema,
/// but this is not a closed set: any server can define its own, additional capabilities.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#servercapabilities>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCapabilities {
    /// Present if the server supports tool operations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,
    /// Present if the server supports resource operations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourcesCapability>,
    /// Present if the server supports prompt templates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptsCapability>,
    /// Present if the server supports argument/value completion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completions: Option<CompletionsCapability>,
    /// Experimental, non-standard capabilities that the server supports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<HashMap<String, Value>>,
}

impl Default for ServerCapabilities {
    fn default() -> Self {
        Self::empty()
    }
}

impl ServerCapabilities {
    /// Creates a new empty [`ServerCapabilities`].
    pub fn empty() -> Self {
        Self {
            tools: None,
            resources: None,
            prompts: None,
            completions: None,
            experimental: None,
        }
    }

    /// Enables tools capability with optional `list_changed` notification support.
    pub fn with_tools(mut self, list_changed: Option<bool>) -> Self {
        self.tools = Some(ToolsCapability { list_changed });
        self
    }

    /// Enables resources capability with optional `subscribe` and `list_changed` flags.
    pub fn with_resources(mut self, subscribe: Option<bool>, list_changed: Option<bool>) -> Self {
        self.resources = Some(ResourcesCapability {
            subscribe,
            list_changed,
        });
        self
    }

    /// Enables prompts capability with optional `list_changed` notification support.
    pub fn with_prompts(mut self, list_changed: Option<bool>) -> Self {
        self.prompts = Some(PromptsCapability { list_changed });
        self
    }

    /// Enables completions capability.
    pub fn with_completions(mut self) -> Self {
        self.completions = Some(CompletionsCapability {});
        self
    }

    /// Adds experimental capabilities.
    pub fn with_experimental(mut self, experimental: HashMap<String, Value>) -> Self {
        self.experimental = Some(experimental);
        self
    }
}

/// Capability configuration for tool operations.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#servercapabilities>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsCapability {
    /// Optional hint indicating whether the server emits notifications when tool lists change.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Capability configuration for resource operations.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#servercapabilities>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesCapability {
    /// Optional hint indicating whether the server supports subscribing to resource updates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscribe: Option<bool>,
    /// Optional hint indicating whether the server emits notifications when resource lists change.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Capability configuration for prompt templates.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#servercapabilities>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptsCapability {
    /// Optional hint indicating whether the server emits notifications when prompt lists change.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Capability configuration for completion operations.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#servercapabilities>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionsCapability {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests serialization and deserialization of [`ClientCapabilities`].
    #[test]
    fn test_client_capabilities_serde() {
        let json_data = serde_json::json!({
            "sampling": {},
            "elicitation": {}
        });

        let caps: ClientCapabilities = serde_json::from_value(json_data).unwrap();
        assert!(caps.sampling.is_some());
        assert!(caps.elicitation.is_some());
        assert!(caps.experimental.is_none());

        let reserialized = serde_json::to_value(&caps).unwrap();
        assert!(reserialized.get("sampling").is_some());
    }

    /// Tests serialization and deserialization of [`ServerCapabilities`].
    #[test]
    fn test_server_capabilities_serde() {
        let mut exp = HashMap::new();
        exp.insert(
            "customFeature".to_string(),
            serde_json::json!({"enabled": true}),
        );

        let server_caps = ServerCapabilities {
            tools: Some(ToolsCapability {
                list_changed: Some(true),
            }),
            resources: Some(ResourcesCapability {
                subscribe: Some(true),
                list_changed: Some(false),
            }),
            prompts: Some(PromptsCapability {
                list_changed: Some(true),
            }),
            completions: Some(CompletionsCapability {}),
            experimental: Some(exp),
        };
        let s_val = serde_json::to_value(&server_caps).unwrap();
        assert_eq!(s_val["tools"]["listChanged"], true);
        assert_eq!(s_val["resources"]["subscribe"], true);
        assert_eq!(s_val["resources"]["listChanged"], false);
        assert_eq!(s_val["prompts"]["listChanged"], true);
        assert!(s_val.get("completions").is_some());
    }
}
