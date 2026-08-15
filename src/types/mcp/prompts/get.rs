// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::{
    jsonrpc::{JsonRpcRequest, JsonRpcResultResponse},
    mcp::{RequestMetaObject, ResultMetaObject, prompts::PromptMessage},
};

pub type GetPromptRequest<A = Value> = JsonRpcRequest<GetPromptParams<A>>;
pub type GetPromptResultResponse = JsonRpcResultResponse<GetPromptResult>;

/// Parameters for a `prompts/get` request.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#getpromptrequest>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPromptParams<A = Value> {
    /// Protocol-level request metadata.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<RequestMetaObject>,
    /// The name of the prompt to retrieve.
    pub name: String,
    /// Arguments to pass to the prompt template.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<A>,
    /// Additional unrecognized or custom metadata properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extras: HashMap<String, Value>,
}

/// The server's response to a `prompts/get` request.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#getpromptresult>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPromptResult {
    /// Protocol-level response metadata.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<ResultMetaObject>,
    /// Result type discriminator string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_type: Option<String>,
    /// An optional human-readable description of the prompt result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The messages making up the prompt template.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub messages: Vec<PromptMessage>,
    /// Additional unrecognized or custom metadata properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extras: HashMap<String, Value>,
}

impl GetPromptResult {
    /// Creates a new [`GetPromptResult`] with the given messages.
    pub fn new(messages: Vec<PromptMessage>) -> Self {
        Self {
            meta: None,
            result_type: Some("complete".to_string()),
            description: None,
            messages,
            extras: HashMap::new(),
        }
    }

    /// Sets the human-readable description for this prompt result.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the result type discriminator string.
    pub fn with_result_type(mut self, result_type: impl Into<String>) -> Self {
        self.result_type = Some(result_type.into());
        self
    }

    /// Returns `true` if this result indicates additional input is required (`resultType == "input_required"`).
    pub fn is_input_required(&self) -> bool {
        self.result_type.as_deref() == Some("input_required")
    }

    /// Returns the MRTR request state if present in the result.
    pub fn request_state(&self) -> Option<&str> {
        self.extras.get("requestState").and_then(|v| v.as_str())
    }

    /// Creates a [`GetPromptResult`] containing a single `user` text message.
    pub fn user(text: impl Into<String>) -> Self {
        Self::new(vec![PromptMessage::user_text(text)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::mcp::{ContentBlock, Role};

    #[test]
    fn test_get_prompt_request_serde() {
        let json_data = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "prompts/get",
            "params": {
                "name": "summarize",
                "arguments": {
                    "text": "Antigravity is great"
                }
            }
        });

        let req: GetPromptRequest = serde_json::from_value(json_data).unwrap();
        assert_eq!(req.method, "prompts/get");
        let params = req.params.unwrap();
        assert_eq!(params.name, "summarize");
        assert_eq!(params.arguments.unwrap()["text"], "Antigravity is great");
    }

    #[test]
    fn test_get_prompt_result_serde() {
        let json_data = serde_json::json!({
            "resultType": "complete",
            "description": "Code review template",
            "messages": [{
                "role": "user",
                "content": {
                    "type": "text",
                    "text": "Review this code"
                }
            }]
        });

        let result: GetPromptResult = serde_json::from_value(json_data).unwrap();
        assert_eq!(result.result_type.as_deref(), Some("complete"));
        assert_eq!(result.description.as_deref(), Some("Code review template"));
        assert_eq!(result.messages.len(), 1);
        assert!(matches!(result.messages[0].role, Role::User));
        if let ContentBlock::Text(ref t) = result.messages[0].content {
            assert_eq!(t.text, "Review this code");
        } else {
            panic!("Expected ContentBlock::Text");
        }

        let reserialized = serde_json::to_value(&result).unwrap();
        assert_eq!(reserialized["description"], "Code review template");
        assert_eq!(reserialized["messages"][0]["role"], "user");
        assert_eq!(
            reserialized["messages"][0]["content"]["text"],
            "Review this code"
        );
    }

    #[test]
    fn test_get_prompt_result_constructors() {
        let res = GetPromptResult::user("Explain Rust lifetimes")
            .with_description("Lifetimes explainer prompt");
        assert_eq!(
            res.description.as_deref(),
            Some("Lifetimes explainer prompt")
        );
        assert_eq!(res.messages.len(), 1);
        assert!(matches!(res.messages[0].role, Role::User));
    }
}
