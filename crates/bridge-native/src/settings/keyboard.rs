//! Settings → Keyboard shortcuts: every act with a key, grouped by where a person stands when they press it, each
//! key a control that takes the next key pressed. A port of Classic's `KeyboardSettings.tsx` and `KeyBindingRow`,
//! with the grouping, spelling and conflicts of `packages/components/src/keymap.ts`. What changes here is kept by
//! Fleet as `key_bindings` and laid over `actions.toml`'s keys (`docs/concepts/key-bindings.md`).

use std::collections::BTreeMap;

use gpui::{div, prelude::*, svg, AnyElement, App, Context, FocusHandle, FontWeight, KeyDownEvent, Keystroke, Role, SharedString, Window};
use serde::{Deserialize, Serialize};

use super::overlay::SettingsOverlay;
use super::view::{card, ghost, ring_colour, ringed, small};
use crate::assets::icon_path;
use crate::generated::actions::{ActionScope, Act as Row, ACTIONS};
use crate::keymap::{parse_slots, Chord, Mods, Rebinding, Slot};
use crate::labels::labelled;
use crate::look::{token, Look};

/// One heading, and the scopes under it (`KEYMAP_GROUPS`).
pub const GROUPS: &[(&str, &[ActionScope])] = &[
    ("Anywhere", &[ActionScope::Anywhere]),
    ("Lists", &[ActionScope::List, ActionScope::ListAndDetail]),
    ("Dashboard", &[ActionScope::Dashboard, ActionScope::Deck]),
    ("Calls", &[ActionScope::Call]),
    ("Reviews", &[ActionScope::Review]),
    ("Logbook", &[ActionScope::Logbook]),
    ("Plans", &[ActionScope::Plan]),
    ("Job thread", &[ActionScope::Thread]),
    ("Terminal", &[ActionScope::Terminal]),
    ("Dispatch", &[ActionScope::Dispatch]),
    ("Review sheet", &[ActionScope::ReviewSheet]),
    ("Issues", &[ActionScope::Issues, ActionScope::Report]),
    ("Projects", &[ActionScope::Projects]),
    ("Studios", &[ActionScope::OpenStudio]),
];

/// What each slot of a many-key act does (`SLOT_NAMES`).
fn slot_names(id: &str) -> &'static [&'static str] {
    match id {
        "history" => &["Back", "Forward"],
        "move_focus" | "finding_move" => &["Down", "Up", "Down", "Up"],
        "deck_reorder" | "projects_reorder" => &["Up", "Down"],
        _ => &[],
    }
}

/// Where an act's keys are heard, so two acts meet only where both listen (`PLACES`).
fn places(scope: ActionScope) -> &'static [&'static str] {
    match scope {
        ActionScope::Anywhere => &["board", "detail", "studio", "dashboard", "deck", "review", "call", "logbook", "issues", "report", "projects", "plan", "thread", "terminal", "dispatch", "review-sheet"],
        ActionScope::List => &["board", "dashboard", "deck"],
        ActionScope::ListAndDetail => &["board", "detail", "dashboard"],
        ActionScope::OpenStudio => &["studio"],
        ActionScope::Dashboard => &["dashboard", "deck"],
        ActionScope::Deck => &["deck"],
        ActionScope::Review => &["review"],
        ActionScope::Issues => &["issues"],
        ActionScope::Report => &["report"],
        ActionScope::Projects => &["projects"],
        ActionScope::Call => &["call"],
        ActionScope::Logbook => &["logbook"],
        ActionScope::Plan => &["plan"],
        ActionScope::Thread => &["thread"],
        ActionScope::Terminal => &["terminal"],
        ActionScope::Dispatch => &["dispatch"],
        ActionScope::ReviewSheet => &["review-sheet"],
    }
}

fn overlaps(a: ActionScope, b: ActionScope) -> bool {
    places(a).iter().any(|place| places(b).contains(place))
}

