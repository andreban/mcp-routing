// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Multi-tenant watchlist tool handlers and definitions.

use std::sync::Arc;
use tokio::sync::RwLock;

use mcp_routing::{
    BearerAuth, Json, McpRouter, State,
    types::mcp::tools::{Tool, ToolAnnotations},
};
use serde_json::json;

use crate::auth::resolve_user;
use crate::models::{
    AddWatchlistParams, CreateWatchlistParams, DeleteWatchlistParams, GetWatchlistParams,
    MovieDb, RemoveWatchlistParams, UserWatchlistsResult, Watchlist, WatchlistItem,
    WatchlistItemSummary, WatchlistSummary,
};

/// Registers all watchlist-related tools onto the [`McpRouter`].
pub fn register(router: McpRouter) -> McpRouter {
    let list_all_tool = Tool::new("list_watchlists")
        .title("List Watchlists")
        .description("Lists all watchlists and summaries for the authenticated user")
        .input_schema(json!({
            "type": "object",
            "properties": {}
        }))
        .annotations(
            ToolAnnotations::new()
                .title("List User Watchlists")
                .read_only(true)
                .idempotent(true),
        );

    let get_list_tool = Tool::new("get_watchlist")
        .title("Get Watchlist")
        .description("Retrieves full contents and movies for a specific watchlist of the authenticated user")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "list_name": { "type": "string", "description": "Name of the target watchlist" }
            },
            "required": ["list_name"]
        }))
        .annotations(
            ToolAnnotations::new()
                .title("Get User Watchlist")
                .read_only(true)
                .idempotent(true),
        );

    let create_list_tool = Tool::new("create_watchlist")
        .title("Create Watchlist")
        .description("Creates a new named watchlist for the authenticated user")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "list_name": { "type": "string", "description": "Unique name for the watchlist" },
                "description": { "type": "string", "description": "Optional description for the watchlist" }
            },
            "required": ["list_name"]
        }))
        .annotations(
            ToolAnnotations::new()
                .title("Create Watchlist")
                .read_only(false)
                .destructive(false),
        );

    let add_item_tool = Tool::new("add_to_watchlist")
        .title("Add to Watchlist")
        .description("Adds a movie to the caller's specified watchlist")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "list_name": { "type": "string", "description": "Name of the target watchlist" },
                "movie_id": { "type": "string", "description": "Movie ID to add" },
                "notes": { "type": "string", "description": "Optional personal notes" }
            },
            "required": ["list_name", "movie_id"]
        }));

    let remove_item_tool = Tool::new("remove_from_watchlist")
        .title("Remove from Watchlist")
        .description("Removes a movie from the caller's specified watchlist")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "list_name": { "type": "string", "description": "Name of the target watchlist" },
                "movie_id": { "type": "string", "description": "Movie ID to remove" }
            },
            "required": ["list_name", "movie_id"]
        }));

    let delete_list_tool = Tool::new("delete_watchlist")
        .title("Delete Watchlist")
        .description("Permanently deletes an existing watchlist for the authenticated user")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "list_name": { "type": "string", "description": "Name of the watchlist to delete" }
            },
            "required": ["list_name"]
        }))
        .annotations(
            ToolAnnotations::new()
                .title("Delete Watchlist")
                .destructive(true),
        );

    router
        .register_tool(list_all_tool, list_watchlists)
        .register_tool(get_list_tool, get_watchlist)
        .register_tool(create_list_tool, create_watchlist)
        .register_tool(add_item_tool, add_to_watchlist)
        .register_tool(remove_item_tool, remove_from_watchlist)
        .register_tool(delete_list_tool, delete_watchlist)
}

/// Lists all watchlists for the authenticated user.
pub async fn list_watchlists(
    State(db): State<Arc<RwLock<MovieDb>>>,
    auth: Option<BearerAuth>,
) -> Result<Json<UserWatchlistsResult>, String> {
    let guard = db.read().await;
    let user_id = resolve_user(auth.as_ref(), &guard)?;

    let user = guard
        .users
        .get(&user_id)
        .ok_or_else(|| format!("User '{user_id}' not found"))?;

    let watchlists = user
        .watchlists
        .values()
        .map(|list| {
            let items_summary = list
                .items
                .iter()
                .filter_map(|item| {
                    guard.catalog.get(&item.movie_id).map(|m| WatchlistItemSummary {
                        movie_id: m.id.clone(),
                        title: m.title.clone(),
                        year: m.year,
                        notes: item.notes.clone(),
                    })
                })
                .collect();

            WatchlistSummary {
                list_name: list.name.clone(),
                item_count: list.items.len(),
                items: items_summary,
            }
        })
        .collect();

    Ok(Json(UserWatchlistsResult {
        user_id,
        watchlists,
    }))
}

/// Retrieves a specific named watchlist for the authenticated user.
pub async fn get_watchlist(
    State(db): State<Arc<RwLock<MovieDb>>>,
    auth: Option<BearerAuth>,
    params: GetWatchlistParams,
) -> Result<Json<WatchlistSummary>, String> {
    let guard = db.read().await;
    let user_id = resolve_user(auth.as_ref(), &guard)?;

    let user = guard
        .users
        .get(&user_id)
        .ok_or_else(|| format!("User '{user_id}' not found"))?;

    let list = user
        .watchlists
        .get(&params.list_name)
        .ok_or_else(|| format!("Watchlist '{}' not found for user '{user_id}'", params.list_name))?;

    let items_summary = list
        .items
        .iter()
        .filter_map(|item| {
            guard.catalog.get(&item.movie_id).map(|m| WatchlistItemSummary {
                movie_id: m.id.clone(),
                title: m.title.clone(),
                year: m.year,
                notes: item.notes.clone(),
            })
        })
        .collect();

    Ok(Json(WatchlistSummary {
        list_name: list.name.clone(),
        item_count: list.items.len(),
        items: items_summary,
    }))
}

