// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Utility Functions
//!
//! Internal helper functions for HTTP header extraction, MIME type negotiation,
//! URI template matching, and method / parameter resolution.

pub(crate) mod headers;
pub(crate) mod params;
pub(crate) mod resolve;
pub(crate) mod sentinel;
pub(crate) mod uri_template;

pub(crate) use headers::{
    extract_body_protocol_version, extract_header_method, extract_header_name, extract_header_uri,
    extract_protocol_version, extract_session_id, is_json_content_type,
};
pub(crate) use params::{extract_header_params_from_schema, validate_tool_header_params};
pub(crate) use resolve::{
    resolve_method, resolve_prompt_name, resolve_resource_uri, resolve_tool_name,
};
pub(crate) use sentinel::decode_sentinel_header;
pub(crate) use uri_template::match_uri_template;