fn mods_text(mods: Mods) -> String {
    [(mods.ctrl, "⌃"), (mods.alt, "⌥"), (mods.shift, "⇧"), (mods.meta, "⌘")].into_iter().filter(|(held, _)| *held).map(|(_, glyph)| glyph).collect()
}

fn mods_of(chord: &Chord) -> Mods {
    Mods { ctrl: chord.ctrl, alt: chord.alt, shift: chord.shift, meta: chord.meta }
}

/// A chord in the registry's spelling: `B` for a shifted letter alone, `⇧⌘R` beside a modifier (`formatChord`).
pub fn spell_chord(chord: &Chord) -> String {
    let letter = chord.key.chars().count() == 1 && chord.key.chars().all(|ch| ch.is_ascii_alphabetic());
    if letter {
        let bare = !chord.ctrl && !chord.alt && !chord.meta;
        if bare {
            return if chord.shift { chord.key.to_uppercase() } else { chord.key.clone() };
        }
        return format!("{}{}", mods_text(mods_of(chord)), chord.key.to_uppercase());
    }
    format!("{}{}", mods_text(mods_of(chord)), chord.key)
}

/// A slot in the registry's spelling, empty where it holds no key (`formatSlot`).
pub fn spell(slot: &Slot) -> String {
    match slot {
        Slot::Chord(None) | Slot::Digits { mods: None, .. } => String::new(),
        Slot::Chord(Some(chord)) => spell_chord(chord),
        Slot::Digits { mods: Some(mods), from, to } => {
            let held = mods_text(*mods);
            format!("{held}{from}–{held}{to}")
        }
    }
}

/// The chords a slot covers: one for a key, one per digit for a row of them.
fn chords_in(slot: &Slot) -> Vec<Chord> {
    match slot {
        Slot::Chord(Some(chord)) => vec![chord.clone()],
        Slot::Digits { mods: Some(mods), from, to } => {
            (*from..=*to).map(|digit| Chord { ctrl: mods.ctrl, alt: mods.alt, shift: mods.shift, meta: mods.meta, key: digit.to_string() }).collect()
        }
        _ => Vec::new(),
    }
}

/// The person's changes, as Fleet keeps them: an act's id to the spelling of each slot.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bindings {
    #[serde(default)]
    pub bindings: BTreeMap<String, Vec<String>>,
}

impl Bindings {
    /// Fleet's text, read as the keymap reads it: text it would not apply is none.
    pub fn read(text: &str) -> Bindings {
        if text.trim().is_empty() || !ipc::key_bindings::problems(text).is_empty() {
            return Bindings::default();
        }
        ipc::decode::<Bindings>("key bindings", text.as_bytes()).unwrap_or_default()
    }

    /// The text Fleet keeps: empty where nothing has changed, which takes the preference back.
    pub fn text(&self) -> String {
        if self.bindings.is_empty() {
            return String::new();
        }
        #[derive(Serialize)]
        struct Kept<'a> {
            version: u8,
            bindings: &'a BTreeMap<String, Vec<String>>,
        }
        serde_json::to_string(&Kept { version: 1, bindings: &self.bindings }).unwrap_or_default()
    }

    /// Every act's slots now, the person's laid over the registry's: read once for each change, not each frame.
    pub fn all_slots(&self) -> BTreeMap<&'static str, Vec<Slot>> {
        let laid = Rebinding::read(&self.text());
        ACTIONS.iter().map(|row| (row.id, laid.slots_of(row))).collect()
    }

    /// Bind one act's slots. A spelling the same as the registry's takes the act's own change back (`rebind`).
    pub fn rebind(&self, row: &Row, slots: &[Slot]) -> Bindings {
        let spelled: Vec<String> = slots.iter().map(spell).collect();
        let shipped: Vec<String> = parse_slots(row.shortcut).iter().map(spell).collect();
        let mut next = self.clone();
        if spelled == shipped {
            next.bindings.remove(row.id);
        } else {
            next.bindings.insert(row.id.to_string(), spelled);
        }
        next
    }

    pub fn reset(&self, id: &str) -> Bindings {
        let mut next = self.clone();
        next.bindings.remove(id);
        next
    }
}

