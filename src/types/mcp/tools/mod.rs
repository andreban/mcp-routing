// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::mcp::{Icon, MetaObject};

pub mod call;
pub mod list;

/// The definition of a tool exposed by an MCP server.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#tool>
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

impl Tool {
    /// Creates a new [`Tool`] definition with the given name and a default object input schema.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            icons: Vec::new(),
            name: name.into(),
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

    /// Sets the human-readable display title for the tool.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the human-readable description of what the tool does.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the JSON Schema defining expected input arguments.
    pub fn input_schema(mut self, input_schema: impl Into<Value>) -> Self {
        self.input_schema = input_schema.into();
        self
    }

    /// Sets the optional JSON Schema defining the output structure.
    pub fn output_schema(mut self, output_schema: impl Into<Value>) -> Self {
        self.output_schema = Some(output_schema.into());
        self
    }

    /// Sets the execution hints and risk annotations for the tool.
    pub fn annotations(mut self, annotations: ToolAnnotations) -> Self {
        self.annotations = Some(annotations);
        self
    }

    /// Appends an icon to the tool's icon list.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icons.push(icon);
        self
    }

    /// Sets the full list of icons for the tool.
    pub fn icons(mut self, icons: Vec<Icon>) -> Self {
        self.icons = icons;
        self
    }

    /// Sets the protocol-level metadata for this tool.
    pub fn meta(mut self, meta: MetaObject) -> Self {
        self.meta = Some(meta);
        self
    }
}

impl From<String> for Tool {
    fn from(name: String) -> Self {
        Self::new(name)
    }
}

impl From<&str> for Tool {
    fn from(name: &str) -> Self {
        Self::new(name)
    }
}

impl From<Cow<'static, str>> for Tool {
    fn from(name: Cow<'static, str>) -> Self {
        Self::new(name.into_owned())
    }
}

/// Execution hints and behavioral annotations for a tool.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#toolannotations>
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

impl Default for ToolAnnotations {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolAnnotations {
    /// Creates a new empty [`ToolAnnotations`].
    pub fn new() -> Self {
        Self {
            title: None,
            read_only_hint: None,
            destructive_hint: None,
            idempotent_hint: None,
            open_world_hint: None,
        }
    }

    /// Sets the human-readable title for the tool.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets whether the tool only reads state without environment modifications.
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only_hint = Some(read_only);
        self
    }

    /// Sets whether environment changes made by the tool are destructive.
    pub fn destructive(mut self, destructive: bool) -> Self {
        self.destructive_hint = Some(destructive);
        self
    }

    /// Sets whether repeated calls with identical arguments yield identical results.
    pub fn idempotent(mut self, idempotent: bool) -> Self {
        self.idempotent_hint = Some(idempotent);
        self
    }

    /// Sets whether the tool interacts with open-world or external systems.
    pub fn open_world(mut self, open_world: bool) -> Self {
        self.open_world_hint = Some(open_world);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests `Into<Tool>` conversions and builder methods.
    #[test]
    fn test_tool_builder_conversions() {
        let t_str: Tool = "my_tool".into();
        assert_eq!(t_str.name, "my_tool");
        assert_eq!(t_str.input_schema, serde_json::json!({ "type": "object" }));

        let t_string: Tool = String::from("my_tool_2").into();
        assert_eq!(t_string.name, "my_tool_2");

        let t_cow: Tool = Cow::Borrowed("my_tool_3").into();
        assert_eq!(t_cow.name, "my_tool_3");

        let custom_tool = Tool::new("structured_tool")
            .title("Structured Output Tool")
            .description("Produces structured outputs")
            .input_schema(serde_json::json!({
                "type": "object",
                "properties": { "query": { "type": "string" } }
            }))
            .output_schema(serde_json::json!({
                "type": "object",
                "properties": { "result": { "type": "string" } }
            }))
            .annotations(
                ToolAnnotations::new()
                    .title("Structured Annotations")
                    .read_only(true)
                    .idempotent(true)
                    .destructive(false)
                    .open_world(false),
            );

        assert_eq!(custom_tool.name, "structured_tool");
        assert_eq!(custom_tool.title.as_deref(), Some("Structured Output Tool"));
        assert_eq!(
            custom_tool.description.as_deref(),
            Some("Produces structured outputs")
        );
        assert!(custom_tool.output_schema.is_some());
        let ann = custom_tool.annotations.unwrap();
        assert_eq!(ann.read_only_hint, Some(true));
        assert_eq!(ann.idempotent_hint, Some(true));
        assert_eq!(ann.destructive_hint, Some(false));
        assert_eq!(ann.open_world_hint, Some(false));
    }

    /// Tests serialization of [`ToolAnnotations`].
    #[test]
    fn test_tool_annotations_serde() {
        let annotations = ToolAnnotations::new()
            .title("Annotated Tool")
            .read_only(true)
            .destructive(false)
            .idempotent(true)
            .open_world(false);

        let json_ann = serde_json::to_value(&annotations).unwrap();
        assert_eq!(json_ann["title"], "Annotated Tool");
        assert_eq!(json_ann["readOnlyHint"], true);
        assert_eq!(json_ann["destructiveHint"], false);
        assert_eq!(json_ann["idempotentHint"], true);
        assert_eq!(json_ann["openWorldHint"], false);
    }
}
