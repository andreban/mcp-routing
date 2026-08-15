# MCP Routing Agent Guidelines

All contributions to this codebase must adhere strictly to the workspace rules defined in `.agents/rules/`:

## 1. Minimal Scope (`.agents/rules/minimal_scope.md`)
- **Mandatory Copyright & License Headers**: Every Rust source (`src/`), test (`tests/`), and example (`examples/`) file must start with:
  ```rust
  // Copyright 2026 André Cipriani Bandarra
  // SPDX-License-Identifier: Apache-2.0
  ```
  Preserve lines 1–2 verbatim in all file edits.
- **Do Not Add Unrequested Trait Derives**: Do not add trait derives (e.g. `Default`, `PartialEq`, `Eq`, `Clone`, `Copy`, `Hash`) unless explicitly requested or required for compilation/correctness. Assert on specific fields or `serde_json::to_value(&x)` in tests.
- **Scope Edits Strictly**: Modify only the types, functions, or files specifically requested.
- **Avoid Unrequested Abstractions & Overengineering**: Implement the simplest direct solution without unrequested concurrency wrappers or speculative abstractions.
- **Do Not Commit Without Explicit User Review**: Never run `git commit` unless explicitly instructed.
- **No Dead Code or Speculative Helpers**: Delete unused items immediately; do not write speculative helpers or use `#[allow(dead_code)]`.
- **Single Canonical API Names**: Expose exactly one canonical method per operation.
- **Flatten Tagged Enum Variants**: Place variant fields directly inside enum variants rather than standalone wrapper structs.

## 2. MCP Protocol Strictness & Tower Architecture (`.agents/rules/mcp_development_guidelines.md`)
- **Strict Schema Validation & Deserialization**: Strict adherence to MCP `2026-07-28` JSON Schema. Omitted required parameters must fail immediately with `-32602` (`InvalidParams`).
- **Exact Method Matching**: Match JSON-RPC method strings exactly (`server/discover`, `tools/list`, etc.).
- **Clean Protocol Layering**: Keep generic JSON-RPC types in `src/types/jsonrpc/` pure; define MCP-specific schemas in `src/types/mcp/`.
- **Modular Subsystems**: Keep files under ~250–300 lines by decomposing into submodules.
- **Separation of Utilities**: Keep utility and helper functions in dedicated utility modules (`src/utils/`).

## 3. Rust Serde Conventions (`.agents/rules/rust_serde.md`)
- **camelCase for String Enums**: Use `#[serde(rename_all = "camelCase")]`.
- **Do Not Use `#[serde(untagged)]` on Unit Enums**.

## 4. Testing Standards (`.agents/rules/testing_standards.md`)
- **Unit Tests (`src/`) vs Integration Tests (`tests/`)**: Unit tests in `src/` cover individual structures and Serde; `tests/` is strictly for black-box integration tests.
- **Mandatory Documentation**: Module-level doc comment (`//!`) on every test module; function-level doc comment (`///`) on every test function.

## 5. Customization Naming (`.agents/rules/workspace_customization_naming.md`)
- Always store customizations, rules, and skills in `.agents/`.