/// The other built acts that answer the same key as this slot, somewhere both listen (`conflictsOf`).
pub fn conflicts(all: &BTreeMap<&'static str, Vec<Slot>>, row: &Row, slot: &Slot) -> Vec<&'static str> {
    if row.unbuilt.is_some() {
        return Vec::new();
    }
    let mine = chords_in(slot);
    if mine.is_empty() {
        return Vec::new();
    }
    ACTIONS
        .iter()
        .filter(|other| other.id != row.id && other.unbuilt.is_none() && overlaps(row.scope, other.scope))
        .filter(|other| all.get(other.id).is_some_and(|slots| slots.iter().any(|theirs| chords_in(theirs).iter().any(|chord| mine.contains(chord)))))
        .map(|other| other.verb)
        .collect()
}

/// A slot with the key pressed in it, or emptied (`changed`).
fn changed(slot: &Slot, pressed: Option<Chord>) -> Option<Slot> {
    match (slot, pressed) {
        (Slot::Chord(_), None) => Some(Slot::Chord(None)),
        (Slot::Digits { from, to, .. }, None) => Some(Slot::Digits { mods: None, from: *from, to: *to }),
        (Slot::Chord(_), Some(chord)) => Some(Slot::Chord(Some(chord))),
        // A row of digits takes the modifiers held with any digit.
        (Slot::Digits { from, to, .. }, Some(chord)) => chord.key.parse::<u8>().is_ok().then(|| Slot::Digits { mods: Some(mods_of(&chord)), from: *from, to: *to }),
    }
}

/// The key pressed, as the registry spells keys: a letter lower case with Shift held apart, a symbol as typed,
/// a named key by its name. `None` for a modifier alone.
pub fn chord_of(keystroke: &Keystroke) -> Option<Chord> {
    let named = match keystroke.key.as_str() {
        "enter" => Some("Enter"),
        "escape" => Some("Esc"),
        "tab" => Some("Tab"),
        "space" => Some("Space"),
        "backspace" => Some("Backspace"),
        "delete" => Some("Delete"),
        "up" => Some("↑"),
        "down" => Some("↓"),
        "left" => Some("←"),
        "right" => Some("→"),
        "home" => Some("Home"),
        "end" => Some("End"),
        "pageup" => Some("PageUp"),
        "pagedown" => Some("PageDown"),
        "shift" | "control" | "alt" | "platform" | "function" | "capslock" => return None,
        _ => None,
    };
    let mods = &keystroke.modifiers;
    let (key, shift) = match named {
        Some(name) => (name.to_string(), mods.shift),
        None if keystroke.key.len() > 1 && keystroke.key.starts_with('f') && keystroke.key[1..].bytes().all(|b| b.is_ascii_digit()) => (keystroke.key.to_uppercase(), mods.shift),
        None if keystroke.key.chars().count() == 1 && keystroke.key.chars().all(|ch| ch.is_ascii_alphanumeric()) => (keystroke.key.to_lowercase(), mods.shift),
        // A symbol is the character typed, and Shift is part of it.
        None => (keystroke.key_char.clone().filter(|typed| !typed.is_empty()).unwrap_or_else(|| keystroke.key.clone()), false),
    };
    Some(Chord { ctrl: mods.control, alt: mods.alt, shift, meta: mods.platform, key })
}

fn words(row: &Row, group: &str) -> String {
    [row.verb.to_string(), row.id.replace('_', " "), row.shortcut.to_string(), group.to_string()].join(" ")
}

/// Every act a search finds, under its heading, the heading kept only where one is found.
pub fn found(query: &str) -> Vec<(&'static str, Vec<&'static Row>)> {
    GROUPS
        .iter()
        .map(|(title, scopes)| {
            let acts = ACTIONS.iter().filter(|row| scopes.contains(&row.scope) && (query.trim().is_empty() || super::entries::holds(&words(row, title), query))).collect::<Vec<_>>();
            (*title, acts)
        })
        .filter(|(_, acts)| !acts.is_empty())
        .collect()
}

/// The words the sidebar's search finds the page by: each act's verb, id, keys and heading.
pub fn all_words() -> String {
    GROUPS.iter().flat_map(|(title, scopes)| ACTIONS.iter().filter(|row| scopes.contains(&row.scope)).map(move |row| words(row, title))).collect::<Vec<_>>().join(" ")
}

/// The page's state: the person's changes, the key a press is waiting on, and each control's keys.
pub struct Keyboard {
    pub bindings: Bindings,
    /// Every act's slots under `bindings`.
    pub slots: BTreeMap<&'static str, Vec<Slot>>,
    /// The act and slot waiting for the next key, while one is.
    pub listening: Option<(&'static str, usize)>,
    pub keys: BTreeMap<(&'static str, usize), FocusHandle>,
    pub resets: BTreeMap<&'static str, FocusHandle>,
    pub reset_all: FocusHandle,
    pub saving: bool,
    pub refused: Option<String>,
}

impl Keyboard {
    pub fn new(cx: &mut App) -> Keyboard {
        let mut keys = BTreeMap::new();
        let mut resets = BTreeMap::new();
        for row in ACTIONS {
            for at in 0..parse_slots(row.shortcut).len() {
                keys.insert((row.id, at), cx.focus_handle().tab_stop(true));
            }
            resets.insert(row.id, cx.focus_handle().tab_stop(true));
        }
        Keyboard { slots: Bindings::default().all_slots(), bindings: Bindings::default(), listening: None, keys, resets, reset_all: cx.focus_handle().tab_stop(true), saving: false, refused: None }
    }

    /// Take the person's changes, and read every act's slots under them.
    pub fn set(&mut self, bindings: Bindings) {
        self.slots = bindings.all_slots();
        self.bindings = bindings;
    }

    pub fn slots_of(&self, row: &Row) -> Vec<Slot> {
        self.slots.get(row.id).cloned().unwrap_or_default()
    }

    /// The stops the page's controls take, in the order Tab reaches them.
    pub fn stops(&self, query: &str) -> Vec<FocusHandle> {
        let mut stops = Vec::new();
        for (_, acts) in found(query) {
            for row in acts {
                for at in 0..self.slots_of(row).len() {
                    stops.extend(self.keys.get(&(row.id, at)).cloned());
                }
                if self.bindings.bindings.contains_key(row.id) {
                    stops.extend(self.resets.get(row.id).cloned());
                }
            }
        }
        if query.trim().is_empty() && !self.bindings.bindings.is_empty() {
            stops.push(self.reset_all.clone());
        }
        stops
    }
}

fn row_by_id(id: &str) -> Option<&'static Row> {
    ACTIONS.iter().find(|row| row.id == id)
}

impl SettingsOverlay {
    /// The key a waiting slot takes: Esc lets go, Delete or Backspace unbinds, anything else is bound.
    pub fn keyboard_key(&mut self, keystroke: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some((id, at)) = self.keyboard.listening else { return false };
        let bare = !keystroke.modifiers.platform && !keystroke.modifiers.control && !keystroke.modifiers.alt && !keystroke.modifiers.shift;
        let pressed = match keystroke.key.as_str() {
            "escape" if bare => {
                self.keyboard.listening = None;
                cx.notify();
                return true;
            }
            "backspace" | "delete" if bare => None,
            _ => match chord_of(keystroke) {
                Some(chord) => Some(chord),
                None => return true,
            },
        };
        let Some(row) = row_by_id(id) else { return true };
        let mut slots = self.keyboard.slots_of(row);
        let Some(next) = slots.get(at).and_then(|slot| changed(slot, pressed)) else { return true };
        slots[at] = next;
        self.keyboard.listening = None;
        let bindings = self.keyboard.bindings.rebind(row, &slots);
        self.save_bindings(bindings, window, cx);
        true
    }

    fn listen(&mut self, id: &'static str, at: usize, cx: &mut Context<Self>) {
        self.keyboard.listening = if self.keyboard.listening == Some((id, at)) { None } else { Some((id, at)) };
        cx.notify();
    }

    /// Send the changes whole, and put the keys Fleet answers with in force at once.
    pub fn save_bindings(&mut self, bindings: Bindings, window: &mut Window, cx: &mut Context<Self>) {
        if self.keyboard.saving || !self.live {
            return;
        }
        self.keyboard.saving = true;
        self.keyboard.refused = None;
        let sent = self.wire.save_preference("key_bindings", bindings.text(), cx);
        cx.spawn_in(window, async move |this, cx| {
            let answer = sent.await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.keyboard.saving = false;
                // A Reset goes once its keys are back: the keys go to the act's first key, or the entries.
                let reset = this.keyboard.resets.iter().find(|(_, handle)| handle.is_focused(window)).map(|(id, _)| *id);
                if let Some(id) = reset {
                    if let Some(key) = this.keyboard.keys.get(&(id, 0)).cloned() {
                        window.focus(&key, cx);
                    }
                } else if this.keyboard.reset_all.is_focused(window) {
                    window.focus(&this.side, cx);
                }
                match answer {
                    Ok(preferences) => {
                        this.keyboard.set(Bindings::read(&preferences.key_bindings));
                        this.wire.preferences_saved(preferences, window, cx);
                    }
                    Err(why) => this.keyboard.refused = Some(why.said),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn render_act(&self, row: &'static Row, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let slots = self.keyboard.slots_of(row);
        let names = slot_names(row.id);
        let off = !self.live || self.keyboard.saving;
        let mut conflicted: Vec<&str> = Vec::new();
        let mut keys: Vec<AnyElement> = Vec::new();
        for (at, slot) in slots.iter().enumerate() {
            let Some(handle) = self.keyboard.keys.get(&(row.id, at)) else { continue };
            let conflicts = conflicts(&self.keyboard.slots, row, slot);
            for one in &conflicts {
                if !conflicted.contains(one) {
                    conflicted.push(one);
                }
            }
            let spelled = spell(slot);
            let waiting = self.keyboard.listening == Some((row.id, at));
            let name = names.get(at).copied();
            let named = name.map_or_else(|| row.verb.to_string(), |name| format!("{}, {}", row.verb, name.to_lowercase()));
            let said = if waiting {
                format!("{named}: press a key, Esc to cancel, Delete to unbind")
            } else {
                format!("{named}: {}", if spelled.is_empty() { "unbound" } else { spelled.as_str() })
            };
            let face: AnyElement = if waiting {
                div().text_color(look.color(token::ACCENT)).child("Press a key").into_any_element()
            } else if spelled.is_empty() {
                div().text_color(look.color(token::FG_MUTED)).child("Unbound").into_any_element()
            } else {
                crate::titlebar::parts::kbd(crate::keymap::cap(&spelled), look).into_any_element()
            };
            let id = row.id;
            let button = div()
                .id(SharedString::from(format!("key-{id}-{at}")))
                .track_focus(handle)
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .min_h(look.length(token::SPACE_8))
                .px(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(if waiting {
                    look.color(token::ACCENT)
                } else if !conflicts.is_empty() {
                    look.color(token::STATUS_ESCALATED)
                } else {
                    look.color(token::BORDER_DEFAULT)
                })
                .bg(if waiting { look.color(token::ACCENT).opacity(0.12) } else { look.color(token::SURFACE_WELL) })
                .text_size(look.length(token::TEXT_XS))
                .when(handle.is_focused(window) && !waiting, |button| button.border_color(ring_colour(look)))
                .when(!off, |button| button.cursor_pointer().hover(|style| style.border_color(look.color(token::BORDER_STRONG))))
                .children(name.map(|name| div().text_color(look.color(token::FG_MUTED)).child(name)))
                .child(face)
                .when(!off, |button| {
                    button
                        .on_click(cx.listener(move |this, _, window, cx| {
                            if let Some(handle) = this.keyboard.keys.get(&(id, at)).cloned() {
                                window.focus(&handle, cx);
                            }
                            this.listen(id, at, cx);
                        }))
                        .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                            if this.keyboard.listening.is_none() && matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                this.listen(id, at, cx);
                                cx.stop_propagation();
                            }
                        }))
                });
            keys.push(labelled(button, Role::Button, said).into_any_element());
        }
        let reset = self.keyboard.bindings.bindings.contains_key(row.id).then(|| self.keyboard.resets.get(row.id)).flatten().map(|handle| {
            let id = row.id;
            let button = ringed(look, ghost(look, SharedString::from(format!("reset-key-{id}")), "Reset", off), handle, window).track_focus(handle).when(!off, |button| {
                let press = move |this: &mut SettingsOverlay, window: &mut Window, cx: &mut Context<SettingsOverlay>| {
                    let next = this.keyboard.bindings.reset(id);
                    this.save_bindings(next, window, cx);
                };
                button.on_click(cx.listener(move |this, _, window, cx| press(this, window, cx))).on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                        press(this, window, cx);
                    }
                }))
            });
            labelled(button, Role::Button, format!("Reset {}", row.verb))
        });
        let mark: AnyElement = match row.glyph {
            Some(glyph) => svg().path(icon_path(glyph)).flex_none().size(look.length(token::SPACE_4)).text_color(look.color(token::FG_MUTED)).into_any_element(),
            None => div().flex_none().w(look.length(token::SPACE_4)).into_any_element(),
        };
        let verb = div()
            .flex()
            .flex_col()
            .min_w_0()
            .text_size(look.length(token::TEXT_SM))
            .text_color(look.color(token::FG_DEFAULT))
            .child(row.verb)
            .children(row.unbuilt.map(|issue| small(look, format!("Nothing answers it yet · {issue}"), look.color(token::FG_MUTED))))
            .when(!conflicted.is_empty(), |verb| verb.child(small(look, format!("Also {}", conflicted.join(", ")), look.color(token::STATUS_ESCALATED))));
        labelled(
            div()
                .id(SharedString::from(format!("act-{}", row.id)))
                .flex()
                .items_center()
                .justify_between()
                .gap(look.length(token::SPACE_4))
                .py(look.length(token::SPACE_2))
                .border_b(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .child(div().flex().items_center().gap(look.length(token::SPACE_3)).min_w_0().child(mark).child(verb))
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .justify_end()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .children(keys)
                        .child(div().flex_none().min_w(look.length(token::SPACE_12)).children(reset)),
                ),
            Role::Group,
            row.verb,
        )
        .into_any_element()
    }

    pub fn render_keyboard(&self, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let query = self.query_text(cx);
        let searching = !query.trim().is_empty();
        let off = !self.live || self.keyboard.saving;
        let mut body: Vec<AnyElement> = Vec::new();
        if !searching {
            body.push(
                small(look, "Press a key to change it, then the key you want. Esc cancels and Delete unbinds. A dialog or menu always closes on Esc.", look.color(token::FG_MUTED))
                    .text_size(look.length(token::TEXT_SM))
                    .into_any_element(),
            );
        }
        if let Some(said) = &self.keyboard.refused {
            body.push(labelled(super::view::refusal(look, said.clone()).id("keys-refused"), Role::Alert, said.clone()).into_any_element());
        }
        for (title, acts) in found(&query) {
            let rows: Vec<AnyElement> = acts.into_iter().map(|row| self.render_act(row, look, window, cx)).collect();
            body.push(
                labelled(
                    div()
                        .id(SharedString::from(format!("keys-{title}")))
                        .flex()
                        .flex_col()
                        .child(div().mb(look.length(token::SPACE_1)).text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child(title))
                        .children(rows),
                    Role::Group,
                    title,
                )
                .into_any_element(),
            );
        }
        if !searching {
            let none = self.keyboard.bindings.bindings.is_empty();
            let handle = &self.keyboard.reset_all;
            let button = ringed(
                look,
                ghost(look, "keys-reset-all", "Reset all to defaults", off || none).border_color(look.color(token::BORDER_DEFAULT)).track_focus(handle),
                handle,
                window,
            )
            .when(!off && !none, |button| {
                button.on_click(cx.listener(|this, _, window, cx| this.save_bindings(Bindings::default(), window, cx))).on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                        this.save_bindings(Bindings::default(), window, cx);
                    }
                }))
            });
            body.push(div().flex().child(labelled(button, Role::Button, "Reset all to defaults")).into_any_element());
        }
        labelled(card(look, "settings-card-keyboard", "Keyboard shortcuts", None, body), Role::Region, "Keyboard shortcuts").into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str) -> &'static Row {
        row_by_id(id).unwrap_or_else(|| panic!("the registry has {id}"))
    }

    #[test]
    fn every_act_is_under_exactly_one_heading() {
        for act in ACTIONS {
            let under = GROUPS.iter().filter(|(_, scopes)| scopes.contains(&act.scope)).count();
            assert_eq!(under, 1, "{}", act.id);
        }
    }

    #[test]
    fn a_slot_is_spelled_as_the_registry_spells_it() {
        for act in ACTIONS {
            let slots = parse_slots(act.shortcut);
            let again: Vec<Slot> = slots.iter().map(|slot| parse_slots(&spell(slot)).into_iter().next().unwrap_or(Slot::Chord(None))).collect();
            assert_eq!(again, slots, "{}: {}", act.id, act.shortcut);
        }
    }

    #[test]
    fn a_rebind_to_the_shipped_keys_takes_the_change_back() {
        let palette = row("command_palette");
        let moved = Bindings::default().rebind(palette, &[Slot::Chord(parse_chord_for_test("⌘P"))]);
        assert_eq!(moved.bindings.get("command_palette"), Some(&vec!["⌘P".to_string()]));
        assert!(moved.text().contains("\"version\":1"));
        let back = moved.rebind(palette, &parse_slots(palette.shortcut));
        assert!(back.bindings.is_empty());
        assert_eq!(back.text(), "");
    }

    #[test]
    fn a_key_two_acts_answer_where_both_listen_is_a_conflict() {
        let palette = row("command_palette");
        let settings = row("open_settings");
        let clash = Bindings::default().rebind(settings, &parse_slots(palette.shortcut)).all_slots();
        assert!(conflicts(&clash, palette, &parse_slots(palette.shortcut)[0]).contains(&settings.verb));
        assert!(conflicts(&Bindings::default().all_slots(), palette, &parse_slots(palette.shortcut)[0]).is_empty());
    }

    #[test]
    fn a_pressed_key_reads_as_the_registry_spells_it() {
        let read = |text: &'static str| Keystroke::parse(text).ok().and_then(|keystroke| chord_of(&keystroke)).map(|chord| spell_chord(&chord));
        assert_eq!(read("cmd-k").as_deref(), Some("⌘K"));
        assert_eq!(read("shift-cmd-r").as_deref(), Some("⇧⌘R"));
        assert_eq!(read("shift-b").as_deref(), Some("B"));
        assert_eq!(read("enter").as_deref(), Some("Enter"));
        assert_eq!(read("alt-up").as_deref(), Some("⌥↑"));
    }

    fn parse_chord_for_test(text: &str) -> Option<Chord> {
        crate::keymap::parse_chord(text)
    }
}
