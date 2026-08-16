// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Validation helpers for tool input schemas and arguments.

/// Validates raw JSON tool arguments against the compiled JSON Schema validator.
pub(crate) fn validate_tool_arguments(
    validator: &jsonschema::Validator,
    arguments: Option<&serde_json::Value>,
) -> Result<(), String> {
    let empty_obj = serde_json::Value::Object(serde_json::Map::new());
    let raw_to_validate = arguments.unwrap_or(&empty_obj);
    let mut errors = Vec::new();
    for error in validator.iter_errors(raw_to_validate) {
        let path = error.instance_path().to_string();
        if path.is_empty() || path == "/" {
            errors.push(error.to_string());
        } else {
            errors.push(format!("at `{path}`: {error}"));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Input schema validation failed: {}",
            errors.join("; ")
        ))
    }
}
