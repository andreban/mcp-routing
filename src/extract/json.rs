// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};

/// A wrapper type for structured JSON content in MCP tool responses and arguments.
///
/// Tool handlers can return `Json(data)` to automatically serialize `data`
/// into [`CallToolResult.structured_content`](crate::types::mcp::tools::call::CallToolResult::structured_content).
///
/// # Example
///
/// ```rust,no_run
/// use mcp_routing::extract::Json;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct WeatherReport {
///     city: String,
///     temperature: f64,
/// }
///
/// async fn get_weather() -> Json<WeatherReport> {
///     Json(WeatherReport {
///         city: "San Francisco".to_string(),
///         temperature: 18.5,
///     })
/// }
/// ```
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Json<T>(pub T);

impl<T> Json<T> {
    /// Creates a new [`Json`] wrapper containing the given value.
    pub fn new(value: T) -> Self {
        Self(value)
    }

    /// Consumes the wrapper, returning the inner value.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> From<T> for Json<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T> Deref for Json<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Json<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> AsRef<T> for Json<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T> AsMut<T> for Json<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.0
    }
}
