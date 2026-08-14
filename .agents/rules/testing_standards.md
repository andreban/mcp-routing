---
trigger: glob
globs: "src/**/*.rs,tests/**/*.rs"
description: "Standards for unit vs. integration test separation and comprehensive test documentation."
---

# Testing Standards & Organization

## 1. Strict Separation of Unit Tests vs. Integration Tests
- **Unit Tests (`src/`)**:
  - Place all unit tests inside `src/` in `#[cfg(test)] mod tests` within or adjacent to the module defining the tested functionality.
  - Unit tests cover: individual data structures, serialization/deserialization (Serde), builder methods, type conversions, internal helper functions, and isolated trait implementations.
- **Integration Tests (`tests/`)**:
  - Reserve the `tests/` directory strictly for black-box integration tests that exercise public interfaces from an external consumer's perspective.
  - Integration tests cover: Tower service execution, HTTP protocol routing, multi-component workflows, framework integration (e.g., Axum), and end-to-end TCP socket communication.
  - Never place isolated struct/enum serialization or internal unit tests inside `tests/`.

## 2. Mandatory Test Documentation
- **Module-Level Documentation (`//!`)**:
  - Every test file in `tests/` and test module in `src/` must contain a top-level doc comment (`//!`) summarizing the features and scenarios tested.
- **Function-Level Documentation (`///`)**:
  - Every test function must contain a doc comment (`///`) explicitly describing what is being tested (e.g., input preconditions, expected behaviors, and assertions).
