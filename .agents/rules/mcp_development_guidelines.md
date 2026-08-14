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
- **Centralized Response Construction**: Place HTTP status and response helpers (`bad_request()`, `not_found()`, `json_response()`) in `src/body.rs` or utility modules rather than inline `Response::builder()` in router code.
