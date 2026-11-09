//! What the app looks like, read from `packages/tokens` and never written here.
//!
//! A screen asks the [`Look`] global for a token by its constant (`token::BG_BASE`) and gets a
//! gpui type back. Which theme is in force is the person's choice, kept by Fleet as the `theme`
//! preference: `dark`, `light`, a catalogue id or a mod's name (see [`Choice`]). Armada's own
//! dark and light follow macOS; a catalogue theme or a mod stays as picked. [`LookController`] watches the choice, the mods Fleet lists and the window's
//! appearance, and replaces the global once when any of them moves.

mod components;
mod controller;
mod css;
mod sheet;
mod sources;
mod value;

use std::collections::BTreeMap;
use std::time::Duration;

use gpui::{App, BoxShadow, Global, Hsla, Pixels, Rgba, SharedString};

pub use controller::LookController;
pub use sheet::Unresolved;
#[allow(unused_imports)]
pub use sources::{catalogue, token, CatalogueEntry, Tone};

use sheet::{Resolved, Sheet};
use value::Color;

/// A token's name, as a constant, so a misspelling does not build. They are made from
/// `tokens.json` by `build.rs`: see [`token`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Token(pub &'static str);

/// What a colour token that is not a colour reads as: loud on purpose, so a view that asks for a
/// font stack as a colour is seen at once. A test holds every colour constant to a real colour.
const NOT_A_COLOUR: Hsla = gpui::hsla(0.83, 1.0, 0.5, 1.0);

/// The resolved theme in force.
#[derive(Clone, Debug, PartialEq)]
pub struct Look {
    id: String,
    tone: Tone,
    resolved: Resolved,
}

impl Global for Look {}

/// What a person's theme word means.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Choice {
    /// Armada's own theme, dark or light as macOS has it now. The owner chose this on 9 Nov 2026:
    /// `dark`, `light`, `system` and no word at all all name it.
    BuiltIn,
    /// `catalogue:ayu-light`.
    Catalogue(String),
    /// A mod's name, drawn once Fleet has checked its sheet.
    Mod(String),
}

impl Choice {
    pub fn parse(word: &str) -> Choice {
        match word.trim() {
            "" | "dark" | "light" | "system" => Choice::BuiltIn,
            id if id.starts_with("catalogue:") => Choice::Catalogue(id.to_string()),
            name => Choice::Mod(name.to_string()),
        }
    }
}

/// Fleet's checked stylesheets of the theme mods, by mod name.
pub type ModSheets = BTreeMap<String, String>;

impl Look {
    /// The theme a word names, under macOS's appearance `system` and the mod sheets Fleet has
    /// checked. **A word that names nothing drawable is Dark**, as in the Electron Bridge: a
    /// catalogue id this build does not have, a mod not yet checked, a mod gone.
    pub fn pick(choice: &Choice, system: Tone, mods: &ModSheets) -> Look {
        match choice {
            Choice::BuiltIn => match system {
                Tone::Dark => Look::dark(),
                Tone::Light => Look::light(),
            },
            Choice::Catalogue(id) => Look::catalogue(id).unwrap_or_else(Look::dark),
            Choice::Mod(name) => mods.get(name).map_or_else(Look::dark, |sheet| Look::of_mod(name, sheet)),
        }
    }

    pub fn dark() -> Look {
        Look::build("dark", Tone::Dark, BTreeMap::new())
    }

    pub fn light() -> Look {
        Look::build("light", Tone::Light, sources::light())
    }

    pub fn catalogue(id: &str) -> Option<Look> {
        let layer = sources::catalogue_layer(id)?;
        let tone = tone_of(&layer).unwrap_or(Tone::Dark);
        Some(Look::build(id, tone, layer))
    }

    /// A mod's theme from the sheet Fleet checked. A sheet that names no block for the mod, or
    /// none at all, draws the defaults.
    pub fn of_mod(name: &str, css: &str) -> Look {
        let layer = css::layer(css, name);
        let tone = tone_of(&layer).unwrap_or(Tone::Dark);
        Look::build(name, tone, layer)
    }

    fn build(id: &str, tone: Tone, layer: BTreeMap<String, String>) -> Look {
        let base = Sheet::new().lay(sources::defaults().clone());
        let mut resolved = Resolved::of(&base.clone().lay(layer.clone()));
        if !resolved.unresolved.is_empty() {
            resolved.fill_from(&Resolved::of(&base));
        }
        Look { id: id.to_string(), tone, resolved }
    }

