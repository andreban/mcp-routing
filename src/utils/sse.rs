// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Server-Sent Events (SSE) Formatting Utilities
//!
//! Formatting helpers for SSE frames and JSON-RPC messages over `text/event-stream`.

use bytes::Bytes;
use serde::Serialize;

/// Formats a Server-Sent Events (SSE) message frame with an optional event name and UTF-8 data payload.
pub fn format_sse_frame(event: Option<&str>, data: &str) -> Bytes {
    let mut frame = String::new();
    if let Some(event_name) = event {
        frame.push_str("event: ");
        frame.push_str(event_name);
        frame.push('\n');
    }
    for line in data.lines() {
        frame.push_str("data: ");
        frame.push_str(line);
        frame.push('\n');
    }
    frame.push('\n');
    Bytes::from(frame)
}

/// Serializes a value to JSON and formats it as an SSE `event: message` frame.
pub fn format_sse_message<T: Serialize>(val: &T) -> Result<Bytes, serde_json::Error> {
    let json_str = serde_json::to_string(val)?;
    Ok(format_sse_frame(Some("message"), &json_str))
}

#[cfg(test)]
mod tests {
    //! Unit tests for SSE frame and message formatting utilities.

    use super::*;

    /// Tests single-line and multi-line SSE frame formatting with and without event headers.
    #[test]
    fn test_format_sse_frame() {
        let frame = format_sse_frame(Some("message"), "hello");
        assert_eq!(
            std::str::from_utf8(&frame).unwrap(),
            "event: message\ndata: hello\n\n"
        );

        let frame_no_event = format_sse_frame(None, "hello\nworld");
        assert_eq!(
            std::str::from_utf8(&frame_no_event).unwrap(),
            "data: hello\ndata: world\n\n"
        );
    }

    /// Tests JSON value serialization into SSE `event: message` frames.
    #[test]
    fn test_format_sse_message() {
        let data = serde_json::json!({"jsonrpc": "2.0", "method": "ping"});
        let frame = format_sse_message(&data).unwrap();
        let s = std::str::from_utf8(&frame).unwrap();
        assert!(s.starts_with("event: message\ndata: "));
        assert!(s.ends_with("\n\n"));
        assert!(s.contains("\"method\":\"ping\""));
    }
}
