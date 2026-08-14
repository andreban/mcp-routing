// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use bytes::Bytes;
use http::{Request, Response};
use http_body_util::BodyExt;
use serde::Deserialize;
use tower::Service;

use crate::body::{
    BoxError, ResponseBody, bad_request, json_response, method_not_allowed, unsupported_media_type,
};
use crate::server;
use crate::tools::{self, IntoToolHandler, ToolHandler};
use crate::types::jsonrpc::{JsonRpcErrorResponse, JsonRpcRequestId};
use crate::types::mcp::{
    Implementation, ServerCapabilities, ToolsCapability,
    server::discover::ServerDiscoverRequest,
    tools::{
        Tool,
        call::{CallToolRequest, CallToolResultResponse},
        list::ListToolsRequest,
    },
};
use crate::utils::{extract_header_name, extract_method, is_json_content_type, resolve_tool_name};

/// A [Tower](tower)-native router for the Model Context Protocol (MCP).
///
/// `McpRouter` implements [`tower::Service`] for HTTP requests and handles routing for:
/// - Built-in `server/discover` discovery endpoint
/// - Built-in `tools/list` tool discovery endpoint
/// - `tools/call` tool execution endpoints (delegating to typed handlers)
#[derive(Clone)]
pub struct McpRouter {
    inner: Arc<McpRouterInner>,
}

#[derive(Clone)]
struct McpRouterInner {
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
            inner: Arc::new(McpRouterInner {
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
            }),
        }
    }

    /// Sets human-readable instructions describing how to use this MCP server.
    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        Arc::make_mut(&mut self.inner).instructions = Some(instructions.into());
        self
    }

    /// Sets the server capabilities advertised in `server/discover`.
    pub fn capabilities(mut self, capabilities: ServerCapabilities) -> Self {
        Arc::make_mut(&mut self.inner).capabilities = capabilities;
        self
    }

    /// Sets the MCP protocol versions supported by this server.
    pub fn supported_versions(mut self, supported_versions: Vec<String>) -> Self {
        Arc::make_mut(&mut self.inner).supported_versions = supported_versions;
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
        let inner = Arc::make_mut(&mut self.inner);
        inner.tool_handlers.insert(name, handler.into_tool_handler());
        inner.tools.push(tool);
        self
    }
}

impl McpRouterInner {
    async fn dispatch<B>(&self, req: Request<B>) -> Response<ResponseBody>
    where
        B: http_body::Body<Data = Bytes> + Send + 'static,
        B::Error: Into<BoxError>,
    {
        if req.method() != http::Method::POST {
            tracing::debug!(method = %req.method(), "HTTP method not allowed, only POST is supported");
            return method_not_allowed();
        }

        if !is_json_content_type(req.headers()) {
            tracing::debug!("Missing or unsupported Content-Type header");
            return unsupported_media_type();
        }

        let (parts, body) = req.into_parts();

        let body_bytes = match body.collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(err) => {
                let err = err.into();
                tracing::error!(?err, "Failed to read request body");
                return bad_request();
            }
        };

        #[derive(Deserialize)]
        struct RequestPeek {
            id: Option<JsonRpcRequestId>,
            method: Option<String>,
        }

        let peek: RequestPeek = match serde_json::from_slice(&body_bytes) {
            Ok(p) => p,
            Err(err) => {
                tracing::debug!(?err, "Failed to parse JSON body");
                let error_response =
                    JsonRpcErrorResponse::parse_error(format!("Parse error: {err}"));
                return json_response(&error_response);
            }
        };

        let method = extract_method(&parts.headers, peek.method.as_deref());
        let Some(method) = method else {
            tracing::debug!("Missing method in both Mcp-Method header and JSON-RPC body");
            let error_response = JsonRpcErrorResponse::invalid_request(
                peek.id,
                "Invalid Request: missing method",
            );
            return json_response(&error_response);
        };

        if method.is_empty() {
            tracing::debug!("Empty method provided");
            let error_response = JsonRpcErrorResponse::invalid_request(
                peek.id,
                "Invalid Request: empty method",
            );
            return json_response(&error_response);
        }

