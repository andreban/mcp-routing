// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::types::mcp::{Icon, MetaObject, Role, content::ContentAnnotations};

pub mod list;
pub mod read;
pub mod templates;

pub use list::{
    ListResourcesParams, ListResourcesRequest, ListResourcesResult, ListResourcesResultResponse,
};
pub use read::{
    ReadResourceParams, ReadResourceRequest, ReadResourceResult, ReadResourceResultResponse,
};
pub use templates::{
    ListResourceTemplatesParams, ListResourceTemplatesRequest, ListResourceTemplatesResult,
    ListResourceTemplatesResultResponse,
};

/// Annotations that can be attached to resources and resource templates.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#resourceannotations>
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceAnnotations {
    /// Intended audience for the resource.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub audience: Vec<Role>,
    /// Priority level for resource inclusion/processing (0.0 to 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<f64>,
    /// Optional ISO-8601 timestamp string when the resource was last modified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,
}

impl ResourceAnnotations {
    /// Creates a new empty [`ResourceAnnotations`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the intended audience roles.
    pub fn audience(mut self, audience: Vec<Role>) -> Self {
        self.audience = audience;
        self
    }

    /// Sets the priority level (0.0 to 1.0).
    pub fn priority(mut self, priority: f64) -> Self {
        self.priority = Some(priority);
        self
    }

    /// Sets the last modified timestamp string.
    pub fn last_modified(mut self, last_modified: impl Into<String>) -> Self {
        self.last_modified = Some(last_modified.into());
        self
    }
}

/// The definition of a resource exposed by an MCP server.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#resource>
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    /// Optional list of icons for display in user interfaces.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub icons: Vec<Icon>,
    /// The URI of this resource.
    pub uri: String,
    /// A human-readable name for this resource.
    pub name: String,
    /// Human-readable display title for the resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// A description of what this resource represents.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The MIME type of this resource, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// The size of the raw resource content, in bytes, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    /// Optional execution hints and annotations for this resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ResourceAnnotations>,
    /// Optional protocol-level metadata.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaObject>,
}

impl Resource {
    /// Creates a new [`Resource`] definition with the given URI and name.
    pub fn new(uri: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            icons: Vec::new(),
            uri: uri.into(),
            name: name.into(),
            title: None,
            description: None,
            mime_type: None,
            size: None,
            annotations: None,
            meta: None,
        }
    }

    /// Sets the human-readable display title for the resource.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the human-readable description for the resource.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the MIME type of the resource.
    pub fn mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }

    /// Sets the size of the resource in bytes.
    pub fn size(mut self, size: u64) -> Self {
        self.size = Some(size);
        self
    }

    /// Sets the annotations for this resource.
    pub fn annotations(mut self, annotations: ResourceAnnotations) -> Self {
        self.annotations = Some(annotations);
        self
    }

    /// Appends an icon to the resource's icon list.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icons.push(icon);
        self
    }

    /// Sets the full list of icons for this resource.
    pub fn icons(mut self, icons: Vec<Icon>) -> Self {
        self.icons = icons;
        self
    }

    /// Sets the protocol-level metadata for this resource.
    pub fn meta(mut self, meta: MetaObject) -> Self {
        self.meta = Some(meta);
        self
    }
}

impl<U, N> From<(U, N)> for Resource
where
    U: Into<String>,
    N: Into<String>,
{
    fn from((uri, name): (U, N)) -> Self {
        Self::new(uri, name)
    }
}

/// The definition of a resource template exposed by an MCP server.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#resourcetemplate>
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceTemplate {
    /// Optional list of icons for display in user interfaces.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub icons: Vec<Icon>,
    /// A URI template (RFC 6570) that can be used to construct resource URIs.
    pub uri_template: String,
    /// A human-readable name for this template.
    pub name: String,
    /// Human-readable display title for the resource template.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// A description of what this resource template represents.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The MIME type of the resources constructed from this template, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Optional execution hints and annotations for this template.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ResourceAnnotations>,
    /// Optional protocol-level metadata.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaObject>,
}

