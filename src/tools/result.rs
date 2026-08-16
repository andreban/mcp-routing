// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Result conversion traits and implementations for MCP tool handler return types.

use serde_json::Value;

use crate::extract::Json;
use crate::types::mcp::{ContentBlock, InputRequiredResult, tools::call::CallToolResult};

/// Trait for types that can be converted into a [`CallToolResult`].
pub trait IntoToolResult: Send {
    fn into_tool_result(self) -> CallToolResult;
}

impl<S> IntoToolResult for CallToolResult<S>
where
    S: serde::Serialize + Send,
{
    fn into_tool_result(self) -> CallToolResult {
        let structured_content = match self.structured_content {
            Some(s) => match serde_json::to_value(s) {
                Ok(v) => Some(v),
                Err(err) => {
                    return CallToolResult::error(format!(
                        "Failed to serialize structured output: {err}"
                    ));
                }
            },
            None => None,
        };

        CallToolResult {
            meta: self.meta,
            result_type: self.result_type,
            content: self.content,
            is_error: self.is_error,
            structured_content,
            extras: self.extras,
        }
    }
}

impl IntoToolResult for InputRequiredResult {
    fn into_tool_result(self) -> CallToolResult {
        let (meta, result_type, extras) = self.into_parts();
        CallToolResult {
            meta,
            result_type: Some(result_type),
            content: Vec::new(),
            is_error: None,
            structured_content: None,
            extras,
        }
    }
}

impl IntoToolResult for String {
    fn into_tool_result(self) -> CallToolResult {
        CallToolResult::text(self)
    }
}

impl IntoToolResult for &str {
    fn into_tool_result(self) -> CallToolResult {
        CallToolResult::text(self)
    }
}

impl IntoToolResult for ContentBlock {
    fn into_tool_result(self) -> CallToolResult {
        CallToolResult::with_content(vec![self])
    }
}

impl IntoToolResult for Vec<ContentBlock> {
    fn into_tool_result(self) -> CallToolResult {
        CallToolResult::with_content(self)
    }
}

impl IntoToolResult for Value {
    fn into_tool_result(self) -> CallToolResult {
        CallToolResult::structured(self)
    }
}

impl<T> IntoToolResult for Json<T>
where
    T: serde::Serialize + Send,
{
    fn into_tool_result(self) -> CallToolResult {
        match serde_json::to_value(&self.0) {
            Ok(val) => CallToolResult::structured(val),
            Err(err) => {
                CallToolResult::error(format!("Failed to serialize structured output: {err}"))
            }
        }
    }
}

macro_rules! impl_into_tool_result_tuple {
    (text: $text_ty:ty) => {
        impl IntoToolResult for (Value, $text_ty) {
            fn into_tool_result(self) -> CallToolResult {
                CallToolResult::structured_with_text(self.0, self.1)
            }
        }

        impl IntoToolResult for ($text_ty, Value) {
            fn into_tool_result(self) -> CallToolResult {
                CallToolResult::structured_with_text(self.1, self.0)
            }
        }

        impl<T> IntoToolResult for (Json<T>, $text_ty)
        where
            T: serde::Serialize + Send,
        {
            fn into_tool_result(self) -> CallToolResult {
                match serde_json::to_value(&self.0 .0) {
                    Ok(val) => CallToolResult::structured_with_text(val, self.1),
                    Err(err) => {
                        CallToolResult::error(format!("Failed to serialize structured output: {err}"))
                    }
                }
            }
        }

        impl<T> IntoToolResult for ($text_ty, Json<T>)
        where
            T: serde::Serialize + Send,
        {
            fn into_tool_result(self) -> CallToolResult {
                match serde_json::to_value(&self.1 .0) {
                    Ok(val) => CallToolResult::structured_with_text(val, self.0),
                    Err(err) => {
                        CallToolResult::error(format!("Failed to serialize structured output: {err}"))
                    }
                }
            }
        }
    };

    (content: $content_ty:ty, $into_vec:expr) => {
        impl IntoToolResult for (Value, $content_ty) {
            fn into_tool_result(self) -> CallToolResult {
                CallToolResult::structured_with_content(self.0, $into_vec(self.1))
            }
        }

        impl IntoToolResult for ($content_ty, Value) {
            fn into_tool_result(self) -> CallToolResult {
                CallToolResult::structured_with_content(self.1, $into_vec(self.0))
            }
        }

        impl<T> IntoToolResult for (Json<T>, $content_ty)
        where
            T: serde::Serialize + Send,
        {
            fn into_tool_result(self) -> CallToolResult {
                match serde_json::to_value(&self.0 .0) {
                    Ok(val) => CallToolResult::structured_with_content(val, $into_vec(self.1)),
                    Err(err) => {
                        CallToolResult::error(format!("Failed to serialize structured output: {err}"))
                    }
                }
            }
        }

        impl<T> IntoToolResult for ($content_ty, Json<T>)
        where
            T: serde::Serialize + Send,
        {
            fn into_tool_result(self) -> CallToolResult {
                match serde_json::to_value(&self.1 .0) {
                    Ok(val) => CallToolResult::structured_with_content(val, $into_vec(self.0)),
                    Err(err) => {
                        CallToolResult::error(format!("Failed to serialize structured output: {err}"))
                    }
                }
            }
        }
    };
}

