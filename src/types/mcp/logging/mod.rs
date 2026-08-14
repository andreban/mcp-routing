// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::{
    jsonrpc::{JsonRpcRequest, JsonRpcResultResponse},
    mcp::{LoggingLevel, RequestMetaObject, ResultMetaObject},
};

pub type SetLevelRequest = JsonRpcRequest<SetLevelParams>;
pub type SetLevelResultResponse = JsonRpcResultResponse<SetLevelResult>;

/// Parameters for a `logging/setLevel` request.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#setlevelrequest>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetLevelParams {
    /// The level of logging that the client wants to receive from the server.
    pub level: LoggingLevel,
    /// Protocol-level request metadata.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<RequestMetaObject>,
    /// Additional unrecognized or custom metadata properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extras: HashMap<String, Value>,
}

impl SetLevelParams {
    /// Creates a new [`SetLevelParams`] with the specified logging level.
    pub fn new(level: LoggingLevel) -> Self {
        Self {
            level,
            meta: None,
            extras: HashMap::new(),
        }
    }

    /// Sets request metadata.
    pub fn with_meta(mut self, meta: RequestMetaObject) -> Self {
        self.meta = Some(meta);
        self
    }
}

impl From<LoggingLevel> for SetLevelParams {
    fn from(level: LoggingLevel) -> Self {
        Self::new(level)
    }
}

/// Result payload for a `logging/setLevel` request.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#setlevelresult>
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetLevelResult {
    /// Protocol-level response metadata.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<ResultMetaObject>,
    /// Additional unrecognized or custom metadata properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extras: HashMap<String, Value>,
}

impl SetLevelResult {
    /// Creates a new empty [`SetLevelResult`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an empty [`SetLevelResult`].
    pub fn empty() -> Self {
        Self::default()
    }

    /// Sets response metadata.
    pub fn with_meta(mut self, meta: ResultMetaObject) -> Self {
        self.meta = Some(meta);
        self
    }
}

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
    use super::*;

    #[test]
    fn test_set_level_params_serde() {
        let json_data = serde_json::json!({
            "level": "debug"
        });

        let params: SetLevelParams = serde_json::from_value(json_data).unwrap();
        assert_eq!(params.level, LoggingLevel::Debug);

        let reserialized = serde_json::to_value(&params).unwrap();
        assert_eq!(reserialized["level"], "debug");
    }

    #[test]
    fn test_set_level_result_serde() {
        let result = SetLevelResult::new();
        let json_val = serde_json::to_value(&result).unwrap();
        assert_eq!(json_val, serde_json::json!({}));

        let parsed: SetLevelResult = serde_json::from_value(json_val).unwrap();
        assert!(parsed.meta.is_none());
        assert!(parsed.extras.is_empty());
    }

    #[test]
    fn test_set_level_request_serde() {
        let json_data = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "req-1",
            "method": "logging/setLevel",
            "params": {
                "level": "warning"
            }
        });

        let req: SetLevelRequest = serde_json::from_value(json_data).unwrap();
        assert_eq!(req.method, "logging/setLevel");
        assert_eq!(req.params.unwrap().level, LoggingLevel::Warning);
    }

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
