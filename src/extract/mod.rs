// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Request Extractors and Context
//!
//! Provides extractors for handler functions in `mcp-routing`, including [`RequestContext`],
//! [`Meta`], [`Authorization`], [`BearerAuth`], [`State`], [`Extension`],
//! [`RegisteredTools`], and [`RegisteredPrompts`].

pub mod auth;
pub mod context;
pub mod error;
pub mod json;
pub mod logging;
pub mod meta;
pub mod mrtr;
pub mod registered;
pub mod state;
pub mod traits;

pub use auth::{Authorization, BearerAuth};
pub use context::RequestContext;
pub use error::ExtractionError;
pub use json::Json;
pub use logging::CurrentLoggingLevel;
pub use meta::Meta;
pub use mrtr::{InputResponses, RequestState};
pub use registered::{
    RegisteredPrompts, RegisteredResourceTemplates, RegisteredResources, RegisteredTools,
};
pub use state::{Extension, State};
pub use traits::FromRequestContext;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use http::HeaderMap;

    use super::*;
    use crate::types::mcp::{Implementation, LoggingLevel, ProgressToken, RequestMetaObject};

    #[test]
    fn test_extractors_from_context() {
        #[derive(Clone, Debug, PartialEq, Eq)]
        struct TestState(String);

        let mut headers = HeaderMap::new();
        headers.insert("Custom-Header", "custom-val".parse().unwrap());

        let mut ext = http::Extensions::new();
        ext.insert(TestState("state-data".to_string()));
        ext.insert(RequestState::new("step2_token"));
        let mut ir_map = std::collections::HashMap::new();
        ir_map.insert(
            "sample_1".to_string(),
            crate::types::mcp::InputResponse::result(&serde_json::json!({"text": "done"})).unwrap(),
        );
        ext.insert(InputResponses::new(ir_map));

        let meta = RequestMetaObject {
            progress_token: Some(ProgressToken::String("prog-1".to_string())),
            client_info: Some(Implementation::new("client-a", "1.0.0")),
            client_capabilities: None,
            protocol_version: Some("2026-07-28".to_string()),
            log_level: Some(LoggingLevel::Debug),
            extra: std::collections::HashMap::new(),
        };

        let ctx = RequestContext::new(
            Some(meta.clone()),
            headers,
            Arc::new(ext),
        );

        // RequestContext extractor
        let extracted_ctx = RequestContext::from_request_context(&ctx).unwrap();
        assert_eq!(extracted_ctx.request_state(), Some("step2_token"));
        assert!(
            extracted_ctx
                .input_responses()
                .unwrap()
                .contains_key("sample_1")
        );
        assert_eq!(extracted_ctx.client_info().unwrap().name, "client-a");
        assert_eq!(extracted_ctx.protocol_version(), Some("2026-07-28"));
        assert_eq!(extracted_ctx.log_level(), Some(&LoggingLevel::Debug));
        assert!(matches!(
            extracted_ctx.progress_token(),
            Some(ProgressToken::String(s)) if s == "prog-1"
        ));

        // RequestState extractor
        let rs = RequestState::from_request_context(&ctx).unwrap();
        assert_eq!(rs.as_str(), "step2_token");
        let opt_rs = Option::<RequestState>::from_request_context(&ctx).unwrap();
        assert_eq!(opt_rs.as_deref(), Some("step2_token"));

        // InputResponses extractor
        let ir = InputResponses::from_request_context(&ctx).unwrap();
        assert!(ir.get("sample_1").is_some());
        let opt_ir = Option::<InputResponses>::from_request_context(&ctx).unwrap();
        assert!(opt_ir.is_some());

        // Meta extractor
        let extracted_meta = Meta::from_request_context(&ctx).unwrap();
        assert_eq!(
            extracted_meta.protocol_version.as_deref(),
            Some("2026-07-28")
        );

        let opt_meta = Option::<Meta>::from_request_context(&ctx).unwrap();
        assert!(opt_meta.is_some());

        // Extension extractor
        let state = Extension::<TestState>::from_request_context(&ctx).unwrap();
        assert_eq!(state.0, TestState("state-data".to_string()));

        let opt_state = Option::<Extension<TestState>>::from_request_context(&ctx).unwrap();
        assert_eq!(opt_state.unwrap().0, TestState("state-data".to_string()));

        // State extractor
        let direct_state = State::<TestState>::from_request_context(&ctx).unwrap();
        assert_eq!(direct_state.0, TestState("state-data".to_string()));

        let opt_direct_state = Option::<State<TestState>>::from_request_context(&ctx).unwrap();
        assert_eq!(
            opt_direct_state.unwrap().0,
            TestState("state-data".to_string())
        );
        assert_eq!(
            ctx.state::<TestState>().unwrap(),
            TestState("state-data".to_string())
        );

        // HeaderMap extractor
        let extracted_headers = HeaderMap::from_request_context(&ctx).unwrap();
        assert_eq!(
            extracted_headers
                .get("Custom-Header")
                .unwrap()
                .to_str()
                .unwrap(),
            "custom-val"
        );
    }

    #[test]
    fn test_extractors_missing_data_errors() {
        let ctx = RequestContext::new(
            None,
            HeaderMap::new(),
            Arc::new(http::Extensions::new()),
        );

        // Missing Meta
        let meta_err = Meta::from_request_context(&ctx).unwrap_err();
        assert!(meta_err.0.contains("Missing required _meta"));

        let opt_meta = Option::<Meta>::from_request_context(&ctx).unwrap();
        assert!(opt_meta.is_none());

        // Missing Extension
        #[derive(Clone, Debug)]
        struct MissingState;
        let ext_err = Extension::<MissingState>::from_request_context(&ctx).unwrap_err();
        assert!(ext_err.0.contains("Missing request extension"));

        let opt_ext = Option::<Extension<MissingState>>::from_request_context(&ctx).unwrap();
        assert!(opt_ext.is_none());

        // Missing State
        let state_err = State::<MissingState>::from_request_context(&ctx).unwrap_err();
        assert!(state_err.0.contains("Missing state"));

        let opt_state_missing = Option::<State<MissingState>>::from_request_context(&ctx).unwrap();
        assert!(opt_state_missing.is_none());
        assert!(ctx.state::<MissingState>().is_none());

        // Missing Authorization and BearerAuth
        let auth_err = Authorization::from_request_context(&ctx).unwrap_err();
        assert!(auth_err.0.contains("Missing required Authorization header"));
        let opt_auth = Option::<Authorization>::from_request_context(&ctx).unwrap();
        assert!(opt_auth.is_none());

        let bearer_err = BearerAuth::from_request_context(&ctx).unwrap_err();
        assert!(
            bearer_err
                .0
                .contains("Missing required Authorization header")
        );
        let opt_bearer = Option::<BearerAuth>::from_request_context(&ctx).unwrap();
        assert!(opt_bearer.is_none());
    }

    #[test]
    fn test_authorization_and_bearer_auth_extractors() {
        let mut headers = HeaderMap::new();
        headers.insert(
            http::header::AUTHORIZATION,
            "Bearer secret-token-xyz".parse().unwrap(),
        );

        let ctx = RequestContext::new(None, headers, Arc::new(http::Extensions::new()));

        assert_eq!(ctx.authorization(), Some("Bearer secret-token-xyz"));
        assert_eq!(ctx.bearer_token(), Some("secret-token-xyz"));

        let auth = Authorization::from_request_context(&ctx).unwrap();
        assert_eq!(auth.as_str(), "Bearer secret-token-xyz");
        assert_eq!(&*auth, "Bearer secret-token-xyz");

        let bearer = BearerAuth::from_request_context(&ctx).unwrap();
        assert_eq!(bearer.token(), "secret-token-xyz");
        assert_eq!(bearer.as_str(), "secret-token-xyz");
        assert_eq!(&*bearer, "secret-token-xyz");
        assert_eq!(format!("{bearer}"), "secret-token-xyz");

        let opt_bearer = Option::<BearerAuth>::from_request_context(&ctx).unwrap();
        assert_eq!(opt_bearer.unwrap().token(), "secret-token-xyz");

        // Invalid non-bearer authorization
        let mut invalid_headers = HeaderMap::new();
        invalid_headers.insert(
            http::header::AUTHORIZATION,
            "Basic dXNlcjpwYXNz".parse().unwrap(),
        );
        let invalid_ctx = RequestContext::new(
            None,
            invalid_headers,
            Arc::new(http::Extensions::new()),
        );
        let raw_auth = Authorization::from_request_context(&invalid_ctx).unwrap();
        assert_eq!(raw_auth.as_str(), "Basic dXNlcjpwYXNz");

        let bearer_err = BearerAuth::from_request_context(&invalid_ctx).unwrap_err();
        assert!(bearer_err.0.contains("expected Bearer token"));

        let opt_bearer_none = Option::<BearerAuth>::from_request_context(&invalid_ctx).unwrap();
        assert!(opt_bearer_none.is_none());
    }
}
