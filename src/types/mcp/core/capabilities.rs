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
    /// Present if the client supports listing roots.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roots: Option<RootsCapability>,
    /// Standardized extensions that the client supports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<HashMap<String, Value>>,
}

/// Capability configuration for root operations.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#clientcapabilities>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RootsCapability {
    /// Optional hint indicating whether the client emits notifications when its list of roots changes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
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
    /// Present if the server supports logging operations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<LoggingCapability>,
    /// Experimental, non-standard capabilities that the server supports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<HashMap<String, Value>>,
    /// Standardized extensions that the server supports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<HashMap<String, Value>>,
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
            logging: None,
            experimental: None,
            extensions: None,
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

    /// Enables logging capability.
    pub fn with_logging(mut self) -> Self {
        self.logging = Some(LoggingCapability {});
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
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CompletionsCapability {}

/// Capability configuration for logging operations.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#servercapabilities>
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LoggingCapability {}

#[cfg(test)]
mod tests {
    //! Unit tests for MCP client and server capability structures and serialization.

    use super::*;

    /// Tests serialization and deserialization of [`ClientCapabilities`] including roots and extensions.
    #[test]
    fn test_client_capabilities_serde() {
        let mut extensions = HashMap::new();
        extensions.insert(
            "io.modelcontextprotocol/oauth".to_string(),
            serde_json::json!({"version": "1.0"}),
        );

        let json_data = serde_json::json!({
            "sampling": {},
            "elicitation": {},
            "roots": {
                "listChanged": true
            },
            "extensions": {
                "io.modelcontextprotocol/oauth": {
                    "version": "1.0"
                }
            }
        });

        let caps: ClientCapabilities = serde_json::from_value(json_data).unwrap();
        assert!(caps.sampling.is_some());
        assert!(caps.elicitation.is_some());
        assert!(caps.experimental.is_none());
        assert_eq!(caps.roots.as_ref().and_then(|r| r.list_changed), Some(true));
        assert!(caps.extensions.is_some());

        let reserialized = serde_json::to_value(&caps).unwrap();
        assert!(reserialized.get("sampling").is_some());
        assert_eq!(reserialized["roots"]["listChanged"], true);
        assert_eq!(
            reserialized["extensions"]["io.modelcontextprotocol/oauth"]["version"],
            "1.0"
        );
    }

    /// Tests serialization and deserialization of [`ServerCapabilities`] including extensions.
    #[test]
    fn test_server_capabilities_serde() {
        let mut exp = HashMap::new();
        exp.insert(
            "customFeature".to_string(),
            serde_json::json!({"enabled": true}),
        );

        let mut extensions = HashMap::new();
        extensions.insert(
            "io.modelcontextprotocol/customExt".to_string(),
            serde_json::json!({"supported": true}),
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
            logging: Some(LoggingCapability {}),
            experimental: Some(exp),
            extensions: Some(extensions),
        };
        let s_val = serde_json::to_value(&server_caps).unwrap();
        assert_eq!(s_val["tools"]["listChanged"], true);
        assert_eq!(s_val["resources"]["subscribe"], true);
        assert_eq!(s_val["resources"]["listChanged"], false);
        assert_eq!(s_val["prompts"]["listChanged"], true);
        assert!(s_val.get("completions").is_some());
        assert!(s_val.get("logging").is_some());
        assert_eq!(
            s_val["extensions"]["io.modelcontextprotocol/customExt"]["supported"],
            true
        );

        let deserialized: ServerCapabilities = serde_json::from_value(s_val).unwrap();
        assert_eq!(
            deserialized.tools.as_ref().and_then(|t| t.list_changed),
            Some(true)
        );
        assert_eq!(
            deserialized.resources.as_ref().and_then(|r| r.subscribe),
            Some(true)
        );
        assert_eq!(
            deserialized.resources.as_ref().and_then(|r| r.list_changed),
            Some(false)
        );
        assert_eq!(
            deserialized.prompts.as_ref().and_then(|p| p.list_changed),
            Some(true)
        );
        assert!(deserialized.completions.is_some());
        assert!(deserialized.logging.is_some());
        assert!(deserialized.experimental.is_some());
        assert!(deserialized.extensions.is_some());
    }
}
