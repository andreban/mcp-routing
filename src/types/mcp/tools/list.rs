// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::{
    jsonrpc::{JsonRpcRequest, JsonRpcResultResponse},
    mcp::{CacheScope, RequestMetaObject, ResultMetaObject, tools::Tool},
};

pub type ListToolsRequest = JsonRpcRequest<ListToolsParams>;
pub type ListToolsResultResponse = JsonRpcResultResponse<ListToolsResult>;

/// Parameters for a `tools/list` request.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#listtoolsrequest>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListToolsParams {
    /// Protocol-level request metadata.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<RequestMetaObject>,
    /// An opaque cursor for retrieving the next page of tools, if paginated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    /// Additional unrecognized or custom metadata properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extras: HashMap<String, Value>,
}

/// The server's response to a `tools/list` request.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#listtoolsresult>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListToolsResult {
    /// Protocol-level response metadata.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<ResultMetaObject>,
    /// Result type discriminator string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_type: Option<String>,
    /// An opaque cursor for retrieving the next page of tools, if paginated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    /// Optional time-to-live in milliseconds for caching the result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_ms: Option<u64>,
    /// Optional cache scope (`public` or `private`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_scope: Option<CacheScope>,
    /// The list of tools provided by the server.
    pub tools: Vec<Tool>,
    /// Additional unrecognized or custom metadata properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extras: HashMap<String, Value>,
}

impl ListToolsResult {
    /// Creates a new [`ListToolsResult`] with the given tools list.
    pub fn new(tools: Vec<Tool>) -> Self {
        Self {
            meta: None,
            result_type: Some("complete".to_string()),
            next_cursor: None,
            ttl_ms: None,
            cache_scope: None,
            tools,
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

    /// Tests serialization and deserialization of `ListToolsResult` payloads.
    #[test]
    fn test_list_tools_result_serde() {
        let json_data = serde_json::json!({
            "tools": [{
                "name": "echo",
                "title": "Echo",
                "description": "Echoes the value back to the client",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "value": { "type": "string" }
                    }
                }
            }],
            "ttlMs": 0,
            "cacheScope": "public"
        });

        let result: ListToolsResult = serde_json::from_value(json_data).unwrap();
        assert_eq!(result.tools.len(), 1);
        assert_eq!(result.tools[0].name, "echo");
        assert_eq!(result.ttl_ms, Some(0));
        assert!(matches!(result.cache_scope, Some(CacheScope::Public)));

        let reserialized = serde_json::to_value(&result).unwrap();
        assert_eq!(reserialized["cacheScope"], "public");
        assert_eq!(reserialized["tools"][0]["name"], "echo");
    }

    /// Tests serialization and deserialization of `ListToolsRequest` payloads with cursor.
    #[test]
    fn test_list_tools_request_serde() {
        let json_data = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
            "params": {
                "cursor": "page_2",
                "_meta": {
                    "io.modelcontextprotocol/protocolVersion": "2026-07-28"
                }
            }
        });

        let req: ListToolsRequest = serde_json::from_value(json_data).unwrap();
        assert_eq!(req.method, "tools/list");
        let params = req.params.unwrap();
        assert_eq!(params.cursor.as_deref(), Some("page_2"));
        assert_eq!(
            params.meta.unwrap().protocol_version.as_deref(),
            Some("2026-07-28")
        );
    }
}
