// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! MCP method, target name, and resource URI resolution and header-vs-body validation.

use crate::types::jsonrpc::JsonRpcErrorResponse;
use crate::types::mcp::header_mismatch_error;

/// Resolves and validates the MCP method against the `Mcp-Method` header and body method.
///
/// In strict MCP Streamable HTTP:
/// - Single requests MUST include an `Mcp-Method` HTTP header.
/// - If the body also contains a `method`, it MUST match the header.
/// - Batch request items can specify a `method` inside each JSON-RPC body object, but if `Mcp-Method`
///   header is present, any body method must match the header.
pub(crate) fn resolve_method<'a>(
    header_method: Option<&'a str>,
    body_method: Option<&'a str>,
    is_batch: bool,
) -> Result<&'a str, JsonRpcErrorResponse> {
    if !is_batch {
        let Some(header_m) = header_method else {
            return Err(header_mismatch_error(
                None,
                "Header mismatch: missing required Mcp-Method header",
            ));
        };
        let h_trim = header_m.trim_matches('/');
        if h_trim.is_empty() {
            return Err(JsonRpcErrorResponse::invalid_request(
                None,
                "Invalid Request: empty method",
            ));
        }
        if let Some(body_m) = body_method {
            let b_trim = body_m.trim_matches('/');
            if h_trim != b_trim {
                return Err(header_mismatch_error(
                    None,
                    format!(
                        "Header mismatch: Mcp-Method header value '{h_trim}' does not match body method '{b_trim}'"
                    ),
                ));
            }
        }
        Ok(h_trim)
    } else if let Some(header_m) = header_method {
        let h_trim = header_m.trim_matches('/');
        if h_trim.is_empty() {
            return Err(JsonRpcErrorResponse::invalid_request(
                None,
                "Invalid Request: empty method",
            ));
        }
        if let Some(body_m) = body_method {
            let b_trim = body_m.trim_matches('/');
            if h_trim != b_trim {
                return Err(header_mismatch_error(
                    None,
                    format!(
                        "Header mismatch: Mcp-Method header value '{h_trim}' does not match body method '{b_trim}'"
                    ),
                ));
            }
        }
        Ok(h_trim)
    } else if let Some(body_m) = body_method {
        let b_trim = body_m.trim_matches('/');
        if b_trim.is_empty() {
            return Err(JsonRpcErrorResponse::invalid_request(
                None,
                "Invalid Request: empty method",
            ));
        }
        Ok(b_trim)
    } else {
        Err(JsonRpcErrorResponse::invalid_request(
            None,
            "Invalid Request: missing method",
        ))
    }
}

/// Resolves and validates the target name for `tools/call` or `prompts/get`.
///
/// In strict MCP Streamable HTTP:
/// - Single requests MUST include an `Mcp-Name` HTTP header.
/// - If the body also contains a `name` parameter, it MUST match the header.
/// - Batch request items can specify a `name` inside the body parameters if `Mcp-Name` header is not present.
pub(crate) fn resolve_required_name<'a>(
    header_name: Option<&'a str>,
    body_name: Option<&'a str>,
    is_batch: bool,
    target_kind: &'static str,
) -> Result<&'a str, JsonRpcErrorResponse> {
    if !is_batch {
        let Some(header_n) = header_name else {
            return Err(header_mismatch_error(
                None,
                format!("Header mismatch: missing required Mcp-Name header for {target_kind}"),
            ));
        };
        let h_trim = header_n.trim_matches('/');
        if h_trim.is_empty() {
            return Err(JsonRpcErrorResponse::invalid_params(
                None,
                format!("Invalid params: empty {target_kind}"),
            ));
        }
        if let Some(body_n) = body_name {
            let b_trim = body_n.trim_matches('/');
            if h_trim != b_trim {
                return Err(header_mismatch_error(
                    None,
                    format!(
                        "Header mismatch: Mcp-Name header value '{h_trim}' does not match body {target_kind} '{b_trim}'"
                    ),
                ));
            }
        }
        Ok(h_trim)
    } else if let Some(header_n) = header_name {
        let h_trim = header_n.trim_matches('/');
        if h_trim.is_empty() {
            return Err(JsonRpcErrorResponse::invalid_params(
                None,
                format!("Invalid params: empty {target_kind}"),
            ));
        }
        if let Some(body_n) = body_name {
            let b_trim = body_n.trim_matches('/');
            if h_trim != b_trim {
                return Err(header_mismatch_error(
                    None,
                    format!(
                        "Header mismatch: Mcp-Name header value '{h_trim}' does not match body {target_kind} '{b_trim}'"
                    ),
                ));
            }
        }
        Ok(h_trim)
    } else if let Some(body_n) = body_name {
        let b_trim = body_n.trim_matches('/');
        if b_trim.is_empty() {
            return Err(JsonRpcErrorResponse::invalid_params(
                None,
                format!("Invalid params: empty {target_kind}"),
            ));
        }
        Ok(b_trim)
    } else {
        Err(JsonRpcErrorResponse::invalid_params(
            None,
            format!("Invalid params: missing {target_kind}"),
        ))
    }
}