    /// `dark`, `light`, a catalogue id or a mod's name: what is drawn, which may not be what was chosen.
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn tone(&self) -> Tone {
        self.tone
    }

    /// Tokens this theme wrote that could not be resolved, by token, with why. Each was drawn with
    /// the default's value instead.
    pub fn unresolved(&self) -> &BTreeMap<String, Unresolved> {
        &self.resolved.unresolved
    }

    pub fn color(&self, token: Token) -> Hsla {
        self.resolved.colors.get(token.0).map_or(NOT_A_COLOUR, |color| hsla_of(*color))
    }

    pub fn length(&self, token: Token) -> Pixels {
        gpui::px(self.resolved.lengths.get(token.0).copied().unwrap_or(0.0))
    }

    pub fn duration(&self, token: Token) -> Duration {
        self.resolved.durations.get(token.0).copied().unwrap_or_default()
    }

    /// A bare number: a weight, a z-index.
    pub fn number(&self, token: Token) -> f32 {
        self.resolved.numbers.get(token.0).copied().unwrap_or(0.0)
    }

    /// A percentage as a fraction: `40%` is 0.4.
    pub fn percent(&self, token: Token) -> f32 {
        self.resolved.percents.get(token.0).copied().unwrap_or(0.0)
    }

    /// The text of a token gpui has no type for, with its `var()` replaced.
    pub fn text(&self, token: Token) -> Option<&str> {
        self.resolved.text.get(token.0).map(String::as_str)
    }

    /// `cubic-bezier(a, b, c, d)` as its four numbers.
    pub fn easing(&self, token: Token) -> Option<[f32; 4]> {
        self.resolved.easing(token.0)
    }

    /// A font stack as its family names, in order.
    pub fn font_stack(&self, token: Token) -> Vec<SharedString> {
        self.resolved.font_stack(token.0).unwrap_or_default().into_iter().map(SharedString::from).collect()
    }

    pub fn shadows(&self, token: Token) -> Vec<BoxShadow> {
        self.resolved
            .shadows(token.0)
            .unwrap_or_default()
            .into_iter()
            .map(|layer| BoxShadow {
                color: hsla_of(layer.color),
                offset: gpui::point(gpui::px(layer.x), gpui::px(layer.y)),
                blur_radius: gpui::px(layer.blur),
                spread_radius: gpui::px(layer.spread),
                inset: layer.inset,
            })
            .collect()
    }

    /// `2px solid #4A9EDB`, as the width and the colour.
    pub fn edge(&self, token: Token) -> Option<(Pixels, Hsla)> {
        let (width, color) = value::parse_edge(self.text(token)?)?;
        Some((gpui::px(width), hsla_of(color)))
    }
}

fn hsla_of(color: Color) -> Hsla {
    Rgba { r: color.r, g: color.g, b: color.b, a: color.a }.into()
}

/// `color-scheme: light` in a theme's block makes it a light theme; anything else, or nothing, dark.
fn tone_of(layer: &BTreeMap<String, String>) -> Option<Tone> {
    match layer.get("color-scheme")?.split_whitespace().next()? {
        "light" => Some(Tone::Light),
        "dark" => Some(Tone::Dark),
        _ => None,
    }
}

pub trait ActiveLook {
    fn look(&self) -> &Look;
}

impl ActiveLook for App {
    fn look(&self) -> &Look {
        self.global::<Look>()
    }
}

/// Put `look` in force: the global a screen reads, gpui-component's own theme, and one refresh of
/// every window. Called by [`LookController`] and by [`init`]; a screen does not call it.
pub fn install(look: Look, cx: &mut App) {
    components::apply(&look, cx);
    cx.set_global(look);
    cx.refresh_windows();
}

