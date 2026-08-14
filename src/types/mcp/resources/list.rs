// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::{
    jsonrpc::{JsonRpcRequest, JsonRpcResultResponse},
    mcp::{CacheScope, RequestMetaObject, ResultMetaObject, resources::Resource},
};

pub type ListResourcesRequest = JsonRpcRequest<ListResourcesParams>;
pub type ListResourcesResultResponse = JsonRpcResultResponse<ListResourcesResult>;

/// Parameters for a `resources/list` request.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#listresourcesrequest>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResourcesParams {
    /// Protocol-level request metadata.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<RequestMetaObject>,
    /// An opaque cursor for retrieving the next page of resources, if paginated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    /// Additional unrecognized or custom metadata properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extras: HashMap<String, Value>,
}

/// The server's response to a `resources/list` request.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#listresourcesresult>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResourcesResult {
    /// Protocol-level response metadata.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<ResultMetaObject>,
    /// Result type discriminator string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_type: Option<String>,
    /// An opaque cursor for retrieving the next page of resources, if paginated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    /// Optional time-to-live in milliseconds for caching the result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_ms: Option<u64>,
    /// Optional cache scope (`public` or `private`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_scope: Option<CacheScope>,
    /// The list of resources provided by the server.
    pub resources: Vec<Resource>,
    /// Additional unrecognized or custom metadata properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extras: HashMap<String, Value>,
}

impl ListResourcesResult {
    /// Creates a new [`ListResourcesResult`] with the given resources list.
    pub fn new(resources: Vec<Resource>) -> Self {
        Self {
            meta: None,
            result_type: Some("complete".to_string()),
            next_cursor: None,
            ttl_ms: None,
            cache_scope: None,
            resources,
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

    #[test]
    fn test_list_resources_result_serde() {
        let json_data = serde_json::json!({
            "resources": [{
                "uri": "file:///project/readme.md",
                "name": "README",
                "title": "Project Readme",
                "description": "Project documentation",
                "mimeType": "text/markdown",
                "size": 1024
            }],
            "ttlMs": 60000,
            "cacheScope": "public"
        });

        let result: ListResourcesResult = serde_json::from_value(json_data).unwrap();
        assert_eq!(result.resources.len(), 1);
        assert_eq!(result.resources[0].uri, "file:///project/readme.md");
        assert_eq!(result.resources[0].name, "README");
        assert_eq!(result.ttl_ms, Some(60000));
        assert!(matches!(result.cache_scope, Some(CacheScope::Public)));

        let reserialized = serde_json::to_value(&result).unwrap();
        assert_eq!(reserialized["cacheScope"], "public");
        assert_eq!(
            reserialized["resources"][0]["uri"],
            "file:///project/readme.md"
        );
    }

    #[test]
    fn test_list_resources_request_serde() {
        let json_data = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "resources/list",
            "params": {
                "cursor": "resource_page_2",
                "_meta": {
                    "io.modelcontextprotocol/protocolVersion": "2026-07-28"
                }
            }
        });

        let req: ListResourcesRequest = serde_json::from_value(json_data).unwrap();
        assert_eq!(req.method, "resources/list");
        let params = req.params.unwrap();
        assert_eq!(params.cursor.as_deref(), Some("resource_page_2"));
        assert_eq!(
            params.meta.unwrap().protocol_version.as_deref(),
            Some("2026-07-28")
        );
    }
}
