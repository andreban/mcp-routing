// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! RFC 2047-style Base64 sentinel header encoding and decoding.

use std::borrow::Cow;

use base64::prelude::*;

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
        if let Ok(bytes) = decoded
            && let Ok(utf8_str) = String::from_utf8(bytes)
        {
            return Cow::Owned(utf8_str);
        }
    }
    Cow::Borrowed(raw_value)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests RFC 2047-style Base64 sentinel header decoding.
    #[test]
    fn test_decode_sentinel_header() {
        // Plain ASCII string (no sentinel) -> borrowed
        assert_eq!(decode_sentinel_header("my_tool"), "my_tool");
        assert!(matches!(
            decode_sentinel_header("my_tool"),
            Cow::Borrowed(_)
        ));

        // Base64 sentinel encoding
        assert_eq!(decode_sentinel_header("=?base64?bXlfdG9vbA==?="), "my_tool");
        assert!(matches!(
            decode_sentinel_header("=?base64?bXlfdG9vbA==?="),
            Cow::Owned(_)
        ));

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
        assert_eq!(
            decode_sentinel_header("=?base64?incomplete"),
            "=?base64?incomplete"
        );
        assert_eq!(
            decode_sentinel_header("=?base64?invalid!@#base64?="),
            "=?base64?invalid!@#base64?="
        );
        assert_eq!(decode_sentinel_header(""), "");
    }
}
