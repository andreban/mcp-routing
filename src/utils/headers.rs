// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! HTTP header extraction and MIME type negotiation utilities.

use std::borrow::Cow;

use http::HeaderMap;

use crate::utils::sentinel::decode_sentinel_header;

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

/// Trims HTTP optional whitespace (spaces and horizontal tabs) around a header field value.
fn trim_ows(value: &str) -> &str {
    value.trim_matches([' ', '\t'])
}

/// Extracts the `Mcp-Name` HTTP header (the tool or prompt name, or the resource URI), decoding
/// any Base64 sentinel value (`=?base64?...?=`).
///
/// The value is otherwise used verbatim: header values are case-sensitive and must exactly match
/// the corresponding body value.
pub(crate) fn extract_header_name(headers: &HeaderMap) -> Option<Cow<'_, str>> {
    headers
        .get("Mcp-Name")
        .and_then(|v| v.to_str().ok())
        .map(|s| decode_sentinel_header(trim_ows(s)))
}

/// Extracts the MCP method from the `Mcp-Method` HTTP header.
pub(crate) fn extract_header_method(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("Mcp-Method")
        .and_then(|v| v.to_str().ok())
        .map(trim_ows)
}

/// Extracts the MCP protocol version from the `MCP-Protocol-Version` HTTP header.
pub(crate) fn extract_protocol_version(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("MCP-Protocol-Version")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim())
}

