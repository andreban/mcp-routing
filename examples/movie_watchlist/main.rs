// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # CineList: Movie Watchlist & Recommendation MCP Server Example
//!
//! A comprehensive, production-grade Model Context Protocol (MCP) server example demonstrating:
//! 1. **Public Catalog Tools & Resources**: Search, details, curated top 250, and multimodal posters.
//! 2. **Multi-Tenant State & Security**: Token-based authentication ([`BearerAuth`]) and IDOR access control.
//! 3. **Dynamic Capability Discovery**: Request-scoped tools, prompts, resources, and discovery instructions.
//! 4. **Personalized Watchlists & Ratings**: Scoped to user profiles with 1.0–10.0 ratings and reviews.
//! 5. **Smart Recommendations**: Dynamic scoring based on user ratings ($\ge 7.0$) and streaming subscriptions.
//! 6. **Multi-Turn Prompts & Autocompletions**: Context-aware suggestions and structured workflows.

mod auth;
mod completions;
mod discovery;
mod models;
mod prompts;
mod resources;
mod seed;
mod tools;

use std::error::Error;
use std::sync::Arc;
use tokio::sync::RwLock;

use axum::Router;
use mcp_routing::{McpRouter, types::mcp::CacheScope};

use models::StreamingSubscriptions;
use seed::seed_database;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let db = Arc::new(RwLock::new(seed_database()));
    let global_subscriptions = StreamingSubscriptions(vec![
        "Netflix".to_string(),
        "Criterion Channel".to_string(),
        "Max".to_string(),
        "Prime Video".to_string(),
    ]);

    // Initialize McpRouter with state and public resource template caching
    let mut mcp_router = McpRouter::new(discovery::server_info())
        .with_state(db.clone())
        .resource_templates_list_cache(Some(600_000), Some(CacheScope::Public));

    // Register modular capabilities from submodules
    mcp_router = discovery::register(mcp_router);
    mcp_router = tools::register(mcp_router);
    mcp_router = resources::register(mcp_router);
    mcp_router = prompts::register(mcp_router);
    mcp_router = completions::register(mcp_router);

    // Mount into Axum with middleware extensions
    let app = Router::new()
        .nest_service("/mcp", mcp_router)
        .layer(axum::Extension(global_subscriptions));

    let addr = "127.0.0.1:3000";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("🍿 CineList MCP Server running at http://{addr}/mcp");
    println!("Pre-seeded accounts:");
    println!("  - Alice (Token: 'token_alice_secret') -> Sci-Fi focus, Netflix/Criterion");
    println!("  - Bob   (Token: 'token_bob_secret')   -> Thriller/Noir focus, Max/Prime");
    println!("\nTry running:");
    println!("  1. Dynamic Server Discovery (Unauthenticated vs Alice):");
    println!(r#"     curl -X POST http://127.0.0.1:3000/mcp -H 'Content-Type: application/json' -d '{{"jsonrpc":"2.0","id":1,"method":"server/discover"}}'"#);
    println!(r#"     curl -X POST http://127.0.0.1:3000/mcp -H 'Content-Type: application/json' -H 'Authorization: Bearer token_alice_secret' -d '{{"jsonrpc":"2.0","id":2,"method":"server/discover"}}'"#);
    println!("  2. Dynamic Tool Discovery (Unauthenticated vs Alice):");
    println!(r#"     curl -X POST http://127.0.0.1:3000/mcp -H 'Content-Type: application/json' -d '{{"jsonrpc":"2.0","id":3,"method":"tools/list"}}'"#);
    println!(r#"     curl -X POST http://127.0.0.1:3000/mcp -H 'Content-Type: application/json' -H 'Authorization: Bearer token_alice_secret' -d '{{"jsonrpc":"2.0","id":4,"method":"tools/list"}}'"#);
    println!("  3. Dynamic Resource Discovery (Alice sees her private watchlists):");
    println!(r#"     curl -X POST http://127.0.0.1:3000/mcp -H 'Content-Type: application/json' -H 'Authorization: Bearer token_alice_secret' -d '{{"jsonrpc":"2.0","id":5,"method":"resources/list"}}'"#);

    axum::serve(listener, app).await?;
    Ok(())
}
