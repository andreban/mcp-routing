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

macro_rules! impl_registered_collection {
    (
        $(#[$meta:meta])*
        $struct_name:ident, $item_ty:ty, $doc_item:expr
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Default)]
        pub struct $struct_name(pub Vec<$item_ty>);

        impl $struct_name {
            #[doc = concat!("Creates a new [`", stringify!($struct_name), "`] collection.")]
            pub fn new(items: Vec<$item_ty>) -> Self {
                Self(items)
            }

            #[doc = concat!("Returns the underlying vector of ", $doc_item, ".")]
            pub fn into_inner(self) -> Vec<$item_ty> {
                self.0
            }

            #[doc = concat!("Returns a slice of the registered ", $doc_item, ".")]
            pub fn as_slice(&self) -> &[$item_ty] {
                &self.0
            }

            #[doc = concat!("Returns the number of registered ", $doc_item, ".")]
            pub fn len(&self) -> usize {
                self.0.len()
            }

            #[doc = concat!("Returns `true` if there are no registered ", $doc_item, ".")]
            pub fn is_empty(&self) -> bool {
                self.0.is_empty()
            }

            #[doc = concat!("Returns an iterator over the registered ", $doc_item, ".")]
            pub fn iter(&self) -> std::slice::Iter<'_, $item_ty> {
                self.0.iter()
            }
        }

        impl std::ops::Deref for $struct_name {
            type Target = [$item_ty];

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::ops::DerefMut for $struct_name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl IntoIterator for $struct_name {
            type Item = $item_ty;
            type IntoIter = std::vec::IntoIter<$item_ty>;

            fn into_iter(self) -> Self::IntoIter {
                self.0.into_iter()
            }
        }

        impl<'a> IntoIterator for &'a $struct_name {
            type Item = &'a $item_ty;
            type IntoIter = std::slice::Iter<'a, $item_ty>;

            fn into_iter(self) -> Self::IntoIter {
                self.0.iter()
            }
        }

        impl From<Vec<$item_ty>> for $struct_name {
            fn from(items: Vec<$item_ty>) -> Self {
                Self(items)
            }
        }

        impl FromRequestContext for $struct_name {
            type Error = std::convert::Infallible;

            fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
                Ok(ctx
                    .extensions
                    .get::<$struct_name>()
                    .cloned()
                    .unwrap_or_default())
            }
        }

        impl FromRequestContext for Option<$struct_name> {
            type Error = std::convert::Infallible;

            fn from_request_context(ctx: &RequestContext) -> Result<Self, Self::Error> {
                Ok(ctx.extensions.get::<$struct_name>().cloned())
            }
        }
    };
}

impl_registered_collection!(
    /// Extractor for all tools pre-registered in the router.
    ///
    /// Can be used in [`tools_list`](crate::McpRouter::tools_list) handlers to inspect and filter
    /// pre-registered tool definitions instead of recreating them manually.
    RegisteredTools,
    Tool,
    "tools"
);

impl_registered_collection!(
    /// Extractor for all prompt templates pre-registered in the router.
    ///
    /// Can be used in [`prompts_list`](crate::McpRouter::prompts_list) handlers to inspect and filter
    /// pre-registered prompt definitions instead of recreating them manually.
    RegisteredPrompts,
    Prompt,
    "prompts"
);

impl_registered_collection!(
    /// Extractor for all direct resources pre-registered in the router.
    ///
    /// Can be used in [`resources_list`](crate::McpRouter::resources_list) handlers to inspect and filter
    /// pre-registered resource definitions instead of recreating them manually.
    RegisteredResources,
    Resource,
    "resources"
);

impl_registered_collection!(
    /// Extractor for all resource templates pre-registered in the router.
    ///
    /// Can be used in [`resource_templates_list`](crate::McpRouter::resource_templates_list) handlers to inspect and filter
    /// pre-registered resource template definitions instead of recreating them manually.
    RegisteredResourceTemplates,
    ResourceTemplate,
    "resource templates"
);

#[cfg(test)]
mod tests {
    //! Unit tests for registered collection extractors.

    use super::*;
    use std::sync::Arc;

    /// Tests extraction of `RegisteredResources` and `RegisteredResourceTemplates` from request context extensions.
    #[test]
    fn test_registered_resources_and_templates_extractors() {
        let res = Resource::new("file:///test.txt", "Test");
        let tmpl = ResourceTemplate::new("file:///{path}", "Template");

        let mut ext = http::Extensions::new();
        ext.insert(RegisteredResources::new(vec![res.clone()]));
        ext.insert(RegisteredResourceTemplates::new(vec![tmpl.clone()]));

        let ctx = RequestContext::new(None, http::HeaderMap::new(), Arc::new(ext));

        let extracted_res = RegisteredResources::from_request_context(&ctx).unwrap();
        assert_eq!(extracted_res.len(), 1);
        assert_eq!(extracted_res[0].uri, "file:///test.txt");

        let extracted_tmpl = RegisteredResourceTemplates::from_request_context(&ctx).unwrap();
        assert_eq!(extracted_tmpl.len(), 1);
        assert_eq!(extracted_tmpl[0].uri_template, "file:///{path}");
    }
}
