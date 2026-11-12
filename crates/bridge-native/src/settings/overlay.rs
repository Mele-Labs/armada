//! The Settings overlay: its state, its keys and its acts. Drawn in `view.rs`.
//!
//! **His departure from Classic (10 Nov 2026)**: Settings is not a title-bar tab but an overlay over the app,
//! 90% of the window, opened from the top-right menu, the macOS menu bar and ⌘,. Its search is cleared every
//! time it opens, and Esc clears a search first, then closes, the keys going back where they were.
//! Everything else is Classic's System page (`apps/desktop/src/renderer/src/system-surface.tsx`).

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{prelude::*, App, Bounds, Focusable, Context, DismissEvent, Entity, FocusHandle, Keystroke, Pixels, SharedString, Subscription, Task, Window};
use gpui_component::input::{InputEvent, InputState, TextareaState};
use ipc::{ResolvedValues, SettingKind, SettingValue, SettingsList};

use super::entries::{self, Body, Entry};
use super::wire::{Refused, Wire};
use crate::dropdown::{Dropdown, Picked};

/// What a typed setting is edited in: one line for a figure or a word, several for a list or a prompt.
pub enum Typed {
    Line(Entity<InputState>),
    Lines(Entity<TextareaState>),
}

impl Typed {
    pub fn text(&self, cx: &App) -> String {
        match self {
            Typed::Line(state) => state.read(cx).value().to_string(),
            Typed::Lines(state) => state.read(cx).value().to_string(),
        }
    }

    pub fn focus(&self, cx: &App) -> FocusHandle {
        match self {
            Typed::Line(state) => state.read(cx).focus_handle(cx),
            Typed::Lines(state) => state.read(cx).focus_handle(cx),
        }
    }

    fn set(&self, text: &str, window: &mut Window, cx: &mut App) {
        match self {
            Typed::Line(state) => state.update(cx, |state, cx| state.set_value(text.to_string(), window, cx)),
            Typed::Lines(state) => state.update(cx, |state, cx| state.set_value(text.to_string(), window, cx)),
        }
    }
}

/// The keys of one setting's controls, so Tab and the arrows reach each of them.
pub struct Stops {
    pub control: FocusHandle,
    pub slider: FocusHandle,
    pub save: FocusHandle,
    pub reset: FocusHandle,
}

/// The project Settings was opened on, from the switcher.
pub struct Scope {
    pub id: String,
    pub name: String,
    /// What its values come to, and from which layer. `None` until Fleet has answered.
    pub values: Option<ResolvedValues>,
    pub saving: Option<&'static str>,
    pub refused: Option<String>,
    /// The words typed into a value that is text: the review model, the theme and the layout.
    pub typed: BTreeMap<&'static str, Entity<InputState>>,
    /// Each choice's key, and each typed value's Save.
    pub stops: BTreeMap<&'static str, FocusHandle>,
    pub saves: BTreeMap<&'static str, FocusHandle>,
}

