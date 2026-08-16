// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! MCP resources subsystem for reading, listing, and template matching.

use crate::types::jsonrpc::{JsonRpcErrorResponse, JsonRpcRequestId};
use crate::types::mcp::{
    CacheScope,
    resources::{
        BlobResourceContents, ResourceContents, TextResourceContents, read::ReadResourceResult,
    },
};

pub mod handler;
pub mod list;
pub mod read;
pub mod registry;
pub mod templates;

pub use handler::{IntoResourceHandler, ResourceHandler};
pub use list::{IntoResourcesListHandler, IntoResourcesListResult, ResourcesListHandler};
pub use read::handle_read_resource;
pub use registry::ResourceRegistry;
pub use templates::{
    IntoResourceTemplatesListHandler, IntoResourceTemplatesListResult, ResourceTemplatesListHandler,
};

/// Error type encountered during resource reading or listing operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceError {
    /// Invalid arguments provided to the resource handler.
    InvalidParams(String),
    /// The requested resource was not found.
    NotFound(String),
    /// Internal execution or business logic error.
    Internal(String),
}

impl std::fmt::Display for ResourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceError::InvalidParams(msg) => write!(f, "Invalid params: {msg}"),
            ResourceError::NotFound(msg) => write!(f, "Resource not found: {msg}"),
            ResourceError::Internal(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for ResourceError {}

impl ResourceError {
    /// Converts this error into a standard JSON-RPC error response.
    pub fn into_error_response(self, id: Option<JsonRpcRequestId>) -> JsonRpcErrorResponse {
        match self {
            ResourceError::InvalidParams(err) | ResourceError::NotFound(err) => {
                JsonRpcErrorResponse::invalid_params(id, format!("Invalid params: {err}"))
            }
            ResourceError::Internal(err) => JsonRpcErrorResponse::internal_error(id, err),
        }
    }
}

/// Trait for types that can be converted into a [`ReadResourceResult`].
pub trait IntoResourceResult: Send {
    fn into_resource_result(
        self,
        uri: &str,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ReadResourceResult, ResourceError>;
}

impl IntoResourceResult for ReadResourceResult {
    fn into_resource_result(
        self,
        _uri: &str,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ReadResourceResult, ResourceError> {
        let mut res = self;
        if res.ttl_ms.is_none() {
            res.ttl_ms = base_ttl_ms.or(Some(0));
        }
        if res.cache_scope.is_none() {
            res.cache_scope = base_cache_scope.or(Some(CacheScope::Public));
        }
        if res.result_type.is_none() {
            res.result_type = Some("complete".to_string());
        }
        Ok(res)
    }
}

impl IntoResourceResult for ResourceContents {
    fn into_resource_result(
        self,
        _uri: &str,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ReadResourceResult, ResourceError> {
        Ok(ReadResourceResult::new(vec![self]).with_cache(
            base_ttl_ms.or(Some(0)),
            base_cache_scope.or(Some(CacheScope::Public)),
        ))
    }
}

impl IntoResourceResult for Vec<ResourceContents> {
    fn into_resource_result(
        self,
        _uri: &str,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ReadResourceResult, ResourceError> {
        Ok(ReadResourceResult::new(self).with_cache(
            base_ttl_ms.or(Some(0)),
            base_cache_scope.or(Some(CacheScope::Public)),
        ))
    }
}

impl IntoResourceResult for TextResourceContents {
    fn into_resource_result(
        self,
        _uri: &str,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ReadResourceResult, ResourceError> {
        Ok(
            ReadResourceResult::new(vec![ResourceContents::Text(self)]).with_cache(
                base_ttl_ms.or(Some(0)),
                base_cache_scope.or(Some(CacheScope::Public)),
            ),
        )
    }
}

impl IntoResourceResult for BlobResourceContents {
    fn into_resource_result(
        self,
        _uri: &str,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ReadResourceResult, ResourceError> {
        Ok(
            ReadResourceResult::new(vec![ResourceContents::Blob(self)]).with_cache(
                base_ttl_ms.or(Some(0)),
                base_cache_scope.or(Some(CacheScope::Public)),
            ),
        )
    }
}

impl IntoResourceResult for String {
    fn into_resource_result(
        self,
        uri: &str,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ReadResourceResult, ResourceError> {
        Ok(
            ReadResourceResult::text(uri, self, None::<String>).with_cache(
                base_ttl_ms.or(Some(0)),
                base_cache_scope.or(Some(CacheScope::Public)),
            ),
        )
    }
}

impl IntoResourceResult for &str {
    fn into_resource_result(
        self,
        uri: &str,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ReadResourceResult, ResourceError> {
        Ok(
            ReadResourceResult::text(uri, self.to_string(), None::<String>).with_cache(
                base_ttl_ms.or(Some(0)),
                base_cache_scope.or(Some(CacheScope::Public)),
            ),
        )
    }
}

impl IntoResourceResult for crate::types::mcp::InputRequiredResult {
    fn into_resource_result(
        self,
        _uri: &str,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ReadResourceResult, ResourceError> {
        let (meta, result_type, extras) = self.into_parts();
        Ok(ReadResourceResult {
            meta,
            result_type: Some(result_type),
            ttl_ms: base_ttl_ms,
            cache_scope: base_cache_scope,
            contents: Vec::new(),
            extras,
        })
    }
}

impl<T, E> IntoResourceResult for Result<T, E>
where
    T: IntoResourceResult,
    E: std::fmt::Display + Send,
{
    fn into_resource_result(
        self,
        uri: &str,
        base_ttl_ms: Option<u64>,
        base_cache_scope: Option<CacheScope>,
    ) -> Result<ReadResourceResult, ResourceError> {
        match self {
            Ok(val) => val.into_resource_result(uri, base_ttl_ms, base_cache_scope),
            Err(err) => Err(ResourceError::Internal(err.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for `IntoResourceResult` conversions.

    use super::*;

    /// Tests `IntoResourceResult` conversions for strings, result types, and errors.
    #[test]
    fn test_into_resource_result() {
        // String
        let res_str = "resource text"
            .into_resource_result("file:///a.txt", None, None)
            .unwrap();
        assert_eq!(res_str.contents.len(), 1);
        assert_eq!(res_str.ttl_ms, Some(0));
        assert_eq!(res_str.cache_scope, Some(CacheScope::Public));
        if let ResourceContents::Text(ref t) = res_str.contents[0] {
            assert_eq!(t.uri, "file:///a.txt");
            assert_eq!(t.text, "resource text");
        } else {
            panic!("Expected text resource");
        }

        // Result::Ok
        let res_ok: Result<&str, &str> = Ok("success content");
        let res = res_ok
            .into_resource_result("memo://1", Some(60000), Some(CacheScope::Private))
            .unwrap();
        assert_eq!(res.contents.len(), 1);
        assert_eq!(res.ttl_ms, Some(60000));
        assert_eq!(res.cache_scope, Some(CacheScope::Private));

        // Result::Err
        let res_err: Result<&str, &str> = Err("file not found");
        let err = res_err
            .into_resource_result("memo://1", None, None)
            .unwrap_err();
        assert!(matches!(err, ResourceError::Internal(ref s) if s == "file not found"));
    }

    /// Tests conversion of `ResourceError` variants into `JsonRpcErrorResponse`.
    #[test]
    fn test_resource_error_into_error_response() {
        let req_id = Some(JsonRpcRequestId::Number(10.0));

        let err_invalid = ResourceError::InvalidParams("bad uri".to_string());
        let resp_invalid = err_invalid.into_error_response(req_id.clone());
        assert_eq!(resp_invalid.id, req_id);
        assert_eq!(
            resp_invalid.error.code,
            crate::types::jsonrpc::JsonRpcErrorCode::InvalidParams
        );
        assert_eq!(resp_invalid.error.message, "Invalid params: bad uri");

        let err_not_found = ResourceError::NotFound("memo://missing".to_string());
        let resp_not_found = err_not_found.into_error_response(req_id.clone());
        assert_eq!(resp_not_found.id, req_id);
        assert_eq!(
            resp_not_found.error.code,
            crate::types::jsonrpc::JsonRpcErrorCode::InvalidParams
        );
        assert_eq!(
            resp_not_found.error.message,
            "Invalid params: memo://missing"
        );

        let err_internal = ResourceError::Internal("disk error".to_string());
        let resp_internal = err_internal.into_error_response(req_id.clone());
        assert_eq!(resp_internal.id, req_id);
        assert_eq!(
            resp_internal.error.code,
            crate::types::jsonrpc::JsonRpcErrorCode::InternalError
        );
        assert_eq!(resp_internal.error.message, "disk error");
    }
}
