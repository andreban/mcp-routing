// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! MCP `tools/call` types for tool execution requests and responses.
//!
//! See <https://modelcontextprotocol.io/specification/2026-07-28/schema#calltoolrequest>

pub mod request;
pub mod response;

#[cfg(test)]
mod tests;

pub use request::*;
pub use response::*;
