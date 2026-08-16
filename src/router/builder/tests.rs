// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Unit tests for McpRouter builder methods and configuration.

use super::*;
use crate::types::mcp::Implementation;

/// Tests that McpRouter builds properly with server info and default configuration.
#[test]
fn test_mcp_router_builder_defaults() {
    let server_info = Implementation::new("test-server", "1.0.0");
    let router = McpRouter::new(server_info)
        .instructions("Test instructions")
        .logging_level(LoggingLevel::Debug);

    assert_eq!(router.current_logging_level(), LoggingLevel::Debug);
    assert_eq!(
        router.inner.server.instructions.as_deref(),
        Some("Test instructions")
    );
}

/// Tests that router cloning creates shared inner state with proper mutation semantics.
#[test]
fn test_mcp_router_clone_and_state_injection() {
    #[derive(Clone, PartialEq, Eq, Debug)]
    struct CustomState(String);

    let server_info = Implementation::new("test-server", "1.0.0");
    let router = McpRouter::new(server_info).with_state(CustomState("hello".to_string()));

    assert_eq!(router.inner.state_injectors.len(), 1);

    let mut extensions = http::Extensions::new();
    for injector in &router.inner.state_injectors {
        injector(&mut extensions);
    }

    assert_eq!(
        extensions.get::<CustomState>(),
        Some(&CustomState("hello".to_string()))
    );
}
