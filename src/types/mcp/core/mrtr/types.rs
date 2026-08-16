// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Result type discriminator and constants for MCP MRTR operations.

use std::fmt;
use serde::{Deserialize, Serialize};

/// Result type discriminator constant for complete results.
pub const RESULT_TYPE_COMPLETE: &str = "complete";

/// Result type discriminator constant for results requiring additional client input.
pub const RESULT_TYPE_INPUT_REQUIRED: &str = "input_required";

/// Indicates the type of a `Result` object, allowing the client to determine how to parse the response.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#resulttype>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultType {
    /// The request completed successfully and contains the final result.
    #[serde(rename = "complete")]
    Complete,
    /// The request requires additional input before it can be completed.
    #[serde(rename = "input_required")]
    InputRequired,
    /// Custom or forward-compatible result type discriminator.
    #[serde(untagged)]
    Custom(String),
}

impl ResultType {
    /// Returns the string representation of the result type.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Complete => RESULT_TYPE_COMPLETE,
            Self::InputRequired => RESULT_TYPE_INPUT_REQUIRED,
            Self::Custom(s) => s.as_str(),
        }
    }

    /// Returns `true` if this is [`ResultType::Complete`].
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Complete)
    }

    /// Returns `true` if this is [`ResultType::InputRequired`].
    pub fn is_input_required(&self) -> bool {
        matches!(self, Self::InputRequired)
    }
}

impl fmt::Display for ResultType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl AsRef<str> for ResultType {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<&str> for ResultType {
    fn from(s: &str) -> Self {
        match s {
            RESULT_TYPE_COMPLETE => Self::Complete,
            RESULT_TYPE_INPUT_REQUIRED => Self::InputRequired,
            other => Self::Custom(other.to_string()),
        }
    }
}

impl From<String> for ResultType {
    fn from(s: String) -> Self {
        match s.as_str() {
            RESULT_TYPE_COMPLETE => Self::Complete,
            RESULT_TYPE_INPUT_REQUIRED => Self::InputRequired,
            _ => Self::Custom(s),
        }
    }
}