        let header_name = extract_header_name(&parts.headers);

        match method.as_str() {
            "server/discover" => self.handle_server_discover(peek.id, &body_bytes),
            "tools/list" => self.handle_tools_list(peek.id, &body_bytes),
            "tools/call" => {
                self.handle_tools_call(peek.id, header_name.as_deref(), &body_bytes)
                    .await
            }
            unknown_method => {
                tracing::debug!(%unknown_method, "Method not found");
                let error_response = JsonRpcErrorResponse::method_not_found(
                    peek.id,
                    format!("Method not found: {unknown_method}"),
                );
                json_response(&error_response)
            }
        }
    }

    fn handle_server_discover(
        &self,
        req_id: Option<JsonRpcRequestId>,
        body: &[u8],
    ) -> Response<ResponseBody> {
        let request: ServerDiscoverRequest = match serde_json::from_slice(body) {
            Ok(r) => r,
            Err(err) => {
                tracing::error!(?err, "Failed to parse ServerDiscoverRequest");
                let error_response = JsonRpcErrorResponse::invalid_params(
                    req_id,
                    format!("Invalid params: {err}"),
                );
                return json_response(&error_response);
            }
        };

        let response = server::discover::handle_server_discover(
            request,
            self.server_info.clone(),
            self.instructions.clone(),
            self.capabilities.clone(),
            self.supported_versions.clone(),
        );

        json_response(&response)
    }

    fn handle_tools_list(
        &self,
        req_id: Option<JsonRpcRequestId>,
        body: &[u8],
    ) -> Response<ResponseBody> {
        let request: ListToolsRequest = match serde_json::from_slice(body) {
            Ok(r) => r,
            Err(err) => {
                tracing::error!(?err, "Failed to parse ListToolsRequest");
                let error_response = JsonRpcErrorResponse::invalid_params(
                    req_id,
                    format!("Invalid params: {err}"),
                );
                return json_response(&error_response);
            }
        };

        let response = tools::list::handle_list_tools(request, self.tools.clone());
        json_response(&response)
    }

    async fn handle_tools_call(
        &self,
        req_id: Option<JsonRpcRequestId>,
        header_name: Option<&str>,
        body: &[u8],
    ) -> Response<ResponseBody> {
        let request: CallToolRequest<serde_json::Value> = match serde_json::from_slice(body) {
            Ok(r) => r,
            Err(err) => {
                tracing::error!(?err, "Failed to parse CallToolRequest");
                let error_response = JsonRpcErrorResponse::invalid_params(
                    req_id,
                    format!("Invalid params: {err}"),
                );
                return json_response(&error_response);
            }
        };

        let tool_name = resolve_tool_name(
            header_name,
            request.params.as_ref().map(|p| p.name.as_str()),
        );

        let Some(tool_name) = tool_name else {
            tracing::debug!("Missing tool name for tools/call");
            let error_response = JsonRpcErrorResponse::invalid_params(
                Some(request.id),
                "Invalid params: missing tool name",
            );
            return json_response(&error_response);
        };

        if tool_name.is_empty() {
            tracing::debug!("Empty tool name for tools/call");
            let error_response = JsonRpcErrorResponse::invalid_params(
                Some(request.id),
                "Invalid params: empty tool name",
            );
            return json_response(&error_response);
        }

        if let Some(handler) = self.tool_handlers.get(tool_name) {
            let req_id = request.id.clone();
            let result = handler.call(request).await;
            let response = CallToolResultResponse::new(req_id, result);
            return json_response(&response);
        }

        tracing::debug!(tool_name, "Tool not found");
        let error_response = JsonRpcErrorResponse::method_not_found(
            Some(request.id),
            format!("Method not found: tool '{tool_name}' not found"),
        );
        json_response(&error_response)
    }
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
        let this = Arc::clone(&self.inner);
        Box::pin(async move { Ok(this.dispatch(req).await) })
    }
}

