// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::types::mcp::{ContentBlock, Icon, MetaObject, Role, TextContent};

pub mod get;
pub mod list;

/// Describes an argument accepted by a prompt template.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#promptargument>
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PromptArgument {
    /// The name of the argument.
    pub name: String,
    /// Human-readable display title for the argument.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// A human-readable description of the argument.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether this argument must be provided.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

impl PromptArgument {
    /// Creates a new prompt argument with the specified name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            title: None,
            description: None,
            required: None,
        }
    }

    /// Sets the human-readable display title for this argument.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the human-readable description for this argument.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets whether this argument is required.
    pub fn required(mut self, required: bool) -> Self {
        self.required = Some(required);
        self
    }
}

/// The definition of a prompt template exposed by an MCP server.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#prompt>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prompt {
    /// Optional list of icons for display in user interfaces.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub icons: Vec<Icon>,
    /// The unique name of the prompt template.
    pub name: String,
    /// Human-readable display title for the prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// A human-readable description of what the prompt does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// A list of arguments accepted by this prompt template.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub arguments: Vec<PromptArgument>,
    /// Optional protocol-level metadata.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaObject>,
}

impl Prompt {
    /// Creates a new [`Prompt`] with the given name and empty arguments.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            icons: Vec::new(),
            name: name.into(),
            title: None,
            description: None,
            arguments: Vec::new(),
            meta: None,
        }
    }

    /// Sets the human-readable display title for the prompt.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the human-readable description for the prompt.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Appends an argument to the prompt's argument list.
    pub fn argument(mut self, argument: PromptArgument) -> Self {
        self.arguments.push(argument);
        self
    }

    /// Sets the full list of arguments for this prompt.
    pub fn arguments(mut self, arguments: Vec<PromptArgument>) -> Self {
        self.arguments = arguments;
        self
    }
}

impl From<String> for Prompt {
    fn from(name: String) -> Self {
        Self::new(name)
    }
}

impl From<&str> for Prompt {
    fn from(name: &str) -> Self {
        Self::new(name)
    }
}

impl From<Cow<'static, str>> for Prompt {
    fn from(name: Cow<'static, str>) -> Self {
        Self::new(name.into_owned())
    }
}

/// A message in an MCP prompt template.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#promptmessage>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptMessage {
    /// The role of the message sender (`user` or `assistant`).
    pub role: Role,
    /// The content of the message.
    pub content: ContentBlock,
}

impl PromptMessage {
    /// Creates a new prompt message with the specified role and content block.
    pub fn new(role: Role, content: impl Into<ContentBlock>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }

    /// Creates a new `user` role prompt message.
    pub fn user(content: impl Into<ContentBlock>) -> Self {
        Self::new(Role::User, content)
    }

    /// Creates a new `assistant` role prompt message.
    pub fn assistant(content: impl Into<ContentBlock>) -> Self {
        Self::new(Role::Assistant, content)
    }

    /// Creates a new `user` prompt message containing a simple text string.
    pub fn user_text(text: impl Into<String>) -> Self {
        Self::user(TextContent {
            text: text.into(),
            annotations: None,
            meta: None,
        })
    }

    /// Creates a new `assistant` prompt message containing a simple text string.
    pub fn assistant_text(text: impl Into<String>) -> Self {
        Self::assistant(TextContent {
            text: text.into(),
            annotations: None,
            meta: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests `Into<Prompt>` conversions from string primitives and `Cow`.
    #[test]
    fn test_prompt_builder_conversions() {
        let p_str: Prompt = "code_review".into();
        assert_eq!(p_str.name, "code_review");
        assert!(p_str.arguments.is_empty());

        let p_string: Prompt = String::from("summarize").into();
        assert_eq!(p_string.name, "summarize");

        let p_cow: Prompt = Cow::Borrowed("explain_code").into();
        assert_eq!(p_cow.name, "explain_code");

        let prompt = Prompt::new("translate")
            .title("Translate Text")
            .description("Translates input text to a target language")
            .argument(
                PromptArgument::new("text")
                    .title("Input Text")
                    .description("The text to translate")
                    .required(true),
            )
            .argument(
                PromptArgument::new("target_language")
                    .title("Target Language")
                    .required(false),
            );

        assert_eq!(prompt.name, "translate");
        assert_eq!(prompt.title.as_deref(), Some("Translate Text"));
        assert_eq!(prompt.arguments.len(), 2);
        assert_eq!(prompt.arguments[0].name, "text");
        assert_eq!(prompt.arguments[0].required, Some(true));
        assert_eq!(prompt.arguments[1].name, "target_language");
        assert_eq!(prompt.arguments[1].required, Some(false));
    }

    /// Tests serialization of [`Prompt`] and [`PromptArgument`].
    #[test]
    fn test_prompt_serde() {
        let prompt = Prompt::new("greeting")
            .title("Greeting Prompt")
            .description("Generates a friendly greeting")
            .argument(
                PromptArgument::new("name")
                    .description("The user's name")
                    .required(true),
            );

        let json_val = serde_json::to_value(&prompt).unwrap();
        assert_eq!(json_val["name"], "greeting");
        assert_eq!(json_val["title"], "Greeting Prompt");
        assert_eq!(json_val["description"], "Generates a friendly greeting");
        assert_eq!(json_val["arguments"][0]["name"], "name");
        assert_eq!(json_val["arguments"][0]["description"], "The user's name");
        assert_eq!(json_val["arguments"][0]["required"], true);

        let deserialized: Prompt = serde_json::from_value(json_val).unwrap();
        assert_eq!(deserialized.name, "greeting");
        assert_eq!(deserialized.arguments.len(), 1);
        assert_eq!(deserialized.arguments[0].name, "name");
    }

    /// Tests [`PromptMessage`] creation and serialization.
    #[test]
    fn test_prompt_message_serde() {
        let user_msg = PromptMessage::user_text("Please review this code");
        assert!(matches!(user_msg.role, Role::User));

        let json_val = serde_json::to_value(&user_msg).unwrap();
        assert_eq!(json_val["role"], "user");
        assert_eq!(json_val["content"]["type"], "text");
        assert_eq!(json_val["content"]["text"], "Please review this code");

        let assistant_msg = PromptMessage::assistant_text("Here is my feedback");
        assert!(matches!(assistant_msg.role, Role::Assistant));

        let json_assistant = serde_json::to_value(&assistant_msg).unwrap();
        assert_eq!(json_assistant["role"], "assistant");
        assert_eq!(json_assistant["content"]["text"], "Here is my feedback");
    }
}
