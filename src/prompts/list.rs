// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use crate::types::mcp::{
    CacheScope,
    prompts::{
        Prompt,
        list::{ListPromptsRequest, ListPromptsResult, ListPromptsResultResponse},
    },
};

/// Handles an MCP `prompts/list` request by constructing a [`ListPromptsResultResponse`] with the registered prompts.
pub fn handle_list_prompts(
    req: ListPromptsRequest,
    prompts: Vec<Prompt>,
    ttl_ms: Option<u64>,
    cache_scope: Option<CacheScope>,
) -> ListPromptsResultResponse {
    ListPromptsResultResponse::new(
        req.id,
        ListPromptsResult {
            meta: None,
            result_type: Some("complete".to_string()),
            next_cursor: None,
            ttl_ms,
            cache_scope,
            prompts,
            extras: HashMap::new(),
        },
    )
}
