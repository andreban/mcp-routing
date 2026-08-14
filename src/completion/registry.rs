// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::sync::Arc;

use crate::completion::{CompletionError, CompletionHandler, IntoCompletionHandler};
use crate::extract::RequestContext;
use crate::router::{DispatchOutcome, MethodContext};
use crate::types::jsonrpc::JsonRpcErrorResponse;
use crate::types::mcp::{
    CacheScope,
    completion::{CompleteParams, CompleteResult, CompleteResultResponse, Reference},
};
use crate::utils::match_uri_template;

/// Registry managing prompt and resource argument autocompletion handlers.
#[derive(Clone)]
pub struct CompletionRegistry {
    pub(crate) default_handler: Option<Arc<dyn CompletionHandler>>,
    pub(crate) prompt_handlers: HashMap<(String, Option<String>), Arc<dyn CompletionHandler>>,
    pub(crate) resource_handlers: Vec<(String, Option<String>, Arc<dyn CompletionHandler>)>,
    pub(crate) cache_ttl_ms: Option<u64>,
    pub(crate) cache_scope: Option<CacheScope>,
}

impl Default for CompletionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CompletionRegistry {
    /// Creates a new empty [`CompletionRegistry`].
    pub fn new() -> Self {
        Self {
            default_handler: None,
            prompt_handlers: HashMap::new(),
            resource_handlers: Vec::new(),
            cache_ttl_ms: None,
            cache_scope: None,
        }
    }

    /// Returns `true` if at least one completion handler or default provider is registered.
    pub fn has_handlers(&self) -> bool {
        self.default_handler.is_some()
            || !self.prompt_handlers.is_empty()
            || !self.resource_handlers.is_empty()
    }

    /// Sets a default / fallback completion handler called when no prompt- or resource-specific handler matches.
    pub fn set_default_handler<H, T>(&mut self, handler: H)
    where
        H: IntoCompletionHandler<T>,
        T: 'static,
    {
        self.default_handler = Some(handler.into_completion_handler());
    }

    /// Registers a completion handler for all arguments of a prompt template.
    pub fn register_prompt<H, T>(&mut self, prompt_name: impl Into<String>, handler: H)
    where
        H: IntoCompletionHandler<T>,
        T: 'static,
    {
        self.prompt_handlers
            .insert((prompt_name.into(), None), handler.into_completion_handler());
    }

    /// Registers a completion handler for a specific argument of a prompt template.
    pub fn register_prompt_arg<H, T>(
        &mut self,
        prompt_name: impl Into<String>,
        arg_name: impl Into<String>,
        handler: H,
    ) where
        H: IntoCompletionHandler<T>,
        T: 'static,
    {
        self.prompt_handlers.insert(
            (prompt_name.into(), Some(arg_name.into())),
            handler.into_completion_handler(),
        );
    }

    /// Registers a completion handler for all variables of a resource URI or URI template.
    pub fn register_resource<H, T>(&mut self, uri_or_template: impl Into<String>, handler: H)
    where
        H: IntoCompletionHandler<T>,
        T: 'static,
    {
        self.resource_handlers
            .push((uri_or_template.into(), None, handler.into_completion_handler()));
    }

    /// Registers a completion handler for a specific variable of a resource URI or URI template.
    pub fn register_resource_arg<H, T>(
        &mut self,
        uri_or_template: impl Into<String>,
        arg_name: impl Into<String>,
        handler: H,
    ) where
        H: IntoCompletionHandler<T>,
        T: 'static,
    {
        self.resource_handlers.push((
            uri_or_template.into(),
            Some(arg_name.into()),
            handler.into_completion_handler(),
        ));
    }

    /// Sets caching directives (`ttl_ms` and `cache_scope`) for `completion/complete` responses.
    pub fn set_cache(&mut self, ttl_ms: Option<u64>, cache_scope: Option<CacheScope>) {
        self.cache_ttl_ms = ttl_ms;
        self.cache_scope = cache_scope;
    }

