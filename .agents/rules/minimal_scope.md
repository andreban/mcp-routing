# Minimal Scope Rule

1. **Do Not Add Unrequested Trait Derives**:
   - Match existing codebase patterns. Do not add unrequested trait derives (e.g. `Default`, `PartialEq`, `Eq`, `Clone`, `Copy`, `Hash`) unless explicitly requested by the user or required for compilation/correctness.
2. **Scope Edits Strictly**:
   - Limit modifications to the types, functions, or files specifically requested by the user. Avoid refactoring surrounding pre-existing code unless asked.
3. **Avoid Unrequested Abstractions & Overengineering**:
   - Implement the simplest and most direct solution to the user's request.
   - Do not introduce new concurrency primitives or wrapper structures (e.g. `Arc`, `Mutex`, `RwLock`, `Box<dyn ...>`, or new state structs) unless explicitly requested or strictly required.
4. **Do Not Commit Without Explicit User Review**:
   - Always wait for the user to review and test the code changes first. Do not run `git commit` or prompt/ask to commit until the user explicitly requests a commit.
5. **No Dead Code or `#[allow(dead_code)]`**:
   - Immediately delete any unused functions, methods, structs, or imports.
   - Never suppress unused code compiler warnings with `#[allow(dead_code)]` or `#[allow(unused)]`.
6. **Single Canonical API Names (No Synonym Aliases)**:
   - Expose exactly one canonical method per operation on builder and router types.
   - Never introduce redundant method aliases (e.g. do not add `discovery` or `dynamic_discovery` alongside `discover`, or `list_tools` alongside `tools_list`).
7. **Flatten Tagged Enum Variants**:
   - In tagged Rust enums (e.g., `#[serde(tag = "type")]`), place variant fields directly inside the enum variant (e.g., `Variant { name: String }`) instead of defining standalone single-field wrapper structs, unless the inner struct is reused independently in multiple distinct places.

