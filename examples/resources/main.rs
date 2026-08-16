// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # MCP Server with Resources Capability Example
//!
//! Demonstrates defining and registering direct resources and URI templates with:
//! 1. Direct resource catalog listing (`resources/list`) with rich metadata and caching.
//! 2. Direct resource reading (`resources/read`) for text and binary blob resources.
//! 3. Resource template discovery (`resources/templates/list`) for dynamic parameterized resources.
//! 4. Dynamic resource reading matching RFC 6570 URI templates.
//! 5. Per-resource caching directives (`Cache-Control` header propagation).
//! 6. Request extractors (`BearerAuth`, `RequestContext`) in resource handlers.

use std::error::Error;

use axum::Router;
use mcp_routing::{
    BearerAuth, McpRouter,
    types::mcp::{
        CacheScope, Implementation, Role,
        resources::{ReadResourceResult, Resource, ResourceAnnotations, ResourceTemplate},
    },
};

/// Handler for reading server README text resource.
async fn readme_resource_handler() -> &'static str {
    "# MCP Resources Demo Server\n\n\
    This server demonstrates the full Model Context Protocol (MCP) Resources capability:\n\
    - `file:///workspace/README.md`: Static markdown documentation\n\
    - `memo://system-status`: Dynamic server health status\n\
    - `file:///assets/logo.png`: Binary image resource (Base64)\n\
    - `file:///{+path}`: Dynamic parameterized filesystem reader\n\
    - `metrics://{service}/{window}`: Dynamic performance metrics template"
}

/// Handler for reading system status with request extractors.
async fn system_status_handler(
    auth: Option<BearerAuth>,
    uri: String,
) -> Result<ReadResourceResult, String> {
    let auth_str = if auth.is_some() {
        "authenticated"
    } else {
        "unauthenticated"
    };

    let status_json = serde_json::json!({
        "status": "healthy",
        "timestamp": "2026-08-15T12:00:00Z",
        "authStatus": auth_str,
        "resourceUri": uri,
        "uptimeSeconds": 86400,
        "activeConnections": 12
    });

    Ok(ReadResourceResult::text(
        uri,
        serde_json::to_string_pretty(&status_json).map_err(|e| e.to_string())?,
        Some("application/json"),
    ))
}

/// Handler for returning a binary logo image (Base64 encoded 1x1 transparent PNG).
async fn logo_blob_handler(uri: String) -> Result<ReadResourceResult, String> {
    // 1x1 transparent PNG encoded in Base64
    let png_base64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

    Ok(ReadResourceResult::blob(uri, png_base64, Some("image/png")))
}

/// Dynamic handler matching the `file:///{+path}` URI template.
async fn dynamic_file_handler(uri: String) -> Result<ReadResourceResult, String> {
    let path = uri.strip_prefix("file:///").unwrap_or(&uri).to_string();

    let content = match path.as_str() {
        "src/main.rs" => "// Entry point\nfn main() { println!(\"Hello MCP!\"); }".to_string(),
        "Cargo.toml" => "[package]\nname = \"mcp-resources-demo\"\nversion = \"0.1.0\"".to_string(),
        other => format!("// Dynamic content for: {other}\n// Last synced: 2026-08-15"),
    };

    Ok(ReadResourceResult::text(uri, content, Some("text/x-rust")))
}

/// Dynamic handler matching the `metrics://{service}/{window}` URI template.
async fn dynamic_metrics_handler(uri: String) -> Result<ReadResourceResult, String> {
    let metrics_json = serde_json::json!({
        "uri": uri,
        "requestsPerSecond": 450.5,
        "p99LatencyMs": 14.2,
        "errorRate": 0.001
    });

    Ok(ReadResourceResult::text(
        uri,
        serde_json::to_string_pretty(&metrics_json).map_err(|e| e.to_string())?,
        Some("application/json"),
    ))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let server_info = Implementation::new("resources-mcp-server", "1.0.0")
        .with_title("Resources Demo MCP Server")
        .with_description("Example server demonstrating MCP resources and URI templates");

    // 1. Define direct resources
    let readme_resource = Resource::new("file:///workspace/README.md", "Project README")
        .title("Project Documentation")
        .description("Overview and usage guide for the MCP resources demo")
        .mime_type("text/markdown")
        .size(1024)
        .annotations(
            ResourceAnnotations::new()
                .audience(vec![Role::User, Role::Assistant])
                .priority(0.9)
                .last_modified("2026-08-15T09:00:00Z"),
        );

    let status_resource = Resource::new("memo://system-status", "System Status")
        .title("Server Health & Diagnostics")
        .description("Real-time operational status and metrics")
        .mime_type("application/json")
        .annotations(
            ResourceAnnotations::new()
                .audience(vec![Role::Assistant])
                .priority(0.7),
        );

    let logo_resource = Resource::new("file:///assets/logo.png", "Server Logo")
        .title("Branding Logo")
        .description("Server logo in PNG format")
        .mime_type("image/png")
        .size(68);

    // 2. Define resource templates (RFC 6570)
    let files_template = ResourceTemplate::new("file:///{+path}", "Workspace Files")
        .title("Dynamic File Explorer")
        .description("Access project source code and configuration files dynamically")
        .mime_type("text/plain")
        .annotations(
            ResourceAnnotations::new()
                .audience(vec![Role::User, Role::Assistant])
                .priority(0.8),
        );

    let metrics_template = ResourceTemplate::new("metrics://{service}/{window}", "Service Metrics")
        .title("Service Telemetry")
        .description("Access performance and health telemetry for named microservices")
        .mime_type("application/json");

    // 3. Build MCP Router with resources and templates
    let mcp_router = McpRouter::new(server_info)
        .instructions("MCP Server demonstrating Resources capability (resources/list, read, and templates/list)")
        // Configure catalog caching (5 minutes, Public)
        .resources_list_cache(Some(300_000), Some(CacheScope::Public))
        .resource_templates_list_cache(Some(600_000), Some(CacheScope::Public))
        // Register direct resources
        .register_resource_with_cache(
            readme_resource,
            readme_resource_handler,
            Some(3_600_000), // 1 hour TTL
            Some(CacheScope::Public),
        )
        .register_resource(status_resource, system_status_handler)
        .register_resource_with_cache(
            logo_resource,
            logo_blob_handler,
            Some(86_400_000), // 24 hour TTL
            Some(CacheScope::Public),
        )
        // Register dynamic resource templates
        .register_resource_template(files_template, dynamic_file_handler)
        .register_resource_template(metrics_template, dynamic_metrics_handler);

    // 4. Nest router in Axum web service
    let app = Router::new().nest_service("/mcp", mcp_router);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    println!("🚀 Resources MCP Server listening on http://127.0.0.1:3000/mcp");
    println!("Available endpoints and resources:");
    println!("  📋 resources/list              -> list registered direct resources (cached 5m)");
    println!(
        "  📋 resources/templates/list    -> list registered RFC 6570 URI templates (cached 10m)"
    );
    println!("  📖 resources/read              -> read resource contents:");
    println!("       - file:///workspace/README.md   (Markdown text, cached 1h)");
    println!("       - memo://system-status          (JSON health metrics with extractor context)");
    println!("       - file:///assets/logo.png       (Base64 PNG blob, cached 24h)");
    println!("       - file:///src/main.rs           (Matched dynamically via file:///{{+path}})");
    println!(
        "       - metrics://billing/1h          (Matched dynamically via metrics:///{{service}}/{{window}})"
    );

    axum::serve(listener, app).await?;
    Ok(())
}
