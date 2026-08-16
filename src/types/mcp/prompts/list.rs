// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::{
    jsonrpc::{JsonRpcRequest, JsonRpcResultResponse},
    mcp::{CacheScope, RequestMetaObject, ResultMetaObject, prompts::Prompt},
};

pub type ListPromptsRequest = JsonRpcRequest<ListPromptsParams>;
pub type ListPromptsResultResponse = JsonRpcResultResponse<ListPromptsResult>;

/// Parameters for a `prompts/list` request.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#listpromptsrequest>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPromptsParams {
    /// Protocol-level request metadata.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<RequestMetaObject>,
    /// An opaque cursor for retrieving the next page of prompts, if paginated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    /// Additional unrecognized or custom metadata properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extras: HashMap<String, Value>,
}

/// The server's response to a `prompts/list` request.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#listpromptsresult>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPromptsResult {
    /// Protocol-level response metadata.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<ResultMetaObject>,
    /// Result type discriminator string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_type: Option<String>,
    /// An opaque cursor for retrieving the next page of prompts, if paginated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    /// Optional time-to-live in milliseconds for caching the result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_ms: Option<u64>,
    /// Optional cache scope (`public` or `private`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_scope: Option<CacheScope>,
    /// The list of prompts provided by the server.
    pub prompts: Vec<Prompt>,
    /// Additional unrecognized or custom metadata properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extras: HashMap<String, Value>,
}

impl ListPromptsResult {
    /// Creates a new [`ListPromptsResult`] with the given prompts list.
    pub fn new(prompts: Vec<Prompt>) -> Self {
        Self {
            meta: None,
            result_type: Some("complete".to_string()),
            next_cursor: None,
            ttl_ms: None,
            cache_scope: None,
            prompts,
            extras: HashMap::new(),
        }
    }

    /// Sets the next pagination cursor.
    pub fn with_next_cursor(mut self, next_cursor: impl Into<String>) -> Self {
        self.next_cursor = Some(next_cursor.into());
        self
    }

    /// Sets caching directives on the list result.
    pub fn with_cache(mut self, ttl_ms: Option<u64>, cache_scope: Option<CacheScope>) -> Self {
        self.ttl_ms = ttl_ms;
        self.cache_scope = cache_scope;
        self
    }

    /// Sets response metadata on the list result.
    pub fn with_meta(mut self, meta: ResultMetaObject) -> Self {
        self.meta = Some(meta);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests serialization and deserialization of `ListPromptsResult` payloads.
    #[test]
    fn test_list_prompts_result_serde() {
        let json_data = serde_json::json!({
            "prompts": [{
                "name": "review_code",
                "title": "Review Code",
                "description": "Reviews source code diffs",
                "arguments": [{
                    "name": "code",
                    "description": "Source code to review",
                    "required": true
                }]
            }],
            "ttlMs": 60000,
            "cacheScope": "public"
        });

        let result: ListPromptsResult = serde_json::from_value(json_data).unwrap();
        assert_eq!(result.prompts.len(), 1);
        assert_eq!(result.prompts[0].name, "review_code");
        assert_eq!(result.ttl_ms, Some(60000));
        assert!(matches!(result.cache_scope, Some(CacheScope::Public)));

        let reserialized = serde_json::to_value(&result).unwrap();
        assert_eq!(reserialized["cacheScope"], "public");
        assert_eq!(reserialized["prompts"][0]["name"], "review_code");
    }

    /// Tests serialization and deserialization of `ListPromptsRequest` payloads with pagination cursor.
    #[test]
    fn test_list_prompts_request_serde() {
        let json_data = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "prompts/list",
            "params": {
                "cursor": "prompt_page_2",
                "_meta": {
                    "io.modelcontextprotocol/protocolVersion": "2026-07-28"
                }
            }
        });

        let req: ListPromptsRequest = serde_json::from_value(json_data).unwrap();
        assert_eq!(req.method, "prompts/list");
        let params = req.params.unwrap();
        assert_eq!(params.cursor.as_deref(), Some("prompt_page_2"));
        assert_eq!(
            params.meta.unwrap().protocol_version.as_deref(),
            Some("2026-07-28")
        );
    }
}
