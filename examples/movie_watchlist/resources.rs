// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Resource handlers, dynamic RFC 6570 URI templates, and request-scoped resource discovery.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use mcp_routing::{
    BearerAuth, McpRouter, State,
    extract::RegisteredResources,
    types::mcp::{
        CacheScope, Role,
        resources::{ReadResourceResult, Resource, ResourceAnnotations, ResourceTemplate},
    },
};

use super::auth::{resolve_optional_user, verify_user_access};
use super::models::MovieDb;

const LOGO_PNG_BASE64: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

/// Dynamic `resources/list` provider demonstrating request-aware resource discovery.
///
/// Returns static curated catalog resources and dynamically appends the authenticated
/// caller's private active watchlists.
pub async fn dynamic_resources_list(
    State(db): State<Arc<RwLock<MovieDb>>>,
    auth: Option<BearerAuth>,
    RegisteredResources(static_resources): RegisteredResources,
) -> Vec<Resource> {
    let mut resources = static_resources;
    let guard = db.read().await;

    if let Some(user_id) = resolve_optional_user(auth.as_ref(), &guard) {
        if let Some(user) = guard.users.get(&user_id) {
            for (list_name, list) in &user.watchlists {
                let mut res = Resource::new(
                    format!("movies://users/{user_id}/watchlists/{list_name}"),
                    format!("Watchlist: {list_name}"),
                )
                .title(format!("{}'s {}", user.display_name, list_name))
                .mime_type("application/json")
                .annotations(
                    ResourceAnnotations::new()
                        .audience(vec![Role::User])
                        .priority(0.95),
                );
                if let Some(ref desc) = list.description {
                    res = res.description(desc);
                }
                resources.push(res);
            }
        }
    }

    resources
}

/// Registers all direct resources, templates, and dynamic resource discovery onto the [`McpRouter`].
pub fn register(router: McpRouter) -> McpRouter {
    let catalog_guide_resource = Resource::new("movies://genres/catalog", "Genres Catalog Guide")
        .title("Movie Catalog & Genre Overview")
        .description("Markdown breakdown of curated genres and title counts")
        .mime_type("text/markdown")
        .annotations(
            ResourceAnnotations::new()
                .audience(vec![Role::User, Role::Assistant])
                .priority(0.9),
        );

    let top250_resource = Resource::new("movies://curated/top250", "Curated Classics")
        .title("Top Rated Masterpieces")
        .description("JSON document of highest-rated catalog films")
        .mime_type("application/json");

    let logo_resource = Resource::new("movies://branding/cinelist-logo.png", "CineList Logo")
        .title("Branding Logo")
        .mime_type("image/png");

    let dynamic_movie_template =
        ResourceTemplate::new("movies://catalog/{genre}/{movie_id}", "Catalog Movie Record")
            .title("Dynamic Movie Inspector")
            .mime_type("application/json");

    let dynamic_watchlist_template = ResourceTemplate::new(
        "movies://users/{user_id}/watchlists/{list_id}",
        "User Watchlist",
    )
    .title("Private User Watchlist Inspector")
    .mime_type("application/json");

    let dynamic_history_template =
        ResourceTemplate::new("movies://users/{user_id}/history", "User Watch History")
            .title("Private User Rating History")
            .mime_type("application/json");

    router
        .register_resource_with_cache(
            catalog_guide_resource,
            genres_catalog_handler,
            Some(86_400_000),
            Some(CacheScope::Public),
        )
        .register_resource_with_cache(
            top250_resource,
            curated_top250_handler,
            Some(3_600_000),
            Some(CacheScope::Public),
        )
        .register_resource_with_cache(
            logo_resource,
            branding_logo_handler,
            Some(86_400_000),
            Some(CacheScope::Public),
        )
        .register_resource_template(dynamic_movie_template, dynamic_catalog_handler)
        .register_resource_template(dynamic_watchlist_template, dynamic_user_watchlist_handler)
        .register_resource_template(dynamic_history_template, dynamic_user_history_handler)
        .resources_list(dynamic_resources_list)
        .resources_list_cache(Some(60_000), Some(CacheScope::Private))
}

/// Handler for the static genres catalog resource (`movies://genres/catalog`).
pub async fn genres_catalog_handler(
    State(db): State<Arc<RwLock<MovieDb>>>,
    uri: String,
) -> Result<ReadResourceResult, String> {
    let guard = db.read().await;

    let mut genre_counts: HashMap<String, usize> = HashMap::new();
    for movie in guard.catalog.values() {
        for genre in &movie.genres {
            *genre_counts.entry(genre.clone()).or_insert(0) += 1;
        }
    }

    let mut sorted_genres: Vec<(String, usize)> = genre_counts.into_iter().collect();
    sorted_genres.sort_by(|a, b| b.1.cmp(&a.1));

    let mut text = String::from("# CineList Movie Catalog & Genres Guide\n\n");
    text.push_str(&format!(
        "**Total Curated Titles**: {}\n\n",
        guard.catalog.len()
    ));
    text.push_str("### Genres Overview\n\n| Genre | Title Count |\n| :--- | :--- |\n");
    for (genre, count) in sorted_genres {
        text.push_str(&format!("| {genre} | {count} |\n"));
    }

    Ok(ReadResourceResult::text(uri, text, Some("text/markdown")))
}

