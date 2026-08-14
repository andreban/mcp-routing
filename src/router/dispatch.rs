// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use crate::extract::SessionId;
use crate::router::{DispatchOutcome, McpRouterInner, MethodContext};
use crate::types::jsonrpc::{JsonRpcErrorResponse, JsonRpcRequestId};
use crate::utils::{extract_header_name, extract_method};

impl McpRouterInner {
    /// Dispatches a single item within a JSON-RPC batch array.
    pub(crate) async fn dispatch_item(
        &self,
        item: serde_json::Value,
        headers: &http::HeaderMap,
        session_id: Option<SessionId>,
        extensions: Arc<http::Extensions>,
    ) -> Option<serde_json::Value> {
        match item {
            serde_json::Value::Object(map) => {
                let outcome = self
                    .dispatch_object(map, headers, session_id, extensions)
                    .await;
                outcome.response
            }
            _ => {
                let err = JsonRpcErrorResponse::invalid_request(
                    None,
                    "Invalid Request: expected object in batch array",
                );
                serde_json::to_value(err).ok()
            }
        }
    }

    /// Dispatches a JSON-RPC object request or notification to the appropriate capability handler.
    pub(crate) async fn dispatch_object(
        &self,
        mut map: serde_json::Map<String, serde_json::Value>,
        headers: &http::HeaderMap,
        session_id: Option<SessionId>,
        extensions: Arc<http::Extensions>,
    ) -> DispatchOutcome {
        let (req_id, is_notification) = match map.remove("id") {
            None => (None, true),
            Some(serde_json::Value::String(s)) => (Some(JsonRpcRequestId::String(s)), false),
            Some(serde_json::Value::Number(n)) => {
                let num = n.as_f64().unwrap_or(0.0);
                (Some(JsonRpcRequestId::Number(num)), false)
            }
            Some(serde_json::Value::Null) => (None, false),
            Some(_) => {
                return DispatchOutcome::error(JsonRpcErrorResponse::invalid_request(
                    None,
                    "Invalid Request: id must be a string, number, or null",
                ));
            }
        };

        if let Some(v) = map.get("jsonrpc")
            && !v.is_string()
        {
            return DispatchOutcome::error(JsonRpcErrorResponse::invalid_request(
                req_id,
                "Invalid Request: jsonrpc must be string \"2.0\"",
            ));
        }

        let method_opt = match map.remove("method") {
            Some(serde_json::Value::String(s)) => Some(s),
            Some(_) => {
                return DispatchOutcome::error(JsonRpcErrorResponse::invalid_request(
                    req_id,
                    "Invalid Request: method must be a string",
                ));
            }
            None => None,
        };

        let method = extract_method(headers, method_opt.as_deref());
        let Some(method) = method else {
            tracing::debug!("Missing method in both Mcp-Method header and JSON-RPC body");
            return DispatchOutcome::error(JsonRpcErrorResponse::invalid_request(
                req_id,
                "Invalid Request: missing method",
            ));
        };

        if method.is_empty() {
            tracing::debug!("Empty method provided");
            return DispatchOutcome::error(JsonRpcErrorResponse::invalid_request(
                req_id,
                "Invalid Request: empty method",
            ));
        }

        let params_val = map.remove("params");
        let header_name = extract_header_name(headers);

        let ctx = MethodContext {
            req_id,
            is_notification,
            header_name: header_name.as_deref(),
            session_id,
            headers,
            extensions,
        };

        match method.as_str() {
            "server/discover" => self.server.dispatch_discover(ctx, params_val).await,
            "tools/list" => self.tools.dispatch_list(ctx, params_val).await,
            "tools/call" => self.tools.dispatch_call(ctx, params_val).await,
            "prompts/list" => self.prompts.dispatch_list(ctx, params_val).await,
            "prompts/get" => self.prompts.dispatch_get(ctx, params_val).await,
            unknown_method => {
                tracing::debug!(%unknown_method, "Method not found");
                if ctx.is_notification {
                    DispatchOutcome::notification()
                } else {
                    DispatchOutcome::error(JsonRpcErrorResponse::method_not_found(
                        ctx.req_id,
                        format!("Method not found: {unknown_method}"),
                    ))
                }
            }
        }
    }
}
