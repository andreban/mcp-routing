// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::sync::Arc;

use http::Response;

use crate::body::{ResponseBody, json_response, json_response_with_caching};
use crate::extract::{RequestContext, SessionId};
use crate::router::{DispatchOutcome, MethodContext};
use crate::tools::{
    IntoToolHandler, IntoToolsListHandler, ToolError, ToolHandler, ToolsListHandler,
};
use crate::types::jsonrpc::{JsonRpcErrorResponse, JsonRpcRequestId};
use crate::types::mcp::{
    CacheScope,
    tools::{
        Tool,
        call::{CallToolParams, CallToolRequest, CallToolResult, CallToolResultResponse},
        list::{ListToolsParams, ListToolsRequest, ListToolsResultResponse},
    },
};
use crate::utils::resolve_tool_name;

/// Registry managing tool definitions, typed handlers, and tool cache configurations.
#[derive(Clone)]
pub struct ToolRegistry {
    pub(crate) tools: Vec<Tool>,
    pub(crate) tool_handlers: HashMap<String, Arc<dyn ToolHandler>>,
    pub(crate) tool_cache_settings: HashMap<String, (Option<u64>, Option<CacheScope>)>,
    pub(crate) tool_validators: HashMap<String, Arc<jsonschema::Validator>>,
    pub(crate) list_ttl_ms: Option<u64>,
    pub(crate) list_cache_scope: Option<CacheScope>,
    pub(crate) list_handler: Option<Arc<dyn ToolsListHandler>>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    /// Creates a new empty [`ToolRegistry`].
    pub fn new() -> Self {
        Self {
            tools: Vec::new(),
            tool_handlers: HashMap::new(),
            tool_cache_settings: HashMap::new(),
            tool_validators: HashMap::new(),
            list_ttl_ms: Some(0),
            list_cache_scope: Some(CacheScope::Public),
            list_handler: None,
        }
    }

    /// Sets a custom handler for `tools/list` requests.
    pub fn set_list_handler<H, T>(&mut self, handler: H)
    where
        H: IntoToolsListHandler<T>,
        T: 'static,
    {
        self.list_handler = Some(handler.into_tools_list_handler());
    }

    /// Registers a tool definition alongside a typed asynchronous handler.
    pub fn register<TTool, H, T>(&mut self, tool: TTool, handler: H)
    where
        TTool: Into<Tool>,
        H: IntoToolHandler<T>,
        T: 'static,
    {
        let tool = tool.into();
        let name = tool.name.clone();
        match jsonschema::validator_for(&tool.input_schema) {
            Ok(validator) => {
                self.tool_validators
                    .insert(name.clone(), Arc::new(validator));
            }
            Err(err) => {
                tracing::warn!(
                    tool_name = %name,
                    %err,
                    "Failed to compile input schema validator for tool"
                );
            }
        }
        self.tool_handlers.insert(name, handler.into_tool_handler());
        self.tools.push(tool);
    }

    /// Registers a tool definition alongside a typed asynchronous handler and tool-specific caching directives.
    pub fn register_with_cache<TTool, H, T>(
        &mut self,
        tool: TTool,
        handler: H,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) where
        TTool: Into<Tool>,
        H: IntoToolHandler<T>,
        T: 'static,
    {
        let tool = tool.into();
        let name = tool.name.clone();
        match jsonschema::validator_for(&tool.input_schema) {
            Ok(validator) => {
                self.tool_validators
                    .insert(name.clone(), Arc::new(validator));
            }
            Err(err) => {
                tracing::warn!(
                    tool_name = %name,
                    %err,
                    "Failed to compile input schema validator for tool"
                );
            }
        }
        self.tool_handlers
            .insert(name.clone(), handler.into_tool_handler());
        self.tool_cache_settings.insert(name, (ttl_ms, cache_scope));
        self.tools.push(tool);
    }

    /// Sets caching directives for a specific registered tool.
    pub fn set_tool_cache(
        &mut self,
        tool_name: impl Into<String>,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) {
        self.tool_cache_settings
            .insert(tool_name.into(), (ttl_ms, cache_scope));
    }

    /// Sets caching directives for `tools/list` responses.
    pub fn set_list_cache(&mut self, ttl_ms: Option<u64>, cache_scope: Option<CacheScope>) {
        self.list_ttl_ms = ttl_ms;
        self.list_cache_scope = cache_scope;
    }