impl_into_tool_result_tuple!(text: String);
impl_into_tool_result_tuple!(text: &'static str);
impl_into_tool_result_tuple!(content: ContentBlock, |b| vec![b]);
impl_into_tool_result_tuple!(content: Vec<ContentBlock>, |b| b);

impl<T, E> IntoToolResult for Result<T, E>
where
    T: IntoToolResult,
    E: std::fmt::Display + Send,
{
    fn into_tool_result(self) -> CallToolResult {
        match self {
            Ok(val) => val.into_tool_result(),
            Err(err) => CallToolResult::error(err.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for `IntoToolResult` conversions and macroized tuple implementations.

    use super::*;
    use crate::types::mcp::TextContent;

    #[derive(serde::Serialize)]
    struct Output {
        count: usize,
    }

    struct FailingSerializer;

    impl serde::Serialize for FailingSerializer {
        fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            Err(serde::ser::Error::custom("serialization error"))
        }
    }

    /// Tests `IntoToolResult` implementations across primitive and standard return types.
    #[test]
    fn test_into_tool_result_primitives_and_results() {
        // &str
        let res_str = "hello".into_tool_result();
        assert_eq!(res_str.is_error, Some(false));
        if let ContentBlock::Text(ref t) = res_str.content[0] {
            assert_eq!(t.text, "hello");
        } else {
            panic!("Expected text block");
        }

        // Owned String
        let res_owned = "world".to_string().into_tool_result();
        assert_eq!(res_owned.is_error, Some(false));
        if let ContentBlock::Text(ref t) = res_owned.content[0] {
            assert_eq!(t.text, "world");
        } else {
            panic!("Expected text block");
        }

        // Result::Ok
        let res_ok: Result<&str, &str> = Ok("success");
        let res = res_ok.into_tool_result();
        assert_eq!(res.is_error, Some(false));
        if let ContentBlock::Text(ref t) = res.content[0] {
            assert_eq!(t.text, "success");
        } else {
            panic!("Expected text block");
        }

        // Result::Err
        let res_err: Result<&str, &str> = Err("failure");
        let res = res_err.into_tool_result();
        assert_eq!(res.is_error, Some(true));
        if let ContentBlock::Text(ref t) = res.content[0] {
            assert_eq!(t.text, "failure");
        } else {
            panic!("Expected text block");
        }

        // ContentBlock
        let block = ContentBlock::Text(TextContent {
            text: "block".to_string(),
            annotations: None,
            meta: None,
        });
        let res_block = block.into_tool_result();
        assert_eq!(res_block.content.len(), 1);

        // Vec<ContentBlock>
        let blocks = vec![
            ContentBlock::Text(TextContent {
                text: "block1".to_string(),
                annotations: None,
                meta: None,
            }),
            ContentBlock::Text(TextContent {
                text: "block2".to_string(),
                annotations: None,
                meta: None,
            }),
        ];
        let res_blocks = blocks.into_tool_result();
        assert_eq!(res_blocks.content.len(), 2);

        // Value
        let val = serde_json::json!({ "answer": 42 });
        let res_val = val.into_tool_result();
        assert_eq!(res_val.structured_content.unwrap()["answer"], 42);

        // Json<T>
        let res_json = Json(Output { count: 5 }).into_tool_result();
        assert_eq!(res_json.structured_content.unwrap()["count"], 5);

        // Json<FailingSerializer>
        let res_failing_json = Json(FailingSerializer).into_tool_result();
        assert_eq!(res_failing_json.is_error, Some(true));

        // Generic CallToolResult<Output>
        let custom_res = CallToolResult::structured(Output { count: 99 }).with_text("Count report");
        let res_custom = custom_res.into_tool_result();
        assert_eq!(res_custom.structured_content.unwrap()["count"], 99);
        if let ContentBlock::Text(ref t) = res_custom.content[0] {
            assert_eq!(t.text, "Count report");
        } else {
            panic!("Expected text block");
        }

        // Generic CallToolResult with FailingSerializer
        let failing_custom_res = CallToolResult::structured(FailingSerializer);
        let res_failing_custom = failing_custom_res.into_tool_result();
        assert_eq!(res_failing_custom.is_error, Some(true));

        // InputRequiredResult
        let input_req = InputRequiredResult {
            meta: None,
            result_type: "input_required".to_string(),
            input_requests: std::collections::HashMap::new(),
            request_state: Some("state-123".to_string()),
            extras: std::collections::HashMap::new(),
        };
        let res_input_req = input_req.into_tool_result();
        assert_eq!(res_input_req.result_type.as_deref(), Some("input_required"));
        assert_eq!(
            res_input_req.extras.get("requestState").and_then(|v| v.as_str()),
            Some("state-123")
        );
    }

    /// Tests macroized `IntoToolResult` tuple conversions for text and structured data.
    #[test]
    fn test_into_tool_result_text_tuples() {
        // (Value, String) and (String, Value)
        let res1 = (serde_json::json!({ "ok": true }), "text1".to_string()).into_tool_result();
        assert_eq!(res1.structured_content.unwrap()["ok"], true);
        if let ContentBlock::Text(ref t) = res1.content[0] {
            assert_eq!(t.text, "text1");
        }

        let res2 = ("text2".to_string(), serde_json::json!({ "ok": true })).into_tool_result();
        assert_eq!(res2.structured_content.unwrap()["ok"], true);
        if let ContentBlock::Text(ref t) = res2.content[0] {
            assert_eq!(t.text, "text2");
        }

        // (Value, &'static str) and (&'static str, Value)
        let res3 = (serde_json::json!({ "ok": true }), "text3").into_tool_result();
        assert_eq!(res3.structured_content.unwrap()["ok"], true);
        if let ContentBlock::Text(ref t) = res3.content[0] {
            assert_eq!(t.text, "text3");
        }

        let res4 = ("text4", serde_json::json!({ "ok": true })).into_tool_result();
        assert_eq!(res4.structured_content.unwrap()["ok"], true);
        if let ContentBlock::Text(ref t) = res4.content[0] {
            assert_eq!(t.text, "text4");
        }

        // (Json<T>, String) and (String, Json<T>)
        let res5 = (Json(Output { count: 10 }), "text5".to_string()).into_tool_result();
        assert_eq!(res5.structured_content.unwrap()["count"], 10);
        if let ContentBlock::Text(ref t) = res5.content[0] {
            assert_eq!(t.text, "text5");
        }

        let res6 = ("text6".to_string(), Json(Output { count: 10 })).into_tool_result();
        assert_eq!(res6.structured_content.unwrap()["count"], 10);
        if let ContentBlock::Text(ref t) = res6.content[0] {
            assert_eq!(t.text, "text6");
        }

        // (Json<T>, &'static str) and (&'static str, Json<T>)
        let res7 = (Json(Output { count: 20 }), "text7").into_tool_result();
        assert_eq!(res7.structured_content.unwrap()["count"], 20);
        if let ContentBlock::Text(ref t) = res7.content[0] {
            assert_eq!(t.text, "text7");
        }

        let res8 = ("text8", Json(Output { count: 20 })).into_tool_result();
        assert_eq!(res8.structured_content.unwrap()["count"], 20);
        if let ContentBlock::Text(ref t) = res8.content[0] {
            assert_eq!(t.text, "text8");
        }

        // Serialization error cases in text tuples
        let res_err1 = (Json(FailingSerializer), "error_test").into_tool_result();
        assert_eq!(res_err1.is_error, Some(true));

        let res_err2 = ("error_test", Json(FailingSerializer)).into_tool_result();
        assert_eq!(res_err2.is_error, Some(true));
    }

    /// Tests macroized `IntoToolResult` tuple conversions for content blocks and structured data.
    #[test]
    fn test_into_tool_result_content_tuples() {
        let make_block = |s: &str| {
            ContentBlock::Text(TextContent {
                text: s.to_string(),
                annotations: None,
                meta: None,
            })
        };

        // (Value, ContentBlock) and (ContentBlock, Value)
        let res1 = (serde_json::json!({ "id": 1 }), make_block("b1")).into_tool_result();
        assert_eq!(res1.structured_content.unwrap()["id"], 1);
        assert_eq!(res1.content.len(), 1);

        let res2 = (make_block("b2"), serde_json::json!({ "id": 2 })).into_tool_result();
        assert_eq!(res2.structured_content.unwrap()["id"], 2);
        assert_eq!(res2.content.len(), 1);

        // (Value, Vec<ContentBlock>) and (Vec<ContentBlock>, Value)
        let res3 = (serde_json::json!({ "id": 3 }), vec![make_block("b3a"), make_block("b3b")]).into_tool_result();
        assert_eq!(res3.structured_content.unwrap()["id"], 3);
        assert_eq!(res3.content.len(), 2);

        let res4 = (vec![make_block("b4a"), make_block("b4b")], serde_json::json!({ "id": 4 })).into_tool_result();
        assert_eq!(res4.structured_content.unwrap()["id"], 4);
        assert_eq!(res4.content.len(), 2);

        // (Json<T>, ContentBlock) and (ContentBlock, Json<T>)
        let res5 = (Json(Output { count: 50 }), make_block("b5")).into_tool_result();
        assert_eq!(res5.structured_content.unwrap()["count"], 50);
        assert_eq!(res5.content.len(), 1);

        let res6 = (make_block("b6"), Json(Output { count: 60 })).into_tool_result();
        assert_eq!(res6.structured_content.unwrap()["count"], 60);
        assert_eq!(res6.content.len(), 1);

        // (Json<T>, Vec<ContentBlock>) and (Vec<ContentBlock>, Json<T>)
        let res7 = (Json(Output { count: 70 }), vec![make_block("b7")]).into_tool_result();
        assert_eq!(res7.structured_content.unwrap()["count"], 70);
        assert_eq!(res7.content.len(), 1);

        let res8 = (vec![make_block("b8")], Json(Output { count: 80 })).into_tool_result();
        assert_eq!(res8.structured_content.unwrap()["count"], 80);
        assert_eq!(res8.content.len(), 1);

        // Serialization error cases in content tuples
        let res_err1 = (Json(FailingSerializer), make_block("err")).into_tool_result();
        assert_eq!(res_err1.is_error, Some(true));

        let res_err2 = (make_block("err"), Json(FailingSerializer)).into_tool_result();
        assert_eq!(res_err2.is_error, Some(true));

        let res_err3 = (Json(FailingSerializer), vec![make_block("err")]).into_tool_result();
        assert_eq!(res_err3.is_error, Some(true));

        let res_err4 = (vec![make_block("err")], Json(FailingSerializer)).into_tool_result();
        assert_eq!(res_err4.is_error, Some(true));
    }
}
