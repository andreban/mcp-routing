// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Movie ratings and personalized recommendation tool handlers and definitions.

use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

use mcp_routing::{
    BearerAuth, Json, McpRouter, State,
    types::mcp::tools::{Tool, ToolAnnotations},
};
use serde_json::json;

use crate::auth::resolve_user;
use crate::models::{
    MovieDb, MovieRating, RateMovieParams, RatingReceipt, RecommendationParams,
    RecommendationResult, RecommendedMovie,
};

/// Registers all rating and recommendation tools onto the [`McpRouter`].
pub fn register(router: McpRouter) -> McpRouter {
    let rate_tool = Tool::new("rate_movie")
        .title("Rate Movie")
        .description("Submits a 1.0–10.0 score and review note for a movie under caller's profile")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "movie_id": { "type": "string", "description": "Movie ID to rate" },
                "rating": { "type": "number", "minimum": 1.0, "maximum": 10.0, "description": "Rating score between 1.0 and 10.0" },
                "review": { "type": "string", "description": "Optional review notes" }
            },
            "required": ["movie_id", "rating"]
        }));

    let recs_tool = Tool::new("get_recommendations")
        .title("Get Movie Recommendations")
        .description("Calculates personalized recommendations based on caller's ratings and streaming subscriptions")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "genre": { "type": "string", "description": "Optional genre filter" },
                "mood": { "type": "string", "description": "Optional mood keyword" },
                "limit": { "type": "integer", "description": "Max recommendations to return" }
            }
        }))
        .annotations(
            ToolAnnotations::new()
                .title("Personalized Recommendations")
                .read_only(true),
        );

    router
        .register_tool(rate_tool, rate_movie)
        .register_tool(recs_tool, get_recommendations)
}

/// Rates and reviews a movie on a 1.0–10.0 scale for the authenticated user.
pub async fn rate_movie(
    State(db): State<Arc<RwLock<MovieDb>>>,
    auth: Option<BearerAuth>,
    params: RateMovieParams,
) -> Result<Json<RatingReceipt>, String> {
    if !(1.0..=10.0).contains(&params.rating) {
        return Err("Rating must be between 1.0 and 10.0".to_string());
    }

    let mut guard = db.write().await;
    let user_id = resolve_user(auth.as_ref(), &guard)?;

    let movie_title = guard
        .catalog
        .get(&params.movie_id)
        .map(|m| m.title.clone())
        .ok_or_else(|| format!("Movie '{}' not found in catalog", params.movie_id))?;

    let user = guard
        .users
        .get_mut(&user_id)
        .ok_or_else(|| format!("User '{user_id}' not found"))?;

    let recorded_at = "2026-08-16T12:10:00Z".to_string();
    user.ratings.insert(
        params.movie_id.clone(),
        MovieRating {
            movie_id: params.movie_id.clone(),
            rating: params.rating,
            review: params.review.clone(),
            watched_at: recorded_at.clone(),
        },
    );

    Ok(Json(RatingReceipt {
        movie_id: params.movie_id,
        movie_title,
        rating: params.rating,
        review: params.review,
        recorded_at,
    }))
}

/// Computes personalized movie recommendations based on user ratings and streaming subscriptions.
pub async fn get_recommendations(
    State(db): State<Arc<RwLock<MovieDb>>>,
    auth: Option<BearerAuth>,
    params: RecommendationParams,
) -> Result<Json<RecommendationResult>, String> {
    let guard = db.read().await;
    let user_id = resolve_user(auth.as_ref(), &guard)?;

    let user = guard
        .users
        .get(&user_id)
        .ok_or_else(|| format!("User '{user_id}' not found"))?;

    let mut liked_genres: HashSet<String> = HashSet::new();
    let mut liked_directors: HashSet<String> = HashSet::new();
    let watched_ids: HashSet<String> = user.ratings.keys().cloned().collect();

    for (movie_id, rating) in &user.ratings {
        if rating.rating >= 7.0 {
            if let Some(m) = guard.catalog.get(movie_id) {
                for g in &m.genres {
                    liked_genres.insert(g.clone());
                }
                liked_directors.insert(m.director.clone());
            }
        }
    }

    let mut candidates: Vec<RecommendedMovie> = guard
        .catalog
        .values()
        .filter(|m| !watched_ids.contains(&m.id))
        .filter(|m| {
            if let Some(ref g) = params.genre {
                m.genres.iter().any(|genre| genre.eq_ignore_ascii_case(g))
            } else {
                true
            }
        })
        .map(|m| {
            let mut score = m.rating;
            let mut reasons = Vec::new();

            let matched_genres: Vec<&String> = m.genres.iter().filter(|g| liked_genres.contains(*g)).collect();
            if !matched_genres.is_empty() {
                score += matched_genres.len() as f32 * 0.5;
                reasons.push(format!("Matches your affinity for {:?}", matched_genres));
            }

            if liked_directors.contains(&m.director) {
                score += 1.5;
                reasons.push(format!("Directed by {}", m.director));
            }

            if let Some(ref mood) = params.mood {
                let mood_lower = mood.to_lowercase();
                let matches_mood = match mood_lower.as_str() {
                    "mind-bending" => m.genres.iter().any(|g| g == "Sci-Fi" || g == "Thriller"),
                    "cozy" => m.genres.iter().any(|g| g == "Animation" || g == "Comedy"),
                    "edge-of-your-seat" => m.genres.iter().any(|g| g == "Action" || g == "Thriller" || g == "Crime"),
                    "tearjerker" => m.genres.iter().any(|g| g == "Drama"),
                    "atmospheric" => m.genres.iter().any(|g| g == "Horror" || g == "Sci-Fi"),
                    _ => false,
                };
                if matches_mood {
                    score += 1.0;
                    reasons.push(format!("Matches requested '{mood}' mood"));
                }
            }

            let available_on: Vec<String> = m
                .streaming_platforms
                .iter()
                .filter(|p| user.streaming_subscriptions.contains(p))
                .cloned()
                .collect();

            if !available_on.is_empty() {
                score += 0.5;
                reasons.push(format!("Streamable on {}", available_on.join(", ")));
            }

            let reason_str = if reasons.is_empty() {
                "Top-rated catalog selection".to_string()
            } else {
                reasons.join("; ")
            };

            RecommendedMovie {
                movie_id: m.id.clone(),
                title: m.title.clone(),
                year: m.year,
                genres: m.genres.clone(),
                score,
                reason: reason_str,
                available_on,
            }
        })
        .collect();

    candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    let limit = params.limit.unwrap_or(5).clamp(1, 20);
    candidates.truncate(limit);

    Ok(Json(RecommendationResult {
        user_id,
        recommendations: candidates,
    }))
}