    /// Dispatches an incoming `tools/list` JSON-RPC request.
    pub(crate) async fn dispatch_list(
        &self,
        ctx: MethodContext<'_>,
        params_val: Option<serde_json::Value>,
    ) -> DispatchOutcome {
        if ctx.is_notification {
            return DispatchOutcome::notification();
        }

        let params: ListToolsParams = match params_val {
            Some(pv) => match serde_json::from_value(pv) {
                Ok(p) => p,
                Err(err) => {
                    return DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                        ctx.req_id,
                        format!("Invalid params: {err}"),
                    ));
                }
            },
            None => ListToolsParams {
                meta: None,
                cursor: None,
                extras: HashMap::new(),
            },
        };

        if let Some(ref handler) = self.list_handler {
            let mut extensions = (*ctx.extensions).clone();
            extensions.insert(crate::extract::RegisteredTools(self.tools.clone()));
            let request_ctx = RequestContext::new(
                ctx.session_id,
                params.meta.clone(),
                ctx.headers.clone(),
                Arc::new(extensions),
            );
            match handler
                .call(
                    request_ctx,
                    params.cursor,
                    self.list_ttl_ms,
                    self.list_cache_scope.clone(),
                )
                .await
            {
                Ok(res) => {
                    let ttl_ms = res.ttl_ms;
                    let cache_scope = res.cache_scope.clone();
                    let response =
                        ListToolsResultResponse::new(ctx.req_id.unwrap_or_else(|| "".into()), res);
                    match serde_json::to_value(response) {
                        Ok(v) => DispatchOutcome::response_with_cache(v, ttl_ms, cache_scope),
                        Err(err) => DispatchOutcome::error(JsonRpcErrorResponse::internal_error(
                            None,
                            format!("Failed to serialize response: {err}"),
                        )),
                    }
                }
                Err(ToolError::InvalidParams(err)) => {
                    DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                        ctx.req_id,
                        format!("Invalid params: {err}"),
                    ))
                }
                Err(ToolError::Internal(err)) => {
                    DispatchOutcome::error(JsonRpcErrorResponse::internal_error(
                        ctx.req_id,
                        format!("Failed to list tools: {err}"),
                    ))
                }
            }
        } else {
            let req = ListToolsRequest::new(
                ctx.req_id.clone().unwrap_or_else(|| "".into()),
                "tools/list",
                Some(params),
            );

            let res = crate::tools::list::handle_list_tools(
                req,
                self.tools.clone(),
                self.list_ttl_ms,
                self.list_cache_scope.clone(),
            );

            let ttl_ms = res.result.ttl_ms;
            let cache_scope = res.result.cache_scope.clone();
            match serde_json::to_value(res) {
                Ok(v) => DispatchOutcome::response_with_cache(v, ttl_ms, cache_scope),
                Err(err) => DispatchOutcome::error(JsonRpcErrorResponse::internal_error(
                    ctx.req_id,
                    format!("Failed to serialize response: {err}"),
                )),
            }
        }
    }

    /// Dispatches an incoming `tools/call` JSON-RPC request to a registered typed tool handler.
    pub(crate) async fn dispatch_call(
        &self,
        ctx: MethodContext<'_>,
        params_val: Option<serde_json::Value>,
    ) -> DispatchOutcome {
        let params: Option<CallToolParams<serde_json::Value>> = match params_val {
            Some(pv) => match serde_json::from_value(pv) {
                Ok(p) => Some(p),
                Err(err) => {
                    return if ctx.is_notification {
                        DispatchOutcome::notification()
                    } else {
                        DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                            ctx.req_id,
                            format!("Invalid params: {err}"),
                        ))
                    };
                }
            },
            None => None,
        };

        let tool_name =
            resolve_tool_name(ctx.header_name, params.as_ref().map(|p| p.name.as_str()))
                .map(String::from);

        let Some(tool_name) = tool_name else {
            tracing::debug!("Missing tool name for tools/call");
            return if ctx.is_notification {
                DispatchOutcome::notification()
            } else {
                DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                    ctx.req_id,
                    "Invalid params: missing tool name",
                ))
            };
        };

        if tool_name.is_empty() {
            tracing::debug!("Empty tool name for tools/call");
            return if ctx.is_notification {
                DispatchOutcome::notification()
            } else {
                DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                    ctx.req_id,
                    "Invalid params: empty tool name",
                ))
            };
        }

        let Some(handler) = self.tool_handlers.get(&tool_name) else {
            tracing::debug!(tool_name, "Tool not found");
            return if ctx.is_notification {
                DispatchOutcome::notification()
            } else {
                DispatchOutcome::error(JsonRpcErrorResponse::method_not_found(
                    ctx.req_id,
                    format!("Method not found: tool '{tool_name}' not found"),
                ))
            };
        };

        let (tool_ttl, tool_scope) = self
            .tool_cache_settings
            .get(&tool_name)
            .cloned()
            .unwrap_or((None, None));
        let (meta, arguments) = match params {
            Some(p) => (p.meta, p.arguments),
            None => (None, None),
        };

        if let Some(validator) = self.tool_validators.get(&tool_name)
            && let Err(err_msg) = validate_tool_arguments(validator, arguments.as_ref())
        {
            if ctx.is_notification {
                return DispatchOutcome::notification();
            } else {
                let response = CallToolResultResponse::new(
                    ctx.req_id.clone().unwrap_or_else(|| "".into()),
                    CallToolResult::<serde_json::Value>::error(err_msg),
                );
                return match serde_json::to_value(response) {
                    Ok(v) => DispatchOutcome::response_with_cache(v, tool_ttl, tool_scope),
                    Err(err) => DispatchOutcome::error(JsonRpcErrorResponse::internal_error(
                        ctx.req_id,
                        format!("Failed to serialize response: {err}"),
                    )),
                };
            }
        }

        let request_ctx =
            RequestContext::new(ctx.session_id, meta, ctx.headers.clone(), ctx.extensions);
        let result = handler.call(request_ctx, arguments).await;
        if ctx.is_notification {
            DispatchOutcome::notification()
        } else {
            let response = CallToolResultResponse::new(
                ctx.req_id.clone().unwrap_or_else(|| "".into()),
                result,
            );
            match serde_json::to_value(response) {
                Ok(v) => DispatchOutcome::response_with_cache(v, tool_ttl, tool_scope),
                Err(err) => DispatchOutcome::error(JsonRpcErrorResponse::internal_error(
                    ctx.req_id,
                    format!("Failed to serialize response: {err}"),
                )),
            }
        }
    }

    /// Handles an incoming `tools/list` JSON-RPC request.
    pub fn handle_list(
        &self,
        req_id: Option<JsonRpcRequestId>,
        body: &[u8],
    ) -> Response<ResponseBody> {
        let request: ListToolsRequest = match serde_json::from_slice(body) {
            Ok(r) => r,
            Err(err) => {
                tracing::error!(?err, "Failed to parse ListToolsRequest");
                let error_response =
                    JsonRpcErrorResponse::invalid_params(req_id, format!("Invalid params: {err}"));
                return json_response(&error_response);
            }
        };

        let response = crate::tools::list::handle_list_tools(
            request,
            self.tools.clone(),
            self.list_ttl_ms,
            self.list_cache_scope.clone(),
        );
        json_response_with_caching(
            &response,
            response.result.ttl_ms,
            response.result.cache_scope.as_ref(),
        )
    }

    /// Handles an incoming `tools/call` JSON-RPC request.
    pub async fn handle_call(
        &self,
        req_id: Option<JsonRpcRequestId>,
        header_name: Option<&str>,
        session_id: Option<SessionId>,
        headers: &http::HeaderMap,
        extensions: Arc<http::Extensions>,
        body: &[u8],
    ) -> Response<ResponseBody> {
        let request: CallToolRequest<serde_json::Value> = match serde_json::from_slice(body) {
            Ok(r) => r,
            Err(err) => {
                tracing::error!(?err, "Failed to parse CallToolRequest");
                let error_response =
                    JsonRpcErrorResponse::invalid_params(req_id, format!("Invalid params: {err}"));
                return json_response(&error_response);
            }
        };

        let tool_name = resolve_tool_name(
            header_name,
            request.params.as_ref().map(|p| p.name.as_str()),
        )
        .map(String::from);

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

        if let Some(handler) = self.tool_handlers.get(&tool_name) {
            let (tool_ttl, tool_scope) = self
                .tool_cache_settings
                .get(&tool_name)
                .cloned()
                .unwrap_or((None, None));
            let req_id = request.id.clone();
            let (meta, raw_args) = match request.params {
                Some(p) => (p.meta, p.arguments),
                None => (None, None),
            };

            if let Some(validator) = self.tool_validators.get(&tool_name)
                && let Err(err_msg) = validate_tool_arguments(validator, raw_args.as_ref())
            {
                let response = CallToolResultResponse::new(
                    req_id,
                    CallToolResult::<serde_json::Value>::error(err_msg),
                );
                return json_response_with_caching(&response, tool_ttl, tool_scope.as_ref());
            }

            let ctx = RequestContext::new(session_id, meta, headers.clone(), extensions);
            let result = handler.call(ctx, raw_args).await;
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
}

/// Validates raw JSON tool arguments against the compiled JSON Schema validator.
fn validate_tool_arguments(
    validator: &jsonschema::Validator,
    arguments: Option<&serde_json::Value>,
) -> Result<(), String> {
    let empty_obj = serde_json::Value::Object(serde_json::Map::new());
    let raw_to_validate = arguments.unwrap_or(&empty_obj);
    let mut errors = Vec::new();
    for error in validator.iter_errors(raw_to_validate) {
        let path = error.instance_path().to_string();
        if path.is_empty() || path == "/" {
            errors.push(error.to_string());
        } else {
            errors.push(format!("at `{path}`: {error}"));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Input schema validation failed: {}",
            errors.join("; ")
        ))
    }
}
