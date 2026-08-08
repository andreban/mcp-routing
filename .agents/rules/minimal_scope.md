# Minimal Scope Rule

1. **Do Not Add Unrequested Trait Derives**:
   - Match existing codebase patterns. Do not add unrequested trait derives (e.g. `Default`, `PartialEq`, `Eq`, `Clone`, `Copy`, `Hash`) unless explicitly requested by the user or required for compilation/correctness.
2. **Scope Edits Strictly**:
   - Limit modifications to the types, functions, or files specifically requested by the user. Avoid refactoring surrounding pre-existing code unless asked.
3. **Do Not Commit Without Explicit User Review**:
   - Always wait for the user to review and test the code changes first. Do not run `git commit` or prompt/ask to commit until the user explicitly requests a commit.
