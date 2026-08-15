// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use crate::extract::SessionId;
use crate::router::{DispatchOutcome, McpRouterInner, MethodContext};
use crate::types::jsonrpc::{JsonRpcErrorResponse, JsonRpcRequestId};
use crate::types::mcp::header_mismatch_error;
use crate::utils::{
    extract_body_protocol_version, extract_header_method, extract_header_name,
    extract_protocol_version, resolve_method,
};

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
                    .dispatch_object(map, headers, session_id, extensions, true)
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
        is_batch: bool,
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

        if self.server.validate_protocol_version
            && let Some(header_ver) = extract_protocol_version(headers)
            && let Some(body_ver) = extract_body_protocol_version(&map)
            && body_ver != header_ver
        {
            tracing::debug!(
                %header_ver,
                %body_ver,
                "MCP-Protocol-Version header value does not match body metadata"
            );
            return DispatchOutcome::error(header_mismatch_error(
                req_id,
                format!(
                    "Header mismatch: MCP-Protocol-Version header value '{header_ver}' does not match body value '{body_ver}'"
                ),
            ));
        }

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

        let header_method = extract_header_method(headers);
        let method = match resolve_method(header_method, method_opt.as_deref(), is_batch) {
            Ok(m) => m,
            Err(mut err) => {
                err.id = req_id;
                return DispatchOutcome::error(err);
            }
        };

        let params_val = map.remove("params");
        let header_name = extract_header_name(headers);

        let mut extensions = extensions;
        if let Some(ref pv) = params_val
            && let Some(param_obj) = pv.as_object()
        {
            let mut ext = (*extensions).clone();
            let mut modified = false;
            if let Some(rs) = param_obj.get("requestState").and_then(|v| v.as_str()) {
                ext.insert(crate::extract::RequestState::new(rs));
                modified = true;
            }
            if let Some(ir) = param_obj.get("inputResponses")
                && let Ok(responses) = serde_json::from_value::<
                    std::collections::HashMap<String, crate::types::mcp::InputResponse>,
                >(ir.clone())
            {
                ext.insert(crate::extract::InputResponses::new(responses));
                modified = true;
            }
            if modified {
                extensions = Arc::new(ext);
            }
        }

        let ctx = MethodContext {
            req_id,
            is_notification,
            is_batch,
            header_name,
            session_id,
            headers,
            extensions,
        };

        match method {
            "server/discover" => self.server.dispatch_discover(ctx, params_val).await,
            "tools/list" => self.tools.dispatch_list(ctx, params_val).await,
            "tools/call" => self.tools.dispatch_call(ctx, params_val).await,
            "prompts/list" => self.prompts.dispatch_list(ctx, params_val).await,
            "prompts/get" => self.prompts.dispatch_get(ctx, params_val).await,
            "resources/list" => self.resources.dispatch_list(ctx, params_val).await,
            "resources/read" => self.resources.dispatch_read(ctx, params_val).await,
            "resources/templates/list" => {
                self.resources
                    .dispatch_templates_list(ctx, params_val)
                    .await
            }
            "completion/complete" => self.completion.dispatch_complete(ctx, params_val).await,
            "logging/setLevel" => self.logging.dispatch_set_level(ctx, params_val).await,
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