pub(crate) use resolve_required_name as resolve_prompt_name;
pub(crate) use resolve_required_name as resolve_tool_name;

/// Resolves and validates the resource URI for `resources/read`.
///
/// In strict MCP Streamable HTTP:
/// - Single requests MUST include an `Mcp-Uri` (or `Mcp-Name`) HTTP header.
/// - If the body also contains a `uri` parameter, it MUST match the header.
/// - Batch request items can specify a `uri` inside body parameters if header is not present.
pub(crate) fn resolve_required_uri<'a>(
    header_uri: Option<&'a str>,
    body_uri: Option<&'a str>,
    is_batch: bool,
) -> Result<&'a str, JsonRpcErrorResponse> {
    if !is_batch {
        let Some(header_u) = header_uri else {
            return Err(header_mismatch_error(
                None,
                "Header mismatch: missing required Mcp-Uri header for resources/read",
            ));
        };
        let h_trim = header_u.trim();
        if h_trim.is_empty() {
            return Err(JsonRpcErrorResponse::invalid_params(
                None,
                "Invalid params: empty resource uri",
            ));
        }
        if let Some(body_u) = body_uri {
            let b_trim = body_u.trim();
            if h_trim != b_trim {
                return Err(header_mismatch_error(
                    None,
                    format!(
                        "Header mismatch: Mcp-Uri header value '{h_trim}' does not match body resource uri '{b_trim}'"
                    ),
                ));
            }
        }
        Ok(h_trim)
    } else if let Some(header_u) = header_uri {
        let h_trim = header_u.trim();
        if h_trim.is_empty() {
            return Err(JsonRpcErrorResponse::invalid_params(
                None,
                "Invalid params: empty resource uri",
            ));
        }
        if let Some(body_u) = body_uri {
            let b_trim = body_u.trim();
            if h_trim != b_trim {
                return Err(header_mismatch_error(
                    None,
                    format!(
                        "Header mismatch: Mcp-Uri header value '{h_trim}' does not match body resource uri '{b_trim}'"
                    ),
                ));
            }
        }
        Ok(h_trim)
    } else if let Some(body_u) = body_uri {
        let b_trim = body_u.trim();
        if b_trim.is_empty() {
            return Err(JsonRpcErrorResponse::invalid_params(
                None,
                "Invalid params: empty resource uri",
            ));
        }
        Ok(b_trim)
    } else {
        Err(JsonRpcErrorResponse::invalid_params(
            None,
            "Invalid params: missing resource uri",
        ))
    }
}

