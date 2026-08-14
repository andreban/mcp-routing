use std::collections::HashMap;
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use axum::{
    body::Body,
    handler::Handler,
    http::{Request, Response, StatusCode},
    response::IntoResponse,
    routing::{MethodRouter, post},
};
use tower::Service;

pub mod server;
pub mod tools;
pub mod types;

#[cfg(test)]
mod test;

use types::mcp::{
    Implementation, ServerCapabilities, ToolsCapability,
    tools::Tool,
};

#[derive(Clone)]
pub struct McpRouter {
    server_info: Implementation,
    instructions: Option<String>,
    capabilities: ServerCapabilities,
    supported_versions: Vec<String>,
    tools: Vec<Tool>,
    tool_handlers: HashMap<String, MethodRouter>,
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
        H: Handler<T, ()>,
        T: 'static,
    {
        let tool = tool.into();
        let name = tool.name.clone();
        self.tool_handlers.insert(name, post(handler));
        self.tools.push(tool);
        self
    }

    pub fn route(mut self, path: &str, method_router: MethodRouter) -> Self {
        let key = path.trim_matches('/').to_string();
        self.custom_handlers.insert(key, method_router);
        self
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
                return Ok(server::discover::handle_server_discover(
                    req,
                    this.server_info,
                    this.instructions,
                    this.capabilities,
                    this.supported_versions,
                )
                .await);
            }

            // Built-in tools/list
            if method == "tools/list" {
                return Ok(tools::list::handle_list_tools(req, this.tools).await);
            }

            // Tool execution: tools/call
            if method == "tools/call" {
                let Some(tool_name) = name else {
                    tracing::debug!("Missing tool name for tools/call");
                    return Ok(StatusCode::BAD_REQUEST.into_response());
                };

                if let Some(handler) = this.tool_handlers.get_mut(&tool_name) {
                    return handler.call(req).await;
                }

                tracing::debug!(tool_name, "Tool not found");
                return Ok(StatusCode::NOT_FOUND.into_response());
            }

            Ok(StatusCode::NOT_FOUND.into_response())
        })
    }
}