// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

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
use crate::prompts::{IntoPromptHandler, PromptRegistry};
use crate::server::ServerConfig;
use crate::tools::{IntoToolHandler, ToolRegistry};
use crate::types::jsonrpc::{JsonRpcErrorResponse, JsonRpcRequestId};
use crate::types::mcp::{
    CacheScope, Implementation, PromptsCapability, ServerCapabilities,
    prompts::Prompt,
    tools::Tool,
};
use crate::utils::{
    extract_header_name, extract_method, extract_session_id, is_json_content_type,
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

type StateInjector = Arc<dyn Fn(&mut http::Extensions) + Send + Sync>;

#[derive(Clone)]
struct McpRouterInner {
    server: ServerConfig,
    tools: ToolRegistry,
    prompts: PromptRegistry,
    state_injectors: Vec<StateInjector>,
}

impl McpRouter {
    /// Creates a new [`McpRouter`] initialized with the given server [`Implementation`] metadata.
    pub fn new(server_info: Implementation) -> Self {
        Self {
            inner: Arc::new(McpRouterInner {
                server: ServerConfig::new(server_info),
                tools: ToolRegistry::new(),
                prompts: PromptRegistry::new(),
                state_injectors: Vec::new(),
            }),
        }
    }

    /// Attaches application state to the router.
    ///
    /// The provided state value will be injected into request context for tool and prompt handlers,
    /// making it accessible via the [`State`](crate::extract::State) or [`Extension`](crate::extract::Extension)
    /// extractors, or through [`RequestContext::state`](crate::extract::RequestContext::state).
    pub fn with_state<S: Clone + Send + Sync + 'static>(mut self, state: S) -> Self {
        let injector: StateInjector = Arc::new(move |exts: &mut http::Extensions| {
            if exts.get::<S>().is_none() {
                exts.insert(state.clone());
            }
        });
        Arc::make_mut(&mut self.inner).state_injectors.push(injector);
        self
    }

    /// Sets human-readable instructions describing how to use this MCP server.
    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        Arc::make_mut(&mut self.inner).server.instructions = Some(instructions.into());
        self
    }

    /// Sets the server capabilities advertised in `server/discover`.
    pub fn capabilities(mut self, capabilities: ServerCapabilities) -> Self {
        Arc::make_mut(&mut self.inner).server.capabilities = capabilities;
        self
    }

    /// Sets the MCP protocol versions supported by this server.
    pub fn supported_versions(mut self, supported_versions: Vec<String>) -> Self {
        Arc::make_mut(&mut self.inner).server.supported_versions = supported_versions;
        self
    }

    /// Sets the time-to-live (`ttl_ms`) and cache scope for `server/discover` responses.
    pub fn server_discover_cache(
        mut self,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) -> Self {
        let server = &mut Arc::make_mut(&mut self.inner).server;
        server.discover_ttl_ms = ttl_ms;
        server.discover_cache_scope = cache_scope;
        self
    }

    /// Sets the time-to-live (`ttl_ms`) in milliseconds for `server/discover` responses.
    pub fn server_discover_ttl(mut self, ttl_ms: u64) -> Self {
        Arc::make_mut(&mut self.inner).server.discover_ttl_ms = Some(ttl_ms);
        self
    }

    /// Sets the cache scope for `server/discover` responses.
    pub fn server_discover_cache_scope(mut self, cache_scope: CacheScope) -> Self {
        Arc::make_mut(&mut self.inner).server.discover_cache_scope = Some(cache_scope);
        self
    }

    /// Sets the time-to-live (`ttl_ms`) and cache scope for `tools/list` responses.
    pub fn tools_list_cache(
        mut self,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) -> Self {
        Arc::make_mut(&mut self.inner)
            .tools
            .set_list_cache(ttl_ms, cache_scope);
        self
    }

    /// Sets the time-to-live (`ttl_ms`) in milliseconds for `tools/list` responses.
    pub fn tools_list_ttl(mut self, ttl_ms: u64) -> Self {
        Arc::make_mut(&mut self.inner).tools.list_ttl_ms = Some(ttl_ms);
        self
    }

    /// Sets the cache scope for `tools/list` responses.
    pub fn tools_list_cache_scope(mut self, cache_scope: CacheScope) -> Self {
        Arc::make_mut(&mut self.inner).tools.list_cache_scope = Some(cache_scope);
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
        Arc::make_mut(&mut self.inner)
            .tools
            .register(tool, handler);
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
        Arc::make_mut(&mut self.inner)
            .tools
            .register_with_cache(tool, handler, ttl_ms, cache_scope);
        self
    }

    /// Sets the cache configuration (`ttl_ms` and `cache_scope`) for a specific registered tool by name.
    pub fn tool_cache(
        mut self,
        tool_name: impl Into<String>,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) -> Self {
        Arc::make_mut(&mut self.inner)
            .tools
            .set_tool_cache(tool_name, ttl_ms, cache_scope);
        self
    }

    /// Sets the time-to-live (`ttl_ms`) and cache scope for `prompts/list` responses.
    pub fn prompts_list_cache(
        mut self,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) -> Self {
        Arc::make_mut(&mut self.inner)
            .prompts
            .set_list_cache(ttl_ms, cache_scope);
        self
    }

    /// Sets the time-to-live (`ttl_ms`) in milliseconds for `prompts/list` responses.
    pub fn prompts_list_ttl(mut self, ttl_ms: u64) -> Self {
        Arc::make_mut(&mut self.inner).prompts.list_ttl_ms = Some(ttl_ms);
        self
    }

    /// Sets the cache scope for `prompts/list` responses.
    pub fn prompts_list_cache_scope(mut self, cache_scope: CacheScope) -> Self {
        Arc::make_mut(&mut self.inner).prompts.list_cache_scope = Some(cache_scope);
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
        let inner = Arc::make_mut(&mut self.inner);
        if inner.server.capabilities.prompts.is_none() {
            inner.server.capabilities.prompts = Some(PromptsCapability { list_changed: None });
        }
        inner.prompts.register(prompt, handler);
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
        let inner = Arc::make_mut(&mut self.inner);
        if inner.server.capabilities.prompts.is_none() {
            inner.server.capabilities.prompts = Some(PromptsCapability { list_changed: None });
        }
        inner
            .prompts
            .register_with_cache(prompt, handler, ttl_ms, cache_scope);
        self
    }

    /// Sets the cache configuration (`ttl_ms` and `cache_scope`) for a specific registered prompt by name.
    pub fn prompt_cache(
        mut self,
        prompt_name: impl Into<String>,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) -> Self {
        Arc::make_mut(&mut self.inner)
            .prompts
            .set_prompt_cache(prompt_name, ttl_ms, cache_scope);
        self
    }
}

