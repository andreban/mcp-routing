// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Checks that input requests only use capabilities the client declared.

use crate::types::mcp::{ClientCapabilities, ElicitationCapability};

/// Returns the client capabilities required by the `inputRequests` of an `input_required`
/// `result` that the client did not declare, or `None` if every request is covered.
///
/// An `elicitation/create` request needs the `elicitation` capability with support for its
/// `mode` (`"form"` when omitted). Other input request methods are not checked.
pub(crate) fn missing_input_capabilities(
    result: &serde_json::Value,
    client_capabilities: Option<&ClientCapabilities>,
) -> Option<ClientCapabilities> {
    let declared = client_capabilities.and_then(|c| c.elicitation.as_ref());
    let mut missing_form = false;
    let mut missing_url = false;

    for request in result
        .get("inputRequests")
        .and_then(|r| r.as_object())
        .into_iter()
        .flat_map(|requests| requests.values())
    {
        if request.get("method").and_then(|m| m.as_str()) != Some("elicitation/create") {
            continue;
        }
        let mode = request
            .get("params")
            .and_then(|p| p.get("mode"))
            .and_then(|m| m.as_str())
            .unwrap_or("form");
        if declared.is_some_and(|e| e.supports_mode(mode)) {
            continue;
        }
        match mode {
            "form" => missing_form = true,
            "url" => missing_url = true,
            _ => {}
        }
    }

    (missing_form || missing_url).then(|| ClientCapabilities {
        experimental: None,
        elicitation: Some(ElicitationCapability {
            form: missing_form.then(serde_json::Map::new),
            url: missing_url.then(serde_json::Map::new),
        }),
        extensions: None,
    })
}

#[cfg(test)]
mod tests {
    //! Unit tests for client capability checks on input requests.

    use super::*;
    use serde_json::json;

    /// Builds an `input_required` result with a single elicitation request in `mode`.
    fn elicitation_result(mode: Option<&str>) -> serde_json::Value {
        let mut params = json!({ "message": "?" });
        if let Some(mode) = mode {
            params["mode"] = json!(mode);
        }
        json!({
            "resultType": "input_required",
            "inputRequests": { "ask": { "method": "elicitation/create", "params": params } }
        })
    }

    /// Parses client capabilities from JSON.
    fn caps(value: serde_json::Value) -> ClientCapabilities {
        serde_json::from_value(value).unwrap()
    }

    /// Tests that elicitation requests require a declared elicitation capability.
    #[test]
    fn test_elicitation_requires_capability() {
        let required = missing_input_capabilities(&elicitation_result(None), None).unwrap();
        let required = serde_json::to_value(required).unwrap();
        assert_eq!(required, json!({ "elicitation": { "form": {} } }));

        let no_elicitation = caps(json!({}));
        assert!(
            missing_input_capabilities(&elicitation_result(None), Some(&no_elicitation)).is_some()
        );
    }

    /// Tests elicitation mode support, including the empty-object form-only default.
    #[test]
    fn test_elicitation_mode_support() {
        let empty = caps(json!({ "elicitation": {} }));
        assert!(missing_input_capabilities(&elicitation_result(None), Some(&empty)).is_none());
        assert!(
            missing_input_capabilities(&elicitation_result(Some("form")), Some(&empty)).is_none()
        );
        let required =
            missing_input_capabilities(&elicitation_result(Some("url")), Some(&empty)).unwrap();
        assert_eq!(
            serde_json::to_value(required).unwrap(),
            json!({ "elicitation": { "url": {} } })
        );

        let url_only = caps(json!({ "elicitation": { "url": {} } }));
        assert!(
            missing_input_capabilities(&elicitation_result(Some("url")), Some(&url_only)).is_none()
        );
        assert!(missing_input_capabilities(&elicitation_result(None), Some(&url_only)).is_some());

        let both = caps(json!({ "elicitation": { "form": {}, "url": {} } }));
        for mode in [None, Some("form"), Some("url")] {
            assert!(missing_input_capabilities(&elicitation_result(mode), Some(&both)).is_none());
        }
    }

    /// Tests that results without elicitation requests require nothing.
    #[test]
    fn test_no_elicitation_requests() {
        let load_shed = json!({ "resultType": "input_required", "requestState": "s" });
        assert!(missing_input_capabilities(&load_shed, None).is_none());

        let other = json!({
            "resultType": "input_required",
            "inputRequests": { "x": { "method": "com.example/custom" } }
        });
        assert!(missing_input_capabilities(&other, None).is_none());
    }
}
