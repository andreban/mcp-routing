// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Utility Functions
//!
//! Internal helper functions for HTTP header extraction, MIME type negotiation,
//! URI template matching, and method / parameter resolution.

use http::HeaderMap;

use crate::extract::SessionId;
use crate::types::jsonrpc::JsonRpcErrorResponse;
use crate::types::mcp::header_mismatch_error;

/// Validates whether the `Content-Type` header specifies `application/json`.
///
/// This check is case-insensitive and ignores optional parameters such as `charset=utf-8`.
pub(crate) fn is_json_content_type(headers: &HeaderMap) -> bool {
    let Some(content_type) = headers.get(http::header::CONTENT_TYPE) else {
        return false;
    };
    let Ok(ct_str) = content_type.to_str() else {
        return false;
    };
    let media_type = ct_str.split(';').next().unwrap_or("").trim();
    media_type.eq_ignore_ascii_case("application/json")
}

/// Extracts the tool or prompt name from the `Mcp-Name` HTTP header, trimming leading and trailing slashes.
pub(crate) fn extract_header_name(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("Mcp-Name")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim_matches('/'))
}

/// Extracts the resource URI from HTTP headers (`Mcp-Uri` or `Mcp-Name`), trimming whitespace.
pub(crate) fn extract_header_uri(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("Mcp-Uri")
        .or_else(|| headers.get("Mcp-Name"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim())
}

/// Extracts the MCP method from the `Mcp-Method` HTTP header, trimming leading and trailing slashes.
pub(crate) fn extract_header_method(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("Mcp-Method")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim_matches('/'))
}

/// Extracts the session ID from the `Mcp-Session-Id` HTTP header, trimming whitespace.
pub(crate) fn extract_session_id(headers: &HeaderMap) -> Option<SessionId> {
    headers
        .get("Mcp-Session-Id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(SessionId::new)
}

/// Extracts the MCP protocol version from the `MCP-Protocol-Version` HTTP header.
pub(crate) fn extract_protocol_version(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("MCP-Protocol-Version")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim())
}

/// Extracts the `protocolVersion` specified in the request body metadata (`params._meta` or `_meta`), if present.
pub(crate) fn extract_body_protocol_version(
    map: &serde_json::Map<String, serde_json::Value>,
) -> Option<&str> {
    // 1. Check params._meta["io.modelcontextprotocol/protocolVersion"] or params.meta.protocolVersion
    if let Some(serde_json::Value::Object(params)) = map.get("params")
        && let Some(serde_json::Value::Object(meta)) =
            params.get("_meta").or_else(|| params.get("meta"))
        && let Some(serde_json::Value::String(ver)) = meta
            .get("io.modelcontextprotocol/protocolVersion")
            .or_else(|| meta.get("protocolVersion"))
    {
        return Some(ver.as_str());
    }
    // 2. Check top-level _meta["io.modelcontextprotocol/protocolVersion"] or _meta.protocolVersion
    if let Some(serde_json::Value::Object(meta)) = map.get("_meta").or_else(|| map.get("meta"))
        && let Some(serde_json::Value::String(ver)) = meta
            .get("io.modelcontextprotocol/protocolVersion")
            .or_else(|| meta.get("protocolVersion"))
    {
        return Some(ver.as_str());
    }
    None
}

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

pub(crate) use resolve_required_name as resolve_tool_name;
pub(crate) use resolve_required_name as resolve_prompt_name;

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

