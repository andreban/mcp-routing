// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Server discovery metadata and dynamic request-scoped discovery instructions.

use std::sync::Arc;
use tokio::sync::RwLock;

use mcp_routing::{
    BearerAuth, McpRouter, State,
    types::mcp::{CacheScope, Implementation},
};

use super::auth::resolve_optional_user;
use super::models::MovieDb;

/// Returns the static [`Implementation`] server metadata.
pub fn server_info() -> Implementation {
    Implementation::new("cinelist-mcp", "1.0.0")
        .with_title("CineList Movie Watchlist & Recommendation Server")
        .with_description("Stateless MCP server for movie discovery, personalized watchlists, and recommendations")
}

/// Dynamic `server/discover` provider customizing instructions based on caller identity.
pub async fn dynamic_server_discover(
    State(db): State<Arc<RwLock<MovieDb>>>,
    auth: Option<BearerAuth>,
) -> String {
    let guard = db.read().await;
    if let Some(user_id) = resolve_optional_user(auth.as_ref(), &guard) {
        if let Some(user) = guard.users.get(&user_id) {
            return format!(
                "Welcome back, {}! You have {} active watchlist(s) and {} rated movie(s). Streaming subscriptions: {}.",
                user.display_name,
                user.watchlists.len(),
                user.ratings.len(),
                user.streaming_subscriptions.join(", ")
            );
        }
    }

    "Stateless CineList MCP server for exploring movies, managing watchlists, and generating recommendations. Pass a Bearer token (e.g. 'token_alice_secret') for personalized features.".to_string()
}

/// Registers the dynamic server discovery provider and caching directives onto the [`McpRouter`].
pub fn register(router: McpRouter) -> McpRouter {
    router
        .discover(dynamic_server_discover)
        .server_discover_cache(Some(60_000), Some(CacheScope::Private))
}
