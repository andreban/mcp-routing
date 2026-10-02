// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Validation of the required per-request `_meta` protocol fields.

const PROTOCOL_VERSION_KEY: &str = "io.modelcontextprotocol/protocolVersion";
const CLIENT_CAPABILITIES_KEY: &str = "io.modelcontextprotocol/clientCapabilities";

/// Validates that a request's `params._meta` carries the fields every request MUST include:
/// `io.modelcontextprotocol/protocolVersion` (a string) and
/// `io.modelcontextprotocol/clientCapabilities` (an object).
///
/// Returns a human-readable description of the first problem found.
pub(crate) fn validate_required_request_meta(
    params: Option<&serde_json::Value>,
) -> Result<(), String> {
    let Some(meta) = params
        .and_then(|p| p.get("_meta"))
        .and_then(|m| m.as_object())
    else {
        return Err("missing required params._meta".to_string());
    };

    if !meta
        .get(PROTOCOL_VERSION_KEY)
        .is_some_and(|v| v.is_string())
    {
        return Err(format!(
            "missing required _meta[\"{PROTOCOL_VERSION_KEY}\"] string"
        ));
    }

    if !meta
        .get(CLIENT_CAPABILITIES_KEY)
        .is_some_and(|v| v.is_object())
    {
        return Err(format!(
            "missing required _meta[\"{CLIENT_CAPABILITIES_KEY}\"] object"
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Unit tests for required per-request `_meta` validation.

    use super::*;
    use serde_json::json;

    /// Tests that requests carrying both required `_meta` fields are accepted.
    #[test]
    fn test_valid_request_meta() {
        let params = json!({
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                "io.modelcontextprotocol/clientCapabilities": {}
            }
        });
        assert!(validate_required_request_meta(Some(&params)).is_ok());
    }

    /// Tests that missing params, `_meta`, or either required field is rejected.
    #[test]
    fn test_missing_request_meta() {
        assert!(validate_required_request_meta(None).is_err());
        assert!(validate_required_request_meta(Some(&json!({}))).is_err());

        let no_version = json!({
            "_meta": { "io.modelcontextprotocol/clientCapabilities": {} }
        });
        let err = validate_required_request_meta(Some(&no_version)).unwrap_err();
        assert!(err.contains("protocolVersion"));

        let no_capabilities = json!({
            "_meta": { "io.modelcontextprotocol/protocolVersion": "2026-07-28" }
        });
        let err = validate_required_request_meta(Some(&no_capabilities)).unwrap_err();
        assert!(err.contains("clientCapabilities"));
    }

    /// Tests that required fields with the wrong JSON type are rejected.
    #[test]
    fn test_wrongly_typed_request_meta() {
        let params = json!({
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": 20260728,
                "io.modelcontextprotocol/clientCapabilities": []
            }
        });
        assert!(validate_required_request_meta(Some(&params)).is_err());
    }
}
