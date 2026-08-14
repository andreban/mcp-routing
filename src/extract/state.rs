// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Extension and State extractors.

use crate::extract::context::RequestContext;
use crate::extract::error::ExtractionError;
use crate::extract::traits::FromRequestContext;

/// Extractor for request extensions provided by Tower middleware or web frameworks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Extension<T>(pub T);

impl<T> std::ops::Deref for Extension<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for Extension<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> From<T> for Extension<T> {
    fn from(inner: T) -> Self {
        Self(inner)
    }
}

/// Extractor for application state passed via `with_state` or request extensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct State<T>(pub T);

impl<T> std::ops::Deref for State<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for State<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> From<T> for State<T> {
    fn from(inner: T) -> Self {
        Self(inner)
    }
}

impl<T> FromRequestContext for Extension<T>
where
    T: Clone + Send + Sync + 'static,
{
    type Error = ExtractionError;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        ctx.extensions
            .get::<T>()
            .cloned()
            .map(Extension)
            .ok_or_else(|| {
                ExtractionError(format!(
                    "Missing request extension: {}",
                    std::any::type_name::<T>()
                ))
            })
    }
}

impl<T> FromRequestContext for Option<Extension<T>>
where
    T: Clone + Send + Sync + 'static,
{
    type Error = std::convert::Infallible;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        Ok(ctx.extensions.get::<T>().cloned().map(Extension))
    }
}

impl<T> FromRequestContext for State<T>
where
    T: Clone + Send + Sync + 'static,
{
    type Error = ExtractionError;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        ctx.extensions
            .get::<T>()
            .cloned()
            .map(State)
            .ok_or_else(|| {
                ExtractionError(format!("Missing state: {}", std::any::type_name::<T>()))
            })
    }
}

impl<T> FromRequestContext for Option<State<T>>
where
    T: Clone + Send + Sync + 'static,
{
    type Error = std::convert::Infallible;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        Ok(ctx.extensions.get::<T>().cloned().map(State))
    }
}
