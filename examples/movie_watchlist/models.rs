// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Domain models and data structures for the CineList MCP server example.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Represents a movie entry in the catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Movie {
    pub id: String,
    pub title: String,
    pub year: u32,
    pub director: String,
    pub cast: Vec<String>,
    pub genres: Vec<String>,
    pub runtime_minutes: u32,
    pub rating: f32,
    pub synopsis: String,
    pub streaming_platforms: Vec<String>,
}

/// An item within a user's watchlist.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchlistItem {
    pub movie_id: String,
    pub added_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// A named watchlist belonging to a user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Watchlist {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub created_at: String,
    pub items: Vec<WatchlistItem>,
}

/// A user review and rating for a movie.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovieRating {
    pub movie_id: String,
    pub rating: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review: Option<String>,
    pub watched_at: String,
}

/// Multi-tenant user profile containing private watchlists and ratings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub user_id: String,
    pub display_name: String,
    pub watchlists: HashMap<String, Watchlist>,
    pub ratings: HashMap<String, MovieRating>,
    pub streaming_subscriptions: Vec<String>,
}

/// Shared in-memory database holding the movie catalog, user profiles, and auth tokens.
#[derive(Debug, Clone)]
pub struct MovieDb {
    pub catalog: HashMap<String, Movie>,
    pub users: HashMap<String, UserProfile>,
    pub auth_tokens: HashMap<String, String>,
}

/// Active streaming platform subscriptions injected via Tower middleware extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingSubscriptions(pub Vec<String>);

/// Parameters for `search_movies` tool.
#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub query: Option<String>,
    pub genre: Option<String>,
    pub min_year: Option<u32>,
    pub max_year: Option<u32>,
    pub min_rating: Option<f32>,
    pub limit: Option<usize>,
}

/// Compact movie summary for search results.
#[derive(Debug, Clone, Serialize)]
pub struct MovieSummary {
    pub id: String,
    pub title: String,
    pub year: u32,
    pub director: String,
    pub genres: Vec<String>,
    pub rating: f32,
    pub streaming_platforms: Vec<String>,
}

/// Structured response for `search_movies`.
#[derive(Debug, Serialize)]
pub struct MovieSearchResults {
    pub total_matches: usize,
    pub movies: Vec<MovieSummary>,
}

/// Parameters for `get_movie_details` tool.
#[derive(Debug, Deserialize)]
pub struct GetMovieDetailsParams {
    pub movie_id: String,
}

/// Structured response for `get_movie_details`.
#[derive(Debug, Serialize)]
pub struct MovieDetails {
    pub movie: Movie,
    pub user_available_on: Vec<String>,
}

/// Parameters for `create_watchlist` tool.
#[derive(Debug, Deserialize)]
pub struct CreateWatchlistParams {
    pub list_name: String,
    pub description: Option<String>,
}

/// Parameters for `add_to_watchlist` tool.
#[derive(Debug, Deserialize)]
pub struct AddWatchlistParams {
    pub list_name: String,
    pub movie_id: String,
    pub notes: Option<String>,
}

/// Parameters for `remove_from_watchlist` tool.
#[derive(Debug, Deserialize)]
pub struct RemoveWatchlistParams {
    pub list_name: String,
    pub movie_id: String,
}

/// Parameters for `delete_watchlist` tool.
#[derive(Debug, Deserialize)]
pub struct DeleteWatchlistParams {
    pub list_name: String,
}

/// Parameters for `get_watchlist` tool.
#[derive(Debug, Deserialize)]
pub struct GetWatchlistParams {
    pub list_name: String,
}

/// Item summary in a watchlist response.
#[derive(Debug, Clone, Serialize)]
pub struct WatchlistItemSummary {
    pub movie_id: String,
    pub title: String,
    pub year: u32,
    pub notes: Option<String>,
}

/// Structured response summarizing a watchlist.
#[derive(Debug, Clone, Serialize)]
pub struct WatchlistSummary {
    pub list_name: String,
    pub item_count: usize,
    pub items: Vec<WatchlistItemSummary>,
}

/// Structured response for `list_watchlists`.
#[derive(Debug, Serialize)]
pub struct UserWatchlistsResult {
    pub user_id: String,
    pub watchlists: Vec<WatchlistSummary>,
}

/// Parameters for `rate_movie` tool.
#[derive(Debug, Deserialize)]
pub struct RateMovieParams {
    pub movie_id: String,
    pub rating: f32,
    pub review: Option<String>,
}

/// Structured response for `rate_movie`.
#[derive(Debug, Serialize)]
pub struct RatingReceipt {
    pub movie_id: String,
    pub movie_title: String,
    pub rating: f32,
    pub review: Option<String>,
    pub recorded_at: String,
}

/// Parameters for `get_recommendations` tool.
#[derive(Debug, Deserialize)]
pub struct RecommendationParams {
    pub genre: Option<String>,
    pub mood: Option<String>,
    pub limit: Option<usize>,
}

/// Recommended movie item.
#[derive(Debug, Clone, Serialize)]
pub struct RecommendedMovie {
    pub movie_id: String,
    pub title: String,
    pub year: u32,
    pub genres: Vec<String>,
    pub score: f32,
    pub reason: String,
    pub available_on: Vec<String>,
}

/// Structured response for `get_recommendations`.
#[derive(Debug, Serialize)]
pub struct RecommendationResult {
    pub user_id: String,
    pub recommendations: Vec<RecommendedMovie>,
}

/// Parameters for `generate_movie_poster` tool.
#[derive(Debug, Deserialize)]
pub struct PosterParams {
    pub movie_id: String,
}

/// Parameters for `movie_night_planner` prompt.
#[derive(Debug, Deserialize)]
pub struct PlannerParams {
    pub group_size: Option<usize>,
    pub mood: Option<String>,
    pub max_runtime_minutes: Option<u32>,
    pub disliked_genres: Option<String>,
}

/// Parameters for `draft_review` prompt.
#[derive(Debug, Deserialize)]
pub struct ReviewPromptParams {
    pub movie_id: String,
    pub rating: f32,
    pub raw_thoughts: Option<String>,
}
