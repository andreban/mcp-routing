// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! RFC 2047-style Base64 sentinel header encoding and decoding.

use std::borrow::Cow;

use base64::prelude::*;

const SENTINEL_PREFIX: &str = "=?base64?";
const SENTINEL_SUFFIX: &str = "?=";

/// Decodes a Base64 sentinel encoded header value (`=?base64?<encoded>?=`).
///
/// According to the MCP Streamable HTTP specification, values of `Mcp-Name` and `Mcp-Param-*`
/// headers that cannot be represented as plain ASCII header values use this sentinel format.
/// The `=?base64?` prefix and `?=` suffix are case-sensitive, and the payload is standard
/// (padded) Base64 of the UTF-8 value.
///
/// Values that do not use the sentinel format are returned unchanged (`Cow::Borrowed`).
/// Values that use the sentinel format but do not decode to valid UTF-8 are rejected.
pub(crate) fn decode_sentinel_header(raw_value: &str) -> Result<Cow<'_, str>, String> {
    let Some(payload) = raw_value
        .strip_prefix(SENTINEL_PREFIX)
        .and_then(|rest| rest.strip_suffix(SENTINEL_SUFFIX))
    else {
        return Ok(Cow::Borrowed(raw_value));
    };

    let bytes = BASE64_STANDARD
        .decode(payload)
        .map_err(|err| format!("invalid Base64 sentinel value '{raw_value}': {err}"))?;
    String::from_utf8(bytes)
        .map(Cow::Owned)
        .map_err(|_| format!("Base64 sentinel value '{raw_value}' is not valid UTF-8"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests Base64 sentinel header decoding.
    #[test]
    fn test_decode_sentinel_header() {
        // Plain ASCII string (no sentinel) -> borrowed
        assert_eq!(decode_sentinel_header("my_tool").unwrap(), "my_tool");
        assert!(matches!(
            decode_sentinel_header("my_tool").unwrap(),
            Cow::Borrowed(_)
        ));

        // Base64 sentinel encoding
        assert_eq!(
            decode_sentinel_header("=?base64?bXlfdG9vbA==?=").unwrap(),
            "my_tool"
        );
        assert!(matches!(
            decode_sentinel_header("=?base64?bXlfdG9vbA==?=").unwrap(),
            Cow::Owned(_)
        ));

        // Non-ASCII UTF-8 string: "Hello, 世界!" in base64 is "SGVsbG8sIOS4lueVjCE="
        assert_eq!(
            decode_sentinel_header("=?base64?SGVsbG8sIOS4lueVjCE=?=").unwrap(),
            "Hello, 世界!"
        );

        // Not a sentinel: incomplete markers and empty values are plain values
        assert_eq!(
            decode_sentinel_header("=?base64?incomplete").unwrap(),
            "=?base64?incomplete"
        );
        assert_eq!(decode_sentinel_header("").unwrap(), "");

        // The markers are case-sensitive: an uppercase prefix is a plain value
        assert_eq!(
            decode_sentinel_header("=?BASE64?bXlfdG9vbA==?=").unwrap(),
            "=?BASE64?bXlfdG9vbA==?="
        );
    }

    /// Tests that sentinel values with an invalid payload are rejected.
    #[test]
    fn test_decode_sentinel_header_rejects_invalid_payloads() {
        // Not Base64
        assert!(decode_sentinel_header("=?base64?invalid!@#base64?=").is_err());
        // Missing padding
        assert!(decode_sentinel_header("=?base64?bXlfdG9vbA?=").is_err());
        // URL-safe alphabet ("echo_世界" -> standard "ZWNob1/kuJbnlYw=")
        assert!(decode_sentinel_header("=?base64?ZWNob1_kuJbnlYw=?=").is_err());
        // Whitespace inside the payload
        assert!(decode_sentinel_header("=?base64? bXlfdG9vbA== ?=").is_err());
        // Valid Base64 but not UTF-8 (0xFF 0xFE)
        assert!(decode_sentinel_header("=?base64?//4=?=").is_err());
    }
}
