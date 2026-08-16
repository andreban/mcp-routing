// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Builder methods for resource and resource template registration, listing, and caching.

use std::sync::Arc;

use crate::resources::{
    IntoResourceHandler, IntoResourceTemplatesListHandler, IntoResourcesListHandler,
};
use crate::router::McpRouter;
use crate::types::mcp::{
    CacheScope, ResourcesCapability,
    resources::{Resource, ResourceTemplate},
};

impl McpRouter {
    /// Sets the time-to-live (`ttl_ms`) and cache scope for `resources/list` responses.
    pub fn resources_list_cache(
        mut self,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) -> Self {
        Arc::make_mut(&mut self.inner)
            .resources
            .set_list_cache(ttl_ms, cache_scope);
        self
    }

    /// Sets the time-to-live (`ttl_ms`) in milliseconds for `resources/list` responses.
    pub fn resources_list_ttl(mut self, ttl_ms: u64) -> Self {
        Arc::make_mut(&mut self.inner).resources.list_ttl_ms = Some(ttl_ms);
        self
    }

    /// Sets the cache scope for `resources/list` responses.
    pub fn resources_list_cache_scope(mut self, cache_scope: CacheScope) -> Self {
        Arc::make_mut(&mut self.inner).resources.list_cache_scope = Some(cache_scope);
        self
    }

    /// Registers a custom handler function for generating the resources list (`resources/list`).
    ///
    /// The handler function can take request extractors (such as [`RequestContext`](crate::extract::RequestContext),
    /// [`State`](crate::extract::State), [`Extension`](crate::extract::Extension),
    /// [`Authorization`](crate::extract::Authorization), or [`BearerAuth`](crate::extract::BearerAuth))
    /// and optionally a `cursor: Option<String>` parameter, and return any type implementing [`IntoResourcesListResult`](crate::resources::IntoResourcesListResult).
    pub fn resources_list<H, T>(mut self, handler: H) -> Self
    where
        H: IntoResourcesListHandler<T>,
        T: 'static,
    {
        let inner = Arc::make_mut(&mut self.inner);
        if inner.server.capabilities.resources.is_none() {
            inner.server.capabilities.resources = Some(ResourcesCapability {
                subscribe: None,
                list_changed: None,
            });
        }
        inner.resources.set_list_handler(handler);
        self
    }

    /// Sets the time-to-live (`ttl_ms`) and cache scope for `resources/templates/list` responses.
    pub fn resource_templates_list_cache(
        mut self,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) -> Self {
        Arc::make_mut(&mut self.inner)
            .resources
            .set_templates_list_cache(ttl_ms, cache_scope);
        self
    }

    /// Sets the time-to-live (`ttl_ms`) in milliseconds for `resources/templates/list` responses.
    pub fn resource_templates_list_ttl(mut self, ttl_ms: u64) -> Self {
        Arc::make_mut(&mut self.inner)
            .resources
            .templates_list_ttl_ms = Some(ttl_ms);
        self
    }

    /// Sets the cache scope for `resources/templates/list` responses.
    pub fn resource_templates_list_cache_scope(mut self, cache_scope: CacheScope) -> Self {
        Arc::make_mut(&mut self.inner)
            .resources
            .templates_list_cache_scope = Some(cache_scope);
        self
    }

    /// Registers a custom handler function for generating the resource templates list (`resources/templates/list`).
    ///
    /// The handler function can take request extractors (such as [`RequestContext`](crate::extract::RequestContext),
    /// [`State`](crate::extract::State), [`Extension`](crate::extract::Extension),
    /// [`Authorization`](crate::extract::Authorization), or [`BearerAuth`](crate::extract::BearerAuth))
    /// and optionally a `cursor: Option<String>` parameter, and return any type implementing [`IntoResourceTemplatesListResult`](crate::resources::IntoResourceTemplatesListResult).
    pub fn resource_templates_list<H, T>(mut self, handler: H) -> Self
    where
        H: IntoResourceTemplatesListHandler<T>,
        T: 'static,
    {
        let inner = Arc::make_mut(&mut self.inner);
        if inner.server.capabilities.resources.is_none() {
            inner.server.capabilities.resources = Some(ResourcesCapability {
                subscribe: None,
                list_changed: None,
            });
        }
        inner.resources.set_templates_list_handler(handler);
        self
    }

