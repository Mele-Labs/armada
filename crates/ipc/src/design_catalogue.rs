//! `get_design_catalogue`: what a Job draws a `mock` with, read once instead of grepped for. The
//! tokens of a repository by role and theme, its components with their real class names, the glyphs
//! a mock may name, and the patterns the repository keeps in `.armada/catalogue/patterns.html`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Which part of the catalogue a call asks for. `Contents` answers names only.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogueArea {
    #[default]
    Contents,
    Tokens,
    Components,
    Icons,
    Patterns,
}

impl CatalogueArea {
    /// The area a query names, or `None` for a word that is not one.
    pub fn named(word: &str) -> Option<CatalogueArea> {
        match word.trim() {
            "" | "contents" => Some(CatalogueArea::Contents),
            "tokens" => Some(CatalogueArea::Tokens),
            "components" => Some(CatalogueArea::Components),
            "icons" => Some(CatalogueArea::Icons),
            "patterns" => Some(CatalogueArea::Patterns),
            _ => None,
        }
    }
}

/// What a `get_design_catalogue` call narrows to, each as the query gave it, trimmed and absent
/// where empty. Not on the wire: the route reads it from the query.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CatalogueAsked {
    pub area: Option<String>,
    pub matching: Option<String>,
    pub repository: Option<String>,
}

/// What a token is for, read from its name and value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenRole {
    Colour,
    Space,
    Type,
    Radius,
    Shadow,
    Motion,
    #[default]
    Other,
}

/// One token: its value in the default theme with `var()` resolved, and where another theme sets it
/// differently, that theme's value.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogueToken {
    pub name: String,
    pub value: String,
    /// The value as written, where it points at another token: what a mock's `css` should write.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    /// Theme name to value, only where it differs from `value`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub themes: BTreeMap<String, String>,
}

/// Tokens of one role sharing a name family, such as `--status` or `--space`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenGroup {
    pub role: TokenRole,
    pub family: String,
    pub tokens: Vec<CatalogueToken>,
}

/// A component of the repository, by the file that defines it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogueComponent {
    pub name: String,
    /// From the repository's root.
    pub file: String,
    /// The stylesheet beside it, where it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sheet: Option<String>,
    pub classes: Vec<String>,
    /// HTML a mock can copy: the elements of the file that carry its classes, nested as written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample: Option<String>,
}

/// A house pattern the repository keeps, as HTML a mock can copy.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CataloguePattern {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub html: String,
}

/// One token group as the contents list it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenFamily {
    pub role: TokenRole,
    pub family: String,
    pub tokens: usize,
}

/// The names in each area, to choose what to ask for.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogueContents {
    pub tokens: Vec<TokenFamily>,
    pub components: Vec<String>,
    pub patterns: Vec<String>,
    pub icons: usize,
}

/// `GET /sessions/design_catalogue?area=&match=&repository=&session_id=`. Only the asked area is
/// filled; the rest are empty.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignCatalogue {
    pub area: CatalogueArea,
    /// The theme names tokens are set under, `default` first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub themes: Vec<String>,
    /// The stylesheets that set the tokens, for a mock's `styles`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sheets: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contents: Option<CatalogueContents>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tokens: Vec<TokenGroup>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<CatalogueComponent>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub icons: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub patterns: Vec<CataloguePattern>,
    /// What the answer left out to stay small, and how to ask for it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cut: Option<String>,
}
