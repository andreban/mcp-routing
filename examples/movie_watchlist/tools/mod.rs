// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Tool handler modules and dynamic request-scoped tool discovery.

pub mod catalog;
pub mod ratings;
pub mod watchlists;

use mcp_routing::{
    BearerAuth, McpRouter,
    extract::RegisteredTools,
    types::mcp::{CacheScope, tools::Tool},
};

/// Dynamic `tools/list` provider demonstrating request-scoped capability filtering.
///
/// Unauthenticated callers discover only public catalog tools, while authenticated
/// callers discover the full suite of personalized watchlist and rating tools.
pub async fn dynamic_tools_list(
    auth: Option<BearerAuth>,
    RegisteredTools(all_tools): RegisteredTools,
) -> Vec<Tool> {
    let is_authenticated = auth.is_some();
    all_tools
        .into_iter()
        .filter(|tool| {
            if is_authenticated {
                true
            } else {
                matches!(
                    tool.name.as_str(),
                    "search_movies" | "get_movie_details" | "generate_movie_poster"
                )
            }
        })
        .collect()
}

/// Registers all tool submodules and the dynamic tools discovery handler onto the [`McpRouter`].
pub fn register(mut router: McpRouter) -> McpRouter {
    router = catalog::register(router);
    router = watchlists::register(router);
    router = ratings::register(router);
    router
        .tools_list(dynamic_tools_list)
        .tools_list_cache(Some(60_000), Some(CacheScope::Private))
}
