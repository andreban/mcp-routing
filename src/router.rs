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
    BoxError, ResponseBody, bad_request, json_response, json_response_with_caching,
    method_not_allowed, unsupported_media_type,
};
use crate::prompts::{self, IntoPromptHandler, PromptError, PromptHandler};
use crate::server;
use crate::tools::{self, IntoToolHandler, ToolHandler};
use crate::types::jsonrpc::{JsonRpcErrorResponse, JsonRpcRequestId};
use crate::types::mcp::{
    CacheScope, Implementation, PromptsCapability, ServerCapabilities, ToolsCapability,
    prompts::{
        Prompt,
        get::{GetPromptRequest, GetPromptResultResponse},
        list::ListPromptsRequest,
    },
    server::discover::ServerDiscoverRequest,
    tools::{
        Tool,
        call::{CallToolRequest, CallToolResultResponse},
        list::ListToolsRequest,
    },
};
use crate::utils::{
    extract_header_name, extract_method, is_json_content_type, resolve_prompt_name,
    resolve_tool_name,
};

/// A [Tower](tower)-native router for the Model Context Protocol (MCP).
///
/// `McpRouter` implements [`tower::Service`] for HTTP requests and handles routing for:
/// - Built-in `server/discover` discovery endpoint
/// - Built-in `tools/list` tool discovery endpoint
/// - `tools/call` tool execution endpoints (delegating to typed handlers)
/// - Built-in `prompts/list` prompt discovery endpoint
/// - `prompts/get` prompt retrieval endpoints (delegating to typed handlers)
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
    tool_cache_settings: HashMap<String, (Option<u64>, Option<CacheScope>)>,
    prompts: Vec<Prompt>,
    prompt_handlers: HashMap<String, Arc<dyn PromptHandler>>,
    prompt_cache_settings: HashMap<String, (Option<u64>, Option<CacheScope>)>,
    server_discover_ttl_ms: Option<u64>,
    server_discover_cache_scope: Option<CacheScope>,
    tools_list_ttl_ms: Option<u64>,
    tools_list_cache_scope: Option<CacheScope>,
    prompts_list_ttl_ms: Option<u64>,
    prompts_list_cache_scope: Option<CacheScope>,
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
                tool_cache_settings: HashMap::new(),
                prompts: Vec::new(),
                prompt_handlers: HashMap::new(),
                prompt_cache_settings: HashMap::new(),
                server_discover_ttl_ms: Some(0),
                server_discover_cache_scope: Some(CacheScope::Public),
                tools_list_ttl_ms: Some(0),
                tools_list_cache_scope: Some(CacheScope::Public),
                prompts_list_ttl_ms: Some(0),
                prompts_list_cache_scope: Some(CacheScope::Public),
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

    /// Sets the time-to-live (`ttl_ms`) and cache scope for `server/discover` responses.
    pub fn server_discover_cache(
        mut self,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) -> Self {
        let inner = Arc::make_mut(&mut self.inner);
        inner.server_discover_ttl_ms = ttl_ms;
        inner.server_discover_cache_scope = cache_scope;
        self
    }

    /// Sets the time-to-live (`ttl_ms`) in milliseconds for `server/discover` responses.
    pub fn server_discover_ttl(mut self, ttl_ms: u64) -> Self {
        Arc::make_mut(&mut self.inner).server_discover_ttl_ms = Some(ttl_ms);
        self
    }

    /// Sets the cache scope for `server/discover` responses.
    pub fn server_discover_cache_scope(mut self, cache_scope: CacheScope) -> Self {
        Arc::make_mut(&mut self.inner).server_discover_cache_scope = Some(cache_scope);
        self
    }

    /// Sets the time-to-live (`ttl_ms`) and cache scope for `tools/list` responses.
    pub fn tools_list_cache(
        mut self,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) -> Self {
        let inner = Arc::make_mut(&mut self.inner);
        inner.tools_list_ttl_ms = ttl_ms;
        inner.tools_list_cache_scope = cache_scope;
        self
    }

    /// Sets the time-to-live (`ttl_ms`) in milliseconds for `tools/list` responses.
    pub fn tools_list_ttl(mut self, ttl_ms: u64) -> Self {
        Arc::make_mut(&mut self.inner).tools_list_ttl_ms = Some(ttl_ms);
        self
    }

    /// Sets the cache scope for `tools/list` responses.
    pub fn tools_list_cache_scope(mut self, cache_scope: CacheScope) -> Self {
        Arc::make_mut(&mut self.inner).tools_list_cache_scope = Some(cache_scope);
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

    /// Registers a tool definition alongside a typed asynchronous handler and tool-specific caching directives.
    ///
    /// The specified `ttl_ms` and `cache_scope` will be propagated as HTTP `Cache-Control` headers
    /// when the tool is executed via `tools/call`.
    pub fn register_tool_with_cache<TTool, H, T>(
        mut self,
        tool: TTool,
        handler: H,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) -> Self
    where
        TTool: Into<Tool>,
        H: IntoToolHandler<T>,
        T: 'static,
    {
        let tool = tool.into();
        let name = tool.name.clone();
        let inner = Arc::make_mut(&mut self.inner);
        inner.tool_handlers.insert(name.clone(), handler.into_tool_handler());
        inner.tool_cache_settings.insert(name, (ttl_ms, cache_scope));
        inner.tools.push(tool);
        self
    }

    /// Sets the cache configuration (`ttl_ms` and `cache_scope`) for a specific registered tool by name.
    pub fn tool_cache(
        mut self,
        tool_name: impl Into<String>,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) -> Self {
        let inner = Arc::make_mut(&mut self.inner);
        inner.tool_cache_settings.insert(tool_name.into(), (ttl_ms, cache_scope));
        self
    }

    /// Sets the time-to-live (`ttl_ms`) and cache scope for `prompts/list` responses.
    pub fn prompts_list_cache(
        mut self,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) -> Self {
        let inner = Arc::make_mut(&mut self.inner);
        inner.prompts_list_ttl_ms = ttl_ms;
        inner.prompts_list_cache_scope = cache_scope;
        self
    }

    /// Sets the time-to-live (`ttl_ms`) in milliseconds for `prompts/list` responses.
    pub fn prompts_list_ttl(mut self, ttl_ms: u64) -> Self {
        Arc::make_mut(&mut self.inner).prompts_list_ttl_ms = Some(ttl_ms);
        self
    }

    /// Sets the cache scope for `prompts/list` responses.
    pub fn prompts_list_cache_scope(mut self, cache_scope: CacheScope) -> Self {
        Arc::make_mut(&mut self.inner).prompts_list_cache_scope = Some(cache_scope);
        self
    }

    /// Registers a prompt template alongside a typed asynchronous handler function.
    ///
    /// The handler function can take typed deserializable arguments (or no arguments)
    /// and return any type implementing [`IntoPromptResult`](crate::prompts::IntoPromptResult).
    pub fn register_prompt<TPrompt, H, T>(mut self, prompt: TPrompt, handler: H) -> Self
    where
        TPrompt: Into<Prompt>,
        H: IntoPromptHandler<T>,
        T: 'static,
    {
        let prompt = prompt.into();
        let name = prompt.name.clone();
        let inner = Arc::make_mut(&mut self.inner);
        if inner.capabilities.prompts.is_none() {
            inner.capabilities.prompts = Some(PromptsCapability { list_changed: None });
        }
        inner.prompt_handlers.insert(name, handler.into_prompt_handler());
        inner.prompts.push(prompt);
        self
    }

    /// Registers a prompt template alongside a typed asynchronous handler and prompt-specific caching directives.
    pub fn register_prompt_with_cache<TPrompt, H, T>(
        mut self,
        prompt: TPrompt,
        handler: H,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) -> Self
    where
        TPrompt: Into<Prompt>,
        H: IntoPromptHandler<T>,
        T: 'static,
    {
        let prompt = prompt.into();
        let name = prompt.name.clone();
        let inner = Arc::make_mut(&mut self.inner);
        if inner.capabilities.prompts.is_none() {
            inner.capabilities.prompts = Some(PromptsCapability { list_changed: None });
        }
        inner.prompt_handlers.insert(name.clone(), handler.into_prompt_handler());
        inner.prompt_cache_settings.insert(name, (ttl_ms, cache_scope));
        inner.prompts.push(prompt);
        self
    }

    /// Sets the cache configuration (`ttl_ms` and `cache_scope`) for a specific registered prompt by name.
    pub fn prompt_cache(
        mut self,
        prompt_name: impl Into<String>,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) -> Self {
        let inner = Arc::make_mut(&mut self.inner);
        inner.prompt_cache_settings.insert(prompt_name.into(), (ttl_ms, cache_scope));
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
            "prompts/list" => self.handle_prompts_list(peek.id, &body_bytes),
            "prompts/get" => {
                self.handle_prompts_get(peek.id, header_name.as_deref(), &body_bytes)
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
            self.server_discover_ttl_ms,
            self.server_discover_cache_scope.clone(),
        );

        json_response_with_caching(
            &response,
            response.result.ttl_ms,
            response.result.cache_scope.as_ref(),
        )
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

        let response = tools::list::handle_list_tools(
            request,
            self.tools.clone(),
            self.tools_list_ttl_ms,
            self.tools_list_cache_scope.clone(),
        );
        json_response_with_caching(
            &response,
            response.result.ttl_ms,
            response.result.cache_scope.as_ref(),
        )
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
            let (tool_ttl, tool_scope) = self
                .tool_cache_settings
                .get(tool_name)
                .cloned()
                .unwrap_or((None, None));
            let req_id = request.id.clone();
            let result = handler.call(request).await;
            let response = CallToolResultResponse::new(req_id, result);
            return json_response_with_caching(&response, tool_ttl, tool_scope.as_ref());
        }

        tracing::debug!(tool_name, "Tool not found");
        let error_response = JsonRpcErrorResponse::method_not_found(
            Some(request.id),
            format!("Method not found: tool '{tool_name}' not found"),
        );
        json_response(&error_response)
    }

    fn handle_prompts_list(
        &self,
        req_id: Option<JsonRpcRequestId>,
        body: &[u8],
    ) -> Response<ResponseBody> {
        let request: ListPromptsRequest = match serde_json::from_slice(body) {
            Ok(r) => r,
            Err(err) => {
                tracing::error!(?err, "Failed to parse ListPromptsRequest");
                let error_response = JsonRpcErrorResponse::invalid_params(
                    req_id,
                    format!("Invalid params: {err}"),
                );
                return json_response(&error_response);
            }
        };

        let response = prompts::list::handle_list_prompts(
            request,
            self.prompts.clone(),
            self.prompts_list_ttl_ms,
            self.prompts_list_cache_scope.clone(),
        );
        json_response_with_caching(
            &response,
            response.result.ttl_ms,
            response.result.cache_scope.as_ref(),
        )
    }

    async fn handle_prompts_get(
        &self,
        req_id: Option<JsonRpcRequestId>,
        header_name: Option<&str>,
        body: &[u8],
    ) -> Response<ResponseBody> {
        let request: GetPromptRequest<serde_json::Value> = match serde_json::from_slice(body) {
            Ok(r) => r,
            Err(err) => {
                tracing::error!(?err, "Failed to parse GetPromptRequest");
                let error_response = JsonRpcErrorResponse::invalid_params(
                    req_id,
                    format!("Invalid params: {err}"),
                );
                return json_response(&error_response);
            }
        };

        let prompt_name = resolve_prompt_name(
            header_name,
            request.params.as_ref().map(|p| p.name.as_str()),
        );

        let Some(prompt_name) = prompt_name else {
            tracing::debug!("Missing prompt name for prompts/get");
            let error_response = JsonRpcErrorResponse::invalid_params(
                Some(request.id),
                "Invalid params: missing prompt name",
            );
            return json_response(&error_response);
        };

        if prompt_name.is_empty() {
            tracing::debug!("Empty prompt name for prompts/get");
            let error_response = JsonRpcErrorResponse::invalid_params(
                Some(request.id),
                "Invalid params: empty prompt name",
            );
            return json_response(&error_response);
        }

        if let Some(handler) = self.prompt_handlers.get(prompt_name) {
            let (prompt_ttl, prompt_scope) = self
                .prompt_cache_settings
                .get(prompt_name)
                .cloned()
                .unwrap_or((None, None));
            let req_id = request.id.clone();
            match handler.call(request).await {
                Ok(result) => {
                    let response = GetPromptResultResponse::new(req_id, result);
                    return json_response_with_caching(&response, prompt_ttl, prompt_scope.as_ref());
                }
                Err(PromptError::InvalidParams(err)) => {
                    let error_response = JsonRpcErrorResponse::invalid_params(
                        Some(req_id),
                        format!("Invalid params: {err}"),
                    );
                    return json_response(&error_response);
                }
                Err(PromptError::Internal(err)) => {
                    let error_response = JsonRpcErrorResponse::internal_error(
                        Some(req_id),
                        format!("Prompt execution failed: {err}"),
                    );
                    return json_response(&error_response);
                }
            }
        }

        tracing::debug!(prompt_name, "Prompt not found");
        let error_response = JsonRpcErrorResponse::method_not_found(
            Some(request.id),
            format!("Method not found: prompt '{prompt_name}' not found"),
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

