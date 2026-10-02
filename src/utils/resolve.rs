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
    use_invalid_request: bool,
}

/// Common helper to resolve and validate a string value from HTTP headers or JSON-RPC body.
///
/// Follows strict MCP Streamable HTTP specification:
/// 1. If the header is missing, returns a missing header (`HeaderMismatch`) error.
/// 2. If the header is empty, returns an empty value error.
/// 3. If the body value is present, it MUST exactly match the header (values are case-sensitive
///    and compared verbatim).
fn resolve_header_or_body_value<'a>(
    header_val: Option<&'a str>,
    body_val: Option<&'a str>,
    options: ResolveOptions,
) -> Result<&'a str, JsonRpcErrorResponse> {
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

    let make_mismatch_err = |h: &str, b: &str| {
        header_mismatch_error(
            None,
            format!(
                "Header mismatch: {} header value '{h}' does not match body {} '{b}'",
                options.header_name, options.target_label
            ),
        )
    };

    let Some(h) = header_val else {
        return Err(header_mismatch_error(None, options.missing_header_msg));
    };
    if h.is_empty() {
        return Err(make_empty_err());
    }
    if let Some(b) = body_val
        && h != b
    {
        return Err(make_mismatch_err(h, b));
    }
    Ok(h)
}

/// Resolves and validates the MCP method against the `Mcp-Method` header and body method.
///
/// In strict MCP Streamable HTTP:
/// - Requests MUST include an `Mcp-Method` HTTP header.
/// - If the body also contains a `method`, it MUST match the header.
pub(crate) fn resolve_method<'a>(
    header_method: Option<&'a str>,
    body_method: Option<&'a str>,
) -> Result<&'a str, JsonRpcErrorResponse> {
    resolve_header_or_body_value(
        header_method,
        body_method,
        ResolveOptions {
            header_name: "Mcp-Method",
            target_label: "method",
            missing_header_msg: Cow::Borrowed(
                "Header mismatch: missing required Mcp-Method header",
            ),
            use_invalid_request: true,
        },
    )
}

/// Resolves and validates the target name for `tools/call` or `prompts/get`.
///
/// In strict MCP Streamable HTTP:
/// - Requests MUST include an `Mcp-Name` HTTP header.
/// - If the body also contains a `name` parameter, it MUST match the header.
pub(crate) fn resolve_required_name<'a>(
    header_name: Option<&'a str>,
    body_name: Option<&'a str>,
    target_kind: &'static str,
) -> Result<&'a str, JsonRpcErrorResponse> {
    resolve_header_or_body_value(
        header_name,
        body_name,
        ResolveOptions {
            header_name: "Mcp-Name",
            target_label: target_kind,
            missing_header_msg: Cow::Owned(format!(
                "Header mismatch: missing required Mcp-Name header for {target_kind}"
            )),
            use_invalid_request: false,
        },
    )
}

pub(crate) use resolve_required_name as resolve_prompt_name;
pub(crate) use resolve_required_name as resolve_tool_name;

/// Resolves and validates the resource URI for `resources/read`.
///
/// In strict MCP Streamable HTTP:
/// - Requests MUST include an `Mcp-Name` HTTP header carrying the URI.
/// - If the body also contains a `uri` parameter, it MUST match the header.
pub(crate) fn resolve_required_uri<'a>(
    header_uri: Option<&'a str>,
    body_uri: Option<&'a str>,
) -> Result<&'a str, JsonRpcErrorResponse> {
    resolve_header_or_body_value(
        header_uri,
        body_uri,
        ResolveOptions {
            header_name: "Mcp-Name",
            target_label: "resource uri",
            missing_header_msg: Cow::Borrowed(
                "Header mismatch: missing required Mcp-Name header for resources/read",
            ),
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
        assert_eq!(
            resolve_method(Some("server/discover"), Some("server/discover")).unwrap(),
            "server/discover"
        );
        // Values are compared verbatim: surrounding slashes are significant
        let err = resolve_method(Some("/server/discover/"), Some("server/discover")).unwrap_err();
        assert_eq!(err.error.code.code(), crate::types::mcp::HEADER_MISMATCH);
        assert_eq!(
            resolve_method(Some("server/discover"), None).unwrap(),
            "server/discover"
        );

        // Missing Mcp-Method header -> HeaderMismatch (-32020)
        let err = resolve_method(None, Some("server/discover")).unwrap_err();
        assert_eq!(err.error.code.code(), crate::types::mcp::HEADER_MISMATCH);

        // Empty Mcp-Method header -> InvalidRequest (-32600)
        let err = resolve_method(Some(""), Some("server/discover")).unwrap_err();
        assert_eq!(err.error.code.code(), -32600);

        // Header and body method mismatch -> HeaderMismatch (-32020)
        let err = resolve_method(Some("tools/call"), Some("server/discover")).unwrap_err();
        assert_eq!(err.error.code.code(), crate::types::mcp::HEADER_MISMATCH);
    }

    /// Tests resolving tool or prompt name.
    #[test]
    fn test_resolve_tool_name() {
        assert_eq!(
            resolve_tool_name(Some("/my_tool/"), Some("/my_tool/"), "tool name").unwrap(),
            "/my_tool/"
        );
        let err = resolve_tool_name(Some("/my_tool/"), Some("my_tool"), "tool name").unwrap_err();
        assert_eq!(err.error.code.code(), crate::types::mcp::HEADER_MISMATCH);
        assert_eq!(
            resolve_tool_name(Some("my_tool"), None, "tool name").unwrap(),
            "my_tool"
        );

        // Missing Mcp-Name header -> HeaderMismatch (-32020)
        let err = resolve_tool_name(None, Some("my_tool"), "tool name").unwrap_err();
        assert_eq!(err.error.code.code(), crate::types::mcp::HEADER_MISMATCH);

        // Empty Mcp-Name header -> InvalidParams (-32602)
        let err = resolve_tool_name(Some(""), Some("my_tool"), "tool name").unwrap_err();
        assert_eq!(err.error.code.code(), -32602);

        // Header and body tool name mismatch -> HeaderMismatch (-32020)
        let err = resolve_tool_name(Some("tool_a"), Some("tool_b"), "tool name").unwrap_err();
        assert_eq!(err.error.code.code(), crate::types::mcp::HEADER_MISMATCH);
    }

    /// Tests resolving resource URI.
    #[test]
    fn test_resolve_resource_uri() {
        assert_eq!(
            resolve_resource_uri(Some("file:///doc.txt"), Some("file:///doc.txt")).unwrap(),
            "file:///doc.txt"
        );
        assert_eq!(
            resolve_resource_uri(Some("file:///doc.txt"), None).unwrap(),
            "file:///doc.txt"
        );

        // Missing Mcp-Name header -> HeaderMismatch (-32020)
        let err = resolve_resource_uri(None, Some("file:///doc.txt")).unwrap_err();
        assert_eq!(err.error.code.code(), crate::types::mcp::HEADER_MISMATCH);

        // Empty Mcp-Name header -> InvalidParams (-32602)
        let err = resolve_resource_uri(Some(""), Some("file:///doc.txt")).unwrap_err();
        assert_eq!(err.error.code.code(), -32602);

        // Header and body URI mismatch -> HeaderMismatch (-32020)
        let err = resolve_resource_uri(Some("file:///a.txt"), Some("file:///b.txt")).unwrap_err();
        assert_eq!(err.error.code.code(), crate::types::mcp::HEADER_MISMATCH);
    }
}
