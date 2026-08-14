// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Extractor traits for request contexts.

use crate::extract::context::RequestContext;

/// Trait for types that can be extracted from a [`RequestContext`].
pub trait FromRequestContext: Sized {
    type Error: std::fmt::Display + Send;
    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error>;
}
