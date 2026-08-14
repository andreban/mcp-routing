// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Error types for request extraction.

/// Error encountered during request extraction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionError(pub String);

impl std::fmt::Display for ExtractionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ExtractionError {}
