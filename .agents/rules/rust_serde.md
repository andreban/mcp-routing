# Rust Serde Enum Conventions

1. **Prefer `camelCase` for String Enums**:
   - Use `#[serde(rename_all = "camelCase")]` on string enums. Avoid `rename_all = "lowercase"` as it will misformat multi-word variants added in the future.
2. **Do Not Use `#[serde(untagged)]` on Unit Enums**:
   - `#[serde(untagged)]` on enums with unit variants causes Serde to expect JSON `null` instead of string matching. To match string values, use `rename_all = "camelCase"` without `untagged`.
