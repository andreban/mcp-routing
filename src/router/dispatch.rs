// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use crate::router::{DispatchOutcome, McpRouterInner, MethodContext};
use crate::types::jsonrpc::{JsonRpcErrorResponse, JsonRpcRequestId};
use crate::types::mcp::{
    ClientCapabilities, header_mismatch_error, unsupported_protocol_version_error,
};
use crate::utils::{
    extract_body_protocol_version, extract_header_method, extract_header_name,
    extract_protocol_version, resolve_method, validate_required_request_meta,
};

impl McpRouterInner {
    /// Dispatches a JSON-RPC object request or notification to the appropriate capability handler.
    pub(crate) async fn dispatch_object(
        &self,
        mut map: serde_json::Map<String, serde_json::Value>,
        headers: &http::HeaderMap,
        extensions: Arc<http::Extensions>,
    ) -> DispatchOutcome {
        let (req_id, is_notification) = match map.remove("id") {
            None => (None, true),
            Some(serde_json::Value::String(s)) => (Some(JsonRpcRequestId::String(s)), false),
            Some(serde_json::Value::Number(n)) => match n.as_i64() {
                Some(num) => (Some(JsonRpcRequestId::Number(num)), false),
                None => {
                    return DispatchOutcome::error(JsonRpcErrorResponse::invalid_request(
                        None,
                        "Invalid Request: numeric id must be an integer",
                    ));
                }
            },
            Some(_) => {
                return DispatchOutcome::error(JsonRpcErrorResponse::invalid_request(
                    None,
                    "Invalid Request: id must be a string or an integer",
                ));
            }
        };

        if self.server.validate_protocol_version {
            let Some(header_ver) = extract_protocol_version(headers) else {
                tracing::debug!("Missing required MCP-Protocol-Version header");
                return DispatchOutcome::error(header_mismatch_error(
                    req_id,
                    "Header mismatch: missing required MCP-Protocol-Version header",
                ));
            };
            if !self
                .server
                .supported_versions
                .iter()
                .any(|v| v == header_ver)
            {
                tracing::debug!(%header_ver, "Unsupported MCP-Protocol-Version header");
                return DispatchOutcome::error(unsupported_protocol_version_error(
                    req_id,
                    format!("Unsupported protocol version '{header_ver}'"),
                    self.server.supported_versions.clone(),
                    header_ver,
                ));
            }
            if let Some(body_ver) = extract_body_protocol_version(&map)
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
        }

        if map.get("jsonrpc").and_then(|v| v.as_str()) != Some("2.0") {
            return DispatchOutcome::error(JsonRpcErrorResponse::invalid_request(
                req_id,
                "Invalid Request: jsonrpc must be \"2.0\"",
            ));
        }

        let body_method = match map.remove("method") {
            Some(serde_json::Value::String(s)) => s,
            Some(_) => {
                return DispatchOutcome::error(JsonRpcErrorResponse::invalid_request(
                    req_id,
                    "Invalid Request: method must be a string",
                ));
            }
            None => {
                return DispatchOutcome::error(JsonRpcErrorResponse::invalid_request(
                    req_id,
                    "Invalid Request: missing method",
                ));
            }
        };

        let header_method = extract_header_method(headers);
        let method = match resolve_method(header_method, Some(&body_method)) {
            Ok(m) => m,
            Err(mut err) => {
                err.id = req_id;
                return DispatchOutcome::error(err);
            }
        };

        if is_notification {
            if method.starts_with("notifications/") {
                return DispatchOutcome::notification();
            }
            tracing::debug!(%method, "Rejected request method sent without an id");
            return DispatchOutcome::error(JsonRpcErrorResponse::invalid_request(
                None,
                format!("Invalid Request: '{method}' is a request and must include an id"),
            ));
        }

        let params_val = map.remove("params");
        let is_retry = params_val
            .as_ref()
            .and_then(|p| p.as_object())
            .is_some_and(|p| p.contains_key("requestState") || p.contains_key("inputResponses"));

        if let Err(reason) = validate_required_request_meta(params_val.as_ref()) {
            tracing::debug!(%reason, "Rejected request with malformed _meta");
            let mut outcome = DispatchOutcome::error(JsonRpcErrorResponse::invalid_params(
                req_id,
                format!("Invalid params: {reason}"),
            ));
            outcome.status_code = http::StatusCode::BAD_REQUEST;
            return outcome;
        }

        let client_capabilities: Option<ClientCapabilities> = params_val
            .as_ref()
            .and_then(|p| p.get("_meta"))
            .and_then(|m| m.get("io.modelcontextprotocol/clientCapabilities"))
            .and_then(|c| serde_json::from_value(c.clone()).ok());
        let response_id = req_id.clone();

        let header_name = match extract_header_name(headers) {
            Ok(name) => name,
            Err(reason) => {
                return DispatchOutcome::error(header_mismatch_error(
                    req_id,
                    format!("Header mismatch: Mcp-Name header is invalid: {reason}"),
                ));
            }
        };

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
            header_name,
            headers,
            extensions,
        };

        let mut outcome = match method {
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
            "subscriptions/listen" => {
                let tools_list_changed = self
                    .server
                    .capabilities
                    .tools
                    .as_ref()
                    .and_then(|t| t.list_changed)
                    .unwrap_or(false);
                let prompts_list_changed = self
                    .server
                    .capabilities
                    .prompts
                    .as_ref()
                    .and_then(|p| p.list_changed)
                    .unwrap_or(false);
                let resources_list_changed = self
                    .server
                    .capabilities
                    .resources
                    .as_ref()
                    .and_then(|r| r.list_changed)
                    .unwrap_or(false);
                let resources_subscribe = self
                    .server
                    .capabilities
                    .resources
                    .as_ref()
                    .and_then(|r| r.subscribe)
                    .unwrap_or(false);
                let known_resources: Vec<String> = if resources_subscribe {
                    self.resources
                        .resources
                        .iter()
                        .map(|r| r.uri.clone())
                        .collect()
                } else {
                    Vec::new()
                };
                self.subscriptions
                    .dispatch_listen(
                        ctx,
                        params_val,
                        tools_list_changed,
                        prompts_list_changed,
                        resources_list_changed,
                        &known_resources,
                    )
                    .await
            }
            unknown_method => {
                tracing::debug!(%unknown_method, "Method not found");
                DispatchOutcome::error(JsonRpcErrorResponse::method_not_found(
                    ctx.req_id,
                    format!("Method not found: {unknown_method}"),
                ))
            }
        };
        outcome.require_client_capabilities(response_id, client_capabilities.as_ref());
        outcome.add_server_info(&self.server.server_info);
        outcome.apply_cache_policy(is_retry);
        outcome
    }
}
