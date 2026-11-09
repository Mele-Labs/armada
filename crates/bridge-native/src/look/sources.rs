//! The files in `packages/tokens`, as the build embedded them, read once.
//!
//! Nothing is copied: `build.rs` names each file in an `include_str!`, so a change to a token, a
//! theme or the catalogue is a change to this crate's build and to nothing else. The shapes read
//! here are the generated ones: `tokens.json`, `themes/index.json`, and one `[data-theme="<id>"]`
//! block per catalogue file and for `light`.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::Deserialize;

use super::css;

include!(concat!(env!("OUT_DIR"), "/look_sources.rs"));

/// The theme a catalogue entry or a mod draws as: the tone says which of macOS's two it sits with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tone {
    Dark,
    Light,
}

#[derive(Deserialize)]
struct TokensFile {
    tokens: Vec<TokenRow>,
}

#[derive(Deserialize)]
struct TokenRow {
    name: String,
    value: String,
}

/// One theme of the catalogue, as `themes/index.json` lists it.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct CatalogueEntry {
    /// `catalogue:ayu-light`: the word the theme preference holds.
    pub id: String,
    pub title: String,
    pub tone: Tone,
}

/// The dark defaults: every token in `tokens.json` at its authored value.
pub fn defaults() -> &'static BTreeMap<String, String> {
    static DEFAULTS: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    DEFAULTS.get_or_init(|| {
        let file: TokensFile = ipc::decode("packages/tokens/tokens.json", TOKENS_JSON.as_bytes())
            .expect("tokens.json is the shape its generator writes");
        file.tokens.into_iter().map(|row| (row.name, row.value)).collect()
    })
}

/// The built-in light theme's declarations.
pub fn light() -> BTreeMap<String, String> {
    css::layer(LIGHT_CSS, "light")
}

/// The catalogue, in the order `themes/index.json` gives.
pub fn catalogue() -> &'static [CatalogueEntry] {
    static ENTRIES: OnceLock<Vec<CatalogueEntry>> = OnceLock::new();
    ENTRIES.get_or_init(|| {
        ipc::decode("packages/tokens/themes/index.json", CATALOGUE_INDEX.as_bytes())
            .expect("themes/index.json is the shape its generator writes")
    })
}

/// A catalogue theme's declarations, by its id.
pub fn catalogue_layer(id: &str) -> Option<BTreeMap<String, String>> {
    CATALOGUE_CSS
        .iter()
        .find(|file| css::theme_ids(file).iter().any(|named| named == id))
        .map(|file| css::layer(file, id))
}
