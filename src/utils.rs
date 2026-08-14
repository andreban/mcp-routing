// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Utility Functions
//!
//! Internal helper functions for HTTP header extraction, MIME type negotiation,
//! and method / tool parameter resolution.

use http::HeaderMap;

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

/// Extracts the tool name from the `Mcp-Name` HTTP header, trimming leading and trailing slashes.
pub(crate) fn extract_header_name(headers: &HeaderMap) -> Option<String> {
    headers
        .get("Mcp-Name")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim_matches('/').to_string())
}

/// Extracts the MCP method to dispatch, prioritizing `Mcp-Method` header and falling back to the body method.
///
/// Leading and trailing slashes are trimmed for normalization.
pub(crate) fn extract_method(headers: &HeaderMap, body_method: Option<&str>) -> Option<String> {
    // 1. Prefer Mcp-Method HTTP header
    if let Some(header_method) = headers
        .get("Mcp-Method")
        .and_then(|v| v.to_str().ok())
    {
        return Some(header_method.trim_matches('/').to_string());
    }

    // 2. Fall back to JSON-RPC request body method
    body_method.map(|m| m.trim_matches('/').to_string())
}

/// Resolves the tool name for `tools/call`, prioritizing the header over body parameters.
pub(crate) fn resolve_tool_name<'a>(
    header_name: Option<&'a str>,
    params_name: Option<&'a str>,
) -> Option<&'a str> {
    if let Some(h) = header_name {
        return Some(h.trim_matches('/'));
    }
    params_name.map(|n| n.trim_matches('/'))
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
}