impl McpRouterInner {
    async fn dispatch<B>(&self, req: Request<B>) -> Response<ResponseBody>
    where
        B: http_body::Body<Data = Bytes> + Send + 'static,
        B::Error: Into<BoxError>,
    {
        let session_id = extract_session_id(req.headers());

        let attach_session = |mut resp: Response<ResponseBody>| {
            if let Some(ref sid) = session_id
                && let Ok(header_val) = http::HeaderValue::from_str(sid.as_str())
            {
                resp.headers_mut().insert(
                    http::header::HeaderName::from_static("mcp-session-id"),
                    header_val,
                );
            }
            resp
        };

        if req.method() != http::Method::POST {
            tracing::debug!(method = %req.method(), "HTTP method not allowed, only POST is supported");
            return attach_session(method_not_allowed());
        }

        if !is_json_content_type(req.headers()) {
            tracing::debug!("Missing or unsupported Content-Type header");
            return attach_session(unsupported_media_type());
        }

        let (mut parts, body) = req.into_parts();
        for injector in &self.state_injectors {
            injector(&mut parts.extensions);
        }
        let extensions = Arc::new(parts.extensions);

        let body_bytes = match body.collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(err) => {
                let err = err.into();
                tracing::error!(?err, "Failed to read request body");
                return attach_session(bad_request());
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
                return attach_session(json_response(&error_response));
            }
        };

        let method = extract_method(&parts.headers, peek.method.as_deref());
        let Some(method) = method else {
            tracing::debug!("Missing method in both Mcp-Method header and JSON-RPC body");
            let error_response = JsonRpcErrorResponse::invalid_request(
                peek.id,
                "Invalid Request: missing method",
            );
            return attach_session(json_response(&error_response));
        };

        if method.is_empty() {
            tracing::debug!("Empty method provided");
            let error_response = JsonRpcErrorResponse::invalid_request(
                peek.id,
                "Invalid Request: empty method",
            );
            return attach_session(json_response(&error_response));
        }

        let header_name = extract_header_name(&parts.headers);

        let response = match method.as_str() {
            "server/discover" => self.server.handle_discover(peek.id, &body_bytes),
            "tools/list" => self.tools.handle_list(peek.id, &body_bytes),
            "tools/call" => {
                self.tools
                    .handle_call(
                        peek.id,
                        header_name.as_deref(),
                        session_id.clone(),
                        &parts.headers,
                        Arc::clone(&extensions),
                        &body_bytes,
                    )
                    .await
            }
            "prompts/list" => self.prompts.handle_list(peek.id, &body_bytes),
            "prompts/get" => {
                self.prompts
                    .handle_get(
                        peek.id,
                        header_name.as_deref(),
                        session_id.clone(),
                        &parts.headers,
                        Arc::clone(&extensions),
                        &body_bytes,
                    )
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
        };

        attach_session(response)
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
