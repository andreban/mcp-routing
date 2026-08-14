// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # MCP Server with HTTP Caching Example
//!
//! Demonstrates configuring HTTP `Cache-Control` (`public`/`private`, `max-age`)
//! and deterministic `ETag` headers for:
//! 1. Server discovery (`server/discover`)
//! 2. Tool catalog discovery (`tools/list`)
//! 3. Single tool executions (`tools/call`)

use std::error::Error;

use axum::Router;
use mcp_routing::{
    McpRouter,
    types::mcp::{
        CacheScope, Implementation,
        tools::{Tool, ToolAnnotations},
    },
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize)]
pub struct LookupTaxRateParams {
    pub state: String,
}

/// A read-only lookup tool suitable for response caching.
async fn lookup_tax_rate(params: LookupTaxRateParams) -> Result<String, String> {
    match params.state.to_uppercase().as_str() {
        "CA" => Ok("California tax rate: 7.25%".to_string()),
        "NY" => Ok("New York tax rate: 4.00%".to_string()),
        "TX" => Ok("Texas tax rate: 6.25%".to_string()),
        state => Ok(format!("{state} tax rate: 5.00% (default)")),
    }
}

#[derive(Serialize, Deserialize)]
pub struct PlaceOrderParams {
    pub item: String,
    pub quantity: u32,
}

/// A mutating or non-idempotent tool that should NOT be cached.
async fn place_order(params: PlaceOrderParams) -> Result<String, String> {
    Ok(format!(
        "Order placed for {}x {} (Order #12345)",
        params.quantity, params.item
    ))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let server_info = Implementation::new("caching-mcp-server", "1.0.0");

    let tax_tool = Tool {
        icons: Vec::new(),
        name: "lookup_tax_rate".to_string(),
        title: Some("Lookup State Tax Rate".to_string()),
        description: Some("Returns the standard sales tax rate for a US state".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "state": { "type": "string", "description": "Two-letter US state code" }
            },
            "required": ["state"]
        }),
        output_schema: None,
        annotations: Some(ToolAnnotations {
            title: Some("State Tax Rate".to_string()),
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(true),
            open_world_hint: Some(false),
        }),
        meta: None,
    };

    let order_tool = Tool {
        icons: Vec::new(),
        name: "place_order".to_string(),
        title: Some("Place Order".to_string()),
        description: Some("Places an order for an item".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "item": { "type": "string" },
                "quantity": { "type": "integer" }
            },
            "required": ["item", "quantity"]
        }),
        output_schema: None,
        annotations: Some(ToolAnnotations {
            title: Some("Order Placement".to_string()),
            read_only_hint: Some(false),
            destructive_hint: Some(true),
            idempotent_hint: Some(false),
            open_world_hint: Some(false),
        }),
        meta: None,
    };

    let mcp_router = McpRouter::new(server_info)
        .instructions("MCP Server demonstrating discovery and per-tool caching")
        // 1. Configure server/discover caching (1 hour, Public)
        // Output HTTP header: `Cache-Control: public, max-age=3600` + `ETag: "..."`
        .server_discover_cache(Some(3_600_000), Some(CacheScope::Public))
        // 2. Configure tools/list catalog caching (10 minutes, Public)
        // Output HTTP header: `Cache-Control: public, max-age=600` + `ETag: "..."`
        .tools_list_cache(Some(600_000), Some(CacheScope::Public))
        // 3. Register a single tool WITH its own cache directives (5 minutes, Public)
        // Output HTTP header on tools/call: `Cache-Control: public, max-age=300` + `ETag: "..."`
        .register_tool_with_cache(
            tax_tool,
            lookup_tax_rate,
            Some(300_000),
            Some(CacheScope::Public),
        )
        // 4. Register an uncached/mutating tool (no Cache-Control, ETag only)
        .register_tool(order_tool, place_order);

    let app = Router::new().nest_service("/mcp", mcp_router);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    println!("Caching MCP Server listening on http://127.0.0.1:3000/mcp");
    println!("  - server/discover -> cached for 1h (Cache-Control: public, max-age=3600)");
    println!("  - tools/list      -> cached for 10m (Cache-Control: public, max-age=600)");
    println!("  - lookup_tax_rate -> cached for 5m (Cache-Control: public, max-age=300)");
    println!("  - place_order     -> uncached");

    axum::serve(listener, app).await?;
    Ok(())
}