/// Creates a new named watchlist for the authenticated user.
pub async fn create_watchlist(
    State(db): State<Arc<RwLock<MovieDb>>>,
    auth: Option<BearerAuth>,
    params: CreateWatchlistParams,
) -> Result<Json<WatchlistSummary>, String> {
    let trimmed_name = params.list_name.trim();
    if trimmed_name.is_empty() {
        return Err("Parameter 'list_name' cannot be empty".to_string());
    }

    let mut guard = db.write().await;
    let user_id = resolve_user(auth.as_ref(), &guard)?;

    let user = guard
        .users
        .get_mut(&user_id)
        .ok_or_else(|| format!("User '{user_id}' not found"))?;

    let new_watchlist = Watchlist {
        name: trimmed_name.to_string(),
        description: params.description,
        created_at: "2026-08-16T12:00:00Z".to_string(),
        items: Vec::new(),
    };

    user.watchlists.insert(trimmed_name.to_string(), new_watchlist);

    Ok(Json(WatchlistSummary {
        list_name: trimmed_name.to_string(),
        item_count: 0,
        items: Vec::new(),
    }))
}

/// Adds a movie to the caller's specified watchlist.
pub async fn add_to_watchlist(
    State(db): State<Arc<RwLock<MovieDb>>>,
    auth: Option<BearerAuth>,
    params: AddWatchlistParams,
) -> Result<Json<WatchlistSummary>, String> {
    let mut guard = db.write().await;
    let user_id = resolve_user(auth.as_ref(), &guard)?;
    let MovieDb { catalog, users, .. } = &mut *guard;

    if !catalog.contains_key(&params.movie_id) {
        return Err(format!("Movie with ID '{}' not found in catalog", params.movie_id));
    }

    let user = users
        .get_mut(&user_id)
        .ok_or_else(|| format!("User '{user_id}' not found"))?;

    let list = user
        .watchlists
        .get_mut(&params.list_name)
        .ok_or_else(|| format!("Watchlist '{}' does not exist for user '{user_id}'", params.list_name))?;

    if !list.items.iter().any(|item| item.movie_id == params.movie_id) {
        list.items.push(WatchlistItem {
            movie_id: params.movie_id.clone(),
            added_at: "2026-08-16T12:05:00Z".to_string(),
            notes: params.notes,
        });
    }

    let items_summary = list
        .items
        .iter()
        .filter_map(|item| {
            catalog.get(&item.movie_id).map(|m| WatchlistItemSummary {
                movie_id: m.id.clone(),
                title: m.title.clone(),
                year: m.year,
                notes: item.notes.clone(),
            })
        })
        .collect();

    Ok(Json(WatchlistSummary {
        list_name: params.list_name,
        item_count: list.items.len(),
        items: items_summary,
    }))
}

/// Removes a movie from the caller's specified watchlist.
pub async fn remove_from_watchlist(
    State(db): State<Arc<RwLock<MovieDb>>>,
    auth: Option<BearerAuth>,
    params: RemoveWatchlistParams,
) -> Result<Json<WatchlistSummary>, String> {
    let mut guard = db.write().await;
    let user_id = resolve_user(auth.as_ref(), &guard)?;

    let MovieDb { catalog, users, .. } = &mut *guard;

    let user = users
        .get_mut(&user_id)
        .ok_or_else(|| format!("User '{user_id}' not found"))?;

    let list = user
        .watchlists
        .get_mut(&params.list_name)
        .ok_or_else(|| format!("Watchlist '{}' not found", params.list_name))?;

    list.items.retain(|item| item.movie_id != params.movie_id);

    let items_summary = list
        .items
        .iter()
        .filter_map(|item| {
            catalog.get(&item.movie_id).map(|m| WatchlistItemSummary {
                movie_id: m.id.clone(),
                title: m.title.clone(),
                year: m.year,
                notes: item.notes.clone(),
            })
        })
        .collect();

    Ok(Json(WatchlistSummary {
        list_name: params.list_name,
        item_count: list.items.len(),
        items: items_summary,
    }))
}

/// Deletes an existing watchlist for the authenticated user.
pub async fn delete_watchlist(
    State(db): State<Arc<RwLock<MovieDb>>>,
    auth: Option<BearerAuth>,
    params: DeleteWatchlistParams,
) -> Result<String, String> {
    let mut guard = db.write().await;
    let user_id = resolve_user(auth.as_ref(), &guard)?;

    let user = guard
        .users
        .get_mut(&user_id)
        .ok_or_else(|| format!("User '{user_id}' not found"))?;

    if user.watchlists.remove(&params.list_name).is_none() {
        return Err(format!("Watchlist '{}' does not exist", params.list_name));
    }

    Ok(format!("Watchlist '{}' was successfully deleted.", params.list_name))
}
