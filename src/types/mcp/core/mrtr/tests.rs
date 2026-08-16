// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Unit tests for Multi Round-Trip Request (MRTR) types.

use std::collections::HashMap;

use super::*;
use serde_json::{Value, json};

/// Tests ResultType serialization, deserialization, and string conversions.
#[test]
fn test_result_type_serde() {
    let rt_complete = ResultType::Complete;
    let serialized = serde_json::to_string(&rt_complete).unwrap();
    assert_eq!(serialized, "\"complete\"");
    let deserialized: ResultType = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized, ResultType::Complete);
    assert!(deserialized.is_complete());
    assert!(!deserialized.is_input_required());

    let rt_input_req = ResultType::InputRequired;
    let serialized = serde_json::to_string(&rt_input_req).unwrap();
    assert_eq!(serialized, "\"input_required\"");
    let deserialized: ResultType = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized, ResultType::InputRequired);
    assert!(!deserialized.is_complete());
    assert!(deserialized.is_input_required());

    let rt_custom = ResultType::from("custom_type");
    assert_eq!(rt_custom.as_str(), "custom_type");
}

/// Tests InputRequiredResult serialization and helper methods.
#[test]
fn test_input_required_result_serde() {
    let sampling_req = InputRequest::sampling(&json!({
        "messages": [{"role": "user", "content": {"type": "text", "text": "Hello"}}]
    }))
    .unwrap();

    let roots_req = InputRequest::roots();

    let result = InputRequiredResult::new()
        .with_request_state("opaque_state_12345")
        .with_input_request("sampling_1", sampling_req)
        .with_input_request("roots_1", roots_req);

    assert!(result.is_valid());
    assert_eq!(result.request_state(), Some("opaque_state_12345"));
    assert_eq!(result.input_requests.len(), 2);

    let json_val = serde_json::to_value(&result).unwrap();
    assert_eq!(json_val["resultType"], "input_required");
    assert_eq!(json_val["requestState"], "opaque_state_12345");
    assert_eq!(
        json_val["inputRequests"]["sampling_1"]["method"],
        "sampling/createMessage"
    );
    assert_eq!(json_val["inputRequests"]["roots_1"]["method"], "roots/list");

    let deserialized: InputRequiredResult = serde_json::from_value(json_val).unwrap();
    assert_eq!(deserialized.result_type, "input_required");
    assert_eq!(
        deserialized.request_state.as_deref(),
        Some("opaque_state_12345")
    );
    assert_eq!(
        deserialized.get_input_request("roots_1").unwrap().method(),
        Some("roots/list")
    );
}

/// Tests InputRequiredResult load shedding configuration.
#[test]
fn test_input_required_load_shed_serde() {
    let result = InputRequiredResult::load_shed("state_shed_999");
    assert!(result.is_valid());
    assert_eq!(result.request_state(), Some("state_shed_999"));
    assert!(result.input_requests().is_empty());

    let json_val = serde_json::to_value(&result).unwrap();
    assert_eq!(json_val["resultType"], "input_required");
    assert_eq!(json_val["requestState"], "state_shed_999");
    assert!(json_val.get("inputRequests").is_none());

    let deserialized: InputRequiredResult = serde_json::from_value(json_val).unwrap();
    assert_eq!(deserialized.result_type, "input_required");
    assert_eq!(
        deserialized.request_state.as_deref(),
        Some("state_shed_999")
    );
}

/// Tests InputResponseRequestParams serialization and result extraction.
#[test]
fn test_input_response_request_params_serde() {
    let sampling_resp = InputResponse::result(&json!({
        "model": "gemini-2.5-flash",
        "content": {"type": "text", "text": "World"}
    }))
    .unwrap();

    let roots_resp = InputResponse::result(&json!({
        "roots": [{"uri": "file:///workspace", "name": "Workspace"}]
    }))
    .unwrap();

    let params = InputResponseRequestParams::new()
        .with_request_state("opaque_state_12345")
        .with_input_response("sampling_1", sampling_resp)
        .with_input_response("roots_1", roots_resp);

    assert_eq!(params.request_state(), Some("opaque_state_12345"));
    assert_eq!(params.input_responses.len(), 2);

    let json_val = serde_json::to_value(&params).unwrap();
    assert_eq!(json_val["requestState"], "opaque_state_12345");
    assert_eq!(
        json_val["inputResponses"]["sampling_1"]["result"]["model"],
        "gemini-2.5-flash"
    );

    let deserialized: InputResponseRequestParams = serde_json::from_value(json_val).unwrap();
    assert_eq!(
        deserialized.request_state.as_deref(),
        Some("opaque_state_12345")
    );

    let resp = deserialized.get_response("sampling_1").unwrap();
    assert!(!resp.is_error());
    let res_json: Option<Value> = resp.get_result().unwrap();
    assert_eq!(res_json.unwrap()["model"], "gemini-2.5-flash");
}

/// Tests InputRequiredResult into_extras conversion.
#[test]
fn test_input_required_into_extras() {
    let mut custom_extras = HashMap::new();
    custom_extras.insert("customField".to_string(), Value::Bool(true));

    let result = InputRequiredResult {
        meta: None,
        result_type: "input_required".to_string(),
        input_requests: HashMap::from([(
            "roots_1".to_string(),
            InputRequest::roots(),
        )]),
        request_state: Some("opaque_state_12345".to_string()),
        extras: custom_extras,
    };

    let extras = result.into_extras();
    assert_eq!(
        extras.get("requestState").and_then(|v| v.as_str()),
        Some("opaque_state_12345")
    );
    assert!(extras.contains_key("inputRequests"));
    assert_eq!(
        extras.get("customField").and_then(|v| v.as_bool()),
        Some(true)
    );
}

/// Tests InputRequiredResult into_extras with empty fields.
#[test]
fn test_input_required_into_extras_empty() {
    let result = InputRequiredResult::new();
    let extras = result.into_extras();
    assert!(extras.is_empty());
}

