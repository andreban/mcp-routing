// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! MCP parameter header (`Mcp-Param-*`) extraction, schema inspection, and validation utilities.

use http::HeaderMap;

use crate::types::jsonrpc::{JsonRpcErrorResponse, JsonRpcRequestId};
use crate::types::mcp::header_mismatch_error;
use crate::utils::sentinel::decode_sentinel_header;

/// A tool parameter mirrored into an `Mcp-Param-{header}` HTTP header via an `x-mcp-header` annotation.
#[derive(Clone)]
pub(crate) struct HeaderParam {
    /// The `{header}` portion of the `Mcp-Param-{header}` header name (the `x-mcp-header` value).
    pub(crate) header: String,
    /// The chain of `properties` keys from the schema root to the annotated property.
    pub(crate) path: Vec<String>,
}

/// Extracts the `x-mcp-header` annotations from a tool's `inputSchema`.
///
/// Per the MCP Streamable HTTP specification, the `x-mcp-header` value is the name portion of the
/// `Mcp-Param-{name}` header. Annotations are valid only when:
/// - the value is a non-empty RFC 9110 token, unique case-insensitively within the schema;
/// - the annotated property has `type` `"string"`, `"integer"`, or `"boolean"`;
/// - the property is statically reachable from the schema root through `properties` keys only.
///
/// Returns a description of the first violated constraint if any annotation is invalid.
pub(crate) fn extract_header_params_from_schema(
    schema: &serde_json::Value,
) -> Result<Vec<HeaderParam>, String> {
    let mut params = Vec::new();
    collect_header_params(schema, &mut Vec::new(), &mut params)?;

    if count_header_annotations(schema, false) != params.len() {
        return Err(
            "x-mcp-header must only annotate properties reachable from the schema root through \
             `properties` keys"
                .to_string(),
        );
    }

    for (i, param) in params.iter().enumerate() {
        if params[..i]
            .iter()
            .any(|p| p.header.eq_ignore_ascii_case(&param.header))
        {
            return Err(format!(
                "x-mcp-header value '{}' is not case-insensitively unique",
                param.header
            ));
        }
    }

    Ok(params)
}

/// Walks `properties` chains from `schema`, collecting and validating annotated properties.
fn collect_header_params(
    schema: &serde_json::Value,
    path: &mut Vec<String>,
    params: &mut Vec<HeaderParam>,
) -> Result<(), String> {
    let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) else {
        return Ok(());
    };
    for (name, prop_schema) in properties {
        path.push(name.clone());
        if let Some(annotation) = prop_schema.get("x-mcp-header") {
            let property = path.join(".");
            let Some(header) = annotation.as_str() else {
                return Err(format!(
                    "x-mcp-header on property '{property}' must be a string"
                ));
            };
            if header.is_empty() || !header.bytes().all(is_tchar) {
                return Err(format!(
                    "x-mcp-header value '{header}' on property '{property}' is not a valid HTTP \
                     header token"
                ));
            }
            let prop_type = prop_schema.get("type").and_then(|t| t.as_str());
            if !matches!(prop_type, Some("string" | "integer" | "boolean")) {
                return Err(format!(
                    "x-mcp-header on property '{property}' requires type string, integer, or \
                     boolean"
                ));
            }
            params.push(HeaderParam {
                header: header.to_string(),
                path: path.clone(),
            });
        }
        collect_header_params(prop_schema, path, params)?;
        path.pop();
    }
    Ok(())
}

/// Counts every `x-mcp-header` keyword anywhere in `value`.
///
/// `in_properties_map` is `true` while visiting the name-to-schema map of a `properties` keyword,
/// whose keys are property names rather than schema keywords.
fn count_header_annotations(value: &serde_json::Value, in_properties_map: bool) -> usize {
    match value {
        serde_json::Value::Object(map) => map
            .iter()
            .map(|(key, child)| {
                let own = usize::from(!in_properties_map && key == "x-mcp-header");
                let child_is_properties_map = !in_properties_map && key == "properties";
                own + count_header_annotations(child, child_is_properties_map)
            })
            .sum(),
        serde_json::Value::Array(items) => items
            .iter()
            .map(|item| count_header_annotations(item, false))
            .sum(),
        _ => 0,
    }
}

/// Returns `true` if `b` is an RFC 9110 `tchar`.
fn is_tchar(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b)
}

