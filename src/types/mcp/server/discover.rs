// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::{
    jsonrpc::{JsonRpcRequest, JsonRpcResultResponse},
    mcp::{CacheScope, RequestMetaObject, ResultMetaObject, ServerCapabilities},
};

pub type ServerDiscoverRequest = JsonRpcRequest<ServerDiscoverParams>;
pub type ServerDiscoverResultResponse = JsonRpcResultResponse<ServerDiscoverResult>;

/// Parameters for a `server/discover` request.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#serverdiscoverrequest>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerDiscoverParams {
    /// Protocol-level request metadata.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<RequestMetaObject>,
    /// Additional unrecognized or custom metadata properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extras: HashMap<String, Value>,
}

/// The server's response to a `server/discover` request.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#serverdiscoverresult>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerDiscoverResult {
    /// Protocol-level response metadata.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<ResultMetaObject>,
    /// Result type discriminator string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_type: Option<String>,
    /// List of protocol versions supported by the server.
    pub supported_versions: Vec<String>,
    /// Capabilities supported by the server.
    pub capabilities: ServerCapabilities,
    /// Optional human-readable instructions or guidance for the server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    /// Optional time-to-live in milliseconds for caching the result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_ms: Option<u64>,
    /// Optional cache scope (`public` or `private`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_scope: Option<CacheScope>,
    /// Additional unrecognized or custom metadata properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extras: HashMap<String, Value>,
}

impl ServerDiscoverResult {
    /// Creates a new [`ServerDiscoverResult`] with the given capabilities and supported versions.
    pub fn new(capabilities: ServerCapabilities, supported_versions: Vec<String>) -> Self {
        Self {
            meta: None,
            result_type: Some("complete".to_string()),
            supported_versions,
            capabilities,
            instructions: None,
            ttl_ms: None,
            cache_scope: None,
            extras: HashMap::new(),
        }
    }

    /// Sets human-readable instructions on the discover result.
    pub fn with_instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    /// Sets caching parameters on the discover result.
    pub fn with_cache(mut self, ttl_ms: Option<u64>, cache_scope: Option<CacheScope>) -> Self {
        self.ttl_ms = ttl_ms;
        self.cache_scope = cache_scope;
        self
    }

    /// Sets the result metadata on the discover result.
    pub fn with_meta(mut self, meta: ResultMetaObject) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Sets the result type discriminator string.
    pub fn with_result_type(mut self, result_type: impl Into<String>) -> Self {
        self.result_type = Some(result_type.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests serialization and deserialization of `ServerDiscoverRequest` payloads.
    #[test]
    fn test_server_discover_request_serde() {
        let json_data = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "server/discover",
            "params": {
                "_meta": {
                    "io.modelcontextprotocol/protocolVersion": "2026-07-28"
                }
            }
        });

        let req: ServerDiscoverRequest = serde_json::from_value(json_data).unwrap();
        assert_eq!(req.method, "server/discover");
        let params = req.params.unwrap();
        assert_eq!(
            params.meta.unwrap().protocol_version.as_deref(),
            Some("2026-07-28")
        );
    }

    /// Tests serialization and deserialization of `ServerDiscoverResult` payloads.
    #[test]
    fn test_server_discover_result_serde() {
        let json_data = serde_json::json!({
            "resultType": "complete",
            "supportedVersions": ["2026-07-28"],
            "capabilities": {
                "tools": {}
            },
            "instructions": "Example server",
            "ttlMs": 0,
            "cacheScope": "public"
        });

        let result: ServerDiscoverResult = serde_json::from_value(json_data).unwrap();
        assert_eq!(result.supported_versions, vec!["2026-07-28"]);
        assert!(result.capabilities.tools.is_some());
        assert_eq!(result.instructions.as_deref(), Some("Example server"));
        assert_eq!(result.ttl_ms, Some(0));
        assert!(matches!(result.cache_scope, Some(CacheScope::Public)));

        let reserialized = serde_json::to_value(&result).unwrap();
        assert_eq!(reserialized["supportedVersions"][0], "2026-07-28");
        assert_eq!(reserialized["instructions"], "Example server");
    }
}
