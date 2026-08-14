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
use crate::service::BoxCloneSyncService;
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

type BoxedService = BoxCloneSyncService<Request<ResponseBody>, Response<ResponseBody>, Infallible>;

/// A [Tower](tower)-native router for the Model Context Protocol (MCP).
///
/// `McpRouter` implements [`tower::Service`] for HTTP requests and handles routing for:
/// - Built-in `server/discover` discovery endpoint
/// - Built-in `tools/list` tool discovery endpoint
/// - `tools/call` tool execution endpoints (delegating to typed handlers or custom services)
/// - Custom user routes overriding default behaviors
#[derive(Clone)]
pub struct McpRouter {
    server_info: Implementation,
    instructions: Option<String>,
    capabilities: ServerCapabilities,
    supported_versions: Vec<String>,
    tools: Vec<Tool>,
    tool_handlers: HashMap<String, Arc<dyn ToolHandler>>,
    tool_routes: HashMap<String, BoxedService>,
    custom_handlers: HashMap<String, BoxedService>,
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
            tool_routes: HashMap::new(),
            custom_handlers: HashMap::new(),
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

    /// Registers a tool definition handled by a raw [`tower::Service`].
    pub fn register_tool_route<S, ResBody>(
        mut self,
        tool: impl Into<Tool>,
        service: S,
    ) -> Self
    where
        S: Service<Request<ResponseBody>, Response = Response<ResBody>, Error = Infallible>
            + Clone
            + Send
            + Sync
            + 'static,
        S::Future: Future<Output = Result<Response<ResBody>, Infallible>> + Send + 'static,
        ResBody: http_body::Body<Data = Bytes> + Send + 'static,
        ResBody::Error: Into<BoxError>,
    {
        let tool = tool.into();
        let name = tool.name.clone();
        let mapped = tower::ServiceBuilder::new()
            .map_response(|resp: Response<ResBody>| resp.map(ResponseBody::new))
            .service(service);
        self.tool_routes
            .insert(name, BoxCloneSyncService::new(mapped));
        self.tools.push(tool);
        self
    }

    /// Registers a custom [`tower::Service`] to handle a specific protocol method or path.
    ///
    /// This can be used to override built-in endpoints such as `server/discover` or `tools/list`.
    pub fn route<S, ResBody>(mut self, path: &str, service: S) -> Self
    where
        S: Service<Request<ResponseBody>, Response = Response<ResBody>, Error = Infallible>
            + Clone
            + Send
            + Sync
            + 'static,
        S::Future: Future<Output = Result<Response<ResBody>, Infallible>> + Send + 'static,
        ResBody: http_body::Body<Data = Bytes> + Send + 'static,
        ResBody::Error: Into<BoxError>,
    {
        let key = path.trim_matches('/').to_string();
        let mapped = tower::ServiceBuilder::new()
            .map_response(|resp: Response<ResBody>| resp.map(ResponseBody::new))
            .service(service);
        self.custom_handlers
            .insert(key, BoxCloneSyncService::new(mapped));
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
    let path = req.uri().path().trim_matches('/');

    let header_method = req.headers().get("Mcp-Method").and_then(|v| v.to_str().ok());
    let header_name = req.headers().get("Mcp-Name").and_then(|v| v.to_str().ok());

    if let Some(method) = header_method {
        let method = method.trim_matches('/').to_string();
        let name = header_name.map(|n| n.trim_matches('/').to_string());
        return Some((method, name));
    }

    if path.is_empty() {
        return None;
    }

    if let Some(tool_name) = path.strip_prefix("tools/call/") {
        return Some(("tools/call".to_string(), Some(tool_name.to_string())));
    }

    Some((path.to_string(), None))
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
        let mut this = self.clone();

        Box::pin(async move {
            let Some((method, name)) = extract_mcp_target(&req) else {
                tracing::debug!("Invalid MCP request: missing Mcp-Method header or path");
                return Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(ResponseBody::empty())
                    .unwrap());
            };

            // Check custom handlers first (e.g. user overrides)
            if let Some(handler) = this.custom_handlers.get_mut(&method) {
                let req = req.map(ResponseBody::new);
                return handler.call(req).await;
            }

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

                // Check raw Service tool handlers
                if let Some(handler) = this.tool_routes.get_mut(&tool_name) {
                    let req = req.map(ResponseBody::new);
                    return handler.call(req).await;
                }

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
