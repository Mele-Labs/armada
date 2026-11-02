//! What a `mock` node in a sketch is drawn with: the stylesheets nearest the code a Job is working
//! on, which the agent names in `styles`, and the style set Fleet kept when it drew the sketch.
//! `.armada/documents/hi-fi-sketches.html`, *How Fleet finds the styles*.

use serde::{Deserialize, Serialize};

/// What a file found for a `mock` is, by how a browser can use it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StyleKind {
    Css,
    CssModule,
    TailwindOutput,
    TailwindReset,
    ThemeCss,
    TailwindConfig,
    Scss,
    Less,
    ThemeModule,
}

/// A design token read out of a stylesheet or a theme file: its name and its value, never the
/// file's text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StyleToken {
    pub name: String,
    pub value: String,
}

/// One stylesheet, or a file that stands for one, found near the code a Job works on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StyleSource {
    /// Relative to the repository.
    pub path: String,
    pub bytes: u64,
    pub kind: StyleKind,
    /// Whether a browser can use the file as it stands. A `scss`, a `less` or a Tailwind config
    /// cannot, and is listed so the agent knows why it was not offered.
    pub usable: bool,
    pub tokens: Vec<StyleToken>,
}

/// `get_style_sources`: the stylesheets nearest the code being worked on, for a `mock` node's
/// `styles`. `GET /sessions/style_sources?path=&session_id=`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StyleSources {
    /// Closest first.
    pub sources: Vec<StyleSource>,
    /// The path the search walked up from. Absent where the call named none and Fleet chose.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    /// The package paths of the repository, listed where Fleet would not guess between them. Empty
    /// where `sources` answers the call.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<String>,
}

/// A stylesheet as Fleet kept it with a sketch.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeptSheet {
    pub path: String,
    pub css: String,
    /// The Job's checkout held uncommitted changes to the file when it was kept.
    pub edited: bool,
}

/// A font a kept stylesheet names. `bytes` is base64.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeptFont {
    pub family: String,
    /// The `@font-face` value as written: `"400"`, or `"100 900"` for a variable font.
    pub weight: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    pub bytes: String,
}

/// `get_kept_styles`: a style set Fleet kept, by the hash a `mock` node's `kept` names.
/// `GET /sessions/kept_styles?hash=`. **Content addressed, so an answer never changes.** Read on
/// demand and never carried on a record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeptStyles {
    /// In the order they apply.
    pub sheets: Vec<KeptSheet>,
    pub fonts: Vec<KeptFont>,
}
