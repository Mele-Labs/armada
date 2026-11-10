//! Every key is an act, bound from the registry and the person's own changes.
//!
//! **One action type carries every act.** gpui keys an action by its Rust type, and the registry has
//! a hundred and one acts that a screen would each have to declare and register; [`Act`] carries the
//! registry's id instead, so adding a row to `actions.toml` binds a key with no Rust change. A
//! screen listens with `on_action(cx.listener(|this, act: &Act, window, cx| ...))` and asks
//! `act.is("send_message")`. The cost is that a typo in an id is a string that matches nothing, which
//! `generated::actions::act` and the test below catch for the ids this file names.
//!
//! The spelling of a binding is the registry's (`⇧⌘R`, `Esc`, `⌘[ ⌘]`, `⌘1–⌘9`) and the person's
//! saved changes use the same one, so this reads them the way `packages/components/src/keymap.ts`
//! does: an act is a row of **slots**, each one key, and the saved list moves slots by place.
//!
//! **A bare key is bound under a key context.** `d` on a list must not fire while a text box inside
//! the list has focus, so a screen sets `key_context(scope.key_context())` on the element that owns
//! those keys and not on a parent of an input. `anywhere` is bound with no context.

use std::collections::BTreeMap;

use gpui::{App, Context, Entity, KeyBinding, KeyBindingContextPredicate, Subscription};
use serde::Deserialize;

use crate::fleet::FleetConnection;
use crate::generated::actions::{ActionKind, ACTIONS, Act as Row};
use crate::log::Note;

/// One act's key, pressed: which act, which of its slots, and for a row of digits which digit.
#[derive(Clone, Debug, PartialEq, Eq, gpui::Action)]
#[action(namespace = armada, no_json)]
pub struct Act {
    pub id: &'static str,
    pub slot: u8,
    pub digit: Option<u8>,
}

impl Act {
    pub fn is(&self, id: &str) -> bool {
        self.id == id
    }
}

/// Tab and Shift+Tab: the next and the previous tab stop, for a key nothing on the way up took. A screen's
/// root calls this from its `on_key_down`, so every element that sets `tab_stop` is reached from the keyboard.
pub fn traverse(event: &gpui::KeyDownEvent, window: &mut gpui::Window, cx: &mut gpui::App) -> bool {
    let key = &event.keystroke;
    if key.key != "tab" || key.modifiers.platform || key.modifiers.control || key.modifiers.alt {
        return false;
    }
    if key.modifiers.shift {
        window.focus_prev(cx);
    } else {
        window.focus_next(cx);
    }
    true
}

/// One key with its modifiers. `key` is a lower-case letter, a digit, a symbol, or a named key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chord {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
    pub key: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mods {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}

/// One thing a person can bind: a key, or the modifiers that go with a row of digits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Slot {
    Chord(Option<Chord>),
    Digits { mods: Option<Mods>, from: u8, to: u8 },
}

const NAMED: &[(&str, &str)] = &[
    ("Escape", "Esc"),
    ("Esc", "Esc"),
    ("Enter", "Enter"),
    ("Return", "Enter"),
    ("↵", "Enter"),
    ("Tab", "Tab"),
    (" ", "Space"),
    ("Space", "Space"),
    ("Backspace", "Backspace"),
    ("Delete", "Delete"),
    ("ArrowUp", "↑"),
    ("ArrowDown", "↓"),
    ("ArrowLeft", "←"),
    ("ArrowRight", "→"),
    ("↑", "↑"),
    ("↓", "↓"),
    ("←", "←"),
    ("→", "→"),
    ("Home", "Home"),
    ("End", "End"),
    ("PageUp", "PageUp"),
    ("PageDown", "PageDown"),
];

/// The name gpui gives a key the registry names in words or arrows.
const GPUI_NAMES: &[(&str, &str)] = &[
    ("Esc", "escape"),
    ("Enter", "enter"),
    ("Tab", "tab"),
    ("Space", "space"),
    ("Backspace", "backspace"),
    ("Delete", "delete"),
    ("↑", "up"),
    ("↓", "down"),
    ("←", "left"),
    ("→", "right"),
    ("Home", "home"),
    ("End", "end"),
    ("PageUp", "pageup"),
    ("PageDown", "pagedown"),
];

fn is_function_key(text: &str) -> bool {
    text.strip_prefix('F').is_some_and(|digits| (1..=2).contains(&digits.len()) && digits.bytes().all(|b| b.is_ascii_digit()))
}

fn is_named(key: &str) -> bool {
    NAMED.iter().any(|(_, named)| *named == key) || is_function_key(key)
}

