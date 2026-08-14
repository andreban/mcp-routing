// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use crate::types::mcp::resources::{
    ResourceContents,
    read::{ReadResourceRequest, ReadResourceResult, ReadResourceResultResponse},
};

/// Handles an MCP `resources/read` request by constructing a [`ReadResourceResultResponse`] with the provided resource contents.
pub fn handle_read_resource(
    req: ReadResourceRequest,
    contents: Vec<ResourceContents>,
) -> ReadResourceResultResponse {
    ReadResourceResultResponse::new(
        req.id,
        ReadResourceResult::new(contents),
    )
}
