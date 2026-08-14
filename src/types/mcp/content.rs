// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use super::core::{MetaObject, Role};
use super::resources::{EmbeddedResource, ResourceLink};

/// Annotations that can be attached to content blocks.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#contentannotations>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentAnnotations {
    /// Intended audience for the content.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub audience: Vec<Role>,
    /// Priority level for content inclusion/processing (0.0 to 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<f32>,
}

/// Text content block.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#textcontent>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextContent {
    /// The text content.
    pub text: String,
    /// Optional annotations for this content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ContentAnnotations>,
    /// Optional protocol-level metadata.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaObject>,
}

/// Image content block.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#imagecontent>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageContent {
    /// Base64-encoded image data.
    pub data: String,
    /// MIME type of the image (e.g., `image/png`, `image/jpeg`).
    pub mime_type: String,
    /// Optional annotations for this content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ContentAnnotations>,
    /// Optional protocol-level metadata.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaObject>,
}

/// Audio content block.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#audiocontent>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioContent {
    /// Base64-encoded audio data.
    pub data: String,
    /// MIME type of the audio (e.g., `audio/wav`, `audio/mp3`).
    pub mime_type: String,
    /// Optional annotations for this content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ContentAnnotations>,
    /// Optional protocol-level metadata.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaObject>,
}

/// A content block in a message or tool result.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#contentblock>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ContentBlock {
    Text(TextContent),
    Image(ImageContent),
    Audio(AudioContent),
    Resource(EmbeddedResource),
    ResourceLink(ResourceLink),
}

impl From<TextContent> for ContentBlock {
    fn from(t: TextContent) -> Self {
        ContentBlock::Text(t)
    }
}

impl From<ImageContent> for ContentBlock {
    fn from(i: ImageContent) -> Self {
        ContentBlock::Image(i)
    }
}

impl From<AudioContent> for ContentBlock {
    fn from(a: AudioContent) -> Self {
        ContentBlock::Audio(a)
    }
}

impl From<EmbeddedResource> for ContentBlock {
    fn from(r: EmbeddedResource) -> Self {
        ContentBlock::Resource(r)
    }
}

impl From<ResourceLink> for ContentBlock {
    fn from(l: ResourceLink) -> Self {
        ContentBlock::ResourceLink(l)
    }
}

impl From<String> for ContentBlock {
    fn from(s: String) -> Self {
        ContentBlock::Text(TextContent {
            text: s,
            annotations: None,
            meta: None,
        })
    }
}

impl From<&str> for ContentBlock {
    fn from(s: &str) -> Self {
        s.to_string().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::mcp::resources::{BlobResourceContents, ResourceContents, TextResourceContents};

    /// Tests serialization and deserialization for all multi-modal [`ContentBlock`] variants.
    #[test]
    fn test_all_content_blocks_serde() {
        let content_blocks = vec![
            ContentBlock::Text(TextContent {
                text: "sample text".to_string(),
                annotations: Some(ContentAnnotations {
                    audience: vec![Role::User],
                    priority: Some(0.7),
                }),
                meta: None,
            }),
            ContentBlock::Image(ImageContent {
                data: "base64image".to_string(),
                mime_type: "image/png".to_string(),
                annotations: None,
                meta: None,
            }),
            ContentBlock::Audio(AudioContent {
                data: "base64audio".to_string(),
                mime_type: "audio/wav".to_string(),
                annotations: None,
                meta: None,
            }),
            ContentBlock::Resource(EmbeddedResource {
                resource: ResourceContents::Text(TextResourceContents {
                    uri: "file:///test.txt".to_string(),
                    text: "text inside resource".to_string(),
                    mime_type: Some("text/plain".to_string()),
                }),
                annotations: None,
                meta: None,
            }),
            ContentBlock::Resource(EmbeddedResource {
                resource: ResourceContents::Blob(BlobResourceContents {
                    uri: "file:///test.bin".to_string(),
                    blob: "base64blob".to_string(),
                    mime_type: Some("application/octet-stream".to_string()),
                }),
                annotations: None,
                meta: None,
            }),
            ContentBlock::ResourceLink(ResourceLink {
                uri: "https://example.com/item".to_string(),
                name: Some("Item link".to_string()),
                description: Some("Item link description".to_string()),
                mime_type: Some("text/html".to_string()),
                annotations: None,
                meta: None,
            }),
        ];

        let val = serde_json::to_value(&content_blocks).unwrap();
        assert_eq!(val[0]["type"], "text");
        assert_eq!(val[1]["type"], "image");
        assert_eq!(val[2]["type"], "audio");
        assert_eq!(val[3]["type"], "resource");
        assert_eq!(val[4]["type"], "resource");
        assert_eq!(val[5]["type"], "resourceLink");

        let deserialized: Vec<ContentBlock> = serde_json::from_value(val).unwrap();
        assert_eq!(deserialized.len(), 6);
    }

    /// Tests string conversions for [`ContentBlock`].
    #[test]
    fn test_content_block_conversions() {
        let block_str: ContentBlock = "hello world".into();
        assert!(matches!(block_str, ContentBlock::Text(t) if t.text == "hello world"));

        let block_string: ContentBlock = String::from("hello string").into();
        assert!(matches!(block_string, ContentBlock::Text(t) if t.text == "hello string"));
    }
}