/// Shift is part of a letter's chord and of a named key's, and never of a symbol's: `?` is shift and
/// `/` on one keyboard and a key of its own on another, so the character typed is the whole answer.
fn shift_counts(key: &str) -> bool {
    key.chars().all(|ch| ch.is_ascii_alphabetic() || ch.is_ascii_digit()) && key.chars().count() == 1 || is_named(key)
}

/// One chord from its spelling: `⇧⌘R`, `B`, `Esc`, `⌥↑`. `None` for one this cannot read.
pub fn parse_chord(text: &str) -> Option<Chord> {
    let mut rest = text.trim();
    let (mut ctrl, mut alt, mut shift, mut meta) = (false, false, false, false);
    while rest.chars().count() > 1 {
        let head = rest.chars().next()?;
        match head {
            '⌃' => ctrl = true,
            '⌥' => alt = true,
            '⇧' => shift = true,
            '⌘' => meta = true,
            _ => break,
        }
        rest = &rest[head.len_utf8()..];
    }
    if rest.is_empty() {
        return None;
    }
    if let Some((_, named)) = NAMED.iter().find(|(spelled, _)| *spelled == rest) {
        return Some(Chord { ctrl, alt, shift, meta, key: (*named).to_string() });
    }
    if is_function_key(rest) {
        return Some(Chord { ctrl, alt, shift, meta, key: rest.to_string() });
    }
    let mut chars = rest.chars();
    let only = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    if only.is_ascii_alphabetic() {
        // Bare, an upper-case letter is the shifted one (`B`); beside ⌘, ⌃ or ⌥ it is only how it is drawn.
        let bare = !ctrl && !alt && !meta;
        return Some(Chord {
            ctrl,
            alt,
            meta,
            shift: shift || (bare && only.is_ascii_uppercase()),
            key: only.to_ascii_lowercase().to_string(),
        });
    }
    Some(Chord { ctrl, alt, meta, shift: shift_counts(rest) && shift, key: rest.to_string() })
}

/// The slots an act's spelling holds. `⌘[ ⌘]` is two, `j / k / ↓ / ↑` four, `⌘1–⌘9` one row of digits.
/// A blank spelling, which is how a person unbinds a key, is a slot with nothing in it.
pub fn parse_slots(text: &str) -> Vec<Slot> {
    if let Some(digits) = parse_digit_range(text.trim()) {
        return vec![digits];
    }
    let parts: Vec<&str> = if text.contains(" / ") { text.split(" / ").collect() } else { text.split(' ').collect() };
    parts
        .into_iter()
        .map(|part| Slot::Chord(if part.trim().is_empty() { None } else { parse_chord(part) }))
        .collect()
}

/// `⌘1–⌘9`, `1–9`, `⌥⇧0–⌥⇧9`: the modifiers, a first digit, an en dash, the modifiers again, a last digit.
fn parse_digit_range(text: &str) -> Option<Slot> {
    let (head, tail) = text.split_once('–')?;
    let from = head.chars().last().filter(char::is_ascii_digit)?;
    let to = tail.chars().last().filter(char::is_ascii_digit)?;
    let prefix = &head[..head.len() - 1];
    if prefix.chars().any(|ch| ch.is_ascii_digit()) {
        return None;
    }
    let probe = parse_chord(&format!("{prefix}{from}"));
    Some(Slot::Digits {
        mods: probe.map(|chord| Mods { ctrl: chord.ctrl, alt: chord.alt, shift: chord.shift, meta: chord.meta }),
        from: from.to_digit(10)? as u8,
        to: to.to_digit(10)? as u8,
    })
}

impl Chord {
    /// The keystroke in gpui's spelling: `alt-cmd-c`, `shift-enter`, `cmd--`.
    pub fn keystroke(&self) -> String {
        keystroke(Mods { ctrl: self.ctrl, alt: self.alt, shift: self.shift, meta: self.meta }, &self.key)
    }
}

fn keystroke(mods: Mods, key: &str) -> String {
    let named = GPUI_NAMES.iter().find(|(spelled, _)| *spelled == key).map(|(_, gpui)| (*gpui).to_string());
    let key = named.unwrap_or_else(|| key.to_ascii_lowercase());
    let mut out = String::new();
    for (held, name) in [(mods.ctrl, "ctrl"), (mods.alt, "alt"), (mods.shift, "shift"), (mods.meta, "cmd")] {
        if held {
            out.push_str(name);
            out.push('-');
        }
    }
    out.push_str(&key);
    out
}

