// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Catalog inspection and search tool handlers and definitions.

use std::sync::Arc;
use tokio::sync::RwLock;

use mcp_routing::{
    BearerAuth, Json, McpRouter, State,
    types::mcp::{
        ContentBlock, ImageContent, TextContent,
        tools::{Tool, ToolAnnotations, call::CallToolResult},
    },
};
use serde_json::json;

use crate::auth::resolve_optional_user;
use crate::models::{
    GetMovieDetailsParams, MovieDb, MovieDetails, MovieSearchResults, MovieSummary, PosterParams,
    SearchParams,
};

/// 1x1 transparent PNG encoded in Base64 for multimodal responses.
const PLACEHOLDER_PNG_BASE64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

/// Registers all catalog-related tools onto the [`McpRouter`].
pub fn register(router: McpRouter) -> McpRouter {
    let search_tool = Tool::new("search_movies")
        .title("Search Movies")
        .description("Searches curated movie catalog with optional genre, year, and rating filters")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Search query across titles, cast, directors" },
                "genre": { "type": "string", "description": "Filter by specific genre (e.g. 'Sci-Fi')" },
                "min_year": { "type": "integer", "description": "Minimum release year" },
                "max_year": { "type": "integer", "description": "Maximum release year" },
                "min_rating": { "type": "number", "description": "Minimum average rating (0.0 to 10.0)" },
                "limit": { "type": "integer", "description": "Maximum number of results (default 10)" }
            }
        }))
        .annotations(
            ToolAnnotations::new()
                .title("Catalog Search")
                .read_only(true)
                .idempotent(true)
                .open_world(false),
        );

    let details_tool = Tool::new("get_movie_details")
        .title("Get Movie Details")
        .description("Retrieves full details for a movie and matches streaming availability with caller subscriptions")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "movie_id": { "type": "string", "description": "The unique movie ID (e.g. 'alien', 'interstellar')" }
            },
            "required": ["movie_id"]
        }))
        .annotations(
            ToolAnnotations::new()
                .title("Movie Details")
                .read_only(true)
                .idempotent(true),
        );

    let poster_tool = Tool::new("generate_movie_poster")
        .title("Generate Movie Poster Card")
        .description(
            "Generates a multimodal visual poster card with PNG badge and markdown summary",
        )
        .input_schema(json!({
            "type": "object",
            "properties": {
                "movie_id": { "type": "string", "description": "Movie ID to render" }
            },
            "required": ["movie_id"]
        }))
        .annotations(
            ToolAnnotations::new()
                .title("Movie Poster Card")
                .read_only(true)
                .idempotent(true),
        );

    router
        .register_tool(search_tool, search_movies)
        .register_tool(details_tool, get_movie_details)
        .register_tool(poster_tool, generate_movie_poster)
}

