// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::{borrow::Cow, collections::HashMap};

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod server;
pub mod tools;

/// A progress token, used to associate progress notifications with the original request.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#progresstoken>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProgressToken {
    /// A numeric progress token.
    Number(f32),
    /// A string progress token.
    String(String),
}

/// An icon that can be displayed in a user interface.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#icon>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Icon {
    /// A standard URI (HTTPS or `data:` URI with Base64-encoded data) pointing to the icon resource.
    pub src: String,
    /// The MIME type of the icon image (e.g. `image/png`, `image/svg+xml`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<Cow<'static, str>>,
    /// Optional array of strings specifying icon dimensions in "WxH" format (e.g. "48x48") or "any" for scalable formats.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sizes: Vec<String>,
    /// The theme background (light or dark) this icon is designed for.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<IconTheme>,
}

/// Specifies whether an icon is intended for a light or dark theme context.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#icontheme>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IconTheme {
    /// Designed for light background themes.
    Light,
    /// Designed for dark background themes.
    Dark,
}

/// The severity level of a log message.
///
/// Maps to RFC-5424 syslog severities.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#logginglevel>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LoggingLevel {
    /// Detailed debugging information.
    Debug,
    /// Normal operational messages.
    Info,
    /// Normal but significant conditions.
    Notice,
    /// Warning conditions.
    Warning,
    /// Error conditions.
    Error,
    /// Critical conditions.
    Critical,
    /// Action must be taken immediately.
    Alert,
    /// System is unusable.
    Emergency,
}

/// An implementation structure identifying a client or server.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#implementation>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Implementation {
    /// Optional set of sized icons that the client can display in a user interface.
    //
    /// Clients that support rendering icons MUST support at least the following MIME types:
    ///  - `image/png` - PNG images (safe, universal compatibility)
    ///  - `image/jpeg` (and `image/jpg`) - JPEG images (safe, universal compatibility)
    ///
    /// Clients that support rendering icons SHOULD also support:
    ///  - image/svg+xml - SVG images (scalable but requires security precautions)
    ///  - image/webp - WebP images (modern, efficient format)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub icons: Vec<Icon>,
    /// Intended for programmatic or logical use, but used as a display name in past specs or
    /// fallback (if title isn’t present).
    pub name: String,
    /// Intended for UI and end-user contexts — optimized to be human-readable and easily
    /// understood, even by those unfamiliar with domain-specific terminology.
    ///
    /// If not provided, the `name` should be used for display (except for `Tool`, where
    /// `annotations.title` should be given precedence over using `name`, if present).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The version of this implementation.
    pub version: String,
    /// An optional human-readable description of what this implementation does.
    ///
    /// This can be used by clients or servers to provide context about their purpose and
    /// capabilities. For example, a server might describe the types of resources or tools it
    /// provides, while a client might describe its intended use case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// An optional URL of the website for this implementation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website_url: Option<String>,
}

impl Implementation {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            icons: Vec::new(),
            name: name.into(),
            title: None,
            version: version.into(),
            description: None,
            website_url: None,
        }
    }
}

/// Capabilities a client may support. Known capabilities are defined here, in this schema,
/// but this is not a closed set: any client can define its own, additional capabilities.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#clientcapabilities>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientCapabilities {
    /// Experimental, non-standard capabilities that the client supports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<HashMap<String, Value>>,
    /// Present if the client supports sampling LLM completions from the server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<SamplingCapability>,
    /// Present if the client supports server-driven elicitation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elicitation: Option<ElicitationCapability>,
}

/// Capability configuration for sampling LLM completions.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#clientcapabilities>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SamplingCapability {}

/// Capability configuration for server-driven elicitation.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#clientcapabilities>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElicitationCapability {}

/// Capabilities a server may support. Known capabilities are defined here, in this schema,
/// but this is not a closed set: any server can define its own, additional capabilities.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#servercapabilities>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCapabilities {
    /// Present if the server supports tool operations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,
    /// Present if the server supports resource operations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourcesCapability>,
    /// Present if the server supports prompt templates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptsCapability>,
    /// Present if the server supports argument/value completion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completions: Option<CompletionsCapability>,
    /// Experimental, non-standard capabilities that the server supports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<HashMap<String, Value>>,
}

/// Capability configuration for tool operations.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#servercapabilities>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsCapability {
    /// Optional hint indicating whether the server emits notifications when tool lists change.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Capability configuration for resource operations.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#servercapabilities>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesCapability {
    /// Optional hint indicating whether the server supports subscribing to resource updates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscribe: Option<bool>,
    /// Optional hint indicating whether the server emits notifications when resource lists change.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Capability configuration for prompt templates.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#servercapabilities>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptsCapability {
    /// Optional hint indicating whether the server emits notifications when prompt lists change.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Capability configuration for completion operations.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#servercapabilities>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionsCapability {}

