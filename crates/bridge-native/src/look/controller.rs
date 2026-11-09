//! Keeps the [`Look`] global in step with what decides it: the person's theme word, the mods Fleet
//! has checked, and macOS's light or dark.
//!
//! One entity watches all three. Any of them moving works out the theme again; when that is the
//! theme already in force nothing is replaced, and when it is not the global is replaced once and
//! every window redraws once, however many things moved in the same turn.

use gpui::{Context, Entity, Subscription, Window, WindowAppearance};

use crate::fleet::FleetConnection;
use crate::log::Note;

use super::{install, ActiveLook as _, Choice, Look, ModSheets, Tone};

/// What the theme was last worked out from. Equal inputs give the same theme, so they are not worked out again.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Inputs {
    choice: Choice,
    system: Tone,
    /// The sheet of the chosen mod, if the choice is a mod; the others do not matter to the result.
    sheet: Option<String>,
}

pub struct LookController {
    /// The word as Fleet holds it, or as the person last chose it in this window.
    word: String,
    /// The word Fleet last gave. A batch that did not change it leaves a choice made here alone.
    fleet_word: Option<String>,
    system: Tone,
    sheets: ModSheets,
    worked_from: Option<Inputs>,
    _subscriptions: Vec<Subscription>,
}

impl LookController {
    /// Watch `fleet` and `window`. The theme is put in force once now, from the Dark a window has
    /// before Fleet has spoken, and again when anything it depends on arrives.
    pub fn new(fleet: &Entity<FleetConnection>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let observe_fleet = cx.observe(fleet, |this, fleet, cx| {
            let held = fleet.read(cx).held();
            if held.theme != this.fleet_word {
                this.fleet_word = held.theme.clone();
                if let Some(word) = &held.theme {
                    this.word = word.clone();
                }
            }
            this.sheets = held.theme_sheets();
            this.refresh(cx);
        });
        let observe_appearance = cx.observe_window_appearance(window, |this, window, cx| {
            this.system = tone_of(window.appearance());
            this.refresh(cx);
        });
        let mut this = LookController {
            word: ipc::DEFAULT_THEME.to_string(),
            fleet_word: None,
            system: tone_of(window.appearance()),
            sheets: ModSheets::new(),
            worked_from: None,
            _subscriptions: vec![observe_fleet, observe_appearance],
        };
        this.refresh(cx);
        this
    }

    /// The person chose a theme here. It takes effect at once; keeping it is Fleet's, and a picker
    /// saves the same word with `SavePreference` so the next window opens on it.
    pub fn choose(&mut self, word: &str, cx: &mut Context<Self>) {
        self.word = word.to_string();
        self.refresh(cx);
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        let choice = Choice::parse(&self.word);
        let sheet = match &choice {
            Choice::Mod(name) => self.sheets.get(name).cloned(),
            _ => None,
        };
        let inputs = Inputs { choice, system: self.system, sheet };
        if self.worked_from.as_ref() == Some(&inputs) {
            return;
        }
        let look = Look::pick(&inputs.choice, inputs.system, &self.sheets);
        self.worked_from = Some(inputs);
        if cx.has_global::<Look>() && cx.look() == &look {
            return;
        }
        for (token, why) in look.unresolved() {
            Note::warn("a theme token could not be resolved and draws the default")
                .field("theme", look.id())
                .field("token", token.as_str())
                .field("why", why.to_string())
                .emit();
        }
        Note::info("theme in force").field("theme", look.id()).emit();
        install(look, cx);
        cx.notify();
    }
}

fn tone_of(appearance: WindowAppearance) -> Tone {
    match appearance {
        WindowAppearance::Dark | WindowAppearance::VibrantDark => Tone::Dark,
        WindowAppearance::Light | WindowAppearance::VibrantLight => Tone::Light,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_vibrant_appearances_are_their_plain_tones() {
        assert_eq!(tone_of(WindowAppearance::VibrantDark), Tone::Dark);
        assert_eq!(tone_of(WindowAppearance::VibrantLight), Tone::Light);
        assert_eq!(tone_of(WindowAppearance::Light), Tone::Light);
    }
}
