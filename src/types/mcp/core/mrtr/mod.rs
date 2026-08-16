// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Multi Round-Trip Request (MRTR) types per MCP 2026-07-28 specification (SEP-2322).
//!
//! MRTR enables stateless multi round-trip interactions (such as model sampling,
//! user confirmation / elicitation, and filesystem root selection) between client
//! and server without requiring persistent connections.
//!
//! See <https://modelcontextprotocol.io/specification/2026-07-28/schema#inputrequiredresult>

pub mod request;
pub mod response;
pub mod types;

#[cfg(test)]
mod tests;

pub use request::*;
pub use response::*;
pub use types::*;