/// An object containing metadata for a result.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#resultmetaobject>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultMetaObject {
    /// Identifies the server software producing the response.
    /// Servers SHOULD include this field on every response unless specifically configured not to do so.
    #[serde(
        rename = "io.modelcontextprotocol/serverInfo",
        skip_serializing_if = "Option::is_none"
    )]
    pub server_info: Option<Implementation>,
    /// Additional metadata properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, Value>,
}

/// An object containing metadata for a request.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#requestmetaobject>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestMetaObject {
    /// A progress token used to associate progress notifications with the original request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress_token: Option<ProgressToken>,
    /// Identifies the client software producing the request.
    #[serde(
        rename = "io.modelcontextprotocol/clientInfo",
        skip_serializing_if = "Option::is_none"
    )]
    pub client_info: Option<Implementation>,
    /// Capabilities supported by the client for this request.
    #[serde(
        rename = "io.modelcontextprotocol/clientCapabilities",
        skip_serializing_if = "Option::is_none"
    )]
    pub client_capabilities: Option<ClientCapabilities>,
    /// Specifies the MCP protocol version being used for the request.
    #[serde(
        rename = "io.modelcontextprotocol/protocolVersion",
        skip_serializing_if = "Option::is_none"
    )]
    pub protocol_version: Option<String>,
    /// Desired log level for the request.
    #[serde(
        rename = "io.modelcontextprotocol/logLevel",
        skip_serializing_if = "Option::is_none"
    )]
    pub log_level: Option<LoggingLevel>,
    /// Additional metadata properties.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, Value>,
}

/// Additional metadata associated with a request or entity.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#metaobject>
pub type MetaObject = HashMap<String, Value>;

/// Specifies the scope for caching responses (`public` or `private`).
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#cachescope>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CacheScope {
    /// The response contains no user-specific data and may be cached publicly.
    Public,
    /// The response contains user-specific data and should only be cached privately.
    Private,
}

