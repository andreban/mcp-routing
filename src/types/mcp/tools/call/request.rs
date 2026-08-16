// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! MCP `tools/call` request types and parameters.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::{
    jsonrpc::JsonRpcRequest,
    mcp::RequestMetaObject,
};

pub type CallToolRequest<A = Value> = JsonRpcRequest<CallToolParams<A>>;

/// Parameters for a `tools/call` request.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#calltoolrequest>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallToolParams<A = Value> {
    /// Protocol-level request metadata.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<RequestMetaObject>,
    /// The name of the tool to call.
    pub name: String,
    /// Arguments to pass to the tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<A>,
    /// Additional unrecognized or custom metadata properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extras: HashMap<String, Value>,
}

impl<A> CallToolParams<A> {
    /// Returns the MRTR request state from params if present.
    pub fn request_state(&self) -> Option<&str> {
        self.extras.get("requestState").and_then(|v| v.as_str())
    }

    /// Returns the MRTR input responses from params if present.
    pub fn input_responses(&self) -> Option<HashMap<String, crate::types::mcp::InputResponse>> {
        self.extras
            .get("inputResponses")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
    }

    /// Sets the MRTR request state in params.
    pub fn with_request_state(mut self, state: impl Into<String>) -> Self {
        self.extras
            .insert("requestState".to_string(), Value::String(state.into()));
        self
    }

    /// Sets the MRTR input responses in params.
    pub fn with_input_responses(
        mut self,
        responses: HashMap<String, crate::types::mcp::InputResponse>,
    ) -> Self {
        if let Ok(v) = serde_json::to_value(responses) {
            self.extras.insert("inputResponses".to_string(), v);
        }
        self
    }
}
