use std::collections::HashMap;

use crate::types::mcp::{
    CacheScope,
    tools::{
        Tool,
        list::{ListToolsRequest, ListToolsResult, ListToolsResultResponse},
    },
};

pub fn handle_list_tools(
    req: ListToolsRequest,
    tools: Vec<Tool>,
) -> ListToolsResultResponse {
    ListToolsResultResponse::new(
        req.id,
        ListToolsResult {
            meta: None,
            result_type: Some("complete".to_string()),
            next_cursor: None,
            ttl_ms: Some(0),
            cache_scope: Some(CacheScope::Public),
            tools,
            extras: HashMap::new(),
        },
    )
}
