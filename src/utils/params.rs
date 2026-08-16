// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! MCP parameter header (`Mcp-Param-*`) extraction, schema inspection, and validation utilities.

use http::HeaderMap;

use crate::types::jsonrpc::{JsonRpcErrorResponse, JsonRpcRequestId};
use crate::types::mcp::header_mismatch_error;
use crate::utils::sentinel::decode_sentinel_header;

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
    if let Ok(hname) = http::header::HeaderName::from_bytes(target.as_bytes())
        && let Some(val) = headers.get(&hname)
        && let Ok(s) = val.to_str()
    {
        return Some(s);
    }
    for (key, val) in headers.iter() {
        if key.as_str().eq_ignore_ascii_case(&target)
            && let Ok(s) = val.to_str()
        {
            return Some(s);
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
        decoded_header
            .parse::<serde_json::Number>()
            .map(|parsed| &parsed == n)
            .unwrap_or(false)
    } else if arg_val.is_null() {
        false
    } else {
        serde_json::from_str::<serde_json::Value>(decoded_header)
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

    /// Tests extracting properties marked with `x-mcp-header: true` from JSON schemas.
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

    /// Tests retrieving case-insensitive `Mcp-Param-{Name}` headers from `HeaderMap`.
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

    /// Tests matching JSON parameter values against header strings.
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

    /// Tests validation of `Mcp-Param-{Name}` request headers against tool call arguments.
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
        assert!(
            validate_tool_header_params(None, &header_params, Some(&args), &headers, false).is_ok()
        );

        // Sentinel encoded match -> Ok
        let mut sentinel_headers = HeaderMap::new();
        // "mcp-routing" in base64 is "bWNwLXJvdXRpbmc="
        sentinel_headers.insert(
            "Mcp-Param-Repo",
            "=?base64?bWNwLXJvdXRpbmc=?=".parse().unwrap(),
        );
        sentinel_headers.insert("Mcp-Param-Branch", "main".parse().unwrap());
        assert!(
            validate_tool_header_params(
                None,
                &header_params,
                Some(&args),
                &sentinel_headers,
                false
            )
            .is_ok()
        );

        // Value mismatch -> Err
        let mut mismatch_headers = HeaderMap::new();
        mismatch_headers.insert("Mcp-Param-Repo", "other-repo".parse().unwrap());
        mismatch_headers.insert("Mcp-Param-Branch", "main".parse().unwrap());
        let err = validate_tool_header_params(
            None,
            &header_params,
            Some(&args),
            &mismatch_headers,
            false,
        )
        .unwrap_err();
        assert_eq!(err.error.code.code(), crate::types::mcp::HEADER_MISMATCH);

        // Missing required header -> Err
        let mut missing_headers = HeaderMap::new();
        missing_headers.insert("Mcp-Param-Repo", "mcp-routing".parse().unwrap());
        let err =
            validate_tool_header_params(None, &header_params, Some(&args), &missing_headers, false)
                .unwrap_err();
        assert_eq!(err.error.code.code(), crate::types::mcp::HEADER_MISMATCH);

        // Header provided but argument missing in body -> Err
        let incomplete_args = serde_json::json!({
            "repo": "mcp-routing"
        });
        let err = validate_tool_header_params(
            None,
            &header_params,
            Some(&incomplete_args),
            &headers,
            false,
        )
        .unwrap_err();
        assert_eq!(err.error.code.code(), crate::types::mcp::HEADER_MISMATCH);

        // In batch mode, missing header is allowed if not sent on HTTP request
        let empty_headers = HeaderMap::new();
        assert!(
            validate_tool_header_params(None, &header_params, Some(&args), &empty_headers, true)
                .is_ok()
        );
    }
}