/// Specifies the role of an entity in a conversation.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#role>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Role {
    User,
    Assistant,
}

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

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests serialization and deserialization of [`ClientCapabilities`].
    #[test]
    fn test_client_capabilities_serde() {
        let json_data = serde_json::json!({
            "sampling": {},
            "elicitation": {}
        });

        let caps: ClientCapabilities = serde_json::from_value(json_data).unwrap();
        assert!(caps.sampling.is_some());
        assert!(caps.elicitation.is_some());
        assert!(caps.experimental.is_none());

        let reserialized = serde_json::to_value(&caps).unwrap();
        assert!(reserialized.get("sampling").is_some());
    }

    /// Tests serialization and deserialization of [`ServerCapabilities`].
    #[test]
    fn test_server_capabilities_serde() {
        let mut exp = HashMap::new();
        exp.insert("customFeature".to_string(), serde_json::json!({"enabled": true}));

        let server_caps = ServerCapabilities {
            tools: Some(ToolsCapability { list_changed: Some(true) }),
            resources: Some(ResourcesCapability { subscribe: Some(true), list_changed: Some(false) }),
            prompts: Some(PromptsCapability { list_changed: Some(true) }),
            completions: Some(CompletionsCapability {}),
            experimental: Some(exp),
        };
        let s_val = serde_json::to_value(&server_caps).unwrap();
        assert_eq!(s_val["tools"]["listChanged"], true);
        assert_eq!(s_val["resources"]["subscribe"], true);
        assert_eq!(s_val["resources"]["listChanged"], false);
        assert_eq!(s_val["prompts"]["listChanged"], true);
        assert!(s_val.get("completions").is_some());
    }

    /// Tests serialization and deserialization of [`ResultMetaObject`].
    #[test]
    fn test_result_meta_object_serde() {
        let json_data = serde_json::json!({
            "io.modelcontextprotocol/serverInfo": {
                "name": "test-server",
                "version": "1.0.0"
            },
            "custom/meta": "value"
        });

        let meta: ResultMetaObject = serde_json::from_value(json_data).unwrap();
        assert_eq!(meta.server_info.as_ref().unwrap().name, "test-server");
        assert_eq!(meta.extra.get("custom/meta").unwrap(), "value");

        let reserialized = serde_json::to_value(&meta).unwrap();
        assert_eq!(
            reserialized["io.modelcontextprotocol/serverInfo"]["name"],
            "test-server"
        );
        assert_eq!(reserialized["custom/meta"], "value");
    }

    /// Tests serialization and deserialization of [`RequestMetaObject`] with [`LoggingLevel`].
    #[test]
    fn test_request_meta_object_with_log_level() {
        let json_data = serde_json::json!({
            "io.modelcontextprotocol/logLevel": "debug"
        });

        let meta: RequestMetaObject = serde_json::from_value(json_data).unwrap();
        assert!(matches!(meta.log_level, Some(LoggingLevel::Debug)));

        let reserialized = serde_json::to_value(&meta).unwrap();
        assert_eq!(
            reserialized["io.modelcontextprotocol/logLevel"],
            "debug"
        );
    }

    /// Tests all RFC-5424 [`LoggingLevel`] enum variants.
    #[test]
    fn test_logging_levels_serde() {
        let levels = vec![
            (LoggingLevel::Debug, "\"debug\""),
            (LoggingLevel::Info, "\"info\""),
            (LoggingLevel::Notice, "\"notice\""),
            (LoggingLevel::Warning, "\"warning\""),
            (LoggingLevel::Error, "\"error\""),
            (LoggingLevel::Critical, "\"critical\""),
            (LoggingLevel::Alert, "\"alert\""),
            (LoggingLevel::Emergency, "\"emergency\""),
        ];

        for (level, expected_json) in levels {
            let serialized = serde_json::to_string(&level).unwrap();
            assert_eq!(serialized, expected_json);

            let deserialized: LoggingLevel = serde_json::from_str(expected_json).unwrap();
            assert_eq!(
                serde_json::to_string(&deserialized).unwrap(),
                expected_json
            );
        }
    }

    /// Tests numeric and string [`ProgressToken`] parsing.
    #[test]
    fn test_progress_token_serde() {
        let num_token: ProgressToken = serde_json::from_value(serde_json::json!(42.0)).unwrap();
        assert!(matches!(num_token, ProgressToken::Number(n) if (n - 42.0).abs() < f32::EPSILON));

        let str_token: ProgressToken = serde_json::from_value(serde_json::json!("tok-123")).unwrap();
        assert!(matches!(str_token, ProgressToken::String(s) if s == "tok-123"));
    }

    /// Tests [`CacheScope`] and [`Role`] enum serialization and deserialization.
    #[test]
    fn test_cache_scope_and_role_serde() {
        let public_scope: CacheScope = serde_json::from_str("\"public\"").unwrap();
        assert!(matches!(public_scope, CacheScope::Public));
        assert_eq!(serde_json::to_string(&public_scope).unwrap(), "\"public\"");

        let private_scope: CacheScope = serde_json::from_str("\"private\"").unwrap();
        assert!(matches!(private_scope, CacheScope::Private));
        assert_eq!(serde_json::to_string(&private_scope).unwrap(), "\"private\"");

        let user_role: Role = serde_json::from_str("\"user\"").unwrap();
        assert!(matches!(user_role, Role::User));
        assert_eq!(serde_json::to_string(&user_role).unwrap(), "\"user\"");

        let assistant_role: Role = serde_json::from_str("\"assistant\"").unwrap();
        assert!(matches!(assistant_role, Role::Assistant));
        assert_eq!(serde_json::to_string(&assistant_role).unwrap(), "\"assistant\"");
    }

    /// Tests [`Implementation`] metadata and [`Icon`] structures.
    #[test]
    fn test_implementation_and_icons_serde() {
        let impl_info = Implementation {
            icons: vec![
                Icon {
                    src: "https://example.com/icon1.png".to_string(),
                    mime_type: Some("image/png".into()),
                    sizes: vec!["32x32".to_string()],
                    theme: Some(IconTheme::Light),
                },
                Icon {
                    src: "https://example.com/icon2.svg".to_string(),
                    mime_type: Some("image/svg+xml".into()),
                    sizes: vec!["any".to_string()],
                    theme: Some(IconTheme::Dark),
                },
            ],
            name: "my-server".to_string(),
            title: Some("My Server Title".to_string()),
            version: "3.2.1".to_string(),
            description: Some("Description here".to_string()),
            website_url: Some("https://example.com".to_string()),
        };

        let val = serde_json::to_value(&impl_info).unwrap();
        assert_eq!(val["name"], "my-server");
        assert_eq!(val["title"], "My Server Title");
        assert_eq!(val["version"], "3.2.1");
        assert_eq!(val["description"], "Description here");
        assert_eq!(val["websiteUrl"], "https://example.com");
        assert_eq!(val["icons"].as_array().unwrap().len(), 2);
        assert_eq!(val["icons"][0]["theme"], "light");
        assert_eq!(val["icons"][1]["theme"], "dark");

        let deserialized: Implementation = serde_json::from_value(val).unwrap();
        assert_eq!(deserialized.name, "my-server");
        assert_eq!(deserialized.icons.len(), 2);
    }

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
}
