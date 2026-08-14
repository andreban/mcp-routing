// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use crate::types::mcp::{
    CacheScope, Implementation, ResultMetaObject, ServerCapabilities,
    server::discover::{
        ServerDiscoverRequest, ServerDiscoverResult, ServerDiscoverResultResponse,
    },
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
            meta: Some(ResultMetaObject {
                server_info: Some(server_info),
                extra: HashMap::new(),
            }),
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
