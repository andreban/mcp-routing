---
trigger: glob
globs: "src/**/*.rs,examples/**/*.rs"
description: "Guidelines for strict MCP 2026-07-28 specification compliance and modular Tower service architecture."
---

# MCP Server Implementation Guidelines

## 1. Protocol Specification Strictness
- **Strict Schema Validation**: Adhere strictly to the Model Context Protocol specification JSON Schema (`2026-07-28`). Do not add `#[serde(default)]` to fields that are marked as required in the MCP schema (e.g., `name` in `CallToolParams`).
- **Exact Method Matching**: MCP JSON-RPC `method` strings are exact identifiers (`server/discover`, `tools/list`, `tools/call`). Never introduce or accommodate non-standard compound methods (such as `tools/call/<name>`).

## 2. Modular Tower Service Architecture
- **Lean Service Entrypoints**: Keep `tower::Service::call` minimal (delegating to a dispatch function or inner method).
- **Dedicated Endpoint Handlers**: Encapsulate logic for each MCP method in standalone private handler functions (e.g., `handle_server_discover`, `handle_tools_list`, `handle_tools_call`).
- **Capability Dispatch Delegation**: Domain registries (`ServerConfig`, `ToolRegistry`, `PromptRegistry`, `ResourceRegistry`) must own their own method dispatching, parameter deserialization, handler execution, and error formatting (`dispatch_discover`, `dispatch_list`, `dispatch_call`, `dispatch_get`) rather than centralizing all method branches into a monolithic router.
- **Subsystem Submodules & File Size Limits**: Avoid single-file modules exceeding ~250–300 lines. Decompose complex subsystems into dedicated submodules under a module folder (e.g., `src/router/` containing `mod.rs`, `builder.rs`, `dispatch.rs`, `service.rs`, `outcome.rs`), clearly isolating the fluent builder API, HTTP transport, JSON-RPC 2.0 batch/notification processing, and domain dispatching.
- **Centralized Response Construction**: Place HTTP status and response helpers (`bad_request()`, `not_found()`, `json_response()`) in `src/body.rs` or utility modules rather than inline `Response::builder()` in router code.
- **Separation of Utility and Helper Functions**: Keep core router, service, and handler files lean and focused. Standalone utility functions (such as HTTP header extraction, MIME/media type negotiation, string/slash normalization, and parameter resolution) must be placed in dedicated utility source files (e.g., `src/utils.rs` or specialized utility modules) along with their unit tests, rather than being defined inline in core domain or routing files.
