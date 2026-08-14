// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Utility Functions
//!
//! Internal helper functions for HTTP header extraction, MIME type negotiation,
//! URI template matching, and method / parameter resolution.

use http::HeaderMap;

use crate::extract::SessionId;

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
pub(crate) fn extract_header_name(headers: &HeaderMap) -> Option<String> {
    headers
        .get("Mcp-Name")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim_matches('/').to_string())
}

/// Extracts the resource URI from HTTP headers (`Mcp-Uri` or `Mcp-Name`), trimming whitespace.
pub(crate) fn extract_header_uri(headers: &HeaderMap) -> Option<String> {
    headers
        .get("Mcp-Uri")
        .or_else(|| headers.get("Mcp-Name"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
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

/// Extracts the MCP method to dispatch, prioritizing `Mcp-Method` header and falling back to the body method.
///
/// Leading and trailing slashes are trimmed for normalization.
pub(crate) fn extract_method(headers: &HeaderMap, body_method: Option<&str>) -> Option<String> {
    // 1. Prefer Mcp-Method HTTP header
    if let Some(header_method) = headers.get("Mcp-Method").and_then(|v| v.to_str().ok()) {
        return Some(header_method.trim_matches('/').to_string());
    }

    // 2. Fall back to JSON-RPC request body method
    body_method.map(|m| m.trim_matches('/').to_string())
}

/// Resolves the target name for `tools/call` or `prompts/get`, prioritizing the header over body parameters.
pub(crate) fn resolve_name<'a>(
    header_name: Option<&'a str>,
    params_name: Option<&'a str>,
) -> Option<&'a str> {
    if let Some(h) = header_name {
        return Some(h.trim_matches('/'));
    }
    params_name.map(|n| n.trim_matches('/'))
}

pub(crate) use resolve_name as resolve_tool_name;
pub(crate) use resolve_name as resolve_prompt_name;

/// Resolves the resource URI for `resources/read`, prioritizing the header over body parameters.
pub(crate) fn resolve_resource_uri<'a>(
    header_uri: Option<&'a str>,
    params_uri: Option<&'a str>,
) -> Option<&'a str> {
    if let Some(h) = header_uri {
        return Some(h.trim());
    }
    params_uri.map(|u| u.trim())
}

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
        assert_eq!(extract_header_name(&headers), Some("my_tool".to_string()));

        headers.insert("Mcp-Name", "///".parse().unwrap());
        assert_eq!(extract_header_name(&headers), Some("".to_string()));
    }

    /// Tests extracting the `Mcp-Uri` and `Mcp-Name` headers for resources.
    #[test]
    fn test_extract_header_uri() {
        let mut headers = HeaderMap::new();
        assert_eq!(extract_header_uri(&headers), None);

        headers.insert("Mcp-Uri", "file:///app/config.json".parse().unwrap());
        assert_eq!(
            extract_header_uri(&headers),
            Some("file:///app/config.json".to_string())
        );

        headers.remove("Mcp-Uri");
        headers.insert("Mcp-Name", "file:///app/config.json".parse().unwrap());
        assert_eq!(
            extract_header_uri(&headers),
            Some("file:///app/config.json".to_string())
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

    /// Tests extracting the method from `Mcp-Method` header or JSON body.
    #[test]
    fn test_extract_method() {
        let mut headers = HeaderMap::new();

        // Preference: header over body
        headers.insert("Mcp-Method", "server/discover".parse().unwrap());
        assert_eq!(
            extract_method(&headers, Some("tools/list")),
            Some("server/discover".to_string())
        );

        // Fallback to body when header is absent
        headers.remove("Mcp-Method");
        assert_eq!(
            extract_method(&headers, Some("tools/list")),
            Some("tools/list".to_string())
        );

        // Slash normalization
        assert_eq!(
            extract_method(&headers, Some("/tools/call/")),
            Some("tools/call".to_string())
        );

        // Invalid / absent method
        assert_eq!(extract_method(&headers, None), None);
    }

    /// Tests resolving tool name between header preference and body parameter fallback.
    #[test]
    fn test_resolve_tool_name() {
        assert_eq!(
            resolve_tool_name(Some("/header_tool/"), Some("body_tool")),
            Some("header_tool")
        );
        assert_eq!(
            resolve_tool_name(None, Some("/body_tool/")),
            Some("body_tool")
        );
        assert_eq!(resolve_tool_name(Some(""), Some("body_tool")), Some(""));
        assert_eq!(resolve_tool_name(None, None), None);
        assert_eq!(resolve_tool_name(Some("///"), Some("///")), Some(""));
    }

    /// Tests resolving resource URI between header preference and body parameter fallback.
    #[test]
    fn test_resolve_resource_uri() {
        assert_eq!(
            resolve_resource_uri(Some("file:///header.txt"), Some("file:///body.txt")),
            Some("file:///header.txt")
        );
        assert_eq!(
            resolve_resource_uri(None, Some("file:///body.txt")),
            Some("file:///body.txt")
        );
        assert_eq!(resolve_resource_uri(None, None), None);
    }

    /// Tests URI template matching.
    #[test]
    fn test_match_uri_template() {
        assert!(match_uri_template(
            "file:///{path}",
            "file:///src/main.rs"
        ));
        assert!(match_uri_template(
            "file:///{+path}",
            "file:///a/b/c/d.txt"
        ));
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
}
