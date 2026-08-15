---
trigger: glob
globs: "src/**/*.rs,examples/**/*.rs"
description: "Guidelines for strict MCP 2026-07-28 specification compliance and modular Tower service architecture."
---

# MCP Server Implementation Guidelines

## 1. Protocol Specification Strictness
- **Strict Schema Validation & Deserialization**: Adhere strictly to the Model Context Protocol specification JSON Schema (`2026-07-28`). Never add `#[serde(default)]` or default to empty values for fields marked as required by the specification (e.g., `name` in `CallToolParams` and `GetPromptParams`, `uri` in `ReadResourceParams`). Omitted required parameters must fail deserialization immediately and return JSON-RPC `-32602` (`InvalidParams`).
- **Exact Method Matching**: MCP JSON-RPC `method` strings are exact identifiers (`server/discover`, `tools/list`, `tools/call`). Never introduce or accommodate non-standard compound methods (such as `tools/call/<name>`).
- **No Unnecessary Backwards Compatibility Layers**: Enforce current normative specification requirements (such as Streamable HTTP headers `Mcp-Method`, `Mcp-Name`, and `Mcp-Uri`) strictly and directly. Avoid adding legacy fallback modes, backwards compatibility flags, or loose compatibility branches unless explicitly requested.
- **Clean Protocol Layering & Separation**: Do not mix higher-level MCP domain protocol types, error codes (`-32020` through `-32022`), or data structures into generic lower-level transport/framing modules (e.g. `src/types/jsonrpc/`). Keep generic JSON-RPC 2.0 types pure to the JSON-RPC 2.0 specification, and define all MCP-specific extensions, errors, and schemas strictly in `src/types/mcp/`.

## 2. Modular Tower Service Architecture
- **Lean Service Entrypoints**: Keep `tower::Service::call` minimal (delegating to a dispatch function or inner method).
- **Dedicated Endpoint Handlers**: Encapsulate logic for each MCP method in standalone private handler functions (e.g., `handle_server_discover`, `handle_tools_list`, `handle_tools_call`).
- **Capability Dispatch Delegation**: Domain registries (`ServerConfig`, `ToolRegistry`, `PromptRegistry`, `ResourceRegistry`) must own their own method dispatching, parameter deserialization, handler execution, and error formatting (`dispatch_discover`, `dispatch_list`, `dispatch_call`, `dispatch_get`) rather than centralizing all method branches into a monolithic router.
- **Subsystem Submodules & File Size Limits**: Avoid single-file modules exceeding ~250–300 lines. Decompose complex subsystems into dedicated submodules under a module folder (e.g., `src/router/` containing `mod.rs`, `builder.rs`, `dispatch.rs`, `service.rs`, `outcome.rs`), clearly isolating the fluent builder API, HTTP transport, JSON-RPC 2.0 batch/notification processing, and domain dispatching.
- **Centralized Response Construction**: Place HTTP status and response helpers (`bad_request()`, `not_found()`, `json_response()`) in `src/body.rs` or utility modules rather than inline `Response::builder()` in router code.
- **Separation of Utility and Helper Functions**: Keep core router, service, and handler files lean and focused. Standalone utility functions (such as HTTP header extraction, MIME/media type negotiation, string/slash normalization, and parameter resolution) must be placed in dedicated utility source files (e.g., `src/utils.rs` or specialized utility modules) along with their unit tests, rather than being defined inline in core domain or routing files.
