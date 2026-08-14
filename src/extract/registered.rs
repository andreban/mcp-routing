// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Extractors for registered router capabilities (`RegisteredTools`, `RegisteredPrompts`, `RegisteredResources`, `RegisteredResourceTemplates`).

use crate::extract::context::RequestContext;
use crate::extract::traits::FromRequestContext;
use crate::types::mcp::{
    prompts::Prompt,
    resources::{Resource, ResourceTemplate},
    tools::Tool,
};

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

/// Extractor for all direct resources pre-registered in the router.
///
/// Can be used in [`resources_list`](crate::McpRouter::resources_list) handlers to inspect and filter
/// pre-registered resource definitions instead of recreating them manually.
#[derive(Debug, Clone, Default)]
pub struct RegisteredResources(pub Vec<Resource>);

impl RegisteredResources {
    /// Creates a new [`RegisteredResources`] collection.
    pub fn new(resources: Vec<Resource>) -> Self {
        Self(resources)
    }

    /// Returns the underlying vector of resources.
    pub fn into_inner(self) -> Vec<Resource> {
        self.0
    }

    /// Returns a slice of the registered resources.
    pub fn as_slice(&self) -> &[Resource] {
        &self.0
    }

    /// Returns the number of registered resources.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if there are no registered resources.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns an iterator over the registered resources.
    pub fn iter(&self) -> std::slice::Iter<'_, Resource> {
        self.0.iter()
    }
}

impl std::ops::Deref for RegisteredResources {
    type Target = [Resource];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for RegisteredResources {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl IntoIterator for RegisteredResources {
    type Item = Resource;
    type IntoIter = std::vec::IntoIter<Resource>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a RegisteredResources {
    type Item = &'a Resource;
    type IntoIter = std::slice::Iter<'a, Resource>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl From<Vec<Resource>> for RegisteredResources {
    fn from(resources: Vec<Resource>) -> Self {
        Self(resources)
    }
}

/// Extractor for all resource templates pre-registered in the router.
///
/// Can be used in [`resource_templates_list`](crate::McpRouter::resource_templates_list) handlers to inspect and filter
/// pre-registered resource template definitions instead of recreating them manually.
#[derive(Debug, Clone, Default)]
pub struct RegisteredResourceTemplates(pub Vec<ResourceTemplate>);

impl RegisteredResourceTemplates {
    /// Creates a new [`RegisteredResourceTemplates`] collection.
    pub fn new(templates: Vec<ResourceTemplate>) -> Self {
        Self(templates)
    }

    /// Returns the underlying vector of resource templates.
    pub fn into_inner(self) -> Vec<ResourceTemplate> {
        self.0
    }

    /// Returns a slice of the registered resource templates.
    pub fn as_slice(&self) -> &[ResourceTemplate] {
        &self.0
    }

    /// Returns the number of registered resource templates.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if there are no registered resource templates.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns an iterator over the registered resource templates.
    pub fn iter(&self) -> std::slice::Iter<'_, ResourceTemplate> {
        self.0.iter()
    }
}

impl std::ops::Deref for RegisteredResourceTemplates {
    type Target = [ResourceTemplate];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for RegisteredResourceTemplates {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl IntoIterator for RegisteredResourceTemplates {
    type Item = ResourceTemplate;
    type IntoIter = std::vec::IntoIter<ResourceTemplate>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a RegisteredResourceTemplates {
    type Item = &'a ResourceTemplate;
    type IntoIter = std::slice::Iter<'a, ResourceTemplate>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl From<Vec<ResourceTemplate>> for RegisteredResourceTemplates {
    fn from(templates: Vec<ResourceTemplate>) -> Self {
        Self(templates)
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

impl FromRequestContext for RegisteredResources {
    type Error = std::convert::Infallible;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        Ok(ctx
            .extensions
            .get::<RegisteredResources>()
            .cloned()
            .unwrap_or_default())
    }
}

impl FromRequestContext for Option<RegisteredResources> {
    type Error = std::convert::Infallible;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        Ok(ctx.extensions.get::<RegisteredResources>().cloned())
    }
}

impl FromRequestContext for RegisteredResourceTemplates {
    type Error = std::convert::Infallible;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        Ok(ctx
            .extensions
            .get::<RegisteredResourceTemplates>()
            .cloned()
            .unwrap_or_default())
    }
}

impl FromRequestContext for Option<RegisteredResourceTemplates> {
    type Error = std::convert::Infallible;

    fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
        Ok(ctx.extensions.get::<RegisteredResourceTemplates>().cloned())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use super::*;

    #[test]
    fn test_registered_resources_and_templates_extractors() {
        let res = Resource::new("file:///test.txt", "Test");
        let tmpl = ResourceTemplate::new("file:///{path}", "Template");

        let mut ext = http::Extensions::new();
        ext.insert(RegisteredResources::new(vec![res.clone()]));
        ext.insert(RegisteredResourceTemplates::new(vec![tmpl.clone()]));

        let ctx = RequestContext::new(None, None, http::HeaderMap::new(), Arc::new(ext));

        let extracted_res = RegisteredResources::from_request_context(&ctx).unwrap();
        assert_eq!(extracted_res.len(), 1);
        assert_eq!(extracted_res[0].uri, "file:///test.txt");

        let extracted_tmpl = RegisteredResourceTemplates::from_request_context(&ctx).unwrap();
        assert_eq!(extracted_tmpl.len(), 1);
        assert_eq!(extracted_tmpl[0].uri_template, "file:///{path}");
    }
}
