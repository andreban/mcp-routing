// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Utility Functions
//!
//! Internal helper functions for HTTP header extraction, MIME type negotiation,
//! URI template matching, and method / parameter resolution.

use std::borrow::Cow;

use base64::prelude::*;
use http::HeaderMap;

use crate::extract::SessionId;
use crate::types::jsonrpc::{JsonRpcErrorResponse, JsonRpcRequestId};
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

/// Decodes an RFC 2047-style Base64 sentinel encoded header value (`=?base64?<encoded>?=`).
///
/// According to the MCP Streamable HTTP specification, values of `Mcp-Name` and `Mcp-Param-*`
/// headers containing non-ASCII characters, spaces, or control characters must use this sentinel format.
///
/// If `raw_value` matches `=?base64?...?=` and the Base64 payload decodes to valid UTF-8,
/// this returns `Cow::Owned(decoded_string)`. Otherwise, it returns `Cow::Borrowed(raw_value)`.
pub(crate) fn decode_sentinel_header(raw_value: &str) -> Cow<'_, str> {
    let trimmed = raw_value.trim();
    if trimmed.len() >= 11
        && (trimmed.starts_with("=?base64?") || trimmed[..9].eq_ignore_ascii_case("=?base64?"))
        && trimmed.ends_with("?=")
    {
        let b64_str = trimmed[9..trimmed.len() - 2].trim();
        let decoded = BASE64_STANDARD
            .decode(b64_str)
            .or_else(|_| BASE64_URL_SAFE.decode(b64_str))
            .or_else(|_| BASE64_STANDARD_NO_PAD.decode(b64_str))
            .or_else(|_| BASE64_URL_SAFE_NO_PAD.decode(b64_str));
        if let Ok(bytes) = decoded {
            if let Ok(utf8_str) = String::from_utf8(bytes) {
                return Cow::Owned(utf8_str);
            }
        }
    }
    Cow::Borrowed(raw_value)
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

/// Extracts all argument property names from a tool's `input_schema` that have `"x-mcp-header": true`.
pub(crate) fn extract_header_params_from_schema(schema: &serde_json::Value) -> Vec<String> {
    let mut header_params = Vec::new();
    if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
        for (prop_name, prop_schema) in properties {
            if prop_schema
                .get("x-mcp-header")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                header_params.push(prop_name.clone());
            }
        }
    }
    header_params
}

/// Retrieves the value of an `Mcp-Param-{param_name}` header from the request headers.
///
/// Matching is case-insensitive for both the `mcp-param-` prefix and the `{param_name}` suffix.
pub(crate) fn get_mcp_param_header<'a>(
    headers: &'a HeaderMap,
    param_name: &str,
) -> Option<&'a str> {
    let target = format!("mcp-param-{}", param_name);
    if let Ok(hname) = http::header::HeaderName::from_bytes(target.as_bytes()) {
        if let Some(val) = headers.get(&hname) {
            if let Ok(s) = val.to_str() {
                return Some(s);
            }
        }
    }
    for (key, val) in headers.iter() {
        if key.as_str().eq_ignore_ascii_case(&target) {
            if let Ok(s) = val.to_str() {
                return Some(s);
            }
        }
    }
    None
}

/// Compares a JSON argument value against a decoded HTTP header parameter string.
pub(crate) fn match_param_value(arg_val: &serde_json::Value, decoded_header: &str) -> bool {
    if let Some(s) = arg_val.as_str() {
        s == decoded_header
    } else if let Some(b) = arg_val.as_bool() {
        (b && decoded_header == "true") || (!b && decoded_header == "false")
    } else if let Some(n) = arg_val.as_number() {
        n.to_string() == decoded_header
            || decoded_header
                .parse::<serde_json::Number>()
                .map(|parsed| &parsed == n)
                .unwrap_or(false)
    } else if arg_val.is_null() {
        false
    } else {
        arg_val.to_string() == decoded_header
            || serde_json::from_str::<serde_json::Value>(decoded_header)
                .map(|v| &v == arg_val)
                .unwrap_or(false)
    }
}

