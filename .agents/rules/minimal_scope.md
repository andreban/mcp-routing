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
