// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Structured Output MCP Server Example
//!
//! Demonstrates how to build MCP tools that produce structured JSON content (`structured_content`)
//! alongside multi-modal content blocks, configure `output_schema` and behavioral annotations,
//! and use ergonomic return types such as [`Json<T>`](mcp_routing::extract::Json), [`CallToolResult<T>`](mcp_routing::types::mcp::tools::call::CallToolResult),
//! and tuple conversions like `(Json<T>, &str)`.

use std::error::Error;

use axum::Router;
use mcp_routing::{
    Json, McpRouter,
    types::mcp::{
        Implementation,
        tools::{Tool, ToolAnnotations, call::CallToolResult},
    },
};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Parameters for querying weather information.
#[derive(Deserialize)]
struct WeatherParams {
    city: String,
}

/// Structured weather report payload.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WeatherReport {
    city: String,
    temperature_celsius: f64,
    condition: String,
    humidity_percent: u32,
    wind_speed_kmh: f64,
}

/// Handler 1: Returning `Json<WeatherReport>` directly.
///
/// The returned [`Json`] wrapper is automatically serialized into
/// `CallToolResult.structured_content` in the JSON-RPC response.
async fn get_weather(params: WeatherParams) -> Result<Json<WeatherReport>, String> {
    if params.city.trim().is_empty() {
        return Err("City name cannot be empty".to_string());
    }

    Ok(Json(WeatherReport {
        city: params.city,
        temperature_celsius: 21.5,
        condition: "Partly Cloudy".to_string(),
        humidity_percent: 65,
        wind_speed_kmh: 12.0,
    }))
}

/// Parameters for database user lookup.
#[derive(Deserialize)]
struct UserLookupParams {
    user_id: u64,
}

/// Structured user profile data.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UserProfile {
    id: u64,
    username: String,
    role: String,
    email: String,
}

/// Handler 2: Returning `(Json<UserProfile>, &'static str)`.
///
/// Returns both structured data and a human-readable text message summary.
async fn get_user_profile(
    params: UserLookupParams,
) -> Result<(Json<UserProfile>, &'static str), String> {
    if params.user_id == 0 {
        return Err("Invalid user_id: ID must be greater than zero".to_string());
    }

    let profile = UserProfile {
        id: params.user_id,
        username: "alex_dev".to_string(),
        role: "Engineer".to_string(),
        email: "alex@example.com".to_string(),
    };

    Ok((Json(profile), "User profile retrieved successfully"))
}

/// Handler 3: Returning a typed [`CallToolResult<T>`] with fluent builders.
///
/// Demonstrates attaching structured data alongside text and multi-modal blocks.
async fn get_system_metrics() -> CallToolResult<serde_json::Value> {
    let metrics = json!({
        "cpuUsagePercent": 14.2,
        "memoryUsedMb": 2048,
        "totalMemoryMb": 8192,
        "diskAvailableGb": 120.5,
        "activeConnections": 42
    });

    CallToolResult::structured(metrics)
        .with_text("System metrics collected from all cluster nodes.")
        .with_extra("samplingIntervalSec", json!(10))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let server_info = Implementation::new("structured-output-mcp-server", "1.0.0")
        .with_title("Structured Output MCP Server")
        .with_description("Demonstrates structured tool output with JSON schemas and annotations");

    // Tool 1: Weather report with input and output schemas + read-only annotations
    let weather_tool = Tool::new("get_weather")
        .title("Current Weather")
        .description("Fetches current weather conditions for a specified city")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "city": { "type": "string", "description": "City name" }
            },
            "required": ["city"]
        }))
        .output_schema(json!({
            "type": "object",
            "properties": {
                "city": { "type": "string" },
                "temperatureCelsius": { "type": "number" },
                "condition": { "type": "string" },
                "humidityPercent": { "type": "integer" },
                "windSpeedKmh": { "type": "number" }
            },
            "required": ["city", "temperatureCelsius", "condition", "humidityPercent", "windSpeedKmh"]
        }))
        .annotations(
            ToolAnnotations::new()
                .title("Weather Query")
                .read_only(true)
                .idempotent(true)
                .open_world(true),
        );

    // Tool 2: User profile lookup returning (Json<T>, text) tuple
    let user_tool = Tool::new("get_user_profile")
        .title("User Profile")
        .description("Retrieves a user profile by user ID")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "user_id": { "type": "integer", "description": "Unique user ID" }
            },
            "required": ["user_id"]
        }))
        .output_schema(json!({
            "type": "object",
            "properties": {
                "id": { "type": "integer" },
                "username": { "type": "string" },
                "role": { "type": "string" },
                "email": { "type": "string" }
            },
            "required": ["id", "username", "role", "email"]
        }))
        .annotations(
            ToolAnnotations::new()
                .title("User Lookup")
                .read_only(true)
                .idempotent(true),
        );

    // Tool 3: System metrics returning CallToolResult with fluent builders
    let metrics_tool = Tool::new("get_system_metrics")
        .title("System Metrics")
        .description("Returns real-time cluster telemetry and system metrics")
        .annotations(
            ToolAnnotations::new()
                .title("Telemetry")
                .read_only(true)
                .idempotent(false),
        );

    let mcp_router = McpRouter::new(server_info)
        .instructions("Provides tools returning structured JSON content alongside text summaries")
        .register_tool(weather_tool, get_weather)
        .register_tool(user_tool, get_user_profile)
        .register_tool(metrics_tool, get_system_metrics);

    let app = Router::new().nest_service("/mcp", mcp_router);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    println!("Structured Output MCP server listening on http://127.0.0.1:3000/mcp");
    println!("Example requests:");
    println!(
        r#"  curl -X POST http://127.0.0.1:3000/mcp -H 'Content-Type: application/json' -H 'Mcp-Method: tools/call' -H 'Mcp-Name: get_weather' -d '{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"get_weather","arguments":{{"city":"London"}}}}}}'"#
    );
    println!(
        r#"  curl -X POST http://127.0.0.1:3000/mcp -H 'Content-Type: application/json' -H 'Mcp-Method: tools/call' -H 'Mcp-Name: get_user_profile' -d '{{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{{"name":"get_user_profile","arguments":{{"user_id":42}}}}}}'"#
    );
    println!(
        r#"  curl -X POST http://127.0.0.1:3000/mcp -H 'Content-Type: application/json' -H 'Mcp-Method: tools/call' -H 'Mcp-Name: get_system_metrics' -d '{{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{{"name":"get_system_metrics"}}}}'"#
    );

    axum::serve(listener, app).await?;
    Ok(())
}