/// Validates that `Mcp-Param-{Name}` headers match the arguments in the request body.
///
/// According to the MCP Streamable HTTP specification:
/// - If a tool argument property specifies `x-mcp-header: true` in its JSON schema,
///   and that argument is provided in the request body, the client MUST provide an
///   `Mcp-Param-{Name}` header matching the argument value.
/// - If `is_batch` is false and an argument with `x-mcp-header: true` is present, the header is REQUIRED.
/// - If any `Mcp-Param-{Name}` header is provided on the request:
///   - If the argument `{Name}` is present in the request body, the decoded header value MUST match the body argument.
///   - If the argument `{Name}` is not present in the body (or null), it is a header mismatch.
/// - Any mismatch or missing required header returns a `-32020` (`HeaderMismatch`) error.
pub(crate) fn validate_tool_header_params(
    req_id: Option<JsonRpcRequestId>,
    header_params: &[String],
    arguments: Option<&serde_json::Value>,
    headers: &HeaderMap,
    is_batch: bool,
) -> Result<(), JsonRpcErrorResponse> {
    let empty_map = serde_json::Map::new();
    let args_map = arguments.and_then(|v| v.as_object()).unwrap_or(&empty_map);

    // 1. Check schema-required header params
    for param_name in header_params {
        let arg_val = args_map.get(param_name);
        let header_val = get_mcp_param_header(headers, param_name);

        match (arg_val, header_val) {
            (Some(val), Some(h_val)) => {
                if val.is_null() {
                    return Err(header_mismatch_error(
                        req_id,
                        format!(
                            "Header mismatch: Mcp-Param-{param_name} header was provided but parameter '{param_name}' was null in request body arguments"
                        ),
                    ));
                }
                let decoded = decode_sentinel_header(h_val);
                if !match_param_value(val, decoded.as_ref()) {
                    return Err(header_mismatch_error(
                        req_id,
                        format!(
                            "Header mismatch: Mcp-Param-{param_name} header value '{h_val}' does not match body argument for '{param_name}'"
                        ),
                    ));
                }
            }
            (Some(val), None) => {
                if !val.is_null() && !is_batch {
                    return Err(header_mismatch_error(
                        req_id,
                        format!(
                            "Header mismatch: missing required Mcp-Param-{param_name} header for parameter '{param_name}'"
                        ),
                    ));
                }
            }
            (None, Some(h_val)) => {
                return Err(header_mismatch_error(
                    req_id,
                    format!(
                        "Header mismatch: Mcp-Param-{param_name} header was provided with value '{h_val}' but parameter '{param_name}' was not present in request body arguments"
                    ),
                ));
            }
            (None, None) => {
                // Parameter not provided in body or headers - valid if optional
            }
        }
    }

    // 2. Check any other Mcp-Param-* headers provided on the request that might not be in header_params
    for (header_key, header_val) in headers.iter() {
        let key_str = header_key.as_str();
        if key_str.len() > 10 && key_str[..10].eq_ignore_ascii_case("mcp-param-") {
            let param_suffix = &key_str[10..];
            if header_params
                .iter()
                .any(|p| p.eq_ignore_ascii_case(param_suffix))
            {
                continue;
            }
            let raw_h_val = match header_val.to_str() {
                Ok(s) => s,
                Err(_) => continue,
            };
            let matching_arg = args_map
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(param_suffix));
            match matching_arg {
                Some((actual_name, val)) => {
                    if val.is_null() {
                        return Err(header_mismatch_error(
                            req_id,
                            format!(
                                "Header mismatch: Mcp-Param-{param_suffix} header was provided but parameter '{actual_name}' was null in request body arguments"
                            ),
                        ));
                    }
                    let decoded = decode_sentinel_header(raw_h_val);
                    if !match_param_value(val, decoded.as_ref()) {
                        return Err(header_mismatch_error(
                            req_id,
                            format!(
                                "Header mismatch: Mcp-Param-{param_suffix} header value '{raw_h_val}' does not match body argument for '{actual_name}'"
                            ),
                        ));
                    }
                }
                None => {
                    return Err(header_mismatch_error(
                        req_id,
                        format!(
                            "Header mismatch: Mcp-Param-{param_suffix} header was provided with value '{raw_h_val}' but parameter '{param_suffix}' was not present in request body arguments"
                        ),
                    ));
                }
            }
        }
    }

    Ok(())
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

    /// Tests RFC 2047-style Base64 sentinel header decoding.
    #[test]
    fn test_decode_sentinel_header() {
        // Plain ASCII string (no sentinel) -> borrowed
        assert_eq!(decode_sentinel_header("my_tool"), "my_tool");
        assert!(matches!(decode_sentinel_header("my_tool"), Cow::Borrowed(_)));

        // Base64 sentinel encoding
        assert_eq!(decode_sentinel_header("=?base64?bXlfdG9vbA==?="), "my_tool");
        assert!(matches!(decode_sentinel_header("=?base64?bXlfdG9vbA==?="), Cow::Owned(_)));

        // Non-ASCII UTF-8 string: "Hello, 世界!" in base64 is "SGVsbG8sIOS4lueVjCE="
        assert_eq!(
            decode_sentinel_header("=?base64?SGVsbG8sIOS4lueVjCE=?="),
            "Hello, 世界!"
        );

        // Spaces and symbols: "custom param with spaces & symbols" in base64 is "Y3VzdG9tIHBhcmFtIHdpdGggc3BhY2VzICYgc3ltYm9scw=="
        assert_eq!(
            decode_sentinel_header("=?base64?Y3VzdG9tIHBhcmFtIHdpdGggc3BhY2VzICYgc3ltYm9scw==?="),
            "custom param with spaces & symbols"
        );

        // Case-insensitive sentinel prefix
        assert_eq!(decode_sentinel_header("=?BASE64?bXlfdG9vbA==?="), "my_tool");

        // Malformed / incomplete sentinels -> returned as-is (borrowed)
        assert_eq!(decode_sentinel_header("=?base64?incomplete"), "=?base64?incomplete");
        assert_eq!(decode_sentinel_header("=?base64?invalid!@#base64?="), "=?base64?invalid!@#base64?=");
        assert_eq!(decode_sentinel_header(""), "");
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
            "=?base64?ZmlsZTovLy9hcHAvZG9jIHdpdGggc3BhY2UudHh0?=".parse().unwrap(),
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

    #[test]
    fn test_extract_header_params_from_schema() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "repo": {
                    "type": "string",
                    "x-mcp-header": true
                },
                "branch": {
                    "type": "string",
                    "x-mcp-header": true
                },
                "path": {
                    "type": "string"
                }
            }
        });
        let mut params = extract_header_params_from_schema(&schema);
        params.sort();
        assert_eq!(params, vec!["branch", "repo"]);

        let no_header_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "foo": { "type": "string" }
            }
        });
        assert!(extract_header_params_from_schema(&no_header_schema).is_empty());
    }

    #[test]
    fn test_get_mcp_param_header() {
        let mut headers = HeaderMap::new();
        headers.insert("Mcp-Param-Repo", "mcp-routing".parse().unwrap());
        headers.insert("mcp-param-branch", "main".parse().unwrap());

        assert_eq!(get_mcp_param_header(&headers, "repo"), Some("mcp-routing"));
        assert_eq!(get_mcp_param_header(&headers, "Repo"), Some("mcp-routing"));
        assert_eq!(get_mcp_param_header(&headers, "branch"), Some("main"));
        assert_eq!(get_mcp_param_header(&headers, "Branch"), Some("main"));
        assert_eq!(get_mcp_param_header(&headers, "unknown"), None);
    }

    #[test]
    fn test_match_param_value() {
        assert!(match_param_value(&serde_json::json!("main"), "main"));
        assert!(!match_param_value(&serde_json::json!("main"), "develop"));

        assert!(match_param_value(&serde_json::json!(42), "42"));
        assert!(!match_param_value(&serde_json::json!(42), "43"));

        assert!(match_param_value(&serde_json::json!(true), "true"));
        assert!(match_param_value(&serde_json::json!(false), "false"));
        assert!(!match_param_value(&serde_json::json!(true), "false"));
    }

    #[test]
    fn test_validate_tool_header_params() {
        let header_params = vec!["repo".to_string(), "branch".to_string()];
        let mut headers = HeaderMap::new();
        headers.insert("Mcp-Param-Repo", "mcp-routing".parse().unwrap());
        headers.insert("Mcp-Param-Branch", "main".parse().unwrap());

        let args = serde_json::json!({
            "repo": "mcp-routing",
            "branch": "main",
            "path": "src/lib.rs"
        });

        // Exact match -> Ok
        assert!(validate_tool_header_params(None, &header_params, Some(&args), &headers, false).is_ok());

        // Sentinel encoded match -> Ok
        let mut sentinel_headers = HeaderMap::new();
        // "mcp-routing" in base64 is "bWNwLXJvdXRpbmc="
        sentinel_headers.insert("Mcp-Param-Repo", "=?base64?bWNwLXJvdXRpbmc=?=".parse().unwrap());
        sentinel_headers.insert("Mcp-Param-Branch", "main".parse().unwrap());
        assert!(validate_tool_header_params(None, &header_params, Some(&args), &sentinel_headers, false).is_ok());

        // Value mismatch -> Err
        let mut mismatch_headers = HeaderMap::new();
        mismatch_headers.insert("Mcp-Param-Repo", "other-repo".parse().unwrap());
        mismatch_headers.insert("Mcp-Param-Branch", "main".parse().unwrap());
        let err = validate_tool_header_params(None, &header_params, Some(&args), &mismatch_headers, false).unwrap_err();
        assert_eq!(err.error.code.code(), crate::types::mcp::HEADER_MISMATCH);

        // Missing required header -> Err
        let mut missing_headers = HeaderMap::new();
        missing_headers.insert("Mcp-Param-Repo", "mcp-routing".parse().unwrap());
        let err = validate_tool_header_params(None, &header_params, Some(&args), &missing_headers, false).unwrap_err();
        assert_eq!(err.error.code.code(), crate::types::mcp::HEADER_MISMATCH);

        // Header provided but argument missing in body -> Err
        let incomplete_args = serde_json::json!({
            "repo": "mcp-routing"
        });
        let err = validate_tool_header_params(None, &header_params, Some(&incomplete_args), &headers, false).unwrap_err();
        assert_eq!(err.error.code.code(), crate::types::mcp::HEADER_MISMATCH);

        // In batch mode, missing header is allowed if not sent on HTTP request
        let empty_headers = HeaderMap::new();
        assert!(validate_tool_header_params(None, &header_params, Some(&args), &empty_headers, true).is_ok());
    }
}

