// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::{
    jsonrpc::{JsonRpcRequest, JsonRpcResultResponse},
    mcp::{CacheScope, RequestMetaObject, ResultMetaObject, resources::ResourceTemplate},
};

pub type ListResourceTemplatesRequest = JsonRpcRequest<ListResourceTemplatesParams>;
pub type ListResourceTemplatesResultResponse = JsonRpcResultResponse<ListResourceTemplatesResult>;

/// Parameters for a `resources/templates/list` request.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#listresourcetemplatesrequest>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResourceTemplatesParams {
    /// Protocol-level request metadata.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<RequestMetaObject>,
    /// An opaque cursor for retrieving the next page of resource templates, if paginated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    /// Additional unrecognized or custom metadata properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extras: HashMap<String, Value>,
}

/// The server's response to a `resources/templates/list` request.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#listresourcetemplatesresult>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResourceTemplatesResult {
    /// Protocol-level response metadata.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<ResultMetaObject>,
    /// Result type discriminator string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_type: Option<String>,
    /// An opaque cursor for retrieving the next page of resource templates, if paginated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    /// Optional time-to-live in milliseconds for caching the result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_ms: Option<u64>,
    /// Optional cache scope (`public` or `private`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_scope: Option<CacheScope>,
    /// The list of resource templates provided by the server.
    pub resource_templates: Vec<ResourceTemplate>,
    /// Additional unrecognized or custom metadata properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extras: HashMap<String, Value>,
}

impl ListResourceTemplatesResult {
    /// Creates a new [`ListResourceTemplatesResult`] with the given resource templates list.
    pub fn new(resource_templates: Vec<ResourceTemplate>) -> Self {
        Self {
            meta: None,
            result_type: Some("complete".to_string()),
            next_cursor: None,
            ttl_ms: None,
            cache_scope: None,
            resource_templates,
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
    fn test_list_resource_templates_result_serde() {
        let json_data = serde_json::json!({
            "resourceTemplates": [{
                "uriTemplate": "file:///{path}",
                "name": "Local Files",
                "title": "Local File Explorer",
                "description": "Access local project files"
            }],
            "ttlMs": 30000,
            "cacheScope": "private"
        });

        let result: ListResourceTemplatesResult = serde_json::from_value(json_data).unwrap();
        assert_eq!(result.resource_templates.len(), 1);
        assert_eq!(
            result.resource_templates[0].uri_template,
            "file:///{path}"
        );
        assert_eq!(result.resource_templates[0].name, "Local Files");
        assert_eq!(result.ttl_ms, Some(30000));
        assert!(matches!(result.cache_scope, Some(CacheScope::Private)));

        let reserialized = serde_json::to_value(&result).unwrap();
        assert_eq!(reserialized["cacheScope"], "private");
        assert_eq!(
            reserialized["resourceTemplates"][0]["uriTemplate"],
            "file:///{path}"
        );
    }

    #[test]
    fn test_list_resource_templates_request_serde() {
        let json_data = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "resources/templates/list",
            "params": {
                "cursor": "template_page_1"
            }
        });

        let req: ListResourceTemplatesRequest = serde_json::from_value(json_data).unwrap();
        assert_eq!(req.method, "resources/templates/list");
        let params = req.params.unwrap();
        assert_eq!(params.cursor.as_deref(), Some("template_page_1"));
    }
}
