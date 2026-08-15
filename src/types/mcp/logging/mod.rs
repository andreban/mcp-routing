// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::mcp::{LoggingLevel, RequestMetaObject};

/// Parameters for a `notifications/message` or `logging/message` notification.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#loggingmessageparams>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggingMessageParams {
    /// The severity of this log message.
    pub level: LoggingLevel,
    /// An optional name of the logger issuing this message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logger: Option<String>,
    /// The data to be logged, such as a string message or structured object.
    pub data: Value,
    /// Protocol-level metadata.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<RequestMetaObject>,
    /// Additional unrecognized or custom metadata properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extras: HashMap<String, Value>,
}

impl LoggingMessageParams {
    /// Creates a new [`LoggingMessageParams`] with the given level and data.
    pub fn new(level: LoggingLevel, data: impl Into<Value>) -> Self {
        Self {
            level,
            logger: None,
            data: data.into(),
            meta: None,
            extras: HashMap::new(),
        }
    }

    /// Creates a new text-based [`LoggingMessageParams`].
    pub fn text(level: LoggingLevel, message: impl Into<String>) -> Self {
        Self::new(level, Value::String(message.into()))
    }

    /// Sets the logger name.
    pub fn with_logger(mut self, logger: impl Into<String>) -> Self {
        self.logger = Some(logger.into());
        self
    }

    /// Sets metadata.
    pub fn with_meta(mut self, meta: RequestMetaObject) -> Self {
        self.meta = Some(meta);
        self
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for MCP logging message parameters.

    use super::*;

    /// Tests serialization and deserialization of [`LoggingMessageParams`].
    #[test]
    fn test_logging_message_params_serde() {
        let msg = LoggingMessageParams::text(LoggingLevel::Info, "Server started successfully")
            .with_logger("server::init");

        let json_val = serde_json::to_value(&msg).unwrap();
        assert_eq!(json_val["level"], "info");
        assert_eq!(json_val["logger"], "server::init");
        assert_eq!(json_val["data"], "Server started successfully");

        let parsed: LoggingMessageParams = serde_json::from_value(json_val).unwrap();
        assert_eq!(parsed.level, LoggingLevel::Info);
        assert_eq!(parsed.logger.as_deref(), Some("server::init"));
    }
}
