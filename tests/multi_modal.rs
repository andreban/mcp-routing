// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # Multi-Modal Content Integration Tests
//!
//! Verifies the Model Context Protocol (MCP) multi-modal content block types, including:
//! - [`TextContent`](mcp_routing::types::mcp::TextContent) with audience and priority annotations
//! - [`ImageContent`](mcp_routing::types::mcp::ImageContent) with base64 payload and MIME types
//! - [`AudioContent`](mcp_routing::types::mcp::AudioContent) with base64 audio and MIME types
//! - [`EmbeddedResource`](mcp_routing::types::mcp::EmbeddedResource) supporting text and binary blob contents
//! - [`ResourceLink`](mcp_routing::types::mcp::ResourceLink) referencing external or hosted resources
//! - Direct return of single [`ContentBlock`](mcp_routing::types::mcp::ContentBlock) vs. multi-block collections
//! - Structured JSON output (`structured_content`) alongside content blocks

mod common;

use http::StatusCode;
use mcp_routing::{
    McpRouter,
    types::mcp::{
        AudioContent, BlobResourceContents, ContentAnnotations, ContentBlock, EmbeddedResource,
        ImageContent, ResourceContents, ResourceLink, Role, TextContent, TextResourceContents,
        tools::call::{CallToolResult, CallToolResultResponse},
    },
};
use serde_json::json;
use std::collections::HashMap;

/// Handler returning a single annotated [`ContentBlock::Text`].
async fn handle_single_text_block() -> ContentBlock {
    let mut meta = HashMap::new();
    meta.insert("source".to_string(), json!("integration-test"));

    ContentBlock::Text(TextContent {
        text: "Single text block content".to_string(),
        annotations: Some(ContentAnnotations {
            audience: vec![Role::User, Role::Assistant],
            priority: Some(0.9),
        }),
        meta: Some(meta),
    })
}

/// Handler returning a single [`ContentBlock::Image`].
async fn handle_single_image_block() -> ContentBlock {
    ContentBlock::Image(ImageContent {
        data: "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==".to_string(),
        mime_type: "image/png".to_string(),
        annotations: Some(ContentAnnotations {
            audience: vec![Role::User],
            priority: Some(1.0),
        }),
        meta: None,
    })
}

/// Handler returning a comprehensive [`CallToolResult`] containing every supported content block type and structured output.
async fn handle_multi_modal_all() -> CallToolResult {
    let text_block = ContentBlock::Text(TextContent {
        text: "Here is the comprehensive report".to_string(),
        annotations: Some(ContentAnnotations {
            audience: vec![Role::User],
            priority: Some(0.8),
        }),
        meta: None,
    });

    let image_block = ContentBlock::Image(ImageContent {
        data: "aW1hZ2UtZGF0YQ==".to_string(),
        mime_type: "image/jpeg".to_string(),
        annotations: None,
        meta: None,
    });

    let audio_block = ContentBlock::Audio(AudioContent {
        data: "YXVkaW8tZGF0YQ==".to_string(),
        mime_type: "audio/wav".to_string(),
        annotations: Some(ContentAnnotations {
            audience: vec![Role::Assistant],
            priority: Some(0.5),
        }),
        meta: None,
    });

    let text_resource = ContentBlock::Resource(EmbeddedResource {
        resource: ResourceContents::Text(TextResourceContents {
            uri: "file:///workspace/notes.txt".to_string(),
            text: "Important configuration notes".to_string(),
            mime_type: Some("text/plain".to_string()),
        }),
        annotations: None,
        meta: None,
    });

    let blob_resource = ContentBlock::Resource(EmbeddedResource {
        resource: ResourceContents::Blob(BlobResourceContents {
            uri: "file:///workspace/binary.dat".to_string(),
            blob: "YmluYXJ5LWRhdGE=".to_string(),
            mime_type: Some("application/octet-stream".to_string()),
        }),
        annotations: None,
        meta: None,
    });

    let resource_link = ContentBlock::ResourceLink(ResourceLink {
        uri: "https://example.com/docs/api".to_string(),
        name: Some("API Documentation".to_string()),
        description: Some("External reference documentation".to_string()),
        mime_type: Some("text/html".to_string()),
        annotations: None,
        meta: None,
    });

    let mut result = CallToolResult::with_content(vec![
        text_block,
        image_block,
        audio_block,
        text_resource,
        blob_resource,
        resource_link,
    ]);

    result.structured_content = Some(json!({
        "summary": "Multi-modal result summary",
        "totalBlocks": 6
    }));

    result
}

/// Tests returning a single [`ContentBlock::Text`] from a tool handler.
///
/// Verifies:
/// - [`IntoToolResult`](mcp_routing::tools::IntoToolResult) conversion for [`ContentBlock`]
/// - Correct serialization of text content, multi-role audience annotations, priority score, and custom metadata
#[tokio::test]
async fn test_multi_modal_single_text_block() {
    let app = McpRouter::new(common::sample_server_info())
        .register_tool("single_text", handle_single_text_block);

    let req = common::build_request(
        Some("tools/call"),
        Some("single_text"),
        json!({
            "jsonrpc": "2.0",
            "id": "text-test",
            "method": "tools/call",
            "params": { "name": "single_text" }
        }),
    );

    let (status, _headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    let res: CallToolResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res.id, "text-test".into());
    assert_eq!(res.result.content.len(), 1);

    if let ContentBlock::Text(ref text_block) = res.result.content[0] {
        assert_eq!(text_block.text, "Single text block content");
        let annotations = text_block.annotations.as_ref().unwrap();
        assert_eq!(annotations.audience.len(), 2);
        assert_eq!(annotations.priority, Some(0.9));
        assert_eq!(
            text_block.meta.as_ref().unwrap().get("source").unwrap(),
            "integration-test"
        );
    } else {
        panic!("Expected ContentBlock::Text");
    }
}