/// What a dropdown open over Settings picks for.
pub enum Picking {
    Setting(String),
    Value(&'static str),
    /// A run's model on Models, by its place in `RUNS`.
    Run(usize),
}

pub struct SettingsOverlay {
    pub wire: Wire,
    pub open: bool,
    back_to: Option<FocusHandle>,
    pub focus: FocusHandle,
    /// The sidebar's keys: ↑ and ↓ move the pick while it holds them.
    pub side: FocusHandle,
    /// The entries' scroll, which brings the group of the entry picked into view.
    pub side_scroll: gpui::ScrollHandle,
    pub query: Entity<InputState>,
    /// The entry last picked, kept across opens; a search that hides it draws the first it found.
    pub picked: &'static str,
    /// settings.json as Fleet last answered. `None` while it is read.
    pub list: Option<SettingsList>,
    pub read_failed: Option<String>,
    /// The connection could not be reached when Settings opened, or since.
    pub live: bool,
    pub typed: BTreeMap<String, Typed>,
    pub stops: BTreeMap<String, Stops>,
    /// One save in flight for the whole page, as Classic's section allows (`SchemaSection`).
    pub saving: Option<String>,
    pub refused: BTreeMap<String, String>,
    pub scope: Option<Scope>,
    /// What opening settings.json answered when it did not open.
    pub file_said: Option<String>,
    pub file_stop: FocusHandle,
    /// A choice's list, open over Settings.
    pub popup: Option<(Entity<Dropdown>, Vec<Subscription>)>,
    /// The keys of a pane's own rows: `reach:<folder>` and `connectors:<id>`.
    pub panes: BTreeMap<String, FocusHandle>,
    /// Settings → Keyboard shortcuts.
    pub keyboard: super::keyboard::Keyboard,
    /// Settings → Theme: its field, and the mod rows under it.
    pub theme: super::theme::ThemeField,
    pub mods: super::mods::ModRows,
    /// Settings → Models.
    pub models: super::models::ModelsPage,
    /// Settings → Workflows.
    pub workflows: super::workflows::WorkflowsPage,
    /// Where each slider's track was drawn last frame, by key, so a press reads as a figure.
    pub tracks: Rc<RefCell<BTreeMap<String, Bounds<Pixels>>>>,
    keys: Option<Subscription>,
    _typing: Vec<Subscription>,
    _read: Option<Task<()>>,
    _save: Option<Task<()>>,
}

/// The ids a choice's entries carry in the dropdown: its place among the options.
pub const AT: [&str; 24] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20", "21", "22", "23"];

/// What a typed setting's field holds before anything is typed: a list one entry a line.
pub fn held_text(value: &SettingValue) -> String {
    match value {
        SettingValue::Text(text) => text.clone(),
        SettingValue::Integer(n) => n.to_string(),
        SettingValue::Float(n) => n.to_string(),
        SettingValue::List(items) => items.iter().filter_map(|item| if let SettingValue::Text(text) = item { Some(text.as_str()) } else { None }).collect::<Vec<_>>().join("\n"),
        SettingValue::Bool(on) => on.to_string(),
        SettingValue::Null | SettingValue::Map(_) => String::new(),
    }
}

/// The value a typed field would save, or `None` where it is not one the kind takes yet: a figure out of
/// range, or not whole.
pub fn typed_value(kind: &SettingKind, text: &str) -> Option<SettingValue> {
    match kind {
        SettingKind::Integer { min, max, .. } | SettingKind::Seconds { min, max } => {
            text.trim().parse::<i64>().ok().filter(|n| (min..=max).contains(&n)).map(SettingValue::Integer)
        }
        SettingKind::TextList => Some(SettingValue::List(text.lines().map(str::trim).filter(|line| !line.is_empty()).map(|line| SettingValue::Text(line.to_string())).collect())),
        SettingKind::Text | SettingKind::Prompt => Some(SettingValue::Text(text.to_string())),
        SettingKind::Boolean | SettingKind::Choice { .. } | SettingKind::Json => None,
    }
}

/// The draft a typed setting keeps across a close and a restart (`drafts.rs`).
fn keep_key(key: &str) -> String {
    format!("settings:{key}")
}

impl SettingsOverlay {
    pub fn new(wire: Wire, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let query = cx.new(|cx| InputState::new(window, cx).placeholder("Search"));
        let theme = super::theme::ThemeField::new(cx);
        let typing = vec![
            cx.subscribe_in(&query, window, |this, _, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    this.query_changed(cx);
                }
            }),
            // The theme's list opens when the keys reach its field, and shuts when they leave.
            cx.on_focus(&theme.focus, window, |this, _, cx| this.theme_focused(cx)),
            cx.on_blur(&theme.focus, window, |this, _, cx| this.theme_shut(cx)),
        ];
        SettingsOverlay {
            wire,
            open: false,
            back_to: None,
            focus: cx.focus_handle(),
            side: cx.focus_handle().tab_stop(true),
            side_scroll: gpui::ScrollHandle::new(),
            query,
            picked: entries::FIRST,
            list: None,
            read_failed: None,
            live: false,
            typed: BTreeMap::new(),
            stops: BTreeMap::new(),
            saving: None,
            refused: BTreeMap::new(),
            scope: None,
            file_said: None,
            file_stop: cx.focus_handle().tab_stop(true),
            popup: None,
            panes: BTreeMap::new(),
            keyboard: super::keyboard::Keyboard::new(cx),
            theme,
            mods: super::mods::ModRows::default(),
            models: super::models::ModelsPage::new(window, cx),
            workflows: super::workflows::WorkflowsPage::new(window, cx),
            tracks: Rc::default(),
            keys: None,
            _typing: typing,
            _read: None,
            _save: None,
        }
    }

    /// Open over whatever has the keys. `project` scopes it to one project, by its id and name. The search is
    /// cleared every time, and settings.json is read again.
    pub fn show(&mut self, project: Option<(String, String)>, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            self.back_to = window.focused(cx);
        }
        self.open = true;
        self.query.update(cx, |query, cx| query.set_value("", window, cx));
        self.scope = project.map(|(id, name)| Scope {
            id,
            name,
            values: None,
            saving: None,
            refused: None,
            typed: BTreeMap::new(),
            stops: super::project::CHOICES.into_iter().map(|name| (name, cx.focus_handle().tab_stop(true))).collect(),
            saves: super::project::TYPED.into_iter().map(|name| (name, cx.focus_handle().tab_stop(true))).collect(),
        });
        if self.scope.is_some() {
            self.picked = "project:values";
            self.read_values(window, cx);
        } else if entries::entry(self.picked).is_none_or(|entry| entry.group == entries::Group::Project) {
            self.picked = entries::FIRST;
        }
        self.read(window, cx);
        self.keyboard.listening = None;
        let bindings = self.wire.key_bindings(cx);
        self.keyboard.set(super::keyboard::Bindings::read(&bindings));
        self.theme.refused = None;
        self.mods.refused.clear();
        self.models.reopen();
        self.mod_keys(cx);
        window.focus(&self.side, cx);
        let (me, here) = (cx.entity().downgrade(), window.window_handle().window_id());
        self.keys = Some(cx.intercept_keystrokes(move |event, window, cx| {
            if window.window_handle().window_id() != here {
                return;
            }
            let Some(me) = me.upgrade() else { return };
            if me.update(cx, |this, cx| this.on_keystroke(&event.keystroke, window, cx)) {
                cx.stop_propagation();
            }
        }));
        cx.notify();
    }

    /// Shut it and give the keys back to whoever had them.
    pub fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }
        self.open = false;
        self.keys = None;
        self.popup = None;
        self.mods.panel = None;
        match self.back_to.take() {
            Some(handle) => window.focus(&handle, cx),
            None => window.blur(cx),
        }
        cx.notify();
    }

    fn read(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.live = self.wire.live(cx);
        let read = self.wire.settings(cx);
        self._read = Some(cx.spawn_in(window, async move |this, cx| {
            let answer = read.await;
            let _ = this.update_in(cx, |this, window, cx| match answer {
                Ok(list) => this.set_list(list, window, cx),
                Err(why) => {
                    this.read_failed = Some(why.said);
                    this.live = false;
                    cx.notify();
                }
            });
        }));
    }

    /// Fleet's settings.json, as a read or a save answered it. A field is made for each typed setting the first
    /// time it is seen; a value republished replaces what was typed, as Classic's field does, unless the person
    /// kept a draft of it.
    pub fn set_list(&mut self, list: SettingsList, window: &mut Window, cx: &mut Context<Self>) {
        let was = self.list.take();
        for one in &list.settings {
            if !entries::drawn(one) {
                continue;
            }
            self.stops.entry(one.key.clone()).or_insert_with(|| Stops {
                control: cx.focus_handle().tab_stop(true),
                slider: cx.focus_handle().tab_stop(true),
                save: cx.focus_handle().tab_stop(true),
                reset: cx.focus_handle().tab_stop(true),
            });
            let lines = match one.kind {
                SettingKind::Integer { .. } | SettingKind::Seconds { .. } | SettingKind::Text => Some(false),
                SettingKind::TextList | SettingKind::Prompt => Some(true),
                _ => None,
            };
            let Some(lines) = lines else { continue };
            let held = held_text(&one.value);
            if !self.typed.contains_key(&one.key) {
                let typed = if lines {
                    let rows = if matches!(one.kind, SettingKind::Prompt) { (8, 24) } else { (4, 4) };
                    Typed::Lines(cx.new(|cx| TextareaState::new(window, cx).auto_grow(rows.0, rows.1)))
                } else {
                    Typed::Line(cx.new(|cx| InputState::new(window, cx)))
                };
                let draft = crate::drafts::kept(&keep_key(&one.key), cx);
                typed.set(draft.as_deref().unwrap_or(&held), window, cx);
                let key = one.key.clone();
                let sub = match &typed {
                    Typed::Line(state) => cx.subscribe_in(state, window, move |this, _, event: &InputEvent, _, cx| this.typed_changed(&key, event, cx)),
                    Typed::Lines(state) => cx.subscribe_in(state, window, move |this, _, event: &InputEvent, _, cx| this.typed_changed(&key, event, cx)),
                };
                self._typing.push(sub);
                self.typed.insert(one.key.clone(), typed);
                continue;
            }
            // A republished value is the answer to a save: what was typed has gone, and the field reads it.
            let before = was.as_ref().and_then(|was| was.settings.iter().find(|row| row.key == one.key)).map(|row| held_text(&row.value));
            if before.as_deref() != Some(held.as_str()) {
                if let Some(typed) = self.typed.get(&one.key) {
                    typed.set(&held, window, cx);
                }
                crate::drafts::clear(&keep_key(&one.key), cx);
            }
        }
        let reach = list.settings.iter().find(|one| one.key == super::view::REACH_FOLDERS).map(|one| super::view::reach_folders(Some(&one.value)));
        let mut rows: Vec<String> = reach.map(|(allow, block)| block.into_iter().chain(allow).map(|folder| format!("reach:{folder}")).collect()).unwrap_or_default();
        if list.settings.iter().any(|one| one.key == super::view::CONNECTORS_OFF) {
            rows.extend(self.wire.repositories(cx).unwrap_or_default().into_iter().map(|(id, _, _)| format!("connectors:{id}")));
        }
        self.panes.retain(|key, _| rows.contains(key));
        for row in rows {
            self.panes.entry(row).or_insert_with(|| cx.focus_handle().tab_stop(true));
        }
        self.read_failed = None;
        self.list = Some(list);
        cx.notify();
    }

    /// Take one folder off the list a Job's card always allows or blocks, the rest kept.
    pub fn remove_folder(&mut self, folder: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(one) = self.setting(super::view::REACH_FOLDERS) else { return };
        let (allow, block) = super::view::reach_folders(Some(&one.value));
        let kept = |list: Vec<String>| SettingValue::List(list.into_iter().filter(|one| one != folder).map(SettingValue::Text).collect());
        let value = SettingValue::Map([("allow".to_string(), kept(allow)), ("block".to_string(), kept(block))].into_iter().collect());
        self.save(super::view::REACH_FOLDERS.to_string(), Some(value), window, cx);
    }

    /// Turn one repository's connectors on or off. A switch put back where its armada.yml has it is unset, so
    /// it follows the file again; nothing set at all removes the key.
    pub fn switch_connectors(&mut self, id: &str, yml_off: bool, on: bool, window: &mut Window, cx: &mut Context<Self>) {
        let mut set = super::view::connectors_set(self.setting(super::view::CONNECTORS_OFF).map(|one| &one.value));
        if on == !yml_off {
            set.remove(id);
        } else {
            set.insert(id.to_string(), !on);
        }
        let value = (!set.is_empty()).then(|| SettingValue::Map(set.into_iter().map(|(id, off)| (id, SettingValue::Bool(off))).collect()));
        self.save(super::view::CONNECTORS_OFF.to_string(), value, window, cx);
    }

    fn typed_changed(&mut self, key: &str, event: &InputEvent, cx: &mut Context<Self>) {
        if !matches!(event, InputEvent::Change) {
            return;
        }
        let Some(text) = self.typed.get(key).map(|typed| typed.text(cx)) else { return };
        let held = self.setting(key).map(|one| held_text(&one.value));
        if held.as_deref() == Some(text.as_str()) {
            crate::drafts::clear(&keep_key(key), cx);
        } else {
            crate::drafts::keep(&keep_key(key), &text, cx);
        }
        cx.notify();
    }

    fn query_changed(&mut self, cx: &mut Context<Self>) {
        cx.notify();
    }

    pub fn setting(&self, key: &str) -> Option<&ipc::Setting> {
        self.list.as_ref()?.settings.iter().find(|one| one.key == key)
    }

    pub fn query_text(&self, cx: &App) -> String {
        self.query.read(cx).value().to_string()
    }

    /// The entries the sidebar lists now. Repository is listed once Fleet serves the switches it holds, as
    /// Classic lists it while `tokens.connectorsOff` is in the list.
    pub fn visible(&self, cx: &App) -> Vec<&'static Entry> {
        let query = self.query_text(cx);
        entries::visible(&query, self.list.as_ref(), self.scope.is_some())
            .into_iter()
            .filter(|one| one.body != Body::Repository || self.setting(super::view::CONNECTORS_OFF).is_some())
            // Tracker and Fleet are listed once a mod of their kind is, as Classic's `made` lists them.
            .filter(|one| match one.body {
                Body::Mods(Some(kind)) => self.wire.mods(cx).iter().any(|row| row.kind == Some(kind)),
                _ => true,
            })
            // A section this Fleet's settings.json does not hold is left out once it has answered, not drawn blank.
            .filter(|one| match (one.body, &self.list) {
                (Body::Schema(ids), Some(list)) => ids.iter().any(|id| !entries::settings_in(list, id).is_empty()),
                _ => true,
            })
            .collect()
    }

    pub fn shown(&self, cx: &App) -> Option<&'static Entry> {
        entries::shown(self.picked, &self.visible(cx))
    }

    pub fn pick(&mut self, id: &'static str, cx: &mut Context<Self>) {
        self.picked = id;
        cx.notify();
    }

    fn step(&mut self, by: isize, cx: &mut Context<Self>) {
        let visible = self.visible(cx);
        let Some(now) = self.shown(cx) else { return };
        let at = visible.iter().position(|one| one.id == now.id).unwrap_or(0) as isize;
        let next = (at + by).clamp(0, visible.len() as isize - 1) as usize;
        self.picked = visible[next].id;
        // The entries scroll by group, as they are drawn: the picked one's group comes into view.
        let group = visible[next].group;
        let drawn: Vec<entries::Group> = entries::GROUPS.into_iter().filter(|one| visible.iter().any(|entry| entry.group == *one)).collect();
        if let Some(at) = drawn.iter().position(|one| *one == group) {
            self.side_scroll.scroll_to_item(at);
        }
        cx.notify();
    }

    // ---- saving -------------------------------------------------------------------------------

    /// Send one change, or a reset where `value` is `None`. One save is out at a time; a refusal is drawn against
    /// the key Fleet names where this page holds it, else the key sent.
    pub fn save(&mut self, key: String, value: Option<SettingValue>, window: &mut Window, cx: &mut Context<Self>) {
        self.save_changes(key.clone(), [(key, value)].into_iter().collect(), window, cx);
    }

    /// Several changes sent as one save, held and refused against `key`: a model switched off moves every setting
    /// that names it in the same save, since Fleet refuses one that leaves any naming it.
    pub fn save_changes(&mut self, key: String, changes: BTreeMap<String, Option<SettingValue>>, window: &mut Window, cx: &mut Context<Self>) {
        if self.saving.is_some() || !self.live {
            return;
        }
        self.saving = Some(key.clone());
        self.refused.remove(&key);
        let sent = self.wire.save(changes, cx);
        self._save = Some(cx.spawn_in(window, async move |this, cx| {
            let answer = sent.await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.saving = None;
                // Save and Reset go once the value is in force: the keys go to the setting's own control.
                let gone = this.stops.get(&key).is_some_and(|stops| stops.save.is_focused(window) || stops.reset.is_focused(window));
                if gone {
                    let to = this.typed.get(&key).map(|typed| typed.focus(cx)).or_else(|| this.stops.get(&key).map(|stops| stops.control.clone()));
                    if let Some(to) = to {
                        window.focus(&to, cx);
                    }
                }
                let taken = answer.is_ok();
                match answer {
                    Ok(list) => this.set_list(list, window, cx),
                    Err(refused) => this.refuse(&key, refused, cx),
                }
                this.models_saved(taken, window, cx);
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn refuse(&mut self, sent: &str, refused: Refused, cx: &mut Context<Self>) {
        let at = refused.key.filter(|key| self.setting(key).is_some()).unwrap_or_else(|| sent.to_string());
        // Taken from outside the overlay, the refusal is said where he is, and held for the page as well.
        if !self.open {
            let said = refused.said.clone();
            cx.defer(move |cx| crate::toasts::tell(said, cx));
        }
        self.refused.insert(at, refused.said);
        cx.notify();
    }

    /// Save what is typed into a figure, a line, a list or a prompt.
    pub fn save_typed(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(one) = self.setting(key) else { return };
        let Some(text) = self.typed.get(key).map(|typed| typed.text(cx)) else { return };
        if text == held_text(&one.value) {
            return;
        }
        let Some(value) = typed_value(&one.kind, &text) else { return };
        self.save(key.to_string(), Some(value), window, cx);
    }

    /// The slider moves what is typed; Save sends it.
    pub fn slide(&mut self, key: &str, to: i64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(typed) = self.typed.get(key) else { return };
        typed.set(&to.to_string(), window, cx);
        cx.notify();
    }

    pub fn open_file(&mut self, cx: &mut Context<Self>) {
        let Some(list) = self.list.clone() else {
            self.file_said = Some("Fleet has not said where settings.json is yet. It does once it answers.".into());
            cx.notify();
            return;
        };
        let opened = self.wire.open_file(&list, cx);
        cx.spawn(async move |this, cx| {
            let answer = opened.await;
            let _ = this.update(cx, |this, cx| {
                this.file_said = answer.err().map(|why| format!("This machine did not open it: {why}"));
                cx.notify();
            });
        })
        .detach();
    }

    // ---- choices ------------------------------------------------------------------------------

    /// Open a choice's list under its control. Picking an option saves it at once, as Classic's select does.
    pub fn open_choice(&mut self, picking: Picking, name: SharedString, options: Vec<(SharedString, bool)>, at: gpui::Point<gpui::Pixels>, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        let entries = options.into_iter().enumerate().take(AT.len()).map(|(n, (label, on))| crate::dropdown::Entry::Choice { id: AT[n], label, on }).collect();
        let back = window.focused(cx);
        let menu = cx.new(|cx| Dropdown::new(name, entries, at, gpui::Anchor::TopLeft, from_keys, cx));
        let picked = cx.subscribe_in(&menu, window, move |this, _, picked: &Picked, window, cx| {
            let Picked::Entry { id, .. } = picked;
            let Some(n) = AT.iter().position(|one| one == id) else { return };
            this.chose(&picking, n, window, cx);
        });
        let dismissed = cx.subscribe_in(&menu, window, move |this, _, _: &DismissEvent, window, cx| {
            this.popup = None;
            if let Some(back) = &back {
                window.focus(back, cx);
            }
            cx.notify();
        });
        let keys = menu.read(cx).focus_handle(cx);
        window.defer(cx, move |window, cx| window.focus(&keys, cx));
        self.popup = Some((menu, vec![picked, dismissed]));
        cx.notify();
    }

    fn chose(&mut self, picking: &Picking, n: usize, window: &mut Window, cx: &mut Context<Self>) {
        match picking {
            Picking::Setting(key) => {
                let Some(SettingKind::Choice { options }) = self.setting(key).map(|one| one.kind.clone()) else { return };
                let Some(word) = options.get(n) else { return };
                if self.setting(key).is_some_and(|one| one.value == SettingValue::Text(word.clone())) {
                    return;
                }
                self.save(key.clone(), Some(SettingValue::Text(word.clone())), window, cx);
            }
            Picking::Value(name) => self.choose_value(name, n, window, cx),
            Picking::Run(at) => self.chose_run(*at, n, window, cx),
        }
    }

    // ---- the project's values -----------------------------------------------------------------

    fn read_values(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.scope.as_ref().map(|scope| scope.id.clone()) else { return };
        let read = self.wire.values(&id, cx);
        cx.spawn_in(window, async move |this, cx| {
            let answer = read.await;
            let _ = this.update_in(cx, |this, window, cx| {
                let Some(scope) = this.scope.as_mut().filter(|scope| scope.id == id) else { return };
                match answer {
                    Ok(values) => {
                        for (name, value) in [("review_model", &values.review_model), ("theme", &values.theme), ("layout", &values.layout)] {
                            let text = value.value.clone().unwrap_or_default();
                            let field = scope.typed.entry(name).or_insert_with(|| cx.new(|cx| InputState::new(window, cx)));
                            field.update(cx, |field, cx| field.set_value(text, window, cx));
                        }
                        scope.values = Some(values);
                        scope.refused = None;
                    }
                    Err(why) => scope.refused = Some(why.said),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn choose_value(&mut self, name: &'static str, n: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(word) = super::project::options(name).get(n).map(|(word, _)| word.to_string()) else { return };
        self.write_value(name, word, window, cx);
    }

    /// Save the text typed into a value.
    pub fn save_value(&mut self, name: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(word) = self.scope.as_ref().and_then(|scope| scope.typed.get(name)).map(|field| field.read(cx).value().trim().to_string()) else { return };
        self.write_value(name, word, window, cx);
    }

    /// Write one value: the project's own block read fresh, the one key changed, and the block sent whole, so
    /// nothing else it sets is taken back.
    fn write_value(&mut self, name: &'static str, word: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(scope) = self.scope.as_mut() else { return };
        if scope.saving.is_some() {
            return;
        }
        scope.saving = Some(name);
        scope.refused = None;
        let id = scope.id.clone();
        let fresh = self.wire.projects_now(cx);
        cx.spawn_in(window, async move |this, cx| {
            let projects = fresh.await;
            let sent = this.update_in(cx, |this, _, cx| {
                let projects = projects?;
                let own = projects.projects.iter().find(|one| one.id == id).ok_or_else(|| Refused::said(format!("there is no project {id}")))?.values.clone();
                let now = this.scope.as_ref().and_then(|scope| scope.values.clone());
                let values = super::project::with_value(own.as_ref(), now.as_ref(), name, &word);
                Ok::<_, Refused>(this.wire.set_values(&id, values, cx))
            });
            let answer = match sent {
                Ok(Ok(task)) => task.await.map(|_| ()),
                Ok(Err(why)) => Err(why),
                Err(_) => return,
            };
            let _ = this.update_in(cx, |this, window, cx| {
                if let Some(scope) = this.scope.as_mut() {
                    scope.saving = None;
                    if let Err(why) = answer {
                        scope.refused = Some(why.said);
                    }
                }
                this.read_values(window, cx);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    // ---- keys ---------------------------------------------------------------------------------

    /// The keys every stop in the detail takes, in the order Tab reaches them: the settings.json press, then
    /// each setting's control, its slider, Save and Reset where drawn.
    pub fn detail_stops(&self, window: &Window, cx: &App) -> Vec<FocusHandle> {
        let Some(shown) = self.shown(cx) else { return Vec::new() };
        let mut stops = Vec::new();
        if let Body::ProjectValues = shown.body {
            return self.scope.as_ref().map(|scope| super::project::stops(scope, cx)).unwrap_or_default();
        }
        if let Body::Keyboard = shown.body {
            return self.keyboard.stops(&self.query_text(cx));
        }
        if let Body::Models = shown.body {
            return self.model_stops(cx);
        }
        if let Body::Workflows = shown.body {
            return self.workflow_stops(cx);
        }
        if let Body::Theme = shown.body {
            let mut stops = vec![self.theme.focus.clone()];
            stops.extend(self.mod_stops(super::mods::Page::Theme, cx));
            return stops;
        }
        if let Some(page) = super::mods::page_of(shown.body) {
            return self.mod_stops(page, cx);
        }
        stops.push(self.file_stop.clone());
        let Some(list) = &self.list else { return stops };
        let sections: Vec<&str> = match shown.body {
            Body::Schema(ids) => ids.to_vec(),
            Body::Features => vec!["features"],
            _ => Vec::new(),
        };
        for id in sections {
            for one in entries::settings_in(list, id) {
                let Some(held) = self.stops.get(&one.key) else { continue };
                match self.typed.get(&one.key) {
                    Some(typed) => stops.push(typed.focus(cx)),
                    None => stops.push(held.control.clone()),
                }
                if super::view::slides(one) {
                    stops.push(held.slider.clone());
                }
                if self.dirty(&one.key, cx) || self.saving.as_deref() == Some(one.key.as_str()) {
                    stops.push(held.save.clone());
                }
                if one.saved.is_some() {
                    stops.push(held.reset.clone());
                }
            }
        }
        if let Body::Repository | Body::Features = shown.body {
            stops.extend(super::view::pane_stops(self, shown, cx));
        }
        let _ = window;
        stops
    }

    /// Whether what is typed into a setting differs from the value in force.
    pub fn dirty(&self, key: &str, cx: &App) -> bool {
        let (Some(one), Some(typed)) = (self.setting(key), self.typed.get(key)) else { return false };
        typed.text(cx) != held_text(&one.value)
    }

    fn all_stops(&self, window: &Window, cx: &App) -> Vec<FocusHandle> {
        let mut stops = vec![self.query.read(cx).focus_handle(cx), self.side.clone()];
        stops.extend(self.detail_stops(window, cx));
        stops
    }

    pub(super) fn in_text(&self, window: &Window, cx: &App) -> bool {
        self.query.read(cx).focus_handle(cx).is_focused(window) || self.theme.focus.is_focused(window) || self.models.filter_focused(window, cx) || self.workflows.filter_focused(window, cx) || self.typed.values().any(|typed| typed.focus(cx).is_focused(window))
            || self.scope.as_ref().is_some_and(|scope| scope.typed.values().any(|field| field.read(cx).focus_handle(cx).is_focused(window)))
    }

    fn focus_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.query.update(cx, |query, cx| query.focus(window, cx));
    }

    /// Tab and Shift+Tab, kept inside the overlay: past its last stop is its first.
    fn tab(&mut self, back: bool, window: &mut Window, cx: &mut Context<Self>) {
        let stops = self.all_stops(window, cx);
        if stops.is_empty() {
            return;
        }
        let at = stops.iter().position(|one| one.is_focused(window));
        let next = match (at, back) {
            (None, false) => 0,
            (None, true) => stops.len() - 1,
            (Some(at), false) => (at + 1) % stops.len(),
            (Some(at), true) => (at + stops.len() - 1) % stops.len(),
        };
        window.focus(&stops[next], cx);
        cx.notify();
    }

    /// The overlay's own keys, ahead of the app's while it is open. `true` where one was taken.
    fn on_keystroke(&mut self, key: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        // A choice's list open over Settings takes its own keys.
        if self.popup.is_some() {
            return false;
        }
        // A mod's panel beside the page takes its own keys; Esc outside it puts it away.
        if let Some(opened) = &self.mods.panel {
            if key.key == "escape" && !opened.view.focus_handle(cx).contains_focused(window, cx) {
                self.close_panel(window, cx);
                return true;
            }
            return false;
        }
        // A key waiting for its binding takes the next key, whatever it is.
        if self.keyboard_key(key, window, cx) {
            return true;
        }
        // The theme field types, and its list takes the arrows, Enter and Esc while it is open.
        if self.theme_key(key, window, cx) {
            return true;
        }
        // Models' own keys: its filter, its rows, its switches, selects and efforts.
        if self.shown(cx).is_some_and(|shown| shown.body == Body::Models) && self.models_key(key, window, cx) {
            return true;
        }
        if self.shown(cx).is_some_and(|shown| shown.body == Body::Workflows) && self.workflows_key(key, window, cx) {
            return true;
        }
        // A mod row's arrows: open and shut what it holds, and move between rows.
        if let Some(page) = self.shown(cx).and_then(|shown| super::mods::page_of(shown.body)) {
            if self.mods_key(page, key, window, cx) {
                return true;
            }
        }
        let mods = &key.modifiers;
        let bare = !mods.platform && !mods.control && !mods.alt;
        match key.key.as_str() {
            "escape" if bare => {
                if !self.query_text(cx).is_empty() {
                    self.query.update(cx, |query, cx| query.set_value("", window, cx));
                    // The page may change under the keys: unless they are in the search, they go to the entries.
                    if !self.query.read(cx).focus_handle(cx).is_focused(window) {
                        window.focus(&self.side, cx);
                    }
                    cx.notify();
                } else {
                    self.close(window, cx);
                }
                true
            }
            "f" if mods.platform && !mods.control && !mods.alt => {
                self.focus_search(window, cx);
                true
            }
            "/" if bare && !self.in_text(window, cx) => {
                self.focus_search(window, cx);
                true
            }
            "tab" if !mods.platform && !mods.control && !mods.alt => {
                self.tab(mods.shift, window, cx);
                true
            }
            // ↓ from the search goes to the entries, as a list under a field does.
            "down" if bare && self.query.read(cx).focus_handle(cx).is_focused(window) => {
                window.focus(&self.side, cx);
                true
            }
            "up" | "down" if bare && self.side.is_focused(window) => {
                self.step(if key.key == "up" { -1 } else { 1 }, cx);
                true
            }
            "right" | "enter" if bare && self.side.is_focused(window) => {
                let first = self.detail_stops(window, cx).into_iter().next();
                if let Some(first) = first {
                    window.focus(&first, cx);
                }
                true
            }
            "left" if bare && !self.in_text(window, cx) && !self.side.is_focused(window) && !self.on_slider(window) => {
                window.focus(&self.side, cx);
                cx.notify();
                true
            }
            "left" | "right" if bare && self.on_slider(window) => {
                self.nudge(if key.key == "left" { -1 } else { 1 }, window, cx);
                true
            }
            _ => false,
        }
    }

    fn on_slider(&self, window: &Window) -> bool {
        self.stops.values().any(|stops| stops.slider.is_focused(window))
    }

    /// ← and → on a slider move what is typed a step.
    fn nudge(&mut self, by: i64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(key) = self.stops.iter().find(|(_, stops)| stops.slider.is_focused(window)).map(|(key, _)| key.clone()) else { return };
        let Some(one) = self.setting(&key) else { return };
        let (SettingKind::Integer { min, max, .. } | SettingKind::Seconds { min, max }) = one.kind else { return };
        let now = self.typed.get(&key).and_then(|typed| typed.text(cx).trim().parse::<i64>().ok()).unwrap_or(match one.value {
            SettingValue::Integer(n) => n,
            _ => min,
        });
        self.slide(&key, (now + by).clamp(min, max), window, cx);
    }
}
