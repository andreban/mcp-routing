// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::capabilities::ClientCapabilities;
use super::info::Implementation;

/// A progress token, used to associate progress notifications with the original request.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#progresstoken>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProgressToken {
    /// A numeric progress token.
    Number(f32),
    /// A string progress token.
    String(String),
}

/// The severity level of a log message.
///
/// Maps to RFC-5424 syslog severities.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#logginglevel>
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LoggingLevel {
    /// Detailed debugging information.
    Debug,
    /// Normal operational messages.
    Info,
    /// Normal but significant conditions.
    Notice,
    /// Warning conditions.
    Warning,
    /// Error conditions.
    Error,
    /// Critical conditions.
    Critical,
    /// Action must be taken immediately.
    Alert,
    /// System is unusable.
    Emergency,
}

impl LoggingLevel {
    /// Returns the RFC-5424 numerical severity code (0 = Emergency, 7 = Debug).
    pub const fn severity_code(&self) -> u8 {
        match self {
            LoggingLevel::Emergency => 0,
            LoggingLevel::Alert => 1,
            LoggingLevel::Critical => 2,
            LoggingLevel::Error => 3,
            LoggingLevel::Warning => 4,
            LoggingLevel::Notice => 5,
            LoggingLevel::Info => 6,
            LoggingLevel::Debug => 7,
        }
    }

    /// Returns the severity rank (0 = Debug, 7 = Emergency) where higher values denote higher severity.
    pub const fn severity_rank(&self) -> u8 {
        7 - self.severity_code()
    }

    /// Checks if this log level meets or exceeds the specified threshold level.
    pub fn is_at_least(&self, threshold: LoggingLevel) -> bool {
        self.severity_code() <= threshold.severity_code()
    }

    /// Converts this [`LoggingLevel`] to the corresponding [`tracing::Level`].
    pub fn as_tracing_level(&self) -> tracing::Level {
        match self {
            LoggingLevel::Emergency
            | LoggingLevel::Alert
            | LoggingLevel::Critical
            | LoggingLevel::Error => tracing::Level::ERROR,
            LoggingLevel::Warning => tracing::Level::WARN,
            LoggingLevel::Notice | LoggingLevel::Info => tracing::Level::INFO,
            LoggingLevel::Debug => tracing::Level::DEBUG,
        }
    }

    /// Returns the string representation matching the MCP specification.
    pub const fn as_str(&self) -> &'static str {
        match self {
            LoggingLevel::Debug => "debug",
            LoggingLevel::Info => "info",
            LoggingLevel::Notice => "notice",
            LoggingLevel::Warning => "warning",
            LoggingLevel::Error => "error",
            LoggingLevel::Critical => "critical",
            LoggingLevel::Alert => "alert",
            LoggingLevel::Emergency => "emergency",
        }
    }
}

impl std::fmt::Display for LoggingLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for LoggingLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "debug" => Ok(LoggingLevel::Debug),
            "info" => Ok(LoggingLevel::Info),
            "notice" => Ok(LoggingLevel::Notice),
            "warning" | "warn" => Ok(LoggingLevel::Warning),
            "error" | "err" => Ok(LoggingLevel::Error),
            "critical" | "crit" => Ok(LoggingLevel::Critical),
            "alert" => Ok(LoggingLevel::Alert),
            "emergency" | "emerg" => Ok(LoggingLevel::Emergency),
            other => Err(format!("Unknown logging level: '{other}'")),
        }
    }
}

/// Additional metadata associated with a request or entity.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#metaobject>
pub type MetaObject = HashMap<String, Value>;

/// An object containing metadata for a result.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#resultmetaobject>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultMetaObject {
    /// Identifies the server software producing the response.
    /// Servers SHOULD include this field on every response unless specifically configured not to do so.
    #[serde(
        rename = "io.modelcontextprotocol/serverInfo",
        skip_serializing_if = "Option::is_none"
    )]
    pub server_info: Option<Implementation>,
    /// Additional metadata properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, Value>,
}

