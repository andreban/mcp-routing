# Minimal Scope Rule

1. **Mandatory Copyright & License Headers**:
   - Every Rust source (`src/`), test (`tests/`), and example (`examples/`) file must start with the standard license header:
     ```rust
     // Copyright 2026 André Cipriani Bandarra
     // SPDX-License-Identifier: Apache-2.0
     ```
   - Never omit, remove, or replace the license header when modifying existing files or creating new files. When editing imports or top-of-file sections (lines 1–10), always verify lines 1–2 are preserved verbatim in replacement chunks.
2. **Do Not Add Unrequested Trait Derives**:
   - Match existing codebase patterns. Do not add unrequested trait derives (e.g. `Default`, `PartialEq`, `Eq`, `Clone`, `Copy`, `Hash`) unless explicitly requested by the user or required for compilation/correctness.
   - Do not add `PartialEq` or `Eq` to protocol or metadata structs solely for unit test convenience. In tests, assert on specific fields or compare `serde_json::to_value(&x)` instead of introducing cascading trait derives across unrelated types.
3. **Scope Edits Strictly**:
   - Limit modifications to the types, functions, or files specifically requested by the user. Avoid refactoring surrounding pre-existing code unless asked.
4. **Avoid Unrequested Abstractions & Overengineering**:
   - Implement the simplest and most direct solution to the user's request.
   - Do not introduce new concurrency primitives or wrapper structures (e.g. `Arc`, `Mutex`, `RwLock`, `Box<dyn ...>`, or new state structs) unless explicitly requested or strictly required.
5. **Do Not Commit Without Explicit User Review**:
   - Always wait for the user to review and test the code changes first. Do not run `git commit` or prompt/ask to commit until the user explicitly requests a commit.
6. **No Dead Code or Speculative Helpers (No `#[allow(dead_code)]`)**:
   - Immediately delete any unused functions, methods, structs, or imports.
   - Do not write speculative helper functions in advance for future roadmap steps; only implement what is directly used.
   - Never suppress unused code compiler warnings with `#[allow(dead_code)]` or `#[allow(unused)]`.
7. **Single Canonical API Names (No Synonym Aliases)**:
   - Expose exactly one canonical method per operation on builder and router types.
   - Never introduce redundant method aliases (e.g. do not add `discovery` or `dynamic_discovery` alongside `discover`, or `list_tools` alongside `tools_list`).
8. **Flatten Tagged Enum Variants**:
   - In tagged Rust enums (e.g., `#[serde(tag = "type")]`), place variant fields directly inside the enum variant (e.g., `Variant { name: String }`) instead of defining standalone single-field wrapper structs, unless the inner struct is reused independently in multiple distinct places.

