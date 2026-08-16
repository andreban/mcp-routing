// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use crate::types::mcp::{
    CacheScope, Implementation, ResultMetaObject, ServerCapabilities,
    server::discover::{ServerDiscoverRequest, ServerDiscoverResult, ServerDiscoverResultResponse},
};

/// Handles an MCP `server/discover` request by constructing a [`ServerDiscoverResultResponse`].
pub fn handle_server_discover(
    req: ServerDiscoverRequest,
    server_info: Implementation,
    instructions: Option<String>,
    capabilities: ServerCapabilities,
    supported_versions: Vec<String>,
    ttl_ms: Option<u64>,
    cache_scope: Option<CacheScope>,
) -> ServerDiscoverResultResponse {
    ServerDiscoverResultResponse::new(
        req.id,
        ServerDiscoverResult {
            meta: Some(ResultMetaObject::new(Some(server_info))),
            result_type: Some("complete".to_string()),
            supported_versions,
            capabilities,
            instructions,
            ttl_ms,
            cache_scope,
            extras: HashMap::new(),
        },
    )
}

/// Validates whether the client's requested protocol version is supported by the server.
///
/// If `client_version` is `Some`, verifies that the version string is present in `supported_versions`.
/// If `client_version` is `None`, version validation succeeds by default.
pub fn validate_protocol_version(
    client_version: Option<&str>,
    supported_versions: &[String],
) -> Result<(), String> {
    if let Some(ver) = client_version
        && !supported_versions.iter().any(|v| v == ver)
    {
        return Err(format!(
            "Unsupported protocol version '{ver}'. Supported versions: {}",
            supported_versions.join(", ")
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_protocol_version() {
        let supported = vec!["2026-07-28".to_string(), "2026-01-01".to_string()];

        // Exact matches
        assert!(validate_protocol_version(Some("2026-07-28"), &supported).is_ok());
        assert!(validate_protocol_version(Some("2026-01-01"), &supported).is_ok());

        // Omitted version
        assert!(validate_protocol_version(None, &supported).is_ok());

        // Unsupported version
        let err = validate_protocol_version(Some("2024-11-05"), &supported).unwrap_err();
        assert!(err.contains("Unsupported protocol version '2024-11-05'"));
        assert!(err.contains("2026-07-28, 2026-01-01"));
    }
}