/// Tests returning a single [`ContentBlock::Image`] from a tool handler.
///
/// Verifies:
/// - Correct serialization of Base64 image payload and MIME type (`image/png`)
/// - Image priority annotation formatting
#[tokio::test]
async fn test_multi_modal_single_image_block() {
    let app = McpRouter::new(common::sample_server_info())
        .register_tool("single_image", handle_single_image_block);

    let req = common::build_request(
        Some("tools/call"),
        Some("single_image"),
        json!({
            "jsonrpc": "2.0",
            "id": "image-test",
            "method": "tools/call",
            "params": { "name": "single_image" }
        }),
    );

    let (status, _headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    let res: CallToolResultResponse = serde_json::from_value(body).unwrap();
    assert_eq!(res.result.content.len(), 1);

    if let ContentBlock::Image(ref img) = res.result.content[0] {
        assert_eq!(img.mime_type, "image/png");
        assert!(img.data.starts_with("iVBORw0KGgoAAAANSUhEUg"));
        assert_eq!(img.annotations.as_ref().unwrap().priority, Some(1.0));
    } else {
        panic!("Expected ContentBlock::Image");
    }
}

/// Tests returning a multi-modal composite tool result containing all 6 content block types.
///
/// Verifies:
/// - Correct tag-based discriminators (`type: "text"`, `"image"`, `"audio"`, `"resource"`, `"resource_link"`)
/// - Text embedded resources containing string text and URI
/// - Binary blob embedded resources containing Base64 data and URI
/// - Resource links containing name, description, and target URI
/// - Structured output (`structured_content`) object attached to the result
#[tokio::test]
async fn test_multi_modal_comprehensive_result() {
    let app = McpRouter::new(common::sample_server_info())
        .register_tool("multi_modal_tool", handle_multi_modal_all);

    let req = common::build_request(
        Some("tools/call"),
        Some("multi_modal_tool"),
        json!({
            "jsonrpc": "2.0",
            "id": 888,
            "method": "tools/call",
            "params": { "name": "multi_modal_tool" }
        }),
    );

    let (status, _headers, body) = common::execute_request(app, req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"]["content"][0]["type"], "text");
    assert_eq!(body["result"]["content"][1]["type"], "image");
    assert_eq!(body["result"]["content"][2]["type"], "audio");
    assert_eq!(body["result"]["content"][3]["type"], "resource");
    assert_eq!(body["result"]["content"][4]["type"], "resource");
    assert_eq!(body["result"]["content"][5]["type"], "resource_link");

    let res: CallToolResultResponse = serde_json::from_value(body.clone()).unwrap();
    assert_eq!(res.id, 888.into());
    assert_eq!(res.result.is_error, Some(false));
    assert_eq!(res.result.content.len(), 6);

    // 0: Text
    if let ContentBlock::Text(ref t) = res.result.content[0] {
        assert_eq!(t.text, "Here is the comprehensive report");
    } else {
        panic!("Block 0 should be Text");
    }

    // 1: Image
    if let ContentBlock::Image(ref img) = res.result.content[1] {
        assert_eq!(img.mime_type, "image/jpeg");
        assert_eq!(img.data, "aW1hZ2UtZGF0YQ==");
    } else {
        panic!("Block 1 should be Image");
    }

    // 2: Audio
    if let ContentBlock::Audio(ref aud) = res.result.content[2] {
        assert_eq!(aud.mime_type, "audio/wav");
        assert_eq!(aud.data, "YXVkaW8tZGF0YQ==");
    } else {
        panic!("Block 2 should be Audio");
    }

    // 3: Text Resource
    if let ContentBlock::Resource(ref r) = res.result.content[3] {
        if let ResourceContents::Text(ref text_res) = r.resource {
            assert_eq!(text_res.uri, "file:///workspace/notes.txt");
            assert_eq!(text_res.text, "Important configuration notes");
            assert_eq!(text_res.mime_type.as_deref(), Some("text/plain"));
        } else {
            panic!("Expected ResourceContents::Text");
        }
    } else {
        panic!("Block 3 should be Resource");
    }

    // 4: Blob Resource
    if let ContentBlock::Resource(ref r) = res.result.content[4] {
        if let ResourceContents::Blob(ref blob_res) = r.resource {
            assert_eq!(blob_res.uri, "file:///workspace/binary.dat");
            assert_eq!(blob_res.blob, "YmluYXJ5LWRhdGE=");
            assert_eq!(
                blob_res.mime_type.as_deref(),
                Some("application/octet-stream")
            );
        } else {
            panic!("Expected ResourceContents::Blob");
        }
    } else {
        panic!("Block 4 should be Resource");
    }

    // 5: Resource Link
    if let ContentBlock::ResourceLink(ref link) = res.result.content[5] {
        assert_eq!(link.uri, "https://example.com/docs/api");
        assert_eq!(link.name.as_deref(), Some("API Documentation"));
        assert_eq!(
            link.description.as_deref(),
            Some("External reference documentation")
        );
        assert_eq!(link.mime_type.as_deref(), Some("text/html"));
    } else {
        panic!("Block 5 should be ResourceLink");
    }

    // Structured content verification
    let structured = res
        .result
        .structured_content
        .expect("structured_content should be present");
    assert_eq!(structured["summary"], "Multi-modal result summary");
    assert_eq!(structured["totalBlocks"], 6);
}