/// Reads the argument value at `path`, treating `null` as absent.
fn argument_at<'a>(
    arguments: Option<&'a serde_json::Value>,
    path: &[String],
) -> Option<&'a serde_json::Value> {
    path.iter()
        .try_fold(arguments?, |value, key| value.get(key))
        .filter(|value| !value.is_null())
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
///
/// Numbers are compared numerically, so a header of `42.0` matches a body value of `42`.
pub(crate) fn match_param_value(arg_val: &serde_json::Value, decoded_header: &str) -> bool {
    if let Some(s) = arg_val.as_str() {
        s == decoded_header
    } else if let Some(b) = arg_val.as_bool() {
        (b && decoded_header == "true") || (!b && decoded_header == "false")
    } else if let Some(n) = arg_val.as_number() {
        match (decoded_header.parse::<f64>(), n.as_f64()) {
            (Ok(header), Some(body)) => header.is_finite() && header == body,
            _ => false,
        }
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
/// According to the MCP Streamable HTTP specification, for each `x-mcp-header` annotated parameter:
/// - If the argument is present (and not `null`), the `Mcp-Param-{Name}` header is REQUIRED
///   and its decoded value MUST match the argument.
/// - If the argument is absent or `null`, the header MUST NOT be sent.
///
/// `Mcp-Param-*` headers that do not correspond to an annotated parameter are ignored.
/// Any mismatch or missing required header returns a `-32020` (`HeaderMismatch`) error.
pub(crate) fn validate_tool_header_params(
    req_id: Option<JsonRpcRequestId>,
    header_params: &[HeaderParam],
    arguments: Option<&serde_json::Value>,
    headers: &HeaderMap,
) -> Result<(), JsonRpcErrorResponse> {
    for param in header_params {
        let name = &param.header;
        let property = param.path.join(".");
        let arg_val = argument_at(arguments, &param.path);
        let header_val = get_mcp_param_header(headers, name);

        match (arg_val, header_val) {
            (Some(val), Some(h_val)) => {
                let decoded = decode_sentinel_header(h_val).map_err(|reason| {
                    header_mismatch_error(
                        req_id.clone(),
                        format!("Header mismatch: Mcp-Param-{name} header is invalid: {reason}"),
                    )
                })?;
                if !match_param_value(val, decoded.as_ref()) {
                    return Err(header_mismatch_error(
                        req_id,
                        format!(
                            "Header mismatch: Mcp-Param-{name} header value '{h_val}' does not match body argument '{property}'"
                        ),
                    ));
                }
            }
            (Some(_), None) => {
                return Err(header_mismatch_error(
                    req_id,
                    format!(
                        "Header mismatch: missing required Mcp-Param-{name} header for argument '{property}'"
                    ),
                ));
            }
            (None, Some(h_val)) => {
                return Err(header_mismatch_error(
                    req_id,
                    format!(
                        "Header mismatch: Mcp-Param-{name} header was provided with value '{h_val}' but argument '{property}' was not present in the request body"
                    ),
                ));
            }
            (None, None) => {}
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Unit tests for `x-mcp-header` extraction and `Mcp-Param-*` header validation.

    use super::*;
    use serde_json::json;

    /// Returns the `(header, dotted path)` pairs of the extracted parameters, sorted by header.
    fn summarize(params: &[HeaderParam]) -> Vec<(String, String)> {
        let mut out: Vec<_> = params
            .iter()
            .map(|p| (p.header.clone(), p.path.join(".")))
            .collect();
        out.sort();
        out
    }

    /// Tests extracting string `x-mcp-header` annotations, including nested `properties` chains.
    #[test]
    fn test_extract_header_params_from_schema() {
        let schema = json!({
            "type": "object",
            "properties": {
                "tenant_id": { "type": "string", "x-mcp-header": "Tenant" },
                "region": { "type": "string", "x-mcp-header": "Region" },
                "options": {
                    "type": "object",
                    "properties": {
                        "priority": { "type": "integer", "x-mcp-header": "Priority" }
                    }
                },
                "query": { "type": "string" }
            }
        });
        let params = extract_header_params_from_schema(&schema).unwrap();
        assert_eq!(
            summarize(&params),
            vec![
                ("Priority".to_string(), "options.priority".to_string()),
                ("Region".to_string(), "region".to_string()),
                ("Tenant".to_string(), "tenant_id".to_string()),
            ]
        );

        let no_header_schema = json!({
            "type": "object",
            "properties": { "foo": { "type": "string" } }
        });
        assert!(
            extract_header_params_from_schema(&no_header_schema)
                .unwrap()
                .is_empty()
        );
    }

    /// Tests that annotations violating the `x-mcp-header` constraints are rejected.
    #[test]
    fn test_extract_header_params_rejects_invalid_annotations() {
        let invalid_schemas = [
            // Non-string value
            json!({ "properties": { "a": { "type": "string", "x-mcp-header": true } } }),
            // Empty value
            json!({ "properties": { "a": { "type": "string", "x-mcp-header": "" } } }),
            // Not an RFC 9110 token
            json!({ "properties": { "a": { "type": "string", "x-mcp-header": "Bad Name" } } }),
            // `number` type is not permitted
            json!({ "properties": { "a": { "type": "number", "x-mcp-header": "A" } } }),
            // Non-primitive type
            json!({ "properties": { "a": { "type": "object", "x-mcp-header": "A" } } }),
            // Duplicate (case-insensitive)
            json!({ "properties": {
                "a": { "type": "string", "x-mcp-header": "Name" },
                "b": { "type": "string", "x-mcp-header": "name" }
            } }),
            // Not statically reachable: inside `items`
            json!({ "properties": { "list": {
                "type": "array",
                "items": { "type": "object", "properties": {
                    "a": { "type": "string", "x-mcp-header": "A" }
                } }
            } } }),
            // Not statically reachable: inside a composition keyword
            json!({ "anyOf": [ { "properties": {
                "a": { "type": "string", "x-mcp-header": "A" }
            } } ] }),
        ];
        for schema in invalid_schemas {
            assert!(
                extract_header_params_from_schema(&schema).is_err(),
                "expected rejection: {schema}"
            );
        }

        // A property merely *named* `x-mcp-header` is not an annotation
        let named = json!({ "properties": { "x-mcp-header": { "type": "string" } } });
        assert!(
            extract_header_params_from_schema(&named)
                .unwrap()
                .is_empty()
        );
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
        assert!(match_param_value(&json!("main"), "main"));
        assert!(!match_param_value(&json!("main"), "develop"));

        assert!(match_param_value(&json!(42), "42"));
        assert!(!match_param_value(&json!(42), "43"));
        assert!(match_param_value(&json!(42), "42.0"));
        assert!(match_param_value(&json!(-7), "-7"));
        assert!(!match_param_value(&json!(42), "forty-two"));
        assert!(!match_param_value(&json!(42), "NaN"));

        assert!(match_param_value(&json!(true), "true"));
        assert!(match_param_value(&json!(false), "false"));
        assert!(!match_param_value(&json!(true), "false"));
    }

    /// Tests validation of `Mcp-Param-{Name}` request headers against tool call arguments.
    #[test]
    fn test_validate_tool_header_params() {
        let schema = json!({
            "type": "object",
            "properties": {
                "tenant_id": { "type": "string", "x-mcp-header": "Tenant" },
                "options": {
                    "type": "object",
                    "properties": {
                        "priority": { "type": "integer", "x-mcp-header": "Priority" }
                    }
                }
            }
        });
        let header_params = extract_header_params_from_schema(&schema).unwrap();
        let args = json!({ "tenant_id": "acme", "options": { "priority": 3 }, "query": "x" });

        let mut headers = HeaderMap::new();
        headers.insert("Mcp-Param-Tenant", "acme".parse().unwrap());
        headers.insert("Mcp-Param-Priority", "3".parse().unwrap());

        // Exact match, including a nested property -> Ok
        assert!(validate_tool_header_params(None, &header_params, Some(&args), &headers).is_ok());

        // Sentinel encoded match -> Ok ("acme" in base64 is "YWNtZQ==")
        let mut sentinel_headers = headers.clone();
        sentinel_headers.insert("Mcp-Param-Tenant", "=?base64?YWNtZQ==?=".parse().unwrap());
        assert!(
            validate_tool_header_params(None, &header_params, Some(&args), &sentinel_headers)
                .is_ok()
        );

        // Unrecognized Mcp-Param-* headers are ignored -> Ok
        let mut extra_headers = headers.clone();
        extra_headers.insert("Mcp-Param-Unrelated", "whatever".parse().unwrap());
        assert!(
            validate_tool_header_params(None, &header_params, Some(&args), &extra_headers).is_ok()
        );

        // Value mismatch -> Err
        let mut mismatch_headers = headers.clone();
        mismatch_headers.insert("Mcp-Param-Tenant", "other".parse().unwrap());
        let err = validate_tool_header_params(None, &header_params, Some(&args), &mismatch_headers)
            .unwrap_err();
        assert_eq!(err.error.code.code(), crate::types::mcp::HEADER_MISMATCH);

        // Missing required header for a nested argument -> Err
        let mut missing_headers = HeaderMap::new();
        missing_headers.insert("Mcp-Param-Tenant", "acme".parse().unwrap());
        let err = validate_tool_header_params(None, &header_params, Some(&args), &missing_headers)
            .unwrap_err();
        assert_eq!(err.error.code.code(), crate::types::mcp::HEADER_MISMATCH);

        // Header provided but argument absent from body -> Err
        let partial_args = json!({ "tenant_id": "acme" });
        let err = validate_tool_header_params(None, &header_params, Some(&partial_args), &headers)
            .unwrap_err();
        assert_eq!(err.error.code.code(), crate::types::mcp::HEADER_MISMATCH);

        // Null argument and no header -> Ok
        let null_args = json!({ "tenant_id": null });
        assert!(
            validate_tool_header_params(None, &header_params, Some(&null_args), &HeaderMap::new())
                .is_ok()
        );
    }
}
