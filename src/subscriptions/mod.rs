// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # MCP Subscriptions Subsystem
//!
//! Handles stateless `subscriptions/listen` notification streaming over HTTP/SSE per SEP-2575.

pub mod handler;
pub mod registry;

pub use handler::{
    IntoSubscriptionsListenHandler, IntoSubscriptionsListenResult, SubscriptionError,
    SubscriptionsListenHandler, SubscriptionsListenOutcome,
};
pub use registry::SubscriptionsRegistry;
