// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Extractors for registered router capabilities (`RegisteredTools` and `RegisteredPrompts`).

use crate::extract::context::RequestContext;
use crate::extract::traits::FromRequestContext;
use crate::types::mcp::{prompts::Prompt, tools::Tool};

/// Extractor for all tools pre-registered in the router.
///
/// Can be used in [`tools_list`](crate::McpRouter::tools_list) handlers to inspect and filter
/// pre-registered tool definitions instead of recreating them manually.
#[derive(Debug, Clone, Default)]
pub struct RegisteredTools(pub Vec<Tool>);

impl RegisteredTools {
    /// Creates a new [`RegisteredTools`] collection.
    pub fn new(tools: Vec<Tool>) -> Self {
        Self(tools)
    }

    /// Returns the underlying vector of tools.
    pub fn into_inner(self) -> Vec<Tool> {
        self.0
    }

    /// Returns a slice of the registered tools.
    pub fn as_slice(&self) -> &[Tool] {
        &self.0
    }

    /// Returns the number of registered tools.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if there are no registered tools.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns an iterator over the registered tools.
    pub fn iter(&self) -> std::slice::Iter<'_, Tool> {
        self.0.iter()
    }
}

impl std::ops::Deref for RegisteredTools {
    type Target = [Tool];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for RegisteredTools {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl IntoIterator for RegisteredTools {
    type Item = Tool;
    type IntoIter = std::vec::IntoIter<Tool>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a RegisteredTools {
    type Item = &'a Tool;
    type IntoIter = std::slice::Iter<'a, Tool>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl From<Vec<Tool>> for RegisteredTools {
    fn from(tools: Vec<Tool>) -> Self {
        Self(tools)
    }
}

/// Extractor for all prompt templates pre-registered in the router.
///
/// Can be used in [`prompts_list`](crate::McpRouter::prompts_list) handlers to inspect and filter
/// pre-registered prompt definitions instead of recreating them manually.
#[derive(Debug, Clone, Default)]
pub struct RegisteredPrompts(pub Vec<Prompt>);

impl RegisteredPrompts {
    /// Creates a new [`RegisteredPrompts`] collection.
    pub fn new(prompts: Vec<Prompt>) -> Self {
        Self(prompts)
    }

    /// Returns the underlying vector of prompts.
    pub fn into_inner(self) -> Vec<Prompt> {
        self.0
    }

    /// Returns a slice of the registered prompts.
    pub fn as_slice(&self) -> &[Prompt] {
        &self.0
    }

    /// Returns the number of registered prompts.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if there are no registered prompts.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns an iterator over the registered prompts.
    pub fn iter(&self) -> std::slice::Iter<'_, Prompt> {
        self.0.iter()
    }
}

impl std::ops::Deref for RegisteredPrompts {
    type Target = [Prompt];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for RegisteredPrompts {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl IntoIterator for RegisteredPrompts {
    type Item = Prompt;
    type IntoIter = std::vec::IntoIter<Prompt>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a RegisteredPrompts {
    type Item = &'a Prompt;
    type IntoIter = std::slice::Iter<'a, Prompt>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl From<Vec<Prompt>> for RegisteredPrompts {
    fn from(prompts: Vec<Prompt>) -> Self {
        Self(prompts)
    }
}

impl FromRequestContext for RegisteredTools {
    type Error = std::convert::Infallible;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        Ok(ctx
            .extensions
            .get::<RegisteredTools>()
            .cloned()
            .unwrap_or_default())
    }
}

impl FromRequestContext for Option<RegisteredTools> {
    type Error = std::convert::Infallible;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        Ok(ctx.extensions.get::<RegisteredTools>().cloned())
    }
}

impl FromRequestContext for RegisteredPrompts {
    type Error = std::convert::Infallible;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        Ok(ctx
            .extensions
            .get::<RegisteredPrompts>()
            .cloned()
            .unwrap_or_default())
    }
}

impl FromRequestContext for Option<RegisteredPrompts> {
    type Error = std::convert::Infallible;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        Ok(ctx.extensions.get::<RegisteredPrompts>().cloned())
    }
}