pub(crate) use resolve_required_uri as resolve_resource_uri;

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests resolving method against header and body.
    #[test]
    fn test_resolve_method() {
        // Single request (is_batch: false)
        assert_eq!(
            resolve_method(Some("server/discover"), Some("server/discover"), false).unwrap(),
            "server/discover"
        );
        assert_eq!(
            resolve_method(Some("/server/discover/"), Some("server/discover"), false).unwrap(),
            "server/discover"
        );
        assert_eq!(
            resolve_method(Some("server/discover"), None, false).unwrap(),
            "server/discover"
        );

        // Missing Mcp-Method header on single request -> HeaderMismatch (-32020)
        let err = resolve_method(None, Some("server/discover"), false).unwrap_err();
        assert_eq!(err.error.code.code(), crate::types::mcp::HEADER_MISMATCH);

        // Header and body method mismatch -> HeaderMismatch (-32020)
        let err = resolve_method(Some("tools/call"), Some("server/discover"), false).unwrap_err();
        assert_eq!(err.error.code.code(), crate::types::mcp::HEADER_MISMATCH);

        // Batch request (is_batch: true)
        assert_eq!(
            resolve_method(None, Some("tools/list"), true).unwrap(),
            "tools/list"
        );
        assert_eq!(
            resolve_method(Some("tools/list"), Some("tools/list"), true).unwrap(),
            "tools/list"
        );
        let err = resolve_method(Some("tools/list"), Some("prompts/list"), true).unwrap_err();
        assert_eq!(err.error.code.code(), crate::types::mcp::HEADER_MISMATCH);
    }

    /// Tests resolving tool or prompt name.
    #[test]
    fn test_resolve_tool_name() {
        // Single request (is_batch: false)
        assert_eq!(
            resolve_tool_name(Some("/my_tool/"), Some("my_tool"), false, "tool name").unwrap(),
            "my_tool"
        );
        assert_eq!(
            resolve_tool_name(Some("my_tool"), None, false, "tool name").unwrap(),
            "my_tool"
        );

        // Missing Mcp-Name header on single request -> HeaderMismatch (-32020)
        let err = resolve_tool_name(None, Some("my_tool"), false, "tool name").unwrap_err();
        assert_eq!(err.error.code.code(), crate::types::mcp::HEADER_MISMATCH);

        // Header and body tool name mismatch -> HeaderMismatch (-32020)
        let err =
            resolve_tool_name(Some("tool_a"), Some("tool_b"), false, "tool name").unwrap_err();
        assert_eq!(err.error.code.code(), crate::types::mcp::HEADER_MISMATCH);

        // Batch request (is_batch: true)
        assert_eq!(
            resolve_tool_name(None, Some("tool_b"), true, "tool name").unwrap(),
            "tool_b"
        );
    }

    /// Tests resolving resource URI.
    #[test]
    fn test_resolve_resource_uri() {
        // Single request (is_batch: false)
        assert_eq!(
            resolve_resource_uri(Some("file:///doc.txt"), Some("file:///doc.txt"), false).unwrap(),
            "file:///doc.txt"
        );
        assert_eq!(
            resolve_resource_uri(Some("file:///doc.txt"), None, false).unwrap(),
            "file:///doc.txt"
        );

        // Missing Mcp-Uri header on single request -> HeaderMismatch (-32020)
        let err = resolve_resource_uri(None, Some("file:///doc.txt"), false).unwrap_err();
        assert_eq!(err.error.code.code(), crate::types::mcp::HEADER_MISMATCH);

        // Header and body URI mismatch -> HeaderMismatch (-32020)
        let err =
            resolve_resource_uri(Some("file:///a.txt"), Some("file:///b.txt"), false).unwrap_err();
        assert_eq!(err.error.code.code(), crate::types::mcp::HEADER_MISMATCH);

        // Batch request (is_batch: true)
        assert_eq!(
            resolve_resource_uri(None, Some("file:///b.txt"), true).unwrap(),
            "file:///b.txt"
        );
    }
}