/// The person's saved changes: an act's id to the spelling of each slot it holds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Rebinding(BTreeMap<String, Vec<String>>);

#[derive(Deserialize)]
struct Saved {
    #[serde(default)]
    bindings: BTreeMap<String, Vec<String>>,
}

impl Rebinding {
    /// Fleet's text back into changes. **One wrong shape and none of it is applied**, the rule
    /// `ipc::key_bindings` states, so a half-read file never moves half the keys. Empty text is none.
    pub fn read(text: &str) -> Rebinding {
        if text.trim().is_empty() || !ipc::key_bindings::problems(text).is_empty() {
            return Rebinding::default();
        }
        ipc::decode::<Saved>("key bindings", text.as_bytes()).map(|saved| Rebinding(saved.bindings)).unwrap_or_default()
    }

    /// The slots an act answers now. A saved list shorter than the registry's leaves the rest as
    /// shipped, and a saved slot of another kind than the shipped one is ignored.
    pub fn slots_of(&self, row: &Row) -> Vec<Slot> {
        let shipped = parse_slots(row.shortcut);
        let Some(saved) = self.0.get(row.id) else { return shipped };
        shipped
            .into_iter()
            .enumerate()
            .map(|(at, slot)| {
                let Some(text) = saved.get(at) else { return slot };
                match slot {
                    Slot::Digits { from, to, .. } => match (text.is_empty(), parse_slots(text).into_iter().next()) {
                        (true, _) => Slot::Digits { mods: None, from, to },
                        (false, Some(Slot::Digits { mods, .. })) => Slot::Digits { mods, from, to },
                        _ => slot,
                    },
                    Slot::Chord(_) => Slot::Chord(if text.is_empty() { None } else { parse_chord(text) }),
                }
            })
            .collect()
    }
}

/// One key to register: what is pressed, what it does, and where it is listened for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bound {
    pub keystroke: String,
    pub act: Act,
    pub context: Option<&'static str>,
}

/// Every key the registry and the person's changes bind. A slot with no key, or one this cannot
/// read, binds nothing.
pub fn bound(rebinding: &Rebinding) -> Vec<Bound> {
    let mut all = Vec::new();
    for row in ACTIONS {
        let context = row.scope.key_context();
        for (at, slot) in rebinding.slots_of(row).into_iter().enumerate() {
            let slot_at = at as u8;
            match slot {
                Slot::Chord(Some(chord)) => all.push(Bound {
                    keystroke: chord.keystroke(),
                    act: Act { id: row.id, slot: slot_at, digit: None },
                    context,
                }),
                Slot::Chord(None) | Slot::Digits { mods: None, .. } => {}
                Slot::Digits { mods: Some(mods), from, to } => {
                    for digit in from..=to {
                        all.push(Bound {
                            keystroke: keystroke(mods, &digit.to_string()),
                            act: Act { id: row.id, slot: slot_at, digit: Some(digit) },
                            context,
                        });
                    }
                }
            }
        }
    }
    all
}

/// Put the registry's keys, under the person's changes, into gpui's keymap, replacing any this
/// function put there before. Bindings other libraries registered stay: gpui-component binds its
/// text editing here too, and a clear would take them away.
pub fn install(rebinding: &Rebinding, cx: &mut App) {
    let keep: Vec<KeyBinding> =
        cx.key_bindings().borrow().bindings().filter(|binding| binding.action().as_any().downcast_ref::<Act>().is_none()).cloned().collect();
    let mut fresh = Vec::new();
    for key in bound(rebinding) {
        let context = match key.context.map(KeyBindingContextPredicate::parse) {
            Some(Ok(predicate)) => Some(std::rc::Rc::new(predicate)),
            Some(Err(why)) => {
                Note::warn("a key context did not parse").field("context", key.context.unwrap_or_default()).field("why", why.to_string()).emit();
                continue;
            }
            None => None,
        };
        match KeyBinding::load(&key.keystroke, Box::new(key.act.clone()), context, false, None, cx) {
            Ok(binding) => fresh.push(binding),
            Err(why) => {
                Note::warn("a key did not parse and is not bound")
                    .field("act", key.act.id)
                    .field("keystroke", key.keystroke.as_str())
                    .field("why", why.to_string())
                    .emit();
            }
        }
    }
    cx.clear_key_bindings();
    cx.bind_keys(keep.into_iter().chain(fresh));
}

/// Keeps gpui's keymap in step with the person's saved changes: the registry's keys at once, and
/// again whenever Fleet reports different changes than the ones applied.
pub struct KeymapController {
    applied: Option<String>,
    _watch: Subscription,
}