    /// Registers a direct resource definition alongside a typed asynchronous handler function.
    ///
    /// The handler function can take request extractors (such as [`RequestContext`](crate::extract::RequestContext),
    /// [`State`](crate::extract::State), [`Extension`](crate::extract::Extension),
    /// [`Authorization`](crate::extract::Authorization), or [`BearerAuth`](crate::extract::BearerAuth))
    /// and optionally the resource URI as `String`, and return any type implementing [`IntoResourceResult`](crate::resources::IntoResourceResult).
    pub fn register_resource<TResource, H, T>(mut self, resource: TResource, handler: H) -> Self
    where
        TResource: Into<Resource>,
        H: IntoResourceHandler<T>,
        T: 'static,
    {
        let inner = Arc::make_mut(&mut self.inner);
        if inner.server.capabilities.resources.is_none() {
            inner.server.capabilities.resources = Some(ResourcesCapability {
                subscribe: None,
                list_changed: None,
            });
        }
        inner.resources.register(resource, handler);
        self
    }

    /// Registers a direct resource definition alongside a typed asynchronous handler and resource-specific caching directives.
    pub fn register_resource_with_cache<TResource, H, T>(
        mut self,
        resource: TResource,
        handler: H,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) -> Self
    where
        TResource: Into<Resource>,
        H: IntoResourceHandler<T>,
        T: 'static,
    {
        let inner = Arc::make_mut(&mut self.inner);
        if inner.server.capabilities.resources.is_none() {
            inner.server.capabilities.resources = Some(ResourcesCapability {
                subscribe: None,
                list_changed: None,
            });
        }
        inner
            .resources
            .register_with_cache(resource, handler, ttl_ms, cache_scope);
        self
    }

    /// Registers a resource template definition alongside a typed asynchronous handler function.
    pub fn register_resource_template<TTemplate, H, T>(
        mut self,
        template: TTemplate,
        handler: H,
    ) -> Self
    where
        TTemplate: Into<ResourceTemplate>,
        H: IntoResourceHandler<T>,
        T: 'static,
    {
        let inner = Arc::make_mut(&mut self.inner);
        if inner.server.capabilities.resources.is_none() {
            inner.server.capabilities.resources = Some(ResourcesCapability {
                subscribe: None,
                list_changed: None,
            });
        }
        inner.resources.register_template(template, handler);
        self
    }

    /// Registers a resource template definition alongside a typed asynchronous handler and template-specific caching directives.
    pub fn register_resource_template_with_cache<TTemplate, H, T>(
        mut self,
        template: TTemplate,
        handler: H,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) -> Self
    where
        TTemplate: Into<ResourceTemplate>,
        H: IntoResourceHandler<T>,
        T: 'static,
    {
        let inner = Arc::make_mut(&mut self.inner);
        if inner.server.capabilities.resources.is_none() {
            inner.server.capabilities.resources = Some(ResourcesCapability {
                subscribe: None,
                list_changed: None,
            });
        }
        inner
            .resources
            .register_template_with_cache(template, handler, ttl_ms, cache_scope);
        self
    }

    /// Sets the cache configuration (`ttl_ms` and `cache_scope`) for a specific registered resource or template by URI.
    pub fn resource_cache(
        mut self,
        uri: impl Into<String>,
        ttl_ms: Option<u64>,
        cache_scope: Option<CacheScope>,
    ) -> Self {
        Arc::make_mut(&mut self.inner)
            .resources
            .set_resource_cache(uri, ttl_ms, cache_scope);
        self
    }
}
