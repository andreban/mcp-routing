use std::error::Error;

use axum::Router;
use mcp_routing::{
    McpRouter,
    types::mcp::{Implementation, tools::Tool},
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize)]
pub struct EchoParams {
    pub value: String,
}

async fn echo(params: EchoParams) -> Result<String, String> {
    if params.value.is_empty() {
        return Err("Missing required string parameter: value".to_string());
    }
    Ok(params.value)
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
