// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::{
    jsonrpc::{JsonRpcRequest, JsonRpcResultResponse},
    mcp::{
        CacheScope, RequestMetaObject, ResultMetaObject,
        resources::{BlobResourceContents, ResourceContents, TextResourceContents},
    },
};

pub type ReadResourceRequest = JsonRpcRequest<ReadResourceParams>;
pub type ReadResourceResultResponse = JsonRpcResultResponse<ReadResourceResult>;

/// Parameters for a `resources/read` request.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#readresourcerequest>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadResourceParams {
    /// Protocol-level request metadata.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<RequestMetaObject>,
    /// The URI of the resource to read.
    pub uri: String,
    /// Additional unrecognized or custom metadata properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extras: HashMap<String, Value>,
}

/// The server's response to a `resources/read` request.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#readresourceresult>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadResourceResult {
    /// Protocol-level response metadata.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<ResultMetaObject>,
    /// Result type discriminator string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_type: Option<String>,
    /// Optional cache time-to-live in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_ms: Option<u64>,
    /// Optional cache scope (`public` or `private`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_scope: Option<CacheScope>,
    /// The contents of the resource.
    pub contents: Vec<ResourceContents>,
    /// Additional unrecognized or custom metadata properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extras: HashMap<String, Value>,
}

impl ReadResourceResult {
    /// Creates a new [`ReadResourceResult`] with the given resource contents.
    pub fn new(contents: Vec<ResourceContents>) -> Self {
        Self {
            meta: None,
            result_type: Some("complete".to_string()),
            ttl_ms: Some(0),
            cache_scope: Some(CacheScope::Public),
            contents,
            extras: HashMap::new(),
        }
    }

    /// Creates a [`ReadResourceResult`] containing a single text resource.
    pub fn text(
        uri: impl Into<String>,
        text: impl Into<String>,
        mime_type: Option<impl Into<String>>,
    ) -> Self {
        Self::new(vec![ResourceContents::Text(TextResourceContents {
            uri: uri.into(),
            text: text.into(),
            mime_type: mime_type.map(Into::into),
        })])
    }

    /// Creates a [`ReadResourceResult`] containing a single binary blob resource.
    pub fn blob(
        uri: impl Into<String>,
        blob: impl Into<String>,
        mime_type: Option<impl Into<String>>,
    ) -> Self {
        Self::new(vec![ResourceContents::Blob(BlobResourceContents {
            uri: uri.into(),
            blob: blob.into(),
            mime_type: mime_type.map(Into::into),
        })])
    }

    /// Sets the caching directives on the read result.
    pub fn with_cache(mut self, ttl_ms: Option<u64>, cache_scope: Option<CacheScope>) -> Self {
        self.ttl_ms = ttl_ms;
        self.cache_scope = cache_scope;
        self
    }

    /// Sets the TTL in milliseconds on the read result.
    pub fn with_ttl_ms(mut self, ttl_ms: u64) -> Self {
        self.ttl_ms = Some(ttl_ms);
        self
    }

    /// Sets the cache scope on the read result.
    pub fn with_cache_scope(mut self, cache_scope: CacheScope) -> Self {
        self.cache_scope = Some(cache_scope);
        self
    }

    /// Sets response metadata on the read result.
    pub fn with_meta(mut self, meta: ResultMetaObject) -> Self {
        self.meta = Some(meta);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_resource_request_serde() {
        let json_data = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "resources/read",
            "params": {
                "uri": "file:///logs/app.log"
            }
        });

        let req: ReadResourceRequest = serde_json::from_value(json_data).unwrap();
        assert_eq!(req.method, "resources/read");
        let params = req.params.unwrap();
        assert_eq!(params.uri, "file:///logs/app.log");
    }

    #[test]
    fn test_read_resource_result_serde() {
        let json_data = serde_json::json!({
            "resultType": "complete",
            "ttlMs": 60000,
            "cacheScope": "public",
            "contents": [{
                "uri": "file:///logs/app.log",
                "text": "2026-08-15 Server initialized",
                "mimeType": "text/plain"
            }]
        });

        let result: ReadResourceResult = serde_json::from_value(json_data).unwrap();
        assert_eq!(result.result_type.as_deref(), Some("complete"));
        assert_eq!(result.ttl_ms, Some(60000));
        assert_eq!(result.cache_scope, Some(CacheScope::Public));
        assert_eq!(result.contents.len(), 1);
        if let ResourceContents::Text(ref text_res) = result.contents[0] {
            assert_eq!(text_res.uri, "file:///logs/app.log");
            assert_eq!(text_res.text, "2026-08-15 Server initialized");
            assert_eq!(text_res.mime_type.as_deref(), Some("text/plain"));
        } else {
            panic!("Expected text resource contents");
        }

        let reserialized = serde_json::to_value(&result).unwrap();
        assert_eq!(reserialized["contents"][0]["uri"], "file:///logs/app.log");
        assert_eq!(
            reserialized["contents"][0]["text"],
            "2026-08-15 Server initialized"
        );
        assert_eq!(reserialized["ttlMs"], 60000);
        assert_eq!(reserialized["cacheScope"], "public");
    }

    #[test]
    fn test_read_resource_result_constructors() {
        let res_text =
            ReadResourceResult::text("file:///memo.txt", "Antigravity notes", Some("text/plain"));
        assert_eq!(res_text.contents.len(), 1);
        assert_eq!(res_text.ttl_ms, Some(0));
        assert_eq!(res_text.cache_scope, Some(CacheScope::Public));
        assert!(matches!(res_text.contents[0], ResourceContents::Text(_)));

        let res_blob = ReadResourceResult::blob("file:///image.png", "aGVsbG8=", Some("image/png"))
            .with_cache(Some(3600000), Some(CacheScope::Private));
        assert_eq!(res_blob.contents.len(), 1);
        assert_eq!(res_blob.ttl_ms, Some(3600000));
        assert_eq!(res_blob.cache_scope, Some(CacheScope::Private));
        assert!(matches!(res_blob.contents[0], ResourceContents::Blob(_)));
    }
}