impl ResourceTemplate {
    /// Creates a new [`ResourceTemplate`] definition with the given URI template and name.
    pub fn new(uri_template: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            icons: Vec::new(),
            uri_template: uri_template.into(),
            name: name.into(),
            title: None,
            description: None,
            mime_type: None,
            annotations: None,
            meta: None,
        }
    }

    /// Sets the human-readable display title for the resource template.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the human-readable description for the resource template.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the MIME type of the resources created by this template.
    pub fn mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }

    /// Sets the annotations for this resource template.
    pub fn annotations(mut self, annotations: ResourceAnnotations) -> Self {
        self.annotations = Some(annotations);
        self
    }

    /// Appends an icon to the resource template's icon list.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icons.push(icon);
        self
    }

    /// Sets the full list of icons for this resource template.
    pub fn icons(mut self, icons: Vec<Icon>) -> Self {
        self.icons = icons;
        self
    }

    /// Sets the protocol-level metadata for this resource template.
    pub fn meta(mut self, meta: MetaObject) -> Self {
        self.meta = Some(meta);
        self
    }
}

impl<U, N> From<(U, N)> for ResourceTemplate
where
    U: Into<String>,
    N: Into<String>,
{
    fn from((uri_template, name): (U, N)) -> Self {
        Self::new(uri_template, name)
    }
}

/// Text resource contents.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ResourceContents {
    Text(TextResourceContents),
    Blob(BlobResourceContents),
}

impl ResourceContents {
    /// Creates a text resource contents.
    pub fn text(
        uri: impl Into<String>,
        text: impl Into<String>,
        mime_type: Option<impl Into<String>>,
    ) -> Self {
        Self::Text(TextResourceContents {
            uri: uri.into(),
            text: text.into(),
            mime_type: mime_type.map(Into::into),
        })
    }

    /// Creates a binary blob resource contents.
    pub fn blob(
        uri: impl Into<String>,
        blob: impl Into<String>,
        mime_type: Option<impl Into<String>>,
    ) -> Self {
        Self::Blob(BlobResourceContents {
            uri: uri.into(),
            blob: blob.into(),
            mime_type: mime_type.map(Into::into),
        })
    }
}

impl From<TextResourceContents> for ResourceContents {
    fn from(t: TextResourceContents) -> Self {
        Self::Text(t)
    }
}

impl From<BlobResourceContents> for ResourceContents {
    fn from(b: BlobResourceContents) -> Self {
        Self::Blob(b)
    }
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

    /// Tests `Into<Resource>` conversions and builder methods.
    #[test]
    fn test_resource_builder_and_conversions() {
        let res: Resource = ("file:///readme.md", "README").into();
        assert_eq!(res.uri, "file:///readme.md");
        assert_eq!(res.name, "README");
        assert_eq!(res.title, None);

        let custom_res = Resource::new("memo://system-prompt", "System Memo")
            .title("System Memo Title")
            .description("Internal server system memo")
            .mime_type("text/plain")
            .size(512)
            .annotations(
                ResourceAnnotations::new()
                    .audience(vec![Role::Assistant])
                    .priority(0.9)
                    .last_modified("2026-08-15T12:00:00Z"),
            );

        assert_eq!(custom_res.uri, "memo://system-prompt");
        assert_eq!(custom_res.name, "System Memo");
        assert_eq!(custom_res.title.as_deref(), Some("System Memo Title"));
        assert_eq!(custom_res.size, Some(512));
        assert_eq!(custom_res.mime_type.as_deref(), Some("text/plain"));
        let ann = custom_res.annotations.unwrap();
        assert_eq!(ann.priority, Some(0.9));
        assert_eq!(ann.last_modified.as_deref(), Some("2026-08-15T12:00:00Z"));
    }

    /// Tests `Into<ResourceTemplate>` conversions and builder methods.
    #[test]
    fn test_resource_template_builder_and_conversions() {
        let template: ResourceTemplate = ("file:///{path}", "Project Files").into();
        assert_eq!(template.uri_template, "file:///{path}");
        assert_eq!(template.name, "Project Files");

        let custom_tmpl = ResourceTemplate::new("postgres://{table}", "Database Tables")
            .title("Database Table View")
            .description("Query table data")
            .mime_type("application/json")
            .annotations(ResourceAnnotations::new().priority(0.8));

        assert_eq!(custom_tmpl.uri_template, "postgres://{table}");
        assert_eq!(custom_tmpl.name, "Database Tables");
        assert_eq!(custom_tmpl.title.as_deref(), Some("Database Table View"));
        assert_eq!(custom_tmpl.mime_type.as_deref(), Some("application/json"));
    }

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
