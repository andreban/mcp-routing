# Minimal Scope Rule

1. **Do Not Add Unrequested Trait Derives**:
   - Match existing codebase patterns. Do not add unrequested trait derives (e.g. `Default`, `PartialEq`, `Eq`, `Clone`, `Copy`, `Hash`) unless explicitly requested by the user or required for compilation/correctness.
2. **Scope Edits Strictly**:
   - Limit modifications to the types, functions, or files specifically requested by the user. Avoid refactoring surrounding pre-existing code unless asked.
