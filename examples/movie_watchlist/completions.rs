// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Autocompletion handlers and registration for prompt arguments and resource templates.

use std::sync::Arc;
use tokio::sync::RwLock;

use mcp_routing::{
    BearerAuth, McpRouter, State,
    types::mcp::completion::{CompleteArgument, CompleteContext},
};

use super::auth::resolve_optional_user;
use super::models::MovieDb;

const GENRES: &[&str] = &[
    "Sci-Fi",
    "Drama",
    "Thriller",
    "Animation",
    "Action",
    "Comedy",
    "Horror",
    "Adventure",
    "Crime",
    "Music",
];

const MOODS: &[&str] = &[
    "mind-bending",
    "cozy",
    "edge-of-your-seat",
    "tearjerker",
    "dark-comedy",
    "inspiring",
    "atmospheric",
];

/// Registers all argument autocompletion handlers onto the [`McpRouter`].
pub fn register(router: McpRouter) -> McpRouter {
    router
        .register_prompt_arg_completion("movie_night_planner", "mood", complete_mood)
        .register_prompt_arg_completion("draft_review", "movie_id", complete_movie_id)
        .register_resource_arg_completion(
            "movies://catalog/{genre}/{movie_id}",
            "genre",
            complete_genre,
        )
        .register_resource_arg_completion(
            "movies://catalog/{genre}/{movie_id}",
            "movie_id",
            complete_movie_id,
        )
        .register_resource_arg_completion(
            "movies://users/{user_id}/watchlists/{list_id}",
            "list_id",
            complete_list_name,
        )
}

/// Autocompletion for `genre` arguments.
pub async fn complete_genre(arg: CompleteArgument) -> Vec<&'static str> {
    let prefix = arg.value.to_lowercase();
    GENRES
        .iter()
        .copied()
        .filter(|g| g.to_lowercase().starts_with(&prefix))
        .collect()
}

/// Autocompletion for `mood` arguments.
pub async fn complete_mood(arg: CompleteArgument) -> Vec<&'static str> {
    let prefix = arg.value.to_lowercase();
    MOODS
        .iter()
        .copied()
        .filter(|m| m.to_lowercase().starts_with(&prefix))
        .collect()
}

/// Autocompletion for user watchlists, securely scoped to the caller's profile.
pub async fn complete_list_name(
    State(db): State<Arc<RwLock<MovieDb>>>,
    auth: Option<BearerAuth>,
    arg: CompleteArgument,
) -> Vec<String> {
    let guard = db.read().await;
    let prefix = arg.value.to_lowercase();

    let user_id = match resolve_optional_user(auth.as_ref(), &guard) {
        Some(uid) => uid,
        None => return Vec::new(),
    };

    let user = match guard.users.get(&user_id) {
        Some(u) => u,
        None => return Vec::new(),
    };

    user.watchlists
        .keys()
        .filter(|name| name.to_lowercase().starts_with(&prefix))
        .cloned()
        .collect()
}

/// Context-aware autocompletion for `movie_id`, filtering by `genre` if present in `CompleteContext`.
pub async fn complete_movie_id(
    State(db): State<Arc<RwLock<MovieDb>>>,
    arg: CompleteArgument,
    context: Option<CompleteContext>,
) -> Vec<String> {
    let guard = db.read().await;
    let prefix = arg.value.to_lowercase();

    let filter_genre = context
        .as_ref()
        .and_then(|ctx| ctx.get_argument("genre"))
        .map(|g| g.to_lowercase());

    guard
        .catalog
        .values()
        .filter(|m| {
            if let Some(ref g) = filter_genre {
                m.genres.iter().any(|genre| genre.to_lowercase() == *g)
            } else {
                true
            }
        })
        .filter(|m| m.id.to_lowercase().starts_with(&prefix))
        .map(|m| m.id.clone())
        .collect()
}