impl KeymapController {
    pub fn new(fleet: &Entity<FleetConnection>, cx: &mut Context<Self>) -> Self {
        install(&Rebinding::default(), cx);
        let watch = cx.observe(fleet, |this, fleet, cx| {
            let Some(text) = fleet.read(cx).held().key_bindings.clone() else { return };
            if this.applied.as_ref() != Some(&text) {
                install(&Rebinding::read(&text), cx);
                this.applied = Some(text);
            }
        });
        KeymapController { applied: None, _watch: watch }
    }
}

/// A registry spelling as a keycap draws it: `⌘Enter` is `⌘↵`.
pub fn cap(spelling: &str) -> String {
    spelling.replace("Enter", "↵").replace("Esc", "esc")
}

/// The key an act is bound to, as its keycap reads. Empty for an id the registry does not know.
pub fn shown(id: &str) -> String {
    crate::generated::actions::act(id).map(|row| cap(row.shortcut)).unwrap_or_default()
}

/// The key of digit `n` in an act's row of digits (`⌥1–⌥9`), as its keycap reads.
pub fn shown_digit(id: &str, n: u8) -> Option<String> {
    let row = crate::generated::actions::act(id)?;
    match parse_slots(row.shortcut).into_iter().next()? {
        Slot::Digits { mods: Some(mods), from, to } if (from..=to).contains(&n) => {
            let mut out = String::new();
            for (held, glyph) in [(mods.ctrl, '⌃'), (mods.alt, '⌥'), (mods.shift, '⇧'), (mods.meta, '⌘')] {
                if held {
                    out.push(glyph);
                }
            }
            out.push_str(&n.to_string());
            Some(out)
        }
        _ => None,
    }
}