/// An object containing metadata for a request.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#requestmetaobject>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestMetaObject {
    /// A progress token used to associate progress notifications with the original request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress_token: Option<ProgressToken>,
    /// Identifies the client software producing the request.
    #[serde(
        rename = "io.modelcontextprotocol/clientInfo",
        skip_serializing_if = "Option::is_none"
    )]
    pub client_info: Option<Implementation>,
    /// Capabilities supported by the client for this request.
    #[serde(
        rename = "io.modelcontextprotocol/clientCapabilities",
        skip_serializing_if = "Option::is_none"
    )]
    pub client_capabilities: Option<ClientCapabilities>,
    /// Specifies the MCP protocol version being used for the request.
    #[serde(
        rename = "io.modelcontextprotocol/protocolVersion",
        skip_serializing_if = "Option::is_none"
    )]
    pub protocol_version: Option<String>,
    /// Desired log level for the request.
    #[serde(
        rename = "io.modelcontextprotocol/logLevel",
        skip_serializing_if = "Option::is_none"
    )]
    pub log_level: Option<LoggingLevel>,
    /// Additional metadata properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests serialization and deserialization of [`ResultMetaObject`].
    #[test]
    fn test_result_meta_object_serde() {
        let json_data = serde_json::json!({
            "io.modelcontextprotocol/serverInfo": {
                "name": "test-server",
                "version": "1.0.0"
            },
            "custom/meta": "value"
        });

        let meta: ResultMetaObject = serde_json::from_value(json_data).unwrap();
        assert_eq!(meta.server_info.as_ref().unwrap().name, "test-server");
        assert_eq!(meta.extra.get("custom/meta").unwrap(), "value");

        let reserialized = serde_json::to_value(&meta).unwrap();
        assert_eq!(
            reserialized["io.modelcontextprotocol/serverInfo"]["name"],
            "test-server"
        );
        assert_eq!(reserialized["custom/meta"], "value");
    }

    /// Tests serialization and deserialization of [`RequestMetaObject`] with [`LoggingLevel`].
    #[test]
    fn test_request_meta_object_with_log_level() {
        let json_data = serde_json::json!({
            "io.modelcontextprotocol/logLevel": "debug"
        });

        let meta: RequestMetaObject = serde_json::from_value(json_data).unwrap();
        assert!(matches!(meta.log_level, Some(LoggingLevel::Debug)));

        let reserialized = serde_json::to_value(&meta).unwrap();
        assert_eq!(reserialized["io.modelcontextprotocol/logLevel"], "debug");
    }

    /// Tests all RFC-5424 [`LoggingLevel`] enum variants.
    #[test]
    fn test_logging_levels_serde() {
        let levels = vec![
            (LoggingLevel::Debug, "\"debug\""),
            (LoggingLevel::Info, "\"info\""),
            (LoggingLevel::Notice, "\"notice\""),
            (LoggingLevel::Warning, "\"warning\""),
            (LoggingLevel::Error, "\"error\""),
            (LoggingLevel::Critical, "\"critical\""),
            (LoggingLevel::Alert, "\"alert\""),
            (LoggingLevel::Emergency, "\"emergency\""),
        ];

        for (level, expected_json) in levels {
            let serialized = serde_json::to_string(&level).unwrap();
            assert_eq!(serialized, expected_json);

            let deserialized: LoggingLevel = serde_json::from_str(expected_json).unwrap();
            assert_eq!(serde_json::to_string(&deserialized).unwrap(), expected_json);
        }
    }

    /// Tests numeric and string [`ProgressToken`] parsing.
    #[test]
    fn test_progress_token_serde() {
        let num_token: ProgressToken = serde_json::from_value(serde_json::json!(42.0)).unwrap();
        assert!(matches!(num_token, ProgressToken::Number(n) if (n - 42.0).abs() < f32::EPSILON));

        let str_token: ProgressToken =
            serde_json::from_value(serde_json::json!("tok-123")).unwrap();
        assert!(matches!(str_token, ProgressToken::String(s) if s == "tok-123"));
    }

    /// Tests [`LoggingLevel`] methods, RFC-5424 ordering, Display, and FromStr.
    #[test]
    fn test_logging_level_methods_and_parsing() {
        assert_eq!(LoggingLevel::Emergency.severity_code(), 0);
        assert_eq!(LoggingLevel::Debug.severity_code(), 7);
        assert_eq!(LoggingLevel::Emergency.severity_rank(), 7);
        assert_eq!(LoggingLevel::Debug.severity_rank(), 0);

        assert!(LoggingLevel::Emergency.is_at_least(LoggingLevel::Error));
        assert!(LoggingLevel::Error.is_at_least(LoggingLevel::Error));
        assert!(LoggingLevel::Error.is_at_least(LoggingLevel::Info));
        assert!(!LoggingLevel::Info.is_at_least(LoggingLevel::Warning));
        assert!(!LoggingLevel::Debug.is_at_least(LoggingLevel::Info));

        assert_eq!(LoggingLevel::Debug.to_string(), "debug");
        assert_eq!(LoggingLevel::Emergency.to_string(), "emergency");

        assert_eq!("debug".parse::<LoggingLevel>().unwrap(), LoggingLevel::Debug);
        assert_eq!("WARN".parse::<LoggingLevel>().unwrap(), LoggingLevel::Warning);
        assert_eq!("Error".parse::<LoggingLevel>().unwrap(), LoggingLevel::Error);
        assert_eq!("emerg".parse::<LoggingLevel>().unwrap(), LoggingLevel::Emergency);
        assert!("invalid".parse::<LoggingLevel>().is_err());

        assert_eq!(LoggingLevel::Debug.as_tracing_level(), tracing::Level::DEBUG);
        assert_eq!(LoggingLevel::Info.as_tracing_level(), tracing::Level::INFO);
        assert_eq!(LoggingLevel::Notice.as_tracing_level(), tracing::Level::INFO);
        assert_eq!(LoggingLevel::Warning.as_tracing_level(), tracing::Level::WARN);
        assert_eq!(LoggingLevel::Error.as_tracing_level(), tracing::Level::ERROR);
        assert_eq!(LoggingLevel::Critical.as_tracing_level(), tracing::Level::ERROR);
        assert_eq!(LoggingLevel::Alert.as_tracing_level(), tracing::Level::ERROR);
        assert_eq!(LoggingLevel::Emergency.as_tracing_level(), tracing::Level::ERROR);
    }
}