    /// Dispatches an incoming `completion/complete` JSON-RPC request.
    pub(crate) async fn dispatch_complete(
        &self,
        ctx: MethodContext<'_>,
        params_val: Option<serde_json::Value>,
    ) -> DispatchOutcome {
        if ctx.is_notification {
            return DispatchOutcome::notification();
        }

        let Some(pv) = params_val else {
            return DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                ctx.req_id,
                "Invalid params: missing parameters for completion/complete",
            ));
        };

        let params: CompleteParams = match serde_json::from_value(pv) {
            Ok(p) => p,
            Err(err) => {
                return DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                    ctx.req_id,
                    format!("Invalid params: {err}"),
                ));
            }
        };

        let handler: Option<Arc<dyn CompletionHandler>> = match &params.reference {
            Reference::Prompt { name } => {
                // 1. Try prompt name + specific arg name
                self.prompt_handlers
                    .get(&(name.clone(), Some(params.argument.name.clone())))
                    .cloned()
                    // 2. Try prompt name + any arg
                    .or_else(|| {
                        self.prompt_handlers
                            .get(&(name.clone(), None))
                            .cloned()
                    })
            }
            Reference::Resource { uri } => {
                // 1. Try exact uri + specific arg name
                self.resource_handlers
                    .iter()
                    .find(|(u, arg, _)| {
                        u == uri && arg.as_deref() == Some(&params.argument.name)
                    })
                    .map(|(_, _, h)| h.clone())
                    // 2. Try exact uri + any arg
                    .or_else(|| {
                        self.resource_handlers
                            .iter()
                            .find(|(u, arg, _)| u == uri && arg.is_none())
                            .map(|(_, _, h)| h.clone())
                    })
                    // 3. Try template match + specific arg name
                    .or_else(|| {
                        self.resource_handlers
                            .iter()
                            .find(|(template, arg, _)| {
                                arg.as_deref() == Some(&params.argument.name)
                                    && match_uri_template(template, uri)
                            })
                            .map(|(_, _, h)| h.clone())
                    })
                    // 4. Try template match + any arg
                    .or_else(|| {
                        self.resource_handlers
                            .iter()
                            .find(|(template, arg, _)| {
                                arg.is_none() && match_uri_template(template, uri)
                            })
                            .map(|(_, _, h)| h.clone())
                    })
            }
        }
        .or_else(|| self.default_handler.clone());

        let Some(handler) = handler else {
            let target = match &params.reference {
                Reference::Prompt { name } => format!("prompt '{name}'"),
                Reference::Resource { uri } => format!("resource '{uri}'"),
            };
            tracing::debug!("No completion handler found for {target}");
            return DispatchOutcome::error(JsonRpcErrorResponse::method_not_found(
                ctx.req_id,
                format!("No completion handler registered for {target}"),
            ));
        };

        let request_ctx = RequestContext::new(
            ctx.session_id,
            params.meta.clone(),
            ctx.headers.clone(),
            ctx.extensions,
        );

        match handler.call(request_ctx, params).await {
            Ok(res) => {
                let clamped_res = CompleteResult {
                    meta: res.meta,
                    completion: res.completion.clamp_to_limit(100),
                    extras: res.extras,
                };
                let response = CompleteResultResponse::new(
                    ctx.req_id.unwrap_or_else(|| "".into()),
                    clamped_res,
                );
                match serde_json::to_value(response) {
                    Ok(v) => DispatchOutcome::response_with_cache(
                        v,
                        self.cache_ttl_ms,
                        self.cache_scope.clone(),
                    ),
                    Err(err) => DispatchOutcome::error(JsonRpcErrorResponse::internal_error(
                        None,
                        format!("Failed to serialize response: {err}"),
                    )),
                }
            }
            Err(CompletionError::InvalidParams(err)) => {
                DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                    ctx.req_id,
                    format!("Invalid params: {err}"),
                ))
            }
            Err(CompletionError::NotFound(err)) => {
                DispatchOutcome::error(JsonRpcErrorResponse::method_not_found(
                    ctx.req_id,
                    format!("Not found: {err}"),
                ))
            }
            Err(CompletionError::Internal(err)) => {
                DispatchOutcome::error(JsonRpcErrorResponse::internal_error(
                    ctx.req_id,
                    format!("Completion failed: {err}"),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::SessionId;
    use crate::types::jsonrpc::JsonRpcRequestId;
    use crate::types::mcp::completion::{CompleteArgument, CompleteContext};

    #[tokio::test]
    async fn test_completion_registry_dispatch_prompt() {
        let mut registry = CompletionRegistry::new();
        registry.register_prompt_arg("review", "lang", |arg: CompleteArgument| async move {
            let matches = vec!["rust", "ruby"];
            matches
                .into_iter()
                .filter(|s| s.starts_with(&arg.value))
                .collect::<Vec<_>>()
        });

        let headers = http::HeaderMap::new();
        let extensions = Arc::new(http::Extensions::new());
        let ctx = MethodContext {
            req_id: Some(JsonRpcRequestId::Number(1.0)),
            is_notification: false,
            header_name: None,
            session_id: Some(SessionId::new("sess-1")),
            headers: &headers,
            extensions,
        };

        let params = serde_json::json!({
            "ref": { "type": "ref/prompt", "name": "review" },
            "argument": { "name": "lang", "value": "ru" }
        });

        let outcome = registry.dispatch_complete(ctx, Some(params)).await;
        let resp = outcome.response.expect("expected response");
        assert_eq!(resp["result"]["completion"]["values"], serde_json::json!(["rust", "ruby"]));
    }

    #[tokio::test]
    async fn test_completion_registry_dispatch_resource_template() {
        let mut registry = CompletionRegistry::new();
        registry.register_resource_arg(
            "db://{schema}/{table}",
            "table",
            |arg: CompleteArgument, ctx: Option<CompleteContext>| async move {
                let schema = ctx.as_ref().and_then(|c| c.get_argument("schema")).unwrap_or("public");
                vec![format!("{schema}_{}", arg.value)]
            },
        );

        let headers = http::HeaderMap::new();
        let extensions = Arc::new(http::Extensions::new());
        let ctx = MethodContext {
            req_id: Some(JsonRpcRequestId::Number(2.0)),
            is_notification: false,
            header_name: None,
            session_id: None,
            headers: &headers,
            extensions,
        };

        let params = serde_json::json!({
            "ref": { "type": "ref/resource", "uri": "db://analytics/users" },
            "argument": { "name": "table", "value": "users" },
            "context": {
                "arguments": {
                    "schema": "analytics"
                }
            }
        });

        let outcome = registry.dispatch_complete(ctx, Some(params)).await;
        let resp = outcome.response.expect("expected response");
        assert_eq!(resp["result"]["completion"]["values"], serde_json::json!(["analytics_users"]));
    }

    #[tokio::test]
    async fn test_completion_registry_unhandled_returns_method_not_found() {
        let registry = CompletionRegistry::new();

        let headers = http::HeaderMap::new();
        let extensions = Arc::new(http::Extensions::new());
        let ctx = MethodContext {
            req_id: Some(JsonRpcRequestId::Number(3.0)),
            is_notification: false,
            header_name: None,
            session_id: None,
            headers: &headers,
            extensions,
        };

        let params = serde_json::json!({
            "ref": { "type": "ref/prompt", "name": "unknown" },
            "argument": { "name": "foo", "value": "bar" }
        });

        let outcome = registry.dispatch_complete(ctx, Some(params)).await;
        let resp = outcome.response.expect("expected error response");
        assert_eq!(resp["error"]["code"], -32601);
    }
}
