// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! MCP method, target name, and resource URI resolution and header-vs-body validation.

use crate::types::jsonrpc::JsonRpcErrorResponse;
use crate::types::mcp::header_mismatch_error;

use std::borrow::Cow;

/// Configuration options for header vs body resolution.
struct ResolveOptions {
    header_name: &'static str,
    target_label: &'static str,
    missing_header_msg: Cow<'static, str>,
    trim_slashes: bool,
    use_invalid_request: bool,
}

/// Common helper to resolve and validate a string value from HTTP headers or JSON-RPC body.
///
/// Follows strict MCP Streamable HTTP specification:
/// 1. If header is present:
///    - Trims header (slashes or whitespace) and ensures non-empty.
///    - If body parameter is also present, trims body and verifies it matches header.
///    - Returns trimmed header value.
/// 2. If header is missing:
///    - For single requests (`!is_batch`), returns missing header error.
///    - For batch requests (`is_batch`):
///      - If body parameter is present, trims body and ensures non-empty.
///      - Returns trimmed body value.
///      - If body parameter is also missing, returns missing parameter/request error.
fn resolve_header_or_body_value<'a>(
    header_val: Option<&'a str>,
    body_val: Option<&'a str>,
    is_batch: bool,
    options: ResolveOptions,
) -> Result<&'a str, JsonRpcErrorResponse> {
    let trim = |s: &'a str| -> &'a str {
        if options.trim_slashes {
            s.trim_matches('/')
        } else {
            s.trim()
        }
    };

    let make_empty_err = || {
        if options.use_invalid_request {
            JsonRpcErrorResponse::invalid_request(
                None,
                format!("Invalid Request: empty {}", options.target_label),
            )
        } else {
            JsonRpcErrorResponse::invalid_params(
                None,
                format!("Invalid params: empty {}", options.target_label),
            )
        }
    };

    let make_missing_err = || {
        if options.use_invalid_request {
            JsonRpcErrorResponse::invalid_request(
                None,
                format!("Invalid Request: missing {}", options.target_label),
            )
        } else {
            JsonRpcErrorResponse::invalid_params(
                None,
                format!("Invalid params: missing {}", options.target_label),
            )
        }
    };

    let make_mismatch_err = |h: &str, b: &str| {
        header_mismatch_error(
            None,
            format!(
                "Header mismatch: {} header value '{h}' does not match body {} '{b}'",
                options.header_name, options.target_label
            ),
        )
    };

    if let Some(h) = header_val {
        let h_trim = trim(h);
        if h_trim.is_empty() {
            return Err(make_empty_err());
        }
        if let Some(b) = body_val {
            let b_trim = trim(b);
            if h_trim != b_trim {
                return Err(make_mismatch_err(h_trim, b_trim));
            }
        }
        Ok(h_trim)
    } else if !is_batch {
        Err(header_mismatch_error(None, options.missing_header_msg))
    } else if let Some(b) = body_val {
        let b_trim = trim(b);
        if b_trim.is_empty() {
            return Err(make_empty_err());
        }
        Ok(b_trim)
    } else {
        Err(make_missing_err())
    }
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
    resolve_header_or_body_value(
        header_method,
        body_method,
        is_batch,
        ResolveOptions {
            header_name: "Mcp-Method",
            target_label: "method",
            missing_header_msg: Cow::Borrowed(
                "Header mismatch: missing required Mcp-Method header",
            ),
            trim_slashes: true,
            use_invalid_request: true,
        },
    )
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
    resolve_header_or_body_value(
        header_name,
        body_name,
        is_batch,
        ResolveOptions {
            header_name: "Mcp-Name",
            target_label: target_kind,
            missing_header_msg: Cow::Owned(format!(
                "Header mismatch: missing required Mcp-Name header for {target_kind}"
            )),
            trim_slashes: true,
            use_invalid_request: false,
        },
    )
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
    resolve_header_or_body_value(
        header_uri,
        body_uri,
        is_batch,
        ResolveOptions {
            header_name: "Mcp-Uri",
            target_label: "resource uri",
            missing_header_msg: Cow::Borrowed(
                "Header mismatch: missing required Mcp-Uri header for resources/read",
            ),
            trim_slashes: false,
            use_invalid_request: false,
        },
    )
}

pub(crate) use resolve_required_uri as resolve_resource_uri;

#[cfg(test)]
mod tests {
    //! Unit tests for MCP method, target name, and resource URI resolution.

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

        // Empty Mcp-Method header on single request -> InvalidRequest (-32600)
        let err = resolve_method(Some(""), Some("server/discover"), false).unwrap_err();
        assert_eq!(err.error.code.code(), -32600);

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

        // Empty header in batch -> InvalidRequest (-32600)
        let err = resolve_method(Some("/"), Some("tools/list"), true).unwrap_err();
        assert_eq!(err.error.code.code(), -32600);

        // Empty body in batch -> InvalidRequest (-32600)
        let err = resolve_method(None, Some("/"), true).unwrap_err();
        assert_eq!(err.error.code.code(), -32600);

        // Missing both in batch -> InvalidRequest (-32600)
        let err = resolve_method(None, None, true).unwrap_err();
        assert_eq!(err.error.code.code(), -32600);
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

        // Empty Mcp-Name header on single request -> InvalidParams (-32602)
        let err = resolve_tool_name(Some("/"), Some("my_tool"), false, "tool name").unwrap_err();
        assert_eq!(err.error.code.code(), -32602);

        // Header and body tool name mismatch -> HeaderMismatch (-32020)
        let err =
            resolve_tool_name(Some("tool_a"), Some("tool_b"), false, "tool name").unwrap_err();
        assert_eq!(err.error.code.code(), crate::types::mcp::HEADER_MISMATCH);

        // Batch request (is_batch: true)
        assert_eq!(
            resolve_tool_name(None, Some("tool_b"), true, "tool name").unwrap(),
            "tool_b"
        );

        // Empty body in batch -> InvalidParams (-32602)
        let err = resolve_tool_name(None, Some(""), true, "tool name").unwrap_err();
        assert_eq!(err.error.code.code(), -32602);

        // Missing both in batch -> InvalidParams (-32602)
        let err = resolve_tool_name(None, None, true, "tool name").unwrap_err();
        assert_eq!(err.error.code.code(), -32602);
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

        // Empty Mcp-Uri header on single request -> InvalidParams (-32602)
        let err = resolve_resource_uri(Some("   "), Some("file:///doc.txt"), false).unwrap_err();
        assert_eq!(err.error.code.code(), -32602);

        // Header and body URI mismatch -> HeaderMismatch (-32020)
        let err =
            resolve_resource_uri(Some("file:///a.txt"), Some("file:///b.txt"), false).unwrap_err();
        assert_eq!(err.error.code.code(), crate::types::mcp::HEADER_MISMATCH);

        // Batch request (is_batch: true)
        assert_eq!(
            resolve_resource_uri(None, Some("file:///b.txt"), true).unwrap(),
            "file:///b.txt"
        );

        // Empty body in batch -> InvalidParams (-32602)
        let err = resolve_resource_uri(None, Some(" "), true).unwrap_err();
        assert_eq!(err.error.code.code(), -32602);

        // Missing both in batch -> InvalidParams (-32602)
        let err = resolve_resource_uri(None, None, true).unwrap_err();
        assert_eq!(err.error.code.code(), -32602);
    }
}
