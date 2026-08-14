// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Request metadata extractor.

use crate::extract::context::RequestContext;
use crate::extract::error::ExtractionError;
use crate::extract::traits::FromRequestContext;
use crate::types::mcp::RequestMetaObject;

/// Extractor for protocol-level request metadata ([`RequestMetaObject`]).
#[derive(Debug, Clone)]
pub struct Meta(pub RequestMetaObject);

impl std::ops::Deref for Meta {
    type Target = RequestMetaObject;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Meta {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<RequestMetaObject> for Meta {
    fn from(meta: RequestMetaObject) -> Self {
        Self(meta)
    }
}

impl FromRequestContext for Meta {
    type Error = ExtractionError;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        ctx.meta.clone().map(Meta).ok_or_else(|| {
            ExtractionError("Missing required _meta in request parameters".to_string())
        })
    }
}

impl FromRequestContext for Option<Meta> {
    type Error = std::convert::Infallible;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        Ok(ctx.meta.clone().map(Meta))
    }
}
