// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! HTTP header extraction and MIME type negotiation utilities.

use std::borrow::Cow;

use http::HeaderMap;

use crate::extract::SessionId;
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

/// Extracts the tool or prompt name from the `Mcp-Name` HTTP header, trimming leading and trailing slashes
/// and decoding any RFC 2047-style Base64 sentinel value (`=?base64?...?=`).
pub(crate) fn extract_header_name(headers: &HeaderMap) -> Option<Cow<'_, str>> {
    headers
        .get("Mcp-Name")
        .and_then(|v| v.to_str().ok())
        .map(|s| {
            let trimmed = s.trim().trim_matches('/');
            let decoded = decode_sentinel_header(trimmed);
            match decoded {
                Cow::Borrowed(b) => Cow::Borrowed(b.trim_matches('/')),
                Cow::Owned(o) => {
                    let trimmed_owned = o.trim_matches('/').to_string();
                    Cow::Owned(trimmed_owned)
                }
            }
        })
}

/// Extracts the resource URI from HTTP headers (`Mcp-Uri` or `Mcp-Name`), trimming whitespace
/// and decoding any RFC 2047-style Base64 sentinel value (`=?base64?...?=`).
pub(crate) fn extract_header_uri(headers: &HeaderMap) -> Option<Cow<'_, str>> {
    headers
        .get("Mcp-Uri")
        .or_else(|| headers.get("Mcp-Name"))
        .and_then(|v| v.to_str().ok())
        .map(|s| {
            let trimmed = s.trim();
            let decoded = decode_sentinel_header(trimmed);
            match decoded {
                Cow::Borrowed(b) => Cow::Borrowed(b.trim()),
                Cow::Owned(o) => Cow::Owned(o.trim().to_string()),
            }
        })
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

/// Validates whether the `Origin` header in the request is permitted.
///
/// If no `Origin` header is present (such as with non-browser clients), returns `true`.
/// If the `Origin` header is present, it must be valid and match at least one allowed origin.
pub(crate) fn is_origin_header_allowed(headers: &HeaderMap, allowed_origins: &[String]) -> bool {
    if !headers.contains_key(http::header::ORIGIN) {
        return true;
    }
    let Some(origin) = extract_origin(headers) else {
        return false;
    };
    is_origin_allowed(origin, allowed_origins)
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

    /// Tests extracting the `Mcp-Name` header and trimming slashes with sentinel decoding.
    #[test]
    fn test_extract_header_name() {
        let mut headers = HeaderMap::new();
        assert_eq!(extract_header_name(&headers), None);

        headers.insert("Mcp-Name", "/my_tool/".parse().unwrap());
        assert_eq!(extract_header_name(&headers).as_deref(), Some("my_tool"));

        headers.insert("Mcp-Name", "///".parse().unwrap());
        assert_eq!(extract_header_name(&headers).as_deref(), Some(""));

        // Sentinel encoded tool name
        headers.insert("Mcp-Name", "=?base64?bXlfdG9vbA==?=".parse().unwrap());
        assert_eq!(extract_header_name(&headers).as_deref(), Some("my_tool"));

        // Sentinel encoded tool name with leading/trailing slashes in decoded value
        headers.insert("Mcp-Name", "=?base64?L215X3Rvb2wv?=".parse().unwrap());
        assert_eq!(extract_header_name(&headers).as_deref(), Some("my_tool"));

        // Sentinel encoded unicode tool name ("echo_世界" -> "ZWNob1/kuJbnlYw=")
        headers.insert("Mcp-Name", "=?base64?ZWNob1/kuJbnlYw=?=".parse().unwrap());
        assert_eq!(extract_header_name(&headers).as_deref(), Some("echo_世界"));
    }

    /// Tests extracting the `Mcp-Uri` and `Mcp-Name` headers for resources with sentinel decoding.
    #[test]
    fn test_extract_header_uri() {
        let mut headers = HeaderMap::new();
        assert_eq!(extract_header_uri(&headers), None);

        headers.insert("Mcp-Uri", "file:///app/config.json".parse().unwrap());
        assert_eq!(
            extract_header_uri(&headers).as_deref(),
            Some("file:///app/config.json")
        );

        headers.remove("Mcp-Uri");
        headers.insert("Mcp-Name", "file:///app/config.json".parse().unwrap());
        assert_eq!(
            extract_header_uri(&headers).as_deref(),
            Some("file:///app/config.json")
        );

        // Sentinel encoded URI ("file:///app/doc with space.txt" -> "ZmlsZTovLy9hcHAvZG9jIHdpdGggc3BhY2UudHh0")
        headers.insert(
            "Mcp-Uri",
            "=?base64?ZmlsZTovLy9hcHAvZG9jIHdpdGggc3BhY2UudHh0?="
                .parse()
                .unwrap(),
        );
        assert_eq!(
            extract_header_uri(&headers).as_deref(),
            Some("file:///app/doc with space.txt")
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
        assert!(is_origin_header_allowed(&headers_empty, &allowed));

        // Valid Origin
        let mut headers_valid = HeaderMap::new();
        headers_valid.insert("Origin", "http://localhost:3000".parse().unwrap());
        assert!(is_origin_header_allowed(&headers_valid, &allowed));

        // Untrusted Origin
        let mut headers_untrusted = HeaderMap::new();
        headers_untrusted.insert("Origin", "http://attacker.com".parse().unwrap());
        assert!(!is_origin_header_allowed(&headers_untrusted, &allowed));

        // Empty Origin
        let mut headers_blank = HeaderMap::new();
        headers_blank.insert("Origin", "".parse().unwrap());
        assert!(!is_origin_header_allowed(&headers_blank, &allowed));
    }
}
