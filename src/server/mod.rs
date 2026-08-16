// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

pub mod config;
pub mod discover;
pub mod handler;
pub mod provider;

pub use config::ServerConfig;
pub use discover::{handle_server_discover, validate_protocol_version};
pub use handler::{
    DiscoveryError, IntoServerDiscoveryHandler, IntoServerDiscoveryResult, ServerDiscoveryHandler,
};
