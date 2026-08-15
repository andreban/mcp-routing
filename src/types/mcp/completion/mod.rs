// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::{
    jsonrpc::{JsonRpcRequest, JsonRpcResultResponse},
    mcp::{RequestMetaObject, ResultMetaObject},
};

pub type CompleteRequest = JsonRpcRequest<CompleteParams>;
pub type CompleteResultResponse = JsonRpcResultResponse<CompleteResult>;

/// A reference identifying the target prompt or resource template being completed.
///
/// Discriminator field is `type`, matching `"ref/prompt"` or `"ref/resource"`.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#completerequest>
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum Reference {
    /// Reference to a prompt template.
    #[serde(rename = "ref/prompt")]
    Prompt {
        /// The name of the prompt template.
        name: String,
    },
    /// Reference to a resource template.
    #[serde(rename = "ref/resource")]
    Resource {
        /// The URI or URI template of the resource.
        uri: String,
    },
}

impl Reference {
    /// Creates a prompt reference.
    pub fn prompt(name: impl Into<String>) -> Self {
        Self::Prompt { name: name.into() }
    }

    /// Creates a resource reference.
    pub fn resource(uri: impl Into<String>) -> Self {
        Self::Resource { uri: uri.into() }
    }

    /// Returns `true` if this reference points to a prompt.
    pub fn is_prompt(&self) -> bool {
        matches!(self, Reference::Prompt { .. })
    }

    /// Returns `true` if this reference points to a resource.
    pub fn is_resource(&self) -> bool {
        matches!(self, Reference::Resource { .. })
    }

    /// Returns the prompt name if this reference is a prompt reference.
    pub fn name(&self) -> Option<&str> {
        match self {
            Reference::Prompt { name } => Some(name),
            Reference::Resource { .. } => None,
        }
    }

    /// Returns the resource URI if this reference is a resource reference.
    pub fn uri(&self) -> Option<&str> {
        match self {
            Reference::Prompt { .. } => None,
            Reference::Resource { uri } => Some(uri),
        }
    }
}

/// The argument being completed in a `completion/complete` request.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#completerequest>
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CompleteArgument {
    /// The name of the argument being completed.
    pub name: String,
    /// The current partial value entered for the argument.
    pub value: String,
}

impl CompleteArgument {
    /// Creates a new [`CompleteArgument`] with the given name and partial value.
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

/// Context information provided in a completion request, such as values for other arguments.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#completerequest>
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CompleteContext {
    /// Map of other previously resolved argument names and their values.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub arguments: HashMap<String, String>,
    /// Additional unrecognized or custom metadata properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extras: HashMap<String, Value>,
}

impl CompleteContext {
    /// Creates a new empty [`CompleteContext`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an argument key-value pair to the context.
    pub fn with_argument(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.arguments.insert(name.into(), value.into());
        self
    }

    /// Sets the entire map of argument key-value pairs.
    pub fn with_arguments(mut self, arguments: HashMap<String, String>) -> Self {
        self.arguments = arguments;
        self
    }

    /// Gets an argument value from the context by name.
    pub fn get_argument(&self, name: &str) -> Option<&str> {
        self.arguments.get(name).map(|s| s.as_str())
    }
}

/// Parameters for a `completion/complete` request.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#completerequest>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteParams {
    /// Protocol-level request metadata.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<RequestMetaObject>,
    /// The prompt or resource reference being completed.
    #[serde(rename = "ref")]
    pub reference: Reference,
    /// The argument being completed and its current value.
    pub argument: CompleteArgument,
    /// Additional context (e.g. other arguments) to assist in completion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<CompleteContext>,
    /// Additional unrecognized or custom metadata properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extras: HashMap<String, Value>,
}

impl CompleteParams {
    /// Creates a new [`CompleteParams`] for a prompt argument.
    pub fn prompt(
        prompt_name: impl Into<String>,
        arg_name: impl Into<String>,
        arg_value: impl Into<String>,
    ) -> Self {
        Self {
            meta: None,
            reference: Reference::prompt(prompt_name),
            argument: CompleteArgument::new(arg_name, arg_value),
            context: None,
            extras: HashMap::new(),
        }
    }

    /// Creates a new [`CompleteParams`] for a resource URI variable.
    pub fn resource(
        resource_uri: impl Into<String>,
        arg_name: impl Into<String>,
        arg_value: impl Into<String>,
    ) -> Self {
        Self {
            meta: None,
            reference: Reference::resource(resource_uri),
            argument: CompleteArgument::new(arg_name, arg_value),
            context: None,
            extras: HashMap::new(),
        }
    }

    /// Sets additional context for the completion request.
    pub fn with_context(mut self, context: CompleteContext) -> Self {
        self.context = Some(context);
        self
    }

    /// Sets request metadata.
    pub fn with_meta(mut self, meta: RequestMetaObject) -> Self {
        self.meta = Some(meta);
        self
    }
}

/// The completion results payload returned in a `CompleteResult`.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#completeresult>
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CompletionValues {
    /// An array of completion values (must not exceed 100 items).
    pub values: Vec<String>,
    /// The total number of completion options available, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    /// Indicates whether there are additional completion options beyond those provided.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_more: Option<bool>,
}

