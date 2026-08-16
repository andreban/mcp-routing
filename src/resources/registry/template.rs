// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Template and direct URI handler resolution for resources.

use std::sync::Arc;

use crate::resources::ResourceHandler;
use crate::resources::registry::ResourceRegistry;
use crate::types::mcp::CacheScope;
use crate::utils::match_uri_template;

pub(crate) type MatchedResourceHandler = (Arc<dyn ResourceHandler>, Option<u64>, Option<CacheScope>);

impl ResourceRegistry {
    /// Resolves an exact or template-matching resource handler for the given URI.
    pub(crate) fn find_handler(&self, resource_uri: &str) -> Option<MatchedResourceHandler> {
        if let Some(handler) = self.resource_handlers.get(resource_uri) {
            let (ttl, scope) = self
                .resource_cache_settings
                .get(resource_uri)
                .cloned()
                .unwrap_or((None, None));
            Some((handler.clone(), ttl, scope))
        } else {
            let mut found = None;
            for (template, handler) in &self.template_handlers {
                if match_uri_template(&template.uri_template, resource_uri) {
                    let (ttl, scope) = self
                        .resource_cache_settings
                        .get(&template.uri_template)
                        .cloned()
                        .unwrap_or((None, None));
                    found = Some((handler.clone(), ttl, scope));
                    break;
                }
            }
            found
        }
    }
}
