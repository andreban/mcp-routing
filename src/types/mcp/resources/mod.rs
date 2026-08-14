// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::types::mcp::{content::ContentAnnotations, core::MetaObject};

/// Text resource contents.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextResourceContents {
    /// The URI of the resource.
    pub uri: String,
    /// The text content of the resource.
    pub text: String,
    /// Optional MIME type of the resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// Blob resource contents.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlobResourceContents {
    /// The URI of the resource.
    pub uri: String,
    /// Base64-encoded binary blob data.
    pub blob: String,
    /// Optional MIME type of the resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// Resource contents (text or blob).
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#resourcecontents>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResourceContents {
    Text(TextResourceContents),
    Blob(BlobResourceContents),
}

/// Embedded resource content block.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#embeddedresource>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddedResource {
    /// The embedded resource contents.
    pub resource: ResourceContents,
    /// Optional annotations for this content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ContentAnnotations>,
    /// Optional protocol-level metadata.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaObject>,
}

/// Resource link content block.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#resourcelink>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceLink {
    /// The URI of the linked resource.
    pub uri: String,
    /// Name or title of the resource link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Description of the resource link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional MIME type of the linked resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Optional annotations for this content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ContentAnnotations>,
    /// Optional protocol-level metadata.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaObject>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests serialization and deserialization of [`ResourceContents`] (Text and Blob).
    #[test]
    fn test_resource_contents_serde() {
        let text_res = ResourceContents::Text(TextResourceContents {
            uri: "file:///test.txt".to_string(),
            text: "hello world".to_string(),
            mime_type: Some("text/plain".to_string()),
        });
        let val = serde_json::to_value(&text_res).unwrap();
        assert_eq!(val["uri"], "file:///test.txt");
        assert_eq!(val["text"], "hello world");
        assert_eq!(val["mimeType"], "text/plain");

        let blob_res = ResourceContents::Blob(BlobResourceContents {
            uri: "file:///test.bin".to_string(),
            blob: "aGVsbG8=".to_string(),
            mime_type: Some("application/octet-stream".to_string()),
        });
        let blob_val = serde_json::to_value(&blob_res).unwrap();
        assert_eq!(blob_val["uri"], "file:///test.bin");
        assert_eq!(blob_val["blob"], "aGVsbG8=");
    }
}
