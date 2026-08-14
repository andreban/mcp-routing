use std::collections::HashMap;
use std::error::Error;

use axum::{Json, Router};
use mcp_routing::{
    McpRouter,
    types::mcp::{
        ContentBlock, Implementation, TextContent,
        tools::{
            Tool,
            call::{CallToolRequest, CallToolResult, CallToolResultResponse},
        },
    },
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize)]
pub struct EchoParams {
    pub value: String,
}
async fn echo(Json(request): Json<CallToolRequest<EchoParams>>) -> Json<CallToolResultResponse> {
    let value = request
        .params
        .as_ref()
        .and_then(|p| p.arguments.as_ref())
        .map(|args| args.value.as_str());

    let (content, is_error) = match value {
        Some(text) => (
            vec![ContentBlock::Text(TextContent {
                text: text.to_string(),
                annotations: None,
                meta: None,
            })],
            false,
        ),
        None => (
            vec![ContentBlock::Text(TextContent {
                text: "Missing required string parameter: value".to_string(),
                annotations: None,
                meta: None,
            })],
            true,
        ),
    };

    Json(CallToolResultResponse::new(
        request.id,
        CallToolResult {
            meta: None,
            result_type: Some("complete".to_string()),
            content,
            is_error: Some(is_error),
            structured_content: None,
            extras: HashMap::new(),
        },
    ))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let server_info = Implementation::new("example-mcp-server", "0.1.0");

    let echo_tool = Tool {
        icons: Vec::new(),
        name: "echo".to_string(),
        title: Some("Echo Tool".to_string()),
        description: Some("Echoes the provided value back to the caller".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "value": {
                    "type": "string",
                    "description": "The value to be echoed",
                }
            },
            "required": ["value"],
        }),
        output_schema: None,
        annotations: None,
        meta: None,
    };

    let mcp_router = McpRouter::new(server_info)
        .instructions("Example MCP server providing an echo tool")
        .register_tool(echo_tool, echo);

    let app = Router::new().nest_service("/mcp", mcp_router);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
