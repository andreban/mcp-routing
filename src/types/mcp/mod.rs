use std::borrow::Cow;

use serde::{Deserialize, Serialize};

pub mod tools;

/// A progress token, used to associate progress notifications with the original request.
///
/// See https://modelcontextprotocol.io/specification/2026-07-28/schema#progresstoken
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProgressToken {
    /// A numeric progress token.
    Number(f32),
    /// A string progress token.
    String(String),
}

/// An icon that can be displayed in a user interface.
///
/// See https://modelcontextprotocol.io/specification/2026-07-28/schema#icon
#[derive(Debug, Serialize, Deserialize)]
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
/// See https://modelcontextprotocol.io/specification/2026-07-28/schema#icontheme
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
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
/// See https://modelcontextprotocol.io/specification/2026-07-28/schema#logginglevel
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
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
/// See https://modelcontextprotocol.io/specification/2026-07-28/schema#implementation
#[derive(Debug, Serialize, Deserialize)]
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
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
