// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Logging-related request extractors and state.

use crate::extract::context::RequestContext;
use crate::extract::error::ExtractionError;
use crate::extract::traits::FromRequestContext;
use crate::types::mcp::LoggingLevel;

/// Extractor for accessing the server's current dynamic logging level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CurrentLoggingLevel(pub LoggingLevel);

impl CurrentLoggingLevel {
    /// Returns the current logging level.
    pub fn level(&self) -> LoggingLevel {
        self.0
    }
}

impl std::ops::Deref for CurrentLoggingLevel {
    type Target = LoggingLevel;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for CurrentLoggingLevel {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<LoggingLevel> for CurrentLoggingLevel {
    fn from(level: LoggingLevel) -> Self {
        Self(level)
    }
}

impl From<CurrentLoggingLevel> for LoggingLevel {
    fn from(current: CurrentLoggingLevel) -> Self {
        current.0
    }
}

impl std::fmt::Display for CurrentLoggingLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromRequestContext for CurrentLoggingLevel {
    type Error = std::convert::Infallible;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        if let Some(level) = ctx.extension::<CurrentLoggingLevel>() {
            Ok(level)
        } else {
            Ok(CurrentLoggingLevel(LoggingLevel::Info))
        }
    }
}

impl FromRequestContext for LoggingLevel {
    type Error = ExtractionError;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, ExtractionError> {
        ctx.log_level().copied().ok_or_else(|| {
            ExtractionError(
                "Missing required _meta.io.modelcontextprotocol/logLevel in request".to_string(),
            )
        })
    }
}

impl FromRequestContext for Option<LoggingLevel> {
    type Error = std::convert::Infallible;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        Ok(ctx.log_level().copied())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use http::HeaderMap;

    use super::*;
    use crate::types::mcp::RequestMetaObject;

    #[test]
    fn test_current_logging_level_extractor() {
        let mut ext = http::Extensions::new();
        ext.insert(CurrentLoggingLevel(LoggingLevel::Debug));

        let ctx = RequestContext::new(
            None,
            HeaderMap::new(),
            Arc::new(ext),
        );

        let current = CurrentLoggingLevel::from_request_context(&ctx).unwrap();
        assert_eq!(current.0, LoggingLevel::Debug);
        assert_eq!(*current, LoggingLevel::Debug);
        assert_eq!(format!("{current}"), "debug");
    }

    #[test]
    fn test_request_logging_level_extractors() {
        let mut meta = RequestMetaObject {
            progress_token: None,
            client_info: None,
            client_capabilities: None,
            protocol_version: None,
            log_level: Some(LoggingLevel::Warning),
            extra: std::collections::HashMap::new(),
        };

        let ctx_with_level = RequestContext::new(
            Some(meta.clone()),
            HeaderMap::new(),
            Arc::new(http::Extensions::new()),
        );

        let level = LoggingLevel::from_request_context(&ctx_with_level).unwrap();
        assert_eq!(level, LoggingLevel::Warning);

        let opt_level = Option::<LoggingLevel>::from_request_context(&ctx_with_level).unwrap();
        assert_eq!(opt_level, Some(LoggingLevel::Warning));

        meta.log_level = None;
        let ctx_no_level = RequestContext::new(
            Some(meta),
            HeaderMap::new(),
            Arc::new(http::Extensions::new()),
        );

        assert!(LoggingLevel::from_request_context(&ctx_no_level).is_err());
        let opt_none = Option::<LoggingLevel>::from_request_context(&ctx_no_level).unwrap();
        assert_eq!(opt_none, None);
    }
}
