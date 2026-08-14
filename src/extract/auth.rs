// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! HTTP authentication extractors (`Authorization` and `BearerAuth`).

use crate::extract::context::RequestContext;
use crate::extract::error::ExtractionError;
use crate::extract::traits::FromRequestContext;

/// An extractor for the raw HTTP `Authorization` header value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Authorization(pub String);

impl Authorization {
    /// Creates a new [`Authorization`].
    pub fn new(auth: impl Into<String>) -> Self {
        Self(auth.into())
    }

    /// Returns the raw authorization header value as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::ops::Deref for Authorization {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for Authorization {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Authorization {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for Authorization {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for Authorization {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// An extractor for a Bearer authentication token from the `Authorization` header.
///
/// Validates that the `Authorization` header begins with `Bearer ` (case-insensitive)
/// and extracts the trimmed token value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BearerAuth(pub String);

impl BearerAuth {
    /// Creates a new [`BearerAuth`].
    pub fn new(token: impl Into<String>) -> Self {
        Self(token.into())
    }

    /// Returns the bearer token string as a string slice.
    pub fn token(&self) -> &str {
        &self.0
    }

    /// Returns the bearer token string as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::ops::Deref for BearerAuth {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for BearerAuth {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for BearerAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for BearerAuth {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for BearerAuth {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl FromRequestContext for Authorization {
    type Error = ExtractionError;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        ctx.authorization()
            .map(|s| Authorization(s.to_string()))
            .ok_or_else(|| ExtractionError("Missing required Authorization header".to_string()))
    }
}

impl FromRequestContext for Option<Authorization> {
    type Error = std::convert::Infallible;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        Ok(ctx.authorization().map(|s| Authorization(s.to_string())))
    }
}

impl FromRequestContext for BearerAuth {
    type Error = ExtractionError;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        match ctx.authorization() {
            None => Err(ExtractionError(
                "Missing required Authorization header".to_string(),
            )),
            Some(auth) => {
                let trimmed = auth.trim();
                if trimmed.len() >= 7 && trimmed[..7].eq_ignore_ascii_case("bearer ") {
                    Ok(BearerAuth(trimmed[7..].trim().to_string()))
                } else {
                    Err(ExtractionError(
                        "Invalid Authorization header: expected Bearer token".to_string(),
                    ))
                }
            }
        }
    }
}

impl FromRequestContext for Option<BearerAuth> {
    type Error = std::convert::Infallible;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        Ok(ctx.bearer_token().map(|t| BearerAuth(t.to_string())))
    }
}
