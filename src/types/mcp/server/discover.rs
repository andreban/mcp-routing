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
/// See https://modelcontextprotocol.io/specification/2026-07-28/schema#serverdiscoverrequest
#[derive(Debug, Serialize, Deserialize)]
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
/// See https://modelcontextprotocol.io/specification/2026-07-28/schema#serverdiscoverresult
#[derive(Debug, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

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
