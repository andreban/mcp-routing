// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use bytes::Bytes;
use http::{Request, Response, StatusCode};
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use tower::Service;

use crate::body::{BoxError, ResponseBody, json_response};
use crate::server;
use crate::tools::{self, IntoToolHandler, ToolHandler};
use crate::types::{
    jsonrpc::JsonRpcRequest,
    mcp::{
        Implementation, ServerCapabilities, ToolsCapability,
        server::discover::ServerDiscoverRequest,
        tools::{
            Tool,
            call::{CallToolRequest, CallToolResultResponse},
            list::ListToolsRequest,
        },
    },
};

/// A [Tower](tower)-native router for the Model Context Protocol (MCP).
///
/// `McpRouter` implements [`tower::Service`] for HTTP requests and handles routing for:
/// - Built-in `server/discover` discovery endpoint
/// - Built-in `tools/list` tool discovery endpoint
/// - `tools/call` tool execution endpoints (delegating to typed handlers)
#[derive(Clone)]
pub struct McpRouter {
    server_info: Implementation,
    instructions: Option<String>,
    capabilities: ServerCapabilities,
    supported_versions: Vec<String>,
    tools: Vec<Tool>,
    tool_handlers: HashMap<String, Arc<dyn ToolHandler>>,
}

impl McpRouter {
    /// Creates a new [`McpRouter`] initialized with the given server [`Implementation`] metadata.
    pub fn new(server_info: Implementation) -> Self {
        Self {
            server_info,
            instructions: None,
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability { list_changed: None }),
                resources: None,
                prompts: None,
                completions: None,
                experimental: None,
            },
            supported_versions: vec!["2026-07-28".to_string()],
            tools: Vec::new(),
            tool_handlers: HashMap::new(),
        }
    }

    /// Sets human-readable instructions describing how to use this MCP server.
    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    /// Sets the server capabilities advertised in `server/discover`.
    pub fn capabilities(mut self, capabilities: ServerCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Sets the MCP protocol versions supported by this server.
    pub fn supported_versions(mut self, supported_versions: Vec<String>) -> Self {
        self.supported_versions = supported_versions;
        self
    }

    /// Registers a tool definition alongside a typed asynchronous handler function.
    ///
    /// The handler function can take typed deserializable arguments (or no arguments)
    /// and return any type implementing [`IntoToolResult`](crate::tools::IntoToolResult).
    pub fn register_tool<TTool, H, T>(mut self, tool: TTool, handler: H) -> Self
    where
        TTool: Into<Tool>,
        H: IntoToolHandler<T>,
        T: 'static,
    {
        let tool = tool.into();
        let name = tool.name.clone();
        self.tool_handlers.insert(name, handler.into_tool_handler());
        self.tools.push(tool);
        self
    }
}

async fn parse_json_rpc_request<P: DeserializeOwned, B>(
    req: Request<B>,
) -> Result<JsonRpcRequest<P>, Response<ResponseBody>>
where
    B: http_body::Body<Data = Bytes> + Send + 'static,
    B::Error: Into<BoxError>,
{
    let body_bytes = match req.into_body().collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(err) => {
            let err = err.into();
            tracing::error!(?err, "Failed to read request body");
            return Err(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(ResponseBody::empty())
                .unwrap());
        }
    };

    match serde_json::from_slice(&body_bytes) {
        Ok(request) => Ok(request),
        Err(err) => {
            tracing::error!(?err, "Failed to parse JSON-RPC request");
            Err(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(ResponseBody::empty())
                .unwrap())
        }
    }
}

fn extract_mcp_target<B>(req: &Request<B>) -> Option<(String, Option<String>)> {
    let method = req.headers().get("Mcp-Method")?.to_str().ok()?;
    let name = req.headers().get("Mcp-Name").and_then(|v| v.to_str().ok());
    Some((
        method.trim_matches('/').to_string(),
        name.map(|n| n.trim_matches('/').to_string()),
    ))
}

impl<B> Service<Request<B>> for McpRouter
where
    B: http_body::Body<Data = Bytes> + Send + 'static,
    B::Error: Into<BoxError>,
{
    type Response = Response<ResponseBody>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let this = self.clone();

        Box::pin(async move {
            let Some((method, name)) = extract_mcp_target(&req) else {
                tracing::debug!("Invalid MCP request: missing Mcp-Method header or path");
                return Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(ResponseBody::empty())
                    .unwrap());
            };

            // Built-in server/discover
            if method == "server/discover" {
                let request: ServerDiscoverRequest = match parse_json_rpc_request(req).await {
                    Ok(r) => r,
                    Err(err_resp) => return Ok(err_resp),
                };

                let response = server::discover::handle_server_discover(
                    request,
                    this.server_info,
                    this.instructions,
                    this.capabilities,
                    this.supported_versions,
                );

                return Ok(json_response(&response));
            }

            // Built-in tools/list
            if method == "tools/list" {
                let request: ListToolsRequest = match parse_json_rpc_request(req).await {
                    Ok(r) => r,
                    Err(err_resp) => return Ok(err_resp),
                };

                let response = tools::list::handle_list_tools(request, this.tools);
                return Ok(json_response(&response));
            }

            // Tool execution: tools/call
            if method == "tools/call" {
                let Some(tool_name) = name else {
                    tracing::debug!("Missing tool name for tools/call");
                    return Ok(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(ResponseBody::empty())
                        .unwrap());
                };

                // Check typed ToolHandler
                if let Some(handler) = this.tool_handlers.get(&tool_name) {
                    let request: CallToolRequest<serde_json::Value> =
                        match parse_json_rpc_request(req).await {
                            Ok(r) => r,
                            Err(err_resp) => return Ok(err_resp),
                        };

                    let req_id = request.id.clone();
                    let result = handler.call(request).await;
                    let response = CallToolResultResponse::new(req_id, result);
                    return Ok(json_response(&response));
                }

                tracing::debug!(tool_name, "Tool not found");
                return Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(ResponseBody::empty())
                    .unwrap());
            }

            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(ResponseBody::empty())
                .unwrap())
        })
    }
}
