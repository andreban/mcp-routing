use std::collections::HashMap;
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use axum::{
    Json,
    body::Body,
    http::{Request, Response, StatusCode},
    response::IntoResponse,
    routing::MethodRouter,
};
use serde::de::DeserializeOwned;
use tower::Service;

pub mod server;
pub mod tools;
pub mod types;

#[cfg(test)]
mod test;

use tools::{IntoToolHandler, ToolHandler};
use types::{
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

#[derive(Clone)]
pub struct McpRouter {
    server_info: Implementation,
    instructions: Option<String>,
    capabilities: ServerCapabilities,
    supported_versions: Vec<String>,
    tools: Vec<Tool>,
    tool_handlers: HashMap<String, Arc<dyn ToolHandler>>,
    tool_routes: HashMap<String, MethodRouter>,
    custom_handlers: HashMap<String, MethodRouter>,
}

impl McpRouter {
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

    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    pub fn capabilities(mut self, capabilities: ServerCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn supported_versions(mut self, supported_versions: Vec<String>) -> Self {
        self.supported_versions = supported_versions;
        self
    }

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

    pub fn register_tool_route(mut self, tool: impl Into<Tool>, method_router: MethodRouter) -> Self {
        let tool = tool.into();
        let name = tool.name.clone();
        self.tool_routes.insert(name, method_router);
        self.tools.push(tool);
        self
    }

    pub fn route(mut self, path: &str, method_router: MethodRouter) -> Self {
        let key = path.trim_matches('/').to_string();
        self.custom_handlers.insert(key, method_router);
        self
    }
}

async fn parse_json_rpc_request<P: DeserializeOwned>(
    req: Request<Body>,
) -> Result<JsonRpcRequest<P>, Response<Body>> {
    let body_bytes = match axum::body::to_bytes(req.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(err) => {
            tracing::error!(?err, "Failed to read request body");
            return Err(StatusCode::BAD_REQUEST.into_response());
        }
    };

    match serde_json::from_slice(&body_bytes) {
        Ok(request) => Ok(request),
        Err(err) => {
            tracing::error!(?err, "Failed to parse JSON-RPC request");
            Err(StatusCode::BAD_REQUEST.into_response())
        }
    }
}

fn extract_mcp_target(req: &Request<Body>) -> Option<(String, Option<String>)> {
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

impl Service<Request<Body>> for McpRouter {
    type Response = Response<Body>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let mut this = self.clone();

        Box::pin(async move {
            let Some((method, name)) = extract_mcp_target(&req) else {
                tracing::debug!("Invalid MCP request: missing Mcp-Method header or path");
                return Ok(StatusCode::BAD_REQUEST.into_response());
            };

            // Check custom handlers first (e.g. user overrides)
            if let Some(handler) = this.custom_handlers.get_mut(&method) {
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

                return Ok(Json(response).into_response());
            }

            // Built-in tools/list
            if method == "tools/list" {
                let request: ListToolsRequest = match parse_json_rpc_request(req).await {
                    Ok(r) => r,
                    Err(err_resp) => return Ok(err_resp),
                };

                let response = tools::list::handle_list_tools(request, this.tools);
                return Ok(Json(response).into_response());
            }

            // Tool execution: tools/call
            if method == "tools/call" {
                let Some(tool_name) = name else {
                    tracing::debug!("Missing tool name for tools/call");
                    return Ok(StatusCode::BAD_REQUEST.into_response());
                };

                // Check raw Axum MethodRouter tool handlers
                if let Some(handler) = this.tool_routes.get_mut(&tool_name) {
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
                    return Ok(Json(response).into_response());
                }

                tracing::debug!(tool_name, "Tool not found");
                return Ok(StatusCode::NOT_FOUND.into_response());
            }

            Ok(StatusCode::NOT_FOUND.into_response())
        })
    }
}