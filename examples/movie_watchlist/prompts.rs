// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Multi-turn and structured prompt templates, handlers, and dynamic prompt discovery.

use std::sync::Arc;
use tokio::sync::RwLock;

use mcp_routing::{
    BearerAuth, McpRouter, State,
    extract::RegisteredPrompts,
    types::mcp::{
        CacheScope,
        prompts::{Prompt, PromptArgument, PromptMessage, get::GetPromptResult},
    },
};

use super::models::{MovieDb, PlannerParams, ReviewPromptParams};

/// Dynamic `prompts/list` provider demonstrating request-scoped prompt filtering.
///
/// Unauthenticated callers discover only general planning prompts, while authenticated
/// callers also discover personalized drafting and review workflow prompts.
pub async fn dynamic_prompts_list(
    auth: Option<BearerAuth>,
    RegisteredPrompts(all_prompts): RegisteredPrompts,
) -> Vec<Prompt> {
    let is_authenticated = auth.is_some();
    all_prompts
        .into_iter()
        .filter(|prompt| {
            if is_authenticated {
                true
            } else {
                prompt.name == "movie_night_planner"
            }
        })
        .collect()
}

/// Registers all prompt templates and dynamic prompt discovery onto the [`McpRouter`].
pub fn register(router: McpRouter) -> McpRouter {
    let planner_prompt_def = Prompt::new("movie_night_planner")
        .title("Movie Night Planner")
        .description("Interactive multi-turn prompt assisting users in curating themed movie nights")
        .argument(PromptArgument::new("group_size").description("Number of participants"))
        .argument(PromptArgument::new("mood").description("Desired theme or emotional tone"))
        .argument(PromptArgument::new("max_runtime_minutes").description("Runtime limit in minutes"))
        .argument(PromptArgument::new("disliked_genres").description("Genres to avoid"));

    let review_prompt_def = Prompt::new("draft_review")
        .title("Film Review Drafter")
        .description("Generates an articulate, spoiler-free film review based on user notes")
        .argument(PromptArgument::new("movie_id").description("Movie ID").required(true))
        .argument(PromptArgument::new("rating").description("Score (1-10)").required(true))
        .argument(PromptArgument::new("raw_thoughts").description("Raw notes and impressions"));

    router
        .register_prompt(planner_prompt_def, movie_night_planner_prompt)
        .register_prompt(review_prompt_def, draft_review_prompt)
        .prompts_list(dynamic_prompts_list)
        .prompts_list_cache(Some(60_000), Some(CacheScope::Private))
}

/// Multi-turn prompt handler generating a customized movie night planner session.
pub async fn movie_night_planner_prompt(params: PlannerParams) -> Result<GetPromptResult, String> {
    let group_size = params.group_size.unwrap_or(2);
    let mood = params.mood.unwrap_or_else(|| "exciting and thought-provoking".to_string());
    let runtime_limit = params
        .max_runtime_minutes
        .map(|m| format!("{m} minutes"))
        .unwrap_or_else(|| "no strict limit".to_string());
    let disliked = params.disliked_genres.unwrap_or_else(|| "none".to_string());

    let user_msg = format!(
        "We are organizing a movie night for {group_size} person(s).\n\
        - Desired Mood / Theme: {mood}\n\
        - Maximum Runtime per film: {runtime_limit}\n\
        - Disliked / Excluded Genres: {disliked}\n\n\
        Please act as an expert film curator and recommend an unforgettable double-feature pairing with \
        thematic contrast, along with discussion talking points and refreshment pairings!"
    );

    let assistant_msg = format!(
        "I'd love to curate a memorable double-feature for your group of {group_size}!\n\n\
        Let's pair a visually captivating feature film that sets the tone for '{mood}', followed by a \
        complementary companion piece with thematic depth.\n\n\
        Let me review the CineList catalog to select titles within your runtime limit and matching streaming availability..."
    );

    Ok(
        GetPromptResult::new(vec![
            PromptMessage::user_text(user_msg),
            PromptMessage::assistant_text(assistant_msg),
        ])
        .with_description("Multi-turn interactive movie night planning workflow"),
    )
}

/// Structured prompt handler generating a spoiler-free review template.
pub async fn draft_review_prompt(
    State(db): State<Arc<RwLock<MovieDb>>>,
    params: ReviewPromptParams,
) -> Result<GetPromptResult, String> {
    let guard = db.read().await;

    let movie_info = if let Some(movie) = guard.catalog.get(&params.movie_id) {
        format!(
            "'{}' ({}, Directed by {}, Genres: {})",
            movie.title,
            movie.year,
            movie.director,
            movie.genres.join(", ")
        )
    } else {
        format!("Movie ID: '{}'", params.movie_id)
    };

    let user_thoughts = params
        .raw_thoughts
        .unwrap_or_else(|| "No specific notes provided.".to_string());

    let prompt_text = format!(
        "You are an articulate film critic and cultural essayist.\n\
        Please craft a polished, engaging, and spoiler-free film review for {movie_info}.\n\n\
        - User Rating: {:.1} / 10.0\n\
        - Raw Notes / Impressions: {}\n\n\
        Structure the review with:\n\
        1. **Catchy Headline & Hook**\n\
        2. **Premise & Themes** (strictly spoiler-free)\n\
        3. **Direction, Performances & Technical Craft**\n\
        4. **Final Verdict & Recommendation**",
        params.rating, user_thoughts
    );

    Ok(
        GetPromptResult::new(vec![PromptMessage::user_text(prompt_text)])
            .with_description("Structured film review drafting assistant"),
    )
}