impl CompletionValues {
    /// Creates a new [`CompletionValues`] with the provided values.
    pub fn new(values: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            values: values.into_iter().map(Into::into).collect(),
            total: None,
            has_more: None,
        }
    }

    /// Creates an empty [`CompletionValues`].
    pub fn empty() -> Self {
        Self::default()
    }

    /// Sets the total number of completion options available.
    pub fn with_total(mut self, total: u64) -> Self {
        self.total = Some(total);
        self
    }

    /// Sets whether more completion options are available.
    pub fn with_has_more(mut self, has_more: bool) -> Self {
        self.has_more = Some(has_more);
        self
    }

    /// Pushes a new completion suggestion.
    pub fn push(&mut self, value: impl Into<String>) {
        self.values.push(value.into());
    }

    /// Clamps the suggestion list to `max_items` (default MCP maximum is 100),
    /// setting `has_more = Some(true)` if truncated.
    pub fn clamp_to_limit(mut self, max_items: usize) -> Self {
        if self.values.len() > max_items {
            if self.total.is_none() {
                self.total = Some(self.values.len() as u64);
            }
            self.values.truncate(max_items);
            self.has_more = Some(true);
        }
        self
    }
}

/// The server's response payload to a `completion/complete` request.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#completeresult>
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CompleteResult {
    /// Protocol-level response metadata.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<ResultMetaObject>,
    /// The completion values, total count, and pagination indicator.
    pub completion: CompletionValues,
    /// Additional unrecognized or custom metadata properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extras: HashMap<String, Value>,
}

impl CompleteResult {
    /// Creates a new [`CompleteResult`] with the given values.
    pub fn new(values: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            meta: None,
            completion: CompletionValues::new(values),
            extras: HashMap::new(),
        }
    }

    /// Creates an empty [`CompleteResult`].
    pub fn empty() -> Self {
        Self::default()
    }

    /// Creates a [`CompleteResult`] from [`CompletionValues`].
    pub fn with_completion(completion: CompletionValues) -> Self {
        Self {
            meta: None,
            completion,
            extras: HashMap::new(),
        }
    }

    /// Sets the total number of completion options available.
    pub fn with_total(mut self, total: u64) -> Self {
        self.completion.total = Some(total);
        self
    }

    /// Sets whether more completion options are available.
    pub fn with_has_more(mut self, has_more: bool) -> Self {
        self.completion.has_more = Some(has_more);
        self
    }

    /// Sets response metadata.
    pub fn with_meta(mut self, meta: ResultMetaObject) -> Self {
        self.meta = Some(meta);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_reference_serde() {
        let reference = Reference::prompt("code_review");
        assert!(reference.is_prompt());
        assert!(!reference.is_resource());
        assert_eq!(reference.name(), Some("code_review"));
        assert_eq!(reference.uri(), None);

        let json = serde_json::to_value(&reference).unwrap();
        assert_eq!(json["type"], "ref/prompt");
        assert_eq!(json["name"], "code_review");

        let parsed: Reference = serde_json::from_value(json).unwrap();
        assert_eq!(parsed, reference);
    }

    #[test]
    fn test_resource_reference_serde() {
        let reference = Reference::resource("file:///{path}");
        assert!(!reference.is_prompt());
        assert!(reference.is_resource());
        assert_eq!(reference.name(), None);
        assert_eq!(reference.uri(), Some("file:///{path}"));

        let json = serde_json::to_value(&reference).unwrap();
        assert_eq!(json["type"], "ref/resource");
        assert_eq!(json["uri"], "file:///{path}");

        let parsed: Reference = serde_json::from_value(json).unwrap();
        assert_eq!(parsed, reference);
    }

    #[test]
    fn test_complete_request_serde() {
        let json_data = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "completion/complete",
            "params": {
                "ref": {
                    "type": "ref/prompt",
                    "name": "code_review"
                },
                "argument": {
                    "name": "language",
                    "value": "py"
                },
                "context": {
                    "arguments": {
                        "framework": "fastapi"
                    }
                }
            }
        });

        let req: CompleteRequest = serde_json::from_value(json_data).unwrap();
        assert_eq!(req.method, "completion/complete");
        let params = req.params.unwrap();
        assert_eq!(params.reference.name(), Some("code_review"));
        assert_eq!(params.argument.name, "language");
        assert_eq!(params.argument.value, "py");
        assert_eq!(
            params
                .context
                .as_ref()
                .and_then(|c| c.get_argument("framework")),
            Some("fastapi")
        );
    }

    #[test]
    fn test_complete_result_serde() {
        let result = CompleteResult::new(vec!["python", "pytorch", "pyside"])
            .with_total(10)
            .with_has_more(true);

        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["completion"]["values"][0], "python");
        assert_eq!(json["completion"]["values"][1], "pytorch");
        assert_eq!(json["completion"]["values"][2], "pyside");
        assert_eq!(json["completion"]["total"], 10);
        assert_eq!(json["completion"]["hasMore"], true);

        let parsed: CompleteResult = serde_json::from_value(json).unwrap();
        assert_eq!(
            parsed.completion.values,
            vec!["python", "pytorch", "pyside"]
        );
        assert_eq!(parsed.completion.total, Some(10));
        assert_eq!(parsed.completion.has_more, Some(true));
    }

    #[test]
    fn test_completion_values_clamp_to_limit() {
        let mut vals = Vec::new();
        for i in 0..150 {
            vals.push(format!("item_{i}"));
        }

        let cv = CompletionValues::new(vals).clamp_to_limit(100);
        assert_eq!(cv.values.len(), 100);
        assert_eq!(cv.total, Some(150));
        assert_eq!(cv.has_more, Some(true));
    }
}
