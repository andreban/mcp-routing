// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

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

impl Icon {
    /// Creates a new [`Icon`] with the specified source URI.
    pub fn new(src: impl Into<String>) -> Self {
        Self {
            src: src.into(),
            mime_type: None,
            sizes: Vec::new(),
            theme: None,
        }
    }

    /// Sets the MIME type of the icon.
    pub fn with_mime_type(mut self, mime_type: impl Into<Cow<'static, str>>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }

    /// Sets the icon dimensions.
    pub fn with_sizes(mut self, sizes: Vec<String>) -> Self {
        self.sizes = sizes;
        self
    }

    /// Sets the theme background context for this icon.
    pub fn with_theme(mut self, theme: IconTheme) -> Self {
        self.theme = Some(theme);
        self
    }
}

/// An implementation structure identifying a client or server.
///
/// See <https://modelcontextprotocol.io/specification/2026-07-28/schema#implementation>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Implementation {
    /// Optional set of sized icons that the client can display in a user interface.
    ///
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
    /// Creates a new [`Implementation`] with a name and version.
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

    /// Sets the human-readable display title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the description of the implementation.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the website URL for this implementation.
    pub fn with_website_url(mut self, website_url: impl Into<String>) -> Self {
        self.website_url = Some(website_url.into());
        self
    }

    /// Adds an icon to the implementation.
    pub fn with_icon(mut self, icon: Icon) -> Self {
        self.icons.push(icon);
        self
    }

    /// Sets the list of icons for this implementation.
    pub fn with_icons(mut self, icons: Vec<Icon>) -> Self {
        self.icons = icons;
        self
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests [`CacheScope`] and [`Role`] enum serialization and deserialization.
    #[test]
    fn test_cache_scope_and_role_serde() {
        let public_scope: CacheScope = serde_json::from_str("\"public\"").unwrap();
        assert!(matches!(public_scope, CacheScope::Public));
        assert_eq!(serde_json::to_string(&public_scope).unwrap(), "\"public\"");

        let private_scope: CacheScope = serde_json::from_str("\"private\"").unwrap();
        assert!(matches!(private_scope, CacheScope::Private));
        assert_eq!(
            serde_json::to_string(&private_scope).unwrap(),
            "\"private\""
        );

        let user_role: Role = serde_json::from_str("\"user\"").unwrap();
        assert!(matches!(user_role, Role::User));
        assert_eq!(serde_json::to_string(&user_role).unwrap(), "\"user\"");

        let assistant_role: Role = serde_json::from_str("\"assistant\"").unwrap();
        assert!(matches!(assistant_role, Role::Assistant));
        assert_eq!(
            serde_json::to_string(&assistant_role).unwrap(),
            "\"assistant\""
        );
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
}