/// Searches the movie catalog with multi-field filtering.
pub async fn search_movies(
    State(db): State<Arc<RwLock<MovieDb>>>,
    params: SearchParams,
) -> Result<(Json<MovieSearchResults>, String), String> {
    let guard = db.read().await;
    let limit = params.limit.unwrap_or(10).clamp(1, 50);

    let query_lower = params.query.as_ref().map(|q| q.to_lowercase());
    let genre_lower = params.genre.as_ref().map(|g| g.to_lowercase());

    let mut matches: Vec<MovieSummary> = guard
        .catalog
        .values()
        .filter(|m| {
            if let Some(ref q) = query_lower {
                let in_title = m.title.to_lowercase().contains(q);
                let in_director = m.director.to_lowercase().contains(q);
                let in_cast = m.cast.iter().any(|c| c.to_lowercase().contains(q));
                let in_synopsis = m.synopsis.to_lowercase().contains(q);
                if !in_title && !in_director && !in_cast && !in_synopsis {
                    return false;
                }
            }
            if let Some(ref g) = genre_lower
                && !m.genres.iter().any(|genre| genre.to_lowercase() == *g)
            {
                return false;
            }
            if let Some(min_yr) = params.min_year
                && m.year < min_yr
            {
                return false;
            }
            if let Some(max_yr) = params.max_year
                && m.year > max_yr
            {
                return false;
            }
            if let Some(min_rat) = params.min_rating
                && m.rating < min_rat
            {
                return false;
            }
            true
        })
        .map(|m| MovieSummary {
            id: m.id.clone(),
            title: m.title.clone(),
            year: m.year,
            director: m.director.clone(),
            genres: m.genres.clone(),
            rating: m.rating,
            streaming_platforms: m.streaming_platforms.clone(),
        })
        .collect();

    matches.sort_by(|a, b| {
        b.rating
            .partial_cmp(&a.rating)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let total = matches.len();
    matches.truncate(limit);

    let summary_md = format!(
        "Found **{total}** movie(s) matching search criteria (showing top {}).",
        matches.len()
    );

    Ok((
        Json(MovieSearchResults {
            total_matches: total,
            movies: matches,
        }),
        summary_md,
    ))
}

/// Retrieves movie details and cross-references streaming availability with the user's subscriptions.
pub async fn get_movie_details(
    State(db): State<Arc<RwLock<MovieDb>>>,
    auth: Option<BearerAuth>,
    params: GetMovieDetailsParams,
) -> Result<Json<MovieDetails>, String> {
    let guard = db.read().await;
    let movie = guard
        .catalog
        .get(&params.movie_id)
        .or_else(|| {
            guard
                .catalog
                .values()
                .find(|m| m.id.eq_ignore_ascii_case(&params.movie_id))
        })
        .cloned()
        .ok_or_else(|| format!("Movie with ID '{}' not found in catalog", params.movie_id))?;

    let user_subs = auth
        .as_ref()
        .and_then(|a| resolve_optional_user(Some(a), &guard))
        .and_then(|uid| guard.users.get(&uid))
        .map(|u| u.streaming_subscriptions.clone())
        .unwrap_or_default();

    let user_available_on: Vec<String> = movie
        .streaming_platforms
        .iter()
        .filter(|platform| user_subs.contains(platform))
        .cloned()
        .collect();

    Ok(Json(MovieDetails {
        movie,
        user_available_on,
    }))
}

/// Generates a multimodal visual poster card for a movie.
pub async fn generate_movie_poster(
    State(db): State<Arc<RwLock<MovieDb>>>,
    params: PosterParams,
) -> Result<CallToolResult, String> {
    let guard = db.read().await;
    let movie = guard
        .catalog
        .get(&params.movie_id)
        .or_else(|| {
            guard
                .catalog
                .values()
                .find(|m| m.id.eq_ignore_ascii_case(&params.movie_id))
        })
        .ok_or_else(|| format!("Movie '{}' not found in catalog", params.movie_id))?;

    let card_markdown = format!(
        "### 🎬 {}\n\n\
        **Year**: {} | **Rating**: ★ {:.1}/10 | **Runtime**: {} min\n\
        **Director**: {}\n\
        **Cast**: {}\n\
        **Genres**: {}\n\n\
        _{}_\n\n\
        **Streaming Platforms**: {}",
        movie.title,
        movie.year,
        movie.rating,
        movie.runtime_minutes,
        movie.director,
        movie.cast.join(", "),
        movie.genres.join(", "),
        movie.synopsis,
        movie.streaming_platforms.join(", ")
    );

    let content = vec![
        ContentBlock::Image(ImageContent {
            data: PLACEHOLDER_PNG_BASE64.to_string(),
            mime_type: "image/png".to_string(),
            annotations: None,
            meta: None,
        }),
        ContentBlock::Text(TextContent {
            text: card_markdown,
            annotations: None,
            meta: None,
        }),
    ];

    Ok(CallToolResult::with_content(content))
}