/// Load the fonts and put the first theme in force, before any window opens. Until Fleet has
/// spoken the theme is Dark; [`LookController`] moves it from there.
pub fn init(cx: &mut App) {
    components::load_fonts(cx);
    install(Look::dark(), cx);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_words_a_preference_holds() {
        for word in ["dark", "", "light", "system"] {
            assert_eq!(Choice::parse(word), Choice::BuiltIn, "{word:?}");
        }
        assert_eq!(Choice::parse("catalogue:ayu-light"), Choice::Catalogue("catalogue:ayu-light".into()));
        assert_eq!(Choice::parse("dusk"), Choice::Mod("dusk".into()));
        assert_eq!(Choice::parse("built-in:calm"), Choice::Mod("built-in:calm".into()));
    }

    #[test]
    fn dark_is_the_defaults_and_light_moves_the_grounds() {
        let dark = Look::dark();
        let light = Look::light();
        assert_eq!(dark.tone(), Tone::Dark);
        assert_eq!(light.tone(), Tone::Light);
        assert_ne!(dark.color(token::BG_BASE), light.color(token::BG_BASE));
        // An alias the light block never mentions follows the token it names.
        assert_eq!(light.color(token::SURFACE_CARD), light.color(token::BG_RAISED));
        assert_eq!(dark.color(token::SURFACE_CARD), dark.color(token::BG_RAISED));
    }

    #[test]
    fn the_built_in_theme_follows_the_appearance_and_a_picked_one_stays() {
        let none = ModSheets::new();
        assert_eq!(Look::pick(&Choice::BuiltIn, Tone::Dark, &none).id(), "dark");
        assert_eq!(Look::pick(&Choice::BuiltIn, Tone::Light, &none).id(), "light");
        assert_eq!(Look::pick(&Choice::Catalogue("catalogue:ayu-light".into()), Tone::Dark, &none).id(), "catalogue:ayu-light");
    }

    #[test]
    fn every_catalogue_entry_is_a_theme_and_says_its_tone() {
        assert_eq!(catalogue().len(), 39);
        for entry in catalogue() {
            let look = Look::catalogue(&entry.id).unwrap_or_else(|| panic!("{} has no block", entry.id));
            assert_eq!(look.tone(), entry.tone, "{}", entry.id);
            assert!(look.unresolved().is_empty(), "{}: {:?}", entry.id, look.unresolved());
        }
    }

    #[test]
    fn a_word_that_names_nothing_drawable_is_dark() {
        let none = ModSheets::new();
        assert_eq!(Look::pick(&Choice::Catalogue("catalogue:nope".into()), Tone::Light, &none).id(), "dark");
        assert_eq!(Look::pick(&Choice::Mod("dusk".into()), Tone::Light, &none).id(), "dark");
    }

    #[test]
    fn dusk_over_the_defaults_changes_what_it_wrote_and_what_follows_it() {
        let dusk = include_str!("../../../../packages/tokens/themes/example-mod/dusk/theme.css");
        let mods: ModSheets = [("dusk".to_string(), dusk.to_string())].into();
        let look = Look::pick(&Choice::Mod("dusk".into()), Tone::Light, &mods);
        let dark = Look::dark();
        assert_eq!(look.id(), "dusk");
        assert_eq!(look.tone(), Tone::Dark);
        assert_ne!(look.color(token::BG_BASE), dark.color(token::BG_BASE));
        assert_ne!(look.color(token::ACCENT), dark.color(token::ACCENT));
        // The faint accent was written by no one in the mod and follows the accent it did write.
        assert_ne!(look.color(token::ACCENT_FAINT), dark.color(token::ACCENT_FAINT));
        // A token the mod left alone is the default's.
        assert_eq!(look.color(token::STATUS_RUNNING), dark.color(token::STATUS_RUNNING));
        assert_eq!(look.color(token::SURFACE_CARD), look.color(token::BG_RAISED));
    }

    #[test]
    fn a_mod_value_that_cannot_resolve_draws_the_default_and_is_reported() {
        let css = "[data-theme=\"broken\"] { --accent: var(--nope); }";
        let look = Look::of_mod("broken", css);
        assert_eq!(look.color(token::ACCENT), Look::dark().color(token::ACCENT));
        assert!(look.unresolved().contains_key("--accent"));
    }

    #[test]
    fn every_colour_constant_in_the_default_theme_is_a_colour() {
        let dark = Look::dark();
        assert!(dark.unresolved().is_empty(), "{:?}", dark.unresolved());
        for token in [token::BG_BASE, token::FG_DEFAULT, token::ACCENT, token::STATUS_RUNNING, token::SCRIM, token::BG_GLASS] {
            assert_ne!(dark.color(token), NOT_A_COLOUR, "{}", token.0);
        }
        assert!(dark.edge(token::FOCUS_RING).is_some());
        assert_eq!(dark.font_stack(token::FONT_SANS)[0], "IBM Plex Sans");
        assert_eq!(dark.shadows(token::SHADOW_PRIMARY).len(), 2);
        assert!(dark.easing(token::EASE).is_some());
        assert_eq!(dark.duration(token::TOOLTIP_DELAY), Duration::from_millis(400));
    }

    #[test]
    fn the_light_theme_resolves_whole() {
        assert!(Look::light().unresolved().is_empty());
    }
}
