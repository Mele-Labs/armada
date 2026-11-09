//! The icons the registry names, served to gpui by name.
//!
//! `packages/icons/icons/` names a glyph by its lucide name, and gpui-kit-assets ships the lucide
//! SVGs under `icons/<name>.svg`. This is the `AssetSource` the app registers, so `svg().path(..)`
//! and gpui-component's own icons read the same set, and [`icon_path`] turns a registry name into
//! the path to draw.

use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};
use gpui_kit_assets::AllAssets;

/// The path an icon is drawn from, by the name the registry gives it: `bot` is `icons/bot.svg`.
pub fn icon_path(name: &str) -> SharedString {
    format!("icons/{name}.svg").into()
}

/// The whole lucide set, 1,830 glyphs, and nothing else. A path it has no file for is `None`
/// rather than an error, so one missing glyph is a gap in a row and not a failure in the log.
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(AllAssets.load(path).unwrap_or(None))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        AllAssets.list(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::actions::ACTIONS;
    use crate::generated::vocabulary::VOCABULARIES;

    fn has(name: &str) -> bool {
        Assets.load(&icon_path(name)).ok().flatten().is_some()
    }

    #[test]
    fn a_registry_name_is_a_path_the_set_serves() {
        assert_eq!(icon_path("bot"), "icons/bot.svg");
        assert!(has("bot"));
        assert!(!has("no-such-glyph"));
    }

    #[test]
    fn every_glyph_the_vocabulary_names_is_in_the_set() {
        let missing: Vec<String> = VOCABULARIES
            .iter()
            .flat_map(|(vocabulary, rows)| rows.iter().map(move |(variant, row)| (*vocabulary, *variant, row.glyph)))
            .filter_map(|(vocabulary, variant, glyph)| glyph.filter(|name| !has(name)).map(|name| format!("{vocabulary}.{variant}: {name}")))
            .collect();
        assert!(missing.is_empty(), "{missing:?}");
    }

    #[test]
    fn every_glyph_an_act_names_is_in_the_set() {
        let missing: Vec<String> = ACTIONS
            .iter()
            .filter_map(|row| row.glyph.filter(|name| !has(name)).map(|name| format!("{}: {name}", row.id)))
            .collect();
        assert!(missing.is_empty(), "{missing:?}");
    }
}