/// Handler for the curated top-rated movies resource (`movies://curated/top250`).
pub async fn curated_top250_handler(
    State(db): State<Arc<RwLock<MovieDb>>>,
    uri: String,
) -> Result<ReadResourceResult, String> {
    let guard = db.read().await;

    let mut movies: Vec<_> = guard.catalog.values().cloned().collect();
    movies.sort_by(|a, b| {
        b.rating
            .partial_cmp(&a.rating)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let json_text = serde_json::to_string_pretty(&movies).map_err(|e| e.to_string())?;
    Ok(ReadResourceResult::text(
        uri,
        json_text,
        Some("application/json"),
    ))
}

/// Handler for the branding logo image blob resource (`movies://branding/cinelist-logo.png`).
pub async fn branding_logo_handler(uri: String) -> Result<ReadResourceResult, String> {
    Ok(ReadResourceResult::blob(
        uri,
        LOGO_PNG_BASE64,
        Some("image/png"),
    ))
}

/// Dynamic handler matching `movies://catalog/{genre}/{movie_id}`.
pub async fn dynamic_catalog_handler(
    State(db): State<Arc<RwLock<MovieDb>>>,
    uri: String,
) -> Result<ReadResourceResult, String> {
    let guard = db.read().await;

    let stripped = uri
        .strip_prefix("movies://catalog/")
        .ok_or_else(|| format!("Invalid URI format: {uri}"))?;

    let parts: Vec<&str> = stripped.split('/').collect();
    let movie_id = parts.last().copied().unwrap_or(stripped);

    let movie = guard
        .catalog
        .get(movie_id)
        .or_else(|| guard.catalog.values().find(|m| m.id.eq_ignore_ascii_case(movie_id)))
        .ok_or_else(|| format!("Movie '{movie_id}' not found in catalog"))?;

    let json_text = serde_json::to_string_pretty(movie).map_err(|e| e.to_string())?;
    Ok(ReadResourceResult::text(
        uri,
        json_text,
        Some("application/json"),
    ))
}

/// Dynamic handler matching `movies://users/{user_id}/watchlists/{list_id}` with IDOR verification.
pub async fn dynamic_user_watchlist_handler(
    State(db): State<Arc<RwLock<MovieDb>>>,
    auth: Option<BearerAuth>,
    uri: String,
) -> Result<ReadResourceResult, String> {
    let guard = db.read().await;

    let stripped = uri
        .strip_prefix("movies://users/")
        .ok_or_else(|| format!("Invalid URI format: {uri}"))?;

    let parts: Vec<&str> = stripped.split('/').collect();
    if parts.len() < 3 || parts[1] != "watchlists" {
        return Err(format!(
            "Expected URI format 'movies://users/{{user_id}}/watchlists/{{list_id}}': {uri}"
        ));
    }

    let user_id = parts[0];
    let list_name = parts[2];

    verify_user_access(auth.as_ref(), user_id, &guard)?;

    let user = guard
        .users
        .get(user_id)
        .ok_or_else(|| format!("User '{user_id}' not found"))?;

    let watchlist = user
        .watchlists
        .get(list_name)
        .ok_or_else(|| format!("Watchlist '{list_name}' not found for user '{user_id}'"))?;

    let json_text = serde_json::to_string_pretty(watchlist).map_err(|e| e.to_string())?;
    Ok(ReadResourceResult::text(
        uri,
        json_text,
        Some("application/json"),
    ))
}

/// Dynamic handler matching `movies://users/{user_id}/history` with IDOR verification.
pub async fn dynamic_user_history_handler(
    State(db): State<Arc<RwLock<MovieDb>>>,
    auth: Option<BearerAuth>,
    uri: String,
) -> Result<ReadResourceResult, String> {
    let guard = db.read().await;

    let stripped = uri
        .strip_prefix("movies://users/")
        .ok_or_else(|| format!("Invalid URI format: {uri}"))?;

    let parts: Vec<&str> = stripped.split('/').collect();
    if parts.len() < 2 || parts[1] != "history" {
        return Err(format!(
            "Expected URI format 'movies://users/{{user_id}}/history': {uri}"
        ));
    }

    let user_id = parts[0];
    verify_user_access(auth.as_ref(), user_id, &guard)?;

    let user = guard
        .users
        .get(user_id)
        .ok_or_else(|| format!("User '{user_id}' not found"))?;

    let json_text = serde_json::to_string_pretty(&user.ratings).map_err(|e| e.to_string())?;
    Ok(ReadResourceResult::text(
        uri,
        json_text,
        Some("application/json"),
    ))
}