/// Matches a URI against an RFC 6570 URI template (e.g., `file:///{path}`, `postgres://{schema}/{table}`).
pub(crate) fn match_uri_template(template: &str, uri: &str) -> bool {
    if template == uri {
        return true;
    }

    let mut t_iter = template;
    let mut u_iter = uri;

    while let Some(start_bracket) = t_iter.find('{') {
        let prefix = &t_iter[..start_bracket];
        if !u_iter.starts_with(prefix) {
            return false;
        }
        u_iter = &u_iter[prefix.len()..];

        let Some(end_bracket) = t_iter[start_bracket..].find('}') else {
            return false;
        };
        let end_idx = start_bracket + end_bracket;
        let var_expr = &t_iter[start_bracket + 1..end_idx];
        t_iter = &t_iter[end_idx + 1..];

        let is_reserved = var_expr.starts_with('+');

        if let Some(next_start) = t_iter.find('{') {
            let next_literal = &t_iter[..next_start];
            if next_literal.is_empty() {
                // Adjacent templates without literal separator
                return false;
            }
            let Some(match_pos) = u_iter.find(next_literal) else {
                return false;
            };
            let matched_var = &u_iter[..match_pos];
            if matched_var.is_empty() {
                return false;
            }
            if !is_reserved && matched_var.contains('/') {
                return false;
            }
            u_iter = &u_iter[match_pos..];
        } else {
            // Trailing variable pattern
            let suffix = t_iter;
            if !u_iter.ends_with(suffix) {
                return false;
            }
            let matched_var_len = u_iter.len() - suffix.len();
            let matched_var = &u_iter[..matched_var_len];
            if matched_var.is_empty() {
                return false;
            }
            if !is_reserved && matched_var.contains('/') && !var_expr.starts_with('/') {
                // Check if variable expansion allows slashes
                // When at the end of template without +, check if path segments are expected
                // In RFC 6570, {path} or {+path} can match remaining URI if no other literal
                return true;
            }
            return true;
        }
    }

    t_iter == u_iter
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests validation of `Content-Type: application/json` headers.
    #[test]
    fn test_is_json_content_type() {
        let mut headers = HeaderMap::new();
        assert!(!is_json_content_type(&headers));

        headers.insert("Content-Type", "application/json".parse().unwrap());
        assert!(is_json_content_type(&headers));

        headers.insert(
            "Content-Type",
            "application/json; charset=utf-8".parse().unwrap(),
        );
        assert!(is_json_content_type(&headers));

        headers.insert(
            "Content-Type",
            "APPLICATION/JSON; CHARSET=UTF-8".parse().unwrap(),
        );
        assert!(is_json_content_type(&headers));

        headers.insert("Content-Type", "text/plain".parse().unwrap());
        assert!(!is_json_content_type(&headers));

        headers.insert("Content-Type", "application/xml".parse().unwrap());
        assert!(!is_json_content_type(&headers));

        headers.insert("Content-Type", "".parse().unwrap());
        assert!(!is_json_content_type(&headers));
    }

    /// Tests extracting the `Mcp-Name` header and trimming slashes.
    #[test]
    fn test_extract_header_name() {
        let mut headers = HeaderMap::new();
        assert_eq!(extract_header_name(&headers), None);

        headers.insert("Mcp-Name", "/my_tool/".parse().unwrap());
        assert_eq!(extract_header_name(&headers), Some("my_tool"));

        headers.insert("Mcp-Name", "///".parse().unwrap());
        assert_eq!(extract_header_name(&headers), Some(""));
    }

    /// Tests extracting the `Mcp-Uri` and `Mcp-Name` headers for resources.
    #[test]
    fn test_extract_header_uri() {
        let mut headers = HeaderMap::new();
        assert_eq!(extract_header_uri(&headers), None);

        headers.insert("Mcp-Uri", "file:///app/config.json".parse().unwrap());
        assert_eq!(
            extract_header_uri(&headers),
            Some("file:///app/config.json")
        );

        headers.remove("Mcp-Uri");
        headers.insert("Mcp-Name", "file:///app/config.json".parse().unwrap());
        assert_eq!(
            extract_header_uri(&headers),
            Some("file:///app/config.json")
        );
    }

    /// Tests extracting the `Mcp-Session-Id` header and trimming whitespace.
    #[test]
    fn test_extract_session_id() {
        let mut headers = HeaderMap::new();
        assert_eq!(extract_session_id(&headers), None);

        headers.insert("Mcp-Session-Id", "session-xyz".parse().unwrap());
        assert_eq!(
            extract_session_id(&headers),
            Some(SessionId::new("session-xyz"))
        );

        headers.insert("Mcp-Session-Id", "  session-with-spaces  ".parse().unwrap());
        assert_eq!(
            extract_session_id(&headers),
            Some(SessionId::new("session-with-spaces"))
        );

        headers.insert("Mcp-Session-Id", "   ".parse().unwrap());
        assert_eq!(extract_session_id(&headers), None);
    }

    /// Tests extracting the `Mcp-Method` header and trimming slashes.
    #[test]
    fn test_extract_header_method() {
        let mut headers = HeaderMap::new();
        assert_eq!(extract_header_method(&headers), None);

        headers.insert("Mcp-Method", "/tools/call/".parse().unwrap());
        assert_eq!(extract_header_method(&headers), Some("tools/call"));

        headers.insert("Mcp-Method", "///".parse().unwrap());
        assert_eq!(extract_header_method(&headers), Some(""));
    }

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

    /// Tests URI template matching.
    #[test]
    fn test_match_uri_template() {
        assert!(match_uri_template("file:///{path}", "file:///src/main.rs"));
        assert!(match_uri_template("file:///{+path}", "file:///a/b/c/d.txt"));
        assert!(match_uri_template(
            "postgres://{schema}/{table}",
            "postgres://public/users"
        ));
        assert!(!match_uri_template(
            "postgres://{schema}/{table}",
            "mysql://public/users"
        ));
        assert!(match_uri_template("memo://all", "memo://all"));
        assert!(!match_uri_template("memo://all", "memo://other"));
    }

    #[test]
    fn test_extract_protocol_version() {
        let mut headers = HeaderMap::new();
        assert_eq!(extract_protocol_version(&headers), None);

        headers.insert("MCP-Protocol-Version", "2026-07-28".parse().unwrap());
        assert_eq!(extract_protocol_version(&headers), Some("2026-07-28"));

        // Case insensitivity
        headers.remove("MCP-Protocol-Version");
        headers.insert("mcp-protocol-version", "2026-07-28".parse().unwrap());
        assert_eq!(extract_protocol_version(&headers), Some("2026-07-28"));
    }

    #[test]
    fn test_extract_body_protocol_version() {
        let body_with_params_meta: serde_json::Map<String, serde_json::Value> =
            serde_json::from_value(serde_json::json!({
                "params": {
                    "_meta": {
                        "io.modelcontextprotocol/protocolVersion": "2026-07-28"
                    }
                }
            }))
            .unwrap();
        assert_eq!(
            extract_body_protocol_version(&body_with_params_meta),
            Some("2026-07-28")
        );

        let body_with_top_level_meta: serde_json::Map<String, serde_json::Value> =
            serde_json::from_value(serde_json::json!({
                "_meta": {
                    "io.modelcontextprotocol/protocolVersion": "2026-07-28"
                }
            }))
            .unwrap();
        assert_eq!(
            extract_body_protocol_version(&body_with_top_level_meta),
            Some("2026-07-28")
        );

        let body_without_meta: serde_json::Map<String, serde_json::Value> =
            serde_json::from_value(serde_json::json!({
                "params": {}
            }))
            .unwrap();
        assert_eq!(extract_body_protocol_version(&body_without_meta), None);
    }
}