/// Extracts the origin from the `Origin` HTTP header, trimming whitespace.
pub(crate) fn extract_origin(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(http::header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
}

/// Validates whether the given origin is permitted according to the allowed origins list.
///
/// Wildcard `"*"` matches any origin.
/// Schemes and domain names are compared case-insensitively, ignoring trailing slashes.
pub(crate) fn is_origin_allowed(origin: &str, allowed_origins: &[String]) -> bool {
    let normalized_origin = origin.trim().trim_end_matches('/');
    allowed_origins.iter().any(|allowed| {
        let allowed_trimmed = allowed.trim().trim_end_matches('/');
        allowed_trimmed == "*" || allowed_trimmed.eq_ignore_ascii_case(normalized_origin)
    })
}

/// Returns `true` if the origin's host is a loopback address (`localhost`, `127.0.0.1`, or `[::1]`).
///
/// The scheme and port are not considered.
pub(crate) fn is_loopback_origin(origin: &str) -> bool {
    let Some((_scheme, authority)) = origin.trim().trim_end_matches('/').split_once("://") else {
        return false;
    };
    let host = if authority.starts_with('[') {
        authority
            .split_once(']')
            .map_or(authority, |(h, _)| h)
            .trim_start_matches('[')
    } else {
        authority.split(':').next().unwrap_or(authority)
    };
    host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1" || host == "::1"
}

/// Validates whether the `Origin` header in the request is permitted.
///
/// If no `Origin` header is present (such as with non-browser clients), returns `true`.
/// If the `Origin` header is present, it must be valid and match at least one allowed origin.
/// When no allowed origins are configured, only loopback origins are permitted, protecting
/// local servers against DNS rebinding by default.
pub(crate) fn is_origin_header_allowed(
    headers: &HeaderMap,
    allowed_origins: Option<&[String]>,
) -> bool {
    if !headers.contains_key(http::header::ORIGIN) {
        return true;
    }
    let Some(origin) = extract_origin(headers) else {
        return false;
    };
    match allowed_origins {
        Some(allowed) => is_origin_allowed(origin, allowed),
        None => is_loopback_origin(origin),
    }
}

/// Extracts `params._meta["io.modelcontextprotocol/protocolVersion"]` from the request body, if present.
pub(crate) fn extract_body_protocol_version(
    map: &serde_json::Map<String, serde_json::Value>,
) -> Option<&str> {
    map.get("params")?
        .get("_meta")?
        .get("io.modelcontextprotocol/protocolVersion")?
        .as_str()
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

    /// Tests that the `Mcp-Name` header is used verbatim apart from HTTP whitespace and sentinel decoding.
    #[test]
    fn test_extract_header_name() {
        let mut headers = HeaderMap::new();
        assert_eq!(extract_header_name(&headers), None);

        headers.insert("Mcp-Name", "my_tool".parse().unwrap());
        assert_eq!(extract_header_name(&headers).as_deref(), Some("my_tool"));

        // Slashes are significant and preserved
        headers.insert("Mcp-Name", "/my_tool/".parse().unwrap());
        assert_eq!(extract_header_name(&headers).as_deref(), Some("/my_tool/"));

        // Resource URIs are carried in Mcp-Name
        headers.insert("Mcp-Name", "file:///app/config.json".parse().unwrap());
        assert_eq!(
            extract_header_name(&headers).as_deref(),
            Some("file:///app/config.json")
        );

        // Sentinel encoded value ("echo_世界" -> "ZWNob1/kuJbnlYw=")
        headers.insert("Mcp-Name", "=?base64?ZWNob1/kuJbnlYw=?=".parse().unwrap());
        assert_eq!(extract_header_name(&headers).as_deref(), Some("echo_世界"));

        // Sentinel encoded value keeps its own surrounding spaces (" padded " -> "IHBhZGRlZCA=")
        headers.insert("Mcp-Name", "=?base64?IHBhZGRlZCA=?=".parse().unwrap());
        assert_eq!(extract_header_name(&headers).as_deref(), Some(" padded "));
    }

    /// Tests that the `Mcp-Method` header is used verbatim apart from HTTP whitespace.
    #[test]
    fn test_extract_header_method() {
        let mut headers = HeaderMap::new();
        assert_eq!(extract_header_method(&headers), None);

        headers.insert("Mcp-Method", "tools/call".parse().unwrap());
        assert_eq!(extract_header_method(&headers), Some("tools/call"));

        headers.insert("Mcp-Method", "/tools/call/".parse().unwrap());
        assert_eq!(extract_header_method(&headers), Some("/tools/call/"));
    }

    /// Tests extracting the `MCP-Protocol-Version` HTTP header with case-insensitivity.
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

    /// Tests that the body protocol version is read only from `params._meta`.
    #[test]
    fn test_extract_body_protocol_version() {
        let body = |value: serde_json::Value| -> serde_json::Map<String, serde_json::Value> {
            serde_json::from_value(value).unwrap()
        };

        let in_params_meta = body(serde_json::json!({
            "params": { "_meta": { "io.modelcontextprotocol/protocolVersion": "2026-07-28" } }
        }));
        assert_eq!(
            extract_body_protocol_version(&in_params_meta),
            Some("2026-07-28")
        );

        // Non-standard locations and keys are not recognized
        for value in [
            serde_json::json!({ "_meta": { "io.modelcontextprotocol/protocolVersion": "2026-07-28" } }),
            serde_json::json!({ "params": { "meta": { "io.modelcontextprotocol/protocolVersion": "2026-07-28" } } }),
            serde_json::json!({ "params": { "_meta": { "protocolVersion": "2026-07-28" } } }),
            serde_json::json!({ "params": {} }),
        ] {
            assert_eq!(extract_body_protocol_version(&body(value)), None);
        }
    }

    /// Tests extracting the `Origin` HTTP header and trimming whitespace.
    #[test]
    fn test_extract_origin() {
        let mut headers = HeaderMap::new();
        assert_eq!(extract_origin(&headers), None);

        headers.insert("Origin", "http://localhost:3000".parse().unwrap());
        assert_eq!(extract_origin(&headers), Some("http://localhost:3000"));

        headers.insert("Origin", "  https://example.com  ".parse().unwrap());
        assert_eq!(extract_origin(&headers), Some("https://example.com"));

        headers.insert("Origin", "   ".parse().unwrap());
        assert_eq!(extract_origin(&headers), None);
    }

    /// Tests origin matching logic including case insensitivity, trailing slash tolerance, and wildcard support.
    #[test]
    fn test_is_origin_allowed() {
        let allowed = vec![
            "http://localhost:3000".to_string(),
            "https://app.example.com".to_string(),
        ];

        // Exact match
        assert!(is_origin_allowed("http://localhost:3000", &allowed));
        // Case-insensitivity
        assert!(is_origin_allowed("HTTP://LOCALHOST:3000", &allowed));
        // Trailing slash tolerance
        assert!(is_origin_allowed("http://localhost:3000/", &allowed));
        assert!(is_origin_allowed("https://app.example.com", &allowed));
        assert!(is_origin_allowed("https://app.example.com/", &allowed));

        // Mismatched origins
        assert!(!is_origin_allowed("http://localhost:8080", &allowed));
        assert!(!is_origin_allowed("http://evil.com", &allowed));
        assert!(!is_origin_allowed("null", &allowed));

        // Wildcard allowed
        let wildcard = vec!["*".to_string()];
        assert!(is_origin_allowed("http://anything.com", &wildcard));
        assert!(is_origin_allowed("null", &wildcard));
    }

    /// Tests origin header validation against allowed origins list.
    #[test]
    fn test_is_origin_header_allowed() {
        let allowed = vec![
            "http://localhost:3000".to_string(),
            "https://app.example.com".to_string(),
        ];

        // No Origin header (non-browser client)
        let headers_empty = HeaderMap::new();
        assert!(is_origin_header_allowed(&headers_empty, Some(&allowed)));

        // Valid Origin
        let mut headers_valid = HeaderMap::new();
        headers_valid.insert("Origin", "http://localhost:3000".parse().unwrap());
        assert!(is_origin_header_allowed(&headers_valid, Some(&allowed)));

        // Untrusted Origin
        let mut headers_untrusted = HeaderMap::new();
        headers_untrusted.insert("Origin", "http://attacker.com".parse().unwrap());
        assert!(!is_origin_header_allowed(
            &headers_untrusted,
            Some(&allowed)
        ));

        // Empty Origin
        let mut headers_blank = HeaderMap::new();
        headers_blank.insert("Origin", "".parse().unwrap());
        assert!(!is_origin_header_allowed(&headers_blank, Some(&allowed)));
    }

    /// Tests loopback origin detection across hosts, schemes, and ports.
    #[test]
    fn test_is_loopback_origin() {
        assert!(is_loopback_origin("http://localhost"));
        assert!(is_loopback_origin("http://localhost:3000"));
        assert!(is_loopback_origin("https://LOCALHOST:8443/"));
        assert!(is_loopback_origin("http://127.0.0.1:8080"));
        assert!(is_loopback_origin("http://[::1]:3000"));
        assert!(is_loopback_origin("http://[::1]"));

        assert!(!is_loopback_origin("http://evil.com"));
        assert!(!is_loopback_origin("http://localhost.evil.com"));
        assert!(!is_loopback_origin("http://127.0.0.1.evil.com"));
        assert!(!is_loopback_origin("null"));
        assert!(!is_loopback_origin(""));
    }

    /// Tests that only loopback origins are permitted when no allowed origins are configured.
    #[test]
    fn test_is_origin_header_allowed_default_loopback_only() {
        assert!(is_origin_header_allowed(&HeaderMap::new(), None));

        let mut local = HeaderMap::new();
        local.insert("Origin", "http://localhost:3000".parse().unwrap());
        assert!(is_origin_header_allowed(&local, None));

        let mut remote = HeaderMap::new();
        remote.insert("Origin", "http://evil.com".parse().unwrap());
        assert!(!is_origin_header_allowed(&remote, None));
    }
}
