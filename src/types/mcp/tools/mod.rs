use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::mcp::{Icon, MetaObject};

pub mod call;
pub mod list;

/// The definition of a tool exposed by an MCP server.
///
/// See https://modelcontextprotocol.io/specification/2026-07-28/schema#tool
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    /// Optional list of icons for display in user interfaces.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub icons: Vec<Icon>,
    /// The unique name of the tool.
    pub name: String,
    /// Human-readable display title for the tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// A human-readable description of what the tool does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// A JSON Schema object defining the expected input arguments.
    pub input_schema: Value,
    /// An optional JSON Schema object defining the output structure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Value>,
    /// Optional execution hints and risk annotations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ToolAnnotations>,
    /// Optional protocol-level metadata.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaObject>,
}

impl From<String> for Tool {
    fn from(name: String) -> Self {
        Self {
            icons: Vec::new(),
            name,
            title: None,
            description: None,
            input_schema: serde_json::json!({
                "type": "object"
            }),
            output_schema: None,
            annotations: None,
            meta: None,
        }
    }
}

impl From<&str> for Tool {
    fn from(name: &str) -> Self {
        name.to_string().into()
    }
}

impl From<Cow<'static, str>> for Tool {
    fn from(name: Cow<'static, str>) -> Self {
        name.into_owned().into()
    }
}

/// Execution hints and behavioral annotations for a tool.
///
/// See https://modelcontextprotocol.io/specification/2026-07-28/schema#toolannotations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolAnnotations {
    /// Human-readable title for the tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Hint indicating whether the tool only reads state without environment modifications.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_only_hint: Option<bool>,
    /// Hint indicating whether environment changes made by the tool are destructive.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destructive_hint: Option<bool>,
    /// Hint indicating whether repeated calls with identical arguments yield identical results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotent_hint: Option<bool>,
    /// Hint indicating whether the tool interacts with open-world or external systems.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_world_hint: Option<bool>,
}