/// The palette lists every Action and no Motion.
pub fn in_palette() -> impl Iterator<Item = &'static Row> {
    ACTIONS.iter().filter(|row| row.kind == ActionKind::Action)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::actions::act;

    fn chord(text: &str) -> Chord {
        parse_chord(text).unwrap_or_else(|| panic!("{text} reads"))
    }

    #[test]
    fn a_chord_reads_its_modifiers_in_any_order_and_its_letter_in_lower_case() {
        assert_eq!(chord("⇧⌘R"), Chord { ctrl: false, alt: false, shift: true, meta: true, key: "r".into() });
        assert_eq!(chord("⌘K").keystroke(), "cmd-k");
        assert_eq!(chord("⌥⌘C").keystroke(), "alt-cmd-c");
        assert_eq!(chord("⇧⌘Enter").keystroke(), "shift-cmd-enter");
        assert_eq!(chord("⌃Tab").keystroke(), "ctrl-tab");
    }

    #[test]
    fn a_bare_capital_is_the_shifted_letter_and_beside_a_modifier_it_is_only_how_it_is_drawn() {
        assert_eq!(chord("D").keystroke(), "shift-d");
        assert_eq!(chord("d").keystroke(), "d");
        assert_eq!(chord("⌘D").keystroke(), "cmd-d");
    }

    #[test]
    fn named_keys_and_arrows_take_gpui_s_names() {
        assert_eq!(chord("Esc").keystroke(), "escape");
        assert_eq!(chord("↵").keystroke(), "enter");
        assert_eq!(chord("⌥↑").keystroke(), "alt-up");
        assert_eq!(chord("Space").keystroke(), "space");
        assert_eq!(chord("F5").keystroke(), "f5");
    }

    #[test]
    fn a_symbol_keeps_its_character_and_never_carries_shift() {
        assert_eq!(chord("?").keystroke(), "?");
        assert_eq!(chord("⇧?").keystroke(), "?");
        assert_eq!(chord("⌘[").keystroke(), "cmd-[");
        assert_eq!(chord("⌘,").keystroke(), "cmd-,");
    }

    #[test]
    fn a_spelling_that_is_not_a_key_reads_as_none() {
        assert!(parse_chord("").is_none());
        assert!(parse_chord("⌘Hyper").is_none());
    }

    #[test]
    fn slots_split_on_slash_or_space_and_a_row_of_digits_is_one_slot() {
        assert_eq!(parse_slots("⌘[ ⌘]").len(), 2);
        assert_eq!(parse_slots("j / k / ↓ / ↑").len(), 4);
        assert_eq!(parse_slots("l b c q f a d").len(), 7);
        assert_eq!(
            parse_slots("⌘1–⌘9"),
            vec![Slot::Digits { mods: Some(Mods { meta: true, ..Mods::default() }), from: 1, to: 9 }]
        );
        assert_eq!(parse_slots("1–9"), vec![Slot::Digits { mods: Some(Mods::default()), from: 1, to: 9 }]);
        assert_eq!(
            parse_slots("⌥⇧0–⌥⇧9"),
            vec![Slot::Digits { mods: Some(Mods { alt: true, shift: true, ..Mods::default() }), from: 0, to: 9 }]
        );
    }

    #[test]
    fn a_blank_slot_is_unbound() {
        assert_eq!(parse_slots(""), vec![Slot::Chord(None)]);
    }

    #[test]
    fn every_registry_spelling_reads_to_a_key_or_an_honest_gap() {
        // A slot that reads as none in the shipped registry is a spelling this parser does not know,
        // which would leave an act with no key and no word about it.
        for row in ACTIONS {
            for slot in parse_slots(row.shortcut) {
                assert!(slot != Slot::Chord(None), "{} ({:?}) has a slot that does not read", row.id, row.shortcut);
            }
        }
    }

    #[test]
    fn every_registry_key_is_one_gpui_will_parse() {
        for key in bound(&Rebinding::default()) {
            let parsed = gpui::Keystroke::parse_with_os(&key.keystroke, gpui::OperatingSystem::Mac);
            assert!(parsed.is_ok(), "{} for {} does not parse", key.keystroke, key.act.id);
        }
    }

    #[test]
    fn the_registry_binds_acts_by_id_and_slot() {
        let all = bound(&Rebinding::default());
        let palette = all.iter().find(|key| key.act.id == "command_palette").expect("bound");
        assert_eq!(palette.keystroke, "cmd-k");
        assert_eq!(palette.context, None);
        // `⌘[ ⌘]` is one act with two slots.
        let history: Vec<_> = all.iter().filter(|key| key.act.id == "history").map(|key| (key.act.slot, key.keystroke.as_str())).collect();
        assert_eq!(history, vec![(0, "cmd-["), (1, "cmd-]")]);
        // A row of digits is one binding per digit, each carrying the digit.
        let digits: Vec<_> = all.iter().filter(|key| key.act.digit.is_some()).collect();
        assert!(!digits.is_empty());
        assert!(digits.iter().all(|key| key.act.digit.is_some_and(|digit| key.keystroke.ends_with(&digit.to_string()))));
    }

    #[test]
    fn a_scope_other_than_anywhere_binds_under_its_context() {
        let all = bound(&Rebinding::default());
        let in_context = all.iter().filter(|key| key.context.is_some()).count();
        assert!(in_context > 0);
        assert!(all.iter().filter_map(|key| key.context).all(|context| !context.contains(' ')));
    }

    #[test]
    fn a_rebinding_replaces_a_slot_by_place_and_leaves_the_rest_as_shipped() {
        let saved = Rebinding::read(r#"{"version":1,"bindings":{"history":["⌘B"]}}"#);
        let row = act("history").expect("a row");
        assert_eq!(saved.slots_of(row), vec![Slot::Chord(Some(chord("⌘B"))), Slot::Chord(Some(chord("⌘]")))]);
    }

    #[test]
    fn a_blank_saved_slot_unbinds_it() {
        let saved = Rebinding::read(r#"{"version":1,"bindings":{"command_palette":[""]}}"#);
        assert!(bound(&saved).iter().all(|key| key.act.id != "command_palette"));
    }

    #[test]
    fn one_wrong_shape_and_none_of_it_is_applied() {
        assert_eq!(Rebinding::read(r#"{"version":2,"bindings":{"history":["⌘B"]}}"#), Rebinding::default());
        assert_eq!(Rebinding::read("not json"), Rebinding::default());
        assert_eq!(Rebinding::read(""), Rebinding::default());
    }

    #[test]
    fn a_digit_row_can_be_given_other_modifiers() {
        let id = ACTIONS.iter().find(|row| row.shortcut.contains('–')).expect("a digit row").id;
        let saved = Rebinding::read(&format!(r#"{{"version":1,"bindings":{{"{id}":["⌃1–⌃9"]}}}}"#));
        let keys: Vec<_> = bound(&saved).into_iter().filter(|key| key.act.id == id).collect();
        assert!(keys.iter().all(|key| key.keystroke.starts_with("ctrl-")), "{keys:?}");
    }

    #[test]
    fn the_palette_lists_actions_and_no_motion() {
        assert!(in_palette().all(|row| row.kind == ActionKind::Action));
        assert!(in_palette().count() < ACTIONS.len());
    }
}
