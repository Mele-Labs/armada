//! The in-app annotation layer: ⌥⌘A outside a walk, point at anything, write a note, then send the
//! notes to a Job picked from a menu or to a new held Job (`apps/desktop/src/renderer/src/annotate/`,
//! inventory section 2).
//!
//! **It reuses the walk's capture** ([`crate::capture::Capture`]): aim at an element or draw a box,
//! the card, the window's picture. What differs is what a note becomes. A walk holds it for Send
//! notes; here it is a numbered pin that stays on the screen until it is sent, and the bar over
//! everything sends every unsent note on this screen. The layer never opens the dispatch sheet: a
//! new Job is started held, on its start card, with the notes' words as its request.
//!
//! The rest of the layer is here: ⌥⌘S for a note on the whole window, the opened note's card with
//! its acts, the All notes panel (`panel.rs`) and the card's tools and pictures (`tools.rs`).
//!
//! **Available only in a development build**, as Electron's is ([`dev_build`]).

pub mod live;
mod panel;
pub mod store;
pub mod tools;

use std::cell::Cell;
use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    actions, canvas, div, point, prelude::*, Anchor, App, Bounds, ClickEvent, Context, DismissEvent, Div, Entity, EventEmitter,
    ExternalPaths, FocusHandle, Focusable, FontWeight, Global, KeyBinding, Pixels, Point, Role, SharedString, Stateful, Subscription, Task, Window,
    WindowId,
};
use serde_json::{json, Value};
use gpui_component::input::{InputEvent, InputState};

use crate::capture::{self, Capture, CaptureEvent, Held, Words};
use crate::dispatch::face::Offered;
use crate::dropdown::{Dropdown, Entry, Picked};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::titlebar::id_of;
use crate::titlebar::parts::{card, glyph, mono, spinner, tip, Btn, Weight};
use crate::walk::bar::chord;
use tools::{Scope, Tools};

actions!(annotate, [Toggle, Shoot]);

/// Where the layer is kept between openings of the screen: the notes survive a toggle.
const KEEP_NOTE: &str = "annotate:note";

/// The deferred priority the layer and its cards paint at: over Settings and the palette (2), under menus (4)
/// and a walk's chrome.
pub const OVER: usize = 3;

/// The windows whose layer is on, so a screen drawn again in one finds the layer on (Electron keeps this in
/// the window's session storage). Per window, not per machine.
#[derive(Default)]
struct Awake(HashSet<WindowId>);

impl Global for Awake {}

/// A screen started over from nothing: no window is on.
pub fn forget_awake(cx: &mut App) {
    cx.set_global(Awake::default());
}

/// ⌥⌘A and ⌥⌘S. **Electron reads them by key place in the layer itself** (`Layer.tsx:75-91`); the
/// registry does not list them, so they are restated here until it does. Bound with no context, so
/// they work from any focus, and only in a development build. The screen that hosts the layer
/// answers them (`dispatch::screen::bind`).
pub fn bind(cx: &mut App) {
    if dev_build() {
        cx.bind_keys([KeyBinding::new("alt-cmd-a", Toggle, None), KeyBinding::new("alt-cmd-s", Shoot, None)]);
    }
}

/// Whether this is a development build: not packaged, or a kept build run from a source folder.
///
/// Electron's gate is `!app.isPackaged || process.defaultApp` (`M/dev-build.ts`): the dev copy of the
/// app is a bundle in name, so a bundle started with a source folder still counts. The native
/// reading of the same: the executable is **packaged** when it lives inside a `.app` bundle's
/// `Contents/MacOS`; a development build is any executable that is not (cargo's `target`), a bundle
/// kept as a build (`scripts/preview` and `scripts/restart` put a `build.toml` beside the `.app`, as
/// Electron's preview is started with its folder), **or** a bundle started with its working
/// directory inside a source folder, one that has a `.git` above it. Launched from the Dock the
/// working directory is `/` and no record stands beside it, so a shipped app is never one.
pub fn dev_build() -> bool {
    let exe = std::env::current_exe().ok();
    let cwd = std::env::current_dir().ok();
    is_dev_build(exe.as_deref(), cwd.as_deref(), &|dir: &Path| dir.join(".git").exists(), &|dir: &Path| dir.join("build.toml").exists())
}

pub fn is_dev_build(exe: Option<&Path>, cwd: Option<&Path>, has_git: &dyn Fn(&Path) -> bool, kept: &dyn Fn(&Path) -> bool) -> bool {
    let bundle = exe.and_then(|exe| {
        exe.ancestors().find(|at| {
            at.extension().is_some_and(|ext| ext == "app") && exe.strip_prefix(at).is_ok_and(|inside| inside.starts_with("Contents/MacOS"))
        })
    });
    match bundle {
        None => true,
        Some(bundle) => bundle.parent().is_some_and(kept) || cwd.is_some_and(|cwd| cwd.ancestors().any(has_git)),
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Status {
    Open,
    Done,
}

/// A note written on the screen, or read back from where Electron keeps them (a file under
/// `.armada/annotations`).
pub(crate) struct Note {
    pub id: u32,
    /// The id the note is filed under, `<key>.json`: it sorts by when the note was written.
    pub key: String,
    /// The note as its file reads. A field this layer does not draw is kept here and written back as it was.
    pub raw: Value,
    pub said: String,
    /// What it is about, as the card's path line reads it.
    pub path: String,
    /// The component or tag a panel row names it by.
    pub on: String,
    pub screen: Option<SharedString>,
    /// Where it stands on this screen, when it does: a note on another screen has no box, so no pin.
    pub ring: Option<Bounds<Pixels>>,
    pub window: bool,
    /// The window as it was when the note was held: the send burns the ring and number into it.
    pub picture: Option<(Arc<image::RgbaImage>, f32)>,
    pub status: Status,
    /// The Job it went to, once Fleet took it.
    pub sent: Option<SharedString>,
    pub tools: Entity<Tools>,
}

/// A note as it was saved, for a layer that reads them again when it comes on.
pub struct Saved {
    pub said: &'static str,
    pub on: &'static str,
    pub screen: Option<&'static str>,
    pub done: bool,
    pub sent: Option<&'static str>,
}

/// What the layer asks of the app.
#[derive(Clone, Debug, PartialEq)]
pub enum Asked {
    /// Notes, to a Job; `None` starts a new held Job with the notes' words.
    Send { to: Option<Offered>, notes: Vec<String> },
}

impl EventEmitter<Asked> for Layer {}

struct Said {
    text: SharedString,
    fault: bool,
    /// A Job's title inside the line, as the words before it and after it: the title is what is cut
    /// when the line is too long for the bar, and the whole line stays in the tooltip.
    title: Option<(SharedString, SharedString, SharedString)>,
}

impl Said {
    fn plain(text: impl Into<SharedString>, fault: bool) -> Said {
        Said { text: text.into(), fault, title: None }
    }

    fn naming(lead: &str, title: &str, tail: &str, fault: bool) -> Said {
        Said { text: format!("{lead}{title}{tail}").into(), fault, title: Some((lead.to_string().into(), title.to_string().into(), tail.to_string().into())) }
    }
}

struct Menu {
    dropdown: Entity<Dropdown>,
    /// The notes it sends.
    ids: Vec<u32>,
    _picked: Subscription,
    _dismissed: Subscription,
}

pub struct Layer {
    on: bool,
    /// The screen the layer sits over, where aim and a box may hold.
    area: Rc<Cell<Option<Bounds<Pixels>>>>,
    /// The screen's name, as a note records it.
    pub screen: Option<SharedString>,
    capture: Option<(Entity<Capture>, Subscription)>,
    pub(crate) notes: Vec<Note>,
    next_id: u32,
    /// Where the notes are kept, once the app says: nothing is read or written before.
    dir: Option<PathBuf>,
    loading: Option<Task<()>>,
    /// The Job the pending send went to, for the note's file.
    sending_session: Option<SharedString>,
    /// The note whose card is open.
    pub(crate) opened: Option<u32>,
    /// The live Jobs the picker lists.
    pub targets: Vec<Offered>,
    menu: Option<Menu>,
    split_at: Rc<Cell<Option<Bounds<Pixels>>>>,
    /// Where the card's and the All notes header's Send to job are drawn, so the picker opens beside them.
    card_at: Rc<Cell<Option<Bounds<Pixels>>>>,
    pub(crate) header_at: Rc<Cell<Option<Bounds<Pixels>>>>,
    bar_at: Rc<Cell<Option<Bounds<Pixels>>>>,
    sending: Option<SharedString>,
    sending_ids: Vec<u32>,
    said: Option<Said>,
    keys_were: Option<FocusHandle>,
    card_focus: FocusHandle,
    /// The opened card's Send to job and Start job: a saved note opens on Send to job, so the send is all keys.
    card_keys: (FocusHandle, FocusHandle),
    /// ⌘V on the opened card reads a picture off the clipboard before anything else does.
    paste: Option<Subscription>,
    deleting_card: bool,
    // The All notes panel.
    pub(crate) listing: bool,
    pub(crate) tab: panel::Tab,
    pub(crate) search: Option<(Entity<InputState>, Subscription)>,
    pub(crate) query: String,
    pub(crate) selected: BTreeSet<u32>,
    pub(crate) anchor: Option<u32>,
    pub(crate) folded: HashSet<Option<SharedString>>,
    pub(crate) deleting_row: Option<u32>,
    pub(crate) deleting_selection: bool,
}

impl Layer {
    pub fn new(area: Rc<Cell<Option<Bounds<Pixels>>>>, targets: Vec<Offered>, cx: &mut Context<Self>) -> Layer {
        Layer {
            on: false,
            area,
            screen: None,
            capture: None,
            notes: Vec::new(),
            next_id: 1,
            dir: None,
            loading: None,
            sending_session: None,
            opened: None,
            targets,
            menu: None,
            split_at: Rc::default(),
            card_at: Rc::default(),
            header_at: Rc::default(),
            bar_at: Rc::default(),
            sending: None,
            sending_ids: Vec::new(),
            said: None,
            keys_were: None,
            card_focus: cx.focus_handle(),
            card_keys: (cx.focus_handle().tab_stop(true), cx.focus_handle().tab_stop(true)),
            paste: None,
            deleting_card: false,
            listing: false,
            tab: panel::Tab::Open,
            search: None,
            query: String::new(),
            selected: BTreeSet::new(),
            anchor: None,
            folded: HashSet::new(),
            deleting_row: None,
            deleting_selection: false,
        }
    }

    /// Notes read back from where they were kept, oldest first: the ones on other screens are in the
    /// panel and have no pin here.
    pub fn read_again(&mut self, saved: Vec<Saved>, cx: &mut Context<Self>) {
        for (at, one) in saved.into_iter().enumerate() {
            let key = format!("00000000-0000{at:02}-seed");
            let mut raw = json!({
                "id": key, "status": if one.done { "done" } else { "open" }, "text": one.said, "component": null, "owners": [],
                "ownersFrom": "parent", "selector": one.on, "element": { "tag": one.on, "text": "", "label": null },
                "screen": one.screen, "layer": null, "location": "/", "scenario": null,
                "box": { "x": 0, "y": 0, "width": 0, "height": 0 }, "window": { "width": 0, "height": 0 },
                "createdAt": store::now(), "updatedAt": store::now(),
            });
            if let Some(title) = one.sent {
                raw["sent"] = json!({ "at": store::now(), "sessionId": "", "title": title });
            }
            let note = self.note_of(raw, cx);
            self.notes.push(note);
        }
    }

    /// The walk's made-up notes go, so the live app lists only what is on disk.
    pub fn forget_seeded(&mut self, cx: &mut Context<Self>) {
        self.notes.retain(|note| !note.key.ends_with("-seed"));
        cx.notify();
    }

    /// A note as its file reads. It has a pin here when it was written on this screen, where it stood.
    fn note_of(&mut self, raw: Value, cx: &mut Context<Self>) -> Note {
        let id = self.next_id;
        self.next_id += 1;
        let text = |value: &Value| value.as_str().map(str::to_string);
        let on = text(&raw["element"]["tag"]).unwrap_or_else(|| "element".to_string());
        let screen = text(&raw["screen"]);
        let window = raw["scope"] == "window";
        let at = |name: &str| raw["box"][name].as_f64().unwrap_or(0.) as f32;
        let ring = (!window && screen.is_some() && screen.as_deref() == self.screen.as_deref() && at("width") > 0.)
            .then(|| Bounds { origin: point(gpui::px(at("x")), gpui::px(at("y"))), size: gpui::size(gpui::px(at("width")), gpui::px(at("height"))) });
        let tools = cx.new(|_| Tools::new(Scope::Element { words: None, stories: Vec::new() }));
        Note {
            id,
            key: text(&raw["id"]).unwrap_or_default(),
            said: text(&raw["text"]).unwrap_or_default(),
            path: text(&raw["element"]["label"]).unwrap_or_else(|| format!("{on} · {}", screen.as_deref().unwrap_or("no screen marked"))),
            on,
            screen: screen.map(Into::into),
            ring,
            window,
            picture: None,
            status: if raw["status"] == "done" { Status::Done } else { Status::Open },
            sent: text(&raw["sent"]["title"]).map(Into::into),
            tools,
            raw,
        }
    }

    /// Keep the notes in `dir`: read what stands there now, and write each note as it is made or changed. The
    /// layer is on again if this window had it on before a reload.
    pub fn keep_in(&mut self, dir: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.dir = Some(dir.clone());
        let was_on = cx.try_global::<Awake>().is_some_and(|awake| awake.0.contains(&window.window_handle().window_id()));
        if was_on && !self.on {
            self.toggle(window, cx);
        }
        self.loading = Some(cx.spawn(async move |this, cx| {
            let found = cx.background_executor().spawn(async move { store::list(&dir) }).await;
            let _ = this.update(cx, |this, cx| this.read_back(found, cx));
        }));
    }

    fn read_back(&mut self, found: Vec<Value>, cx: &mut Context<Self>) {
        for raw in found {
            if self.notes.iter().any(|note| raw["id"].as_str() == Some(note.key.as_str())) {
                continue;
            }
            let note = self.note_of(raw, cx);
            self.notes.push(note);
        }
        // Oldest first, as Electron's `byCreation` does: by when it was written, and by its id for two in one millisecond.
        self.notes.sort_by(|a, b| (a.raw["createdAt"].as_str(), &a.key).cmp(&(b.raw["createdAt"].as_str(), &b.key)));
        cx.notify();
    }

    /// Every note written out and read back at once, on the main thread: a walk has no time for the background
    /// to finish between two steps, and shows a reload from what the disk holds.
    pub fn round_trip_now(&mut self, cx: &mut Context<Self>) {
        let Some(dir) = self.dir.clone() else { return };
        for note in &self.notes {
            let _ = store::save(&dir, &note.raw);
        }
        let found = store::list(&dir);
        self.read_back(found, cx);
    }

    /// Write the note's file, and any pictures with it, off the main thread. A write that fails is said in the bar.
    fn persist(&self, id: u32, pictures: Vec<(String, Vec<u8>)>, cx: &mut Context<Self>) {
        let (Some(dir), Some(note)) = (self.dir.clone(), self.note(id)) else { return };
        let raw = note.raw.clone();
        cx.spawn(async move |this, cx| {
            let wrote = cx
                .background_executor()
                .spawn(async move {
                    for (name, png) in pictures {
                        store::save_png(&dir, &name, &png)?;
                    }
                    store::save(&dir, &raw)
                })
                .await;
            if let Err(why) = wrote {
                let _ = this.update(cx, |this, cx| {
                    this.said = Some(Said::plain(format!("Note not saved: {why}"), true));
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// A note changed: its file says when, and is written again.
    fn changed(&mut self, id: u32, cx: &mut Context<Self>) {
        if let Some(note) = self.notes.iter_mut().find(|note| note.id == id) {
            note.raw["updatedAt"] = json!(store::now());
        }
        self.persist(id, Vec::new(), cx);
    }

    /// The note's number among the open ones, oldest first: a done note carries none and renumbers nothing.
    pub(crate) fn number(&self, id: u32) -> Option<usize> {
        self.notes.iter().filter(|note| note.status == Status::Open).position(|note| note.id == id).map(|at| at + 1)
    }

    fn note(&self, id: u32) -> Option<&Note> {
        self.notes.iter().find(|note| note.id == id)
    }

    /// ⌥⌘A: on, and again off. The notes stay for the next time.
    pub fn toggle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.on {
            self.on = false;
            self.capture = None;
            self.menu = None;
            self.opened = None;
            self.paste = None;
            self.listing = false;
            if let Some(keys) = self.keys_were.take() {
                window.focus(&keys, cx);
            }
        } else {
            self.on = true;
            self.keys_were = window.focused(cx);
            self.arm(window, cx);
        }
        let window_id = window.window_handle().window_id();
        let awake = cx.default_global::<Awake>();
        if self.on {
            awake.0.insert(window_id);
        } else {
            awake.0.remove(&window_id);
        }
        cx.notify();
    }

    /// ⌥⌘S and the bar's Screenshot window: a note on the whole window.
    pub fn shoot(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.on {
            return;
        }
        self.opened = None;
        self.paste = None;
        let screen = self.screen.clone();
        if let Some((capture, _)) = &self.capture {
            capture.update(cx, |capture, cx| capture.whole_window(screen, cx));
            let focus = capture.read(cx).focus_handle(cx);
            window.defer(cx, move |window, cx| window.focus(&focus, cx));
        }
        cx.notify();
    }

    /// Report on a point menu: the layer wakes if it slept, and a note begins on what is under `at`.
    pub fn report_at(&mut self, at: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        if !self.on {
            self.toggle(window, cx);
        }
        self.opened = None;
        self.paste = None;
        if let Some((capture, _)) = &self.capture {
            capture.update(cx, |capture, cx| capture.element_at_point(at, window, cx));
            let focus = capture.read(cx).focus_handle(cx);
            window.defer(cx, move |window, cx| window.focus(&focus, cx));
        }
        cx.notify();
    }

    /// A fresh capture over the screen, with the keys on it.
    fn arm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let words = Words { save: "Save note", band: Some("New note"), card: "New note", tools: true };
        let capture = cx.new(|cx| Capture::new(Rc::clone(&self.area), words, Some(KEEP_NOTE.into()), cx));
        let watch = cx.subscribe_in(&capture, window, |this, _, event: &CaptureEvent, window, cx| match event {
            CaptureEvent::Noted(noted) => this.noted(noted, window, cx),
            CaptureEvent::Closed => this.arm(window, cx),
        });
        let focus = capture.read(cx).focus_handle(cx);
        window.defer(cx, move |window, cx| window.focus(&focus, cx));
        self.capture = Some((capture, watch));
        cx.notify();
    }

    fn noted(&mut self, noted: &capture::Noted, window: &mut Window, cx: &mut Context<Self>) {
        let path = noted.held.said().to_string();
        let on = match &noted.held {
            Held::Element(drawn) => capture::role_word(drawn.role).unwrap_or("element").to_lowercase(),
            Held::Region(_) => "region".to_string(),
            Held::Window { .. } => "window".to_string(),
        };
        let tools = noted.tools.clone().unwrap_or_else(|| cx.new(|_| Tools::new(Scope::Window)));
        let now = store::now();
        let key = store::new_id(chrono::Utc::now());
        let ring = noted.held.bounds();
        let view = window.viewport_size();
        let (selector, name, label) = match &noted.held {
            Held::Element(drawn) => (format!("{} \"{}\"", on, drawn.name), drawn.name.to_string(), Some(path.clone())),
            Held::Region(_) => ("region".to_string(), String::new(), Some(path.clone())),
            Held::Window { .. } => ("window".to_string(), String::new(), None),
        };
        let mut raw = json!({
            "id": key, "status": "open", "text": noted.said, "component": null, "owners": [], "ownersFrom": "parent",
            "selector": selector, "element": { "tag": on, "text": name, "label": label },
            "screen": self.screen, "layer": null, "location": "/", "scenario": null,
            "box": { "x": f32::from(ring.origin.x), "y": f32::from(ring.origin.y), "width": f32::from(ring.size.width), "height": f32::from(ring.size.height) },
            "window": { "width": f32::from(view.width), "height": f32::from(view.height) },
            "createdAt": now, "updatedAt": now,
        });
        let mut pictures: Vec<(String, Vec<u8>)> = Vec::new();
        if matches!(noted.held, Held::Window { .. }) {
            raw["scope"] = json!("window");
        }
        if let Some((picture, _)) = &noted.picture {
            let mut png = Vec::new();
            if picture.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).is_ok() {
                raw["screenshot"] = json!(format!("{key}.png"));
                pictures.push((format!("{key}.png"), png));
            }
        }
        let kept = tools.read(cx);
        let changes: Vec<Value> = kept
            .changes()
            .iter()
            .map(|change| {
                let mut one = json!({ "kind": format!("{:?}", change.kind).to_lowercase(), "property": change.property, "before": change.before, "after": change.after });
                if let Some(token) = change.token {
                    one["token"] = json!(token);
                }
                one
            })
            .collect();
        if !changes.is_empty() {
            raw["changes"] = Value::Array(changes);
        }
        let examples: Vec<String> = kept.examples().iter().enumerate().map(|(at, _)| format!("{key}-example-{}.png", at + 1)).collect();
        if !examples.is_empty() {
            pictures.extend(kept.examples().iter().zip(&examples).map(|(example, name)| (name.clone(), example.image.bytes.clone())));
            raw["examples"] = json!(examples);
        }
        let id = self.next_id;
        self.next_id += 1;
        self.notes.push(Note {
            id,
            key,
            raw,
            said: noted.said.clone(),
            path,
            on,
            screen: self.screen.clone(),
            ring: Some(ring),
            window: matches!(noted.held, Held::Window { .. }),
            picture: noted.picture.clone(),
            status: Status::Open,
            sent: None,
            tools,
        });
        self.persist(id, pictures, cx);
        self.said = None;
        self.arm(window, cx);
        // The saved note stays open on Send to job, so the send is all keys: Enter opens the live Jobs, ⌥1–9
        // or Enter picks one, Esc closes. With no Job live the keys wait on Start job.
        self.open_note(id, window, cx);
        let start = if self.targets.is_empty() { self.card_keys.1.clone() } else { self.card_keys.0.clone() };
        window.defer(cx, move |window, cx| window.focus(&start, cx));
    }

    /// The unsent open notes standing on this screen, in the order they were written.
    fn sendable_here(&self) -> Vec<u32> {
        self.notes.iter().filter(|note| note.status == Status::Open && note.sent.is_none() && note.ring.is_some()).map(|note| note.id).collect()
    }

    /// Send these notes, those of them that are open and unsent: pending in this frame, the splits
    /// off. `to` of `None` starts a new held Job.
    pub(crate) fn send(&mut self, ids: Vec<u32>, to: Option<Offered>, cx: &mut Context<Self>) {
        let ids: Vec<u32> = ids.into_iter().filter(|id| self.note(*id).is_some_and(|note| note.status == Status::Open && note.sent.is_none())).collect();
        if self.sending.is_some() || ids.is_empty() {
            return;
        }
        let notes = self.notes.iter().filter(|note| ids.contains(&note.id)).map(|note| note.said.clone()).collect();
        self.sending = Some(to.as_ref().map_or("Starting job".into(), |job| format!("Sending to {}", job.title).into()));
        self.sending_session = to.as_ref().map(|job| job.id.clone());
        self.sending_ids = ids;
        self.said = None;
        self.opened = None;
        self.paste = None;
        cx.emit(Asked::Send { to, notes });
        cx.notify();
    }

    /// The live Jobs the picker lists, as Fleet says them now.
    pub fn set_targets(&mut self, targets: Vec<Offered>, cx: &mut Context<Self>) {
        if self.targets != targets {
            self.targets = targets;
            cx.notify();
        }
    }

    /// The pending send's notes as the live send needs them, numbered in the order they were written.
    pub fn outgoing(&self) -> Vec<live::Outgoing> {
        self.notes
            .iter()
            .filter(|note| self.sending_ids.contains(&note.id))
            .enumerate()
            .map(|(at, note)| live::Outgoing {
                key: note.key.clone(),
                raw: note.raw.clone(),
                window: note.window,
                number: at + 1,
                picture: match (&note.picture, note.raw["screenshot"].as_str(), &self.dir) {
                    (Some((picture, scale)), _, _) => live::Picture::Held(Arc::clone(picture), *scale, note.ring),
                    (None, Some(name), Some(dir)) => live::Picture::File(dir.join(name)),
                    _ => live::Picture::None,
                },
            })
            .collect()
    }

    /// The Job the pending send went to, once a new one has its id.
    pub fn sending_to(&mut self, session: SharedString) {
        self.sending_session = Some(session);
    }

    pub(crate) fn is_sending(&self) -> bool {
        self.sending.is_some()
    }

    /// The Job the pending send named, for a mock Fleet that answers whoever was asked.
    pub fn asked_of(&self) -> SharedString {
        self.sending.as_ref().and_then(|word| word.strip_prefix("Sending to ").map(|title| title.to_string().into())).unwrap_or_else(|| "Dispatch sheet notes".into())
    }

    /// Fleet took the notes: each is saved with the Job it went to.
    pub fn sent(&mut self, title: &str, new_job: bool, cx: &mut Context<Self>) {
        self.sending = None;
        let ids = std::mem::take(&mut self.sending_ids);
        let session = self.sending_session.take().unwrap_or_default();
        for note in self.notes.iter_mut().filter(|note| ids.contains(&note.id)) {
            note.sent = Some(title.to_string().into());
            note.raw["sent"] = json!({ "at": store::now(), "sessionId": session, "title": title });
        }
        for id in ids {
            self.changed(id, cx);
        }
        self.said = Some(if new_job { Said::naming("Job started: ", title, "", false) } else { Said::naming("Sent to Job ", title, "", false) });
        cx.notify();
    }

    /// Fleet refused: the notes are as they were, and the bar says why.
    pub fn refused(&mut self, title: &str, why: &str, cx: &mut Context<Self>) {
        self.sending = None;
        self.sending_ids.clear();
        self.said = Some(Said::naming("Notes not sent to \"", title, &format!("\": {why}"), true));
        cx.notify();
    }

    /// A picture arrives on the card that is up, as a paste or a drop would bring one. A walk cannot
    /// paste, so Fleet's mock answers with this.
    pub fn picture_arrives(&mut self, cx: &mut Context<Self>) {
        let tools = match self.opened.and_then(|id| self.note(id)) {
            Some(note) => Some(note.tools.clone()),
            None => self.capture.as_ref().and_then(|(capture, _)| capture.read(cx).tools()),
        };
        if let Some(tools) = tools {
            tools.update(cx, |tools, cx| tools.mock_picture(cx));
        }
    }

    // -- the notes' own acts -------------------------------------------------------------------------

    pub(crate) fn set_status(&mut self, id: u32, status: Status, cx: &mut Context<Self>) {
        if let Some(note) = self.notes.iter_mut().find(|note| note.id == id) {
            note.status = status;
            note.raw["status"] = json!(if status == Status::Done { "done" } else { "open" });
        }
        self.changed(id, cx);
        cx.notify();
    }

    pub(crate) fn remove(&mut self, id: u32, cx: &mut Context<Self>) {
        if let (Some(dir), Some(note)) = (self.dir.clone(), self.note(id)) {
            let key = note.key.clone();
            cx.background_executor().spawn(async move { store::remove(&dir, &key) }).detach();
        }
        self.notes.retain(|note| note.id != id);
        self.selected.remove(&id);
        if self.opened == Some(id) {
            self.opened = None;
            self.paste = None;
        }
        cx.notify();
    }

    /// A pin or a row opens the note's card, and the keys go to it; pressed again, it shuts.
    pub(crate) fn open_note(&mut self, id: u32, window: &mut Window, cx: &mut Context<Self>) {
        if self.opened == Some(id) {
            self.close_card(window, cx);
            return;
        }
        self.opened = Some(id);
        self.deleting_card = false;
        let card = self.card_focus.clone();
        window.defer(cx, move |window, cx| window.focus(&card, cx));
        let tools = self.note(id).map(|note| note.tools.clone());
        let me = cx.entity().downgrade();
        self.paste = Some(cx.intercept_keystrokes(move |event, window, cx| {
            let key = &event.keystroke;
            if key.key != "v" || !key.modifiers.platform || key.modifiers.alt || key.modifiers.shift {
                return;
            }
            let (Some(me), Some(tools)) = (me.upgrade(), tools.clone()) else { return };
            if me.read(cx).card_focus.contains_focused(window, cx) && tools.update(cx, |tools, cx| tools.paste(cx)) {
                cx.stop_propagation();
            }
        }));
        cx.notify();
    }

    fn close_card(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.opened = None;
        self.paste = None;
        self.deleting_card = false;
        self.refocus(window, cx);
        cx.notify();
    }

    // -- the picker --------------------------------------------------------------------------------

    /// The Jobs' menu, for these notes, with its corner at `at`.
    pub(crate) fn open_menu(&mut self, ids: Vec<u32>, at: Point<Pixels>, corner: Anchor, window: &mut Window, cx: &mut Context<Self>) {
        if self.sending.is_some() || self.menu.is_some() {
            return;
        }
        // Searchable by typing, and the first nine take ⌥1–9, as Classic's picker and the dispatch chips.
        let entries: Vec<Entry> = std::iter::once(Entry::Search { label: "Find a Job", to_rule: false })
            .chain(self.targets.iter().enumerate().map(|(at, job)| Entry::Item {
                id: id_of(format!("job:{}", job.id)),
                label: job.title.clone(),
                icon: Some("square-terminal"),
                danger: false,
                key: (at < 9).then(|| format!("⌥{}", at + 1).into()),
                off: None,
            }))
            .collect();
        let dropdown = cx.new(|cx| Dropdown::new("Send to a Job", entries, at, corner, true, cx));
        let picked = cx.subscribe_in(&dropdown, window, |this, _, picked: &Picked, window, cx| {
            let Picked::Entry { id, .. } = picked;
            let ids = this.menu.take().map(|menu| menu.ids).unwrap_or_default();
            this.refocus(window, cx);
            let job = id.strip_prefix("job:").and_then(|id| this.targets.iter().find(|one| one.id == id)).cloned();
            if job.is_some() {
                this.send(ids, job, cx);
            }
        });
        let dismissed = cx.subscribe_in(&dropdown, window, |this, _, _: &DismissEvent, window, cx| {
            this.menu = None;
            // Esc goes back to the card's Send to job where a card is open.
            if this.opened.is_some() {
                let send = this.card_keys.0.clone();
                window.focus(&send, cx);
            } else {
                this.refocus(window, cx);
            }
            cx.notify();
        });
        let keys = dropdown.read(cx).focus_handle(cx);
        window.defer(cx, move |window, cx| window.focus(&keys, cx));
        self.menu = Some(Menu { dropdown, ids, _picked: picked, _dismissed: dismissed });
        cx.notify();
    }

    fn open_bar_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let ids = self.sendable_here();
        let from = Rc::clone(&self.split_at);
        self.open_menu_by(ids, &from, window, cx);
    }

    /// The Jobs' picker right beside the Send to job drawn at `from`: under it, or over it in the window's
    /// lower half.
    pub(crate) fn open_menu_by(&mut self, ids: Vec<u32>, from: &Cell<Option<Bounds<Pixels>>>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(from) = from.get() else { return };
        let gap = cx.look().length(token::SPACE_1);
        let (at, corner) = if from.center().y < window.viewport_size().height / 2. {
            (point(from.origin.x, from.origin.y + from.size.height + gap), Anchor::TopLeft)
        } else {
            (point(from.origin.x, from.origin.y - gap), Anchor::BottomLeft)
        };
        self.open_menu(ids, at, corner, window, cx);
    }

    /// The keys go back to the capture after a menu or a card.
    pub(crate) fn refocus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some((capture, _)) = &self.capture {
            let focus = capture.read(cx).focus_handle(cx);
            window.focus(&focus, cx);
        }
    }
}

/// Start job and the caret that lists the Jobs, as Electron's `SplitButton`: one control in two
/// parts. A press on the first sends to a new Job, a press on the caret opens the menu.
#[allow(clippy::too_many_arguments)]
pub(crate) fn split(
    look: &Look,
    cx: &App,
    id: String,
    label: SharedString,
    off: bool,
    spin: bool,
    at: Option<Rc<Cell<Option<Bounds<Pixels>>>>>,
    keys: Option<(&(FocusHandle, FocusHandle), [bool; 2])>,
    main: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    caret: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let accent = look.color(token::ACCENT);
    let ink = look.color(token::TEXT_ON_ACCENT);
    let part = |id: SharedString| {
        div()
            .id(id)
            .h(look.length(token::H_CONTROL))
            .flex()
            .items_center()
            .justify_center()
            .bg(accent)
            .text_color(ink)
            .when(!off, |part| part.cursor_pointer().hover(|style| style.opacity(0.9)))
            .when(off, |part| part.opacity(0.7))
    };
    let keyed = |part: Stateful<Div>, focus: Option<(&FocusHandle, bool)>| keyed(look, part, focus);
    let measure = at.map(|at| canvas(move |bounds, _, _| at.set(Some(bounds)), |_, _, _, _| {}).absolute().size_full());
    div()
        .relative()
        .flex_none()
        .flex()
        .gap(look.length(token::BORDER_WIDTH))
        .children(measure)
        .child(labelled(
            keyed(part(format!("{id}-main").into()), keys.map(|(keys, on)| (&keys.0, on[0])))
                .px(look.length(token::SPACE_4))
                .gap(look.length(token::SPACE_2))
                .rounded_l(look.length(token::RADIUS_LG))
                .text_size(look.length(token::TEXT_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .children(spin.then(|| spinner(SharedString::from(format!("{id}-spin")), look.length(token::SPACE_4), ink, cx)))
                .child(label.clone())
                .when(!off, |part| part.on_click(main)),
            Role::Button,
            label,
        ))
        .child(labelled(
            keyed(part(format!("{id}-caret").into()), keys.map(|(keys, on)| (&keys.1, on[1])))
                .w(look.length(token::H_CONTROL))
                .rounded_r(look.length(token::RADIUS_LG))
                .child(glyph("chevron-down", look.length(token::SPACE_3), ink))
                .when(!off, |part| part.on_click(caret)),
            Role::Button,
            "Send to a Job",
        ))
}

/// Held by keys, a part takes Enter and Space as a press; focused, it is ringed as Classic's `:focus-visible`,
/// an accent outline set off by `--focus-ring-offset`.
fn keyed(look: &Look, part: Stateful<Div>, focus: Option<(&FocusHandle, bool)>) -> Stateful<Div> {
    let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
    match focus {
        Some((focus, on)) => part.relative().track_focus(focus).when(on, |part| {
            let out = -(look.length(token::FOCUS_RING_WIDTH) + gpui::px(2.));
            part.child(
                div()
                    .absolute()
                    .top(out)
                    .bottom(out)
                    .left(out)
                    .right(out)
                    .rounded(look.length(token::RADIUS_LG) + gpui::px(4.))
                    .border(look.length(token::FOCUS_RING_WIDTH))
                    .border_color(ring),
            )
        }),
        None => part,
    }
}

/// Send to job, then Start job, both in sight: the send leads, and its press opens the live Jobs beside
/// it. `pending` names a send under way, drawn turning in Start job's place. `name` tells Send to job
/// apart where two are drawn at once.
#[allow(clippy::too_many_arguments)]
pub(crate) fn sends(
    look: &Look,
    cx: &App,
    id: &str,
    name: &str,
    off: bool,
    no_jobs: bool,
    pending: Option<SharedString>,
    at: Rc<Cell<Option<Bounds<Pixels>>>>,
    keys: Option<(&(FocusHandle, FocusHandle), [bool; 2])>,
    send: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    start: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let busy = pending.is_some();
    let send_off = off || busy || no_jobs;
    let measure = canvas(move |bounds, _, _| at.set(Some(bounds)), |_, _, _, _| {}).absolute().size_full();
    let to = Btn::new(SharedString::from(format!("{id}-send")), "Send to job", Weight::Primary)
        .disabled(send_off)
        .draw(look, cx)
        .relative()
        .child(measure)
        .child(glyph("chevron-down", look.length(token::SPACE_3), look.color(token::TEXT_ON_ACCENT)));
    let to = keyed(look, to, keys.map(|(keys, on)| (&keys.0, on[0])))
        .when(no_jobs && !busy, |to| to.tooltip(tip("No Job is live")))
        .when(!send_off, |to| to.on_click(send));
    let started = match pending {
        Some(said) => labelled(Btn::new(SharedString::from(format!("{id}-start")), said.clone(), Weight::Secondary).pending(true).draw(look, cx), Role::Button, said),
        None => {
            let button = Btn::new(SharedString::from(format!("{id}-start")), "Start job", Weight::Secondary).disabled(off).draw(look, cx);
            labelled(keyed(look, button, keys.map(|(keys, on)| (&keys.1, on[1]))).when(!off, |button| button.on_click(start)), Role::Button, "Start job")
        }
    };
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_2))
        .child(labelled(to, Role::Button, name.to_string()))
        .child(started)
}

impl Render for Layer {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.on {
            return div().into_any_element();
        }
        let look = cx.look();
        let focus_ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
        let view = window.viewport_size();
        // Everything here is placed in the window's own coordinates, which is where the labelled
        // elements and the pointer are read; the screen this sits in starts below the title bar.
        let origin = self.area.get().map(|area| area.origin).unwrap_or_default();
        // Pins are the open notes', and the done ones' too while the Done tab is chosen.
        let shown = |note: &Note| note.status == Status::Open || self.tab == panel::Tab::Done;
        let pins = self.notes.iter().filter(|note| shown(note) && !note.window).filter_map(|note| {
            let ring = note.ring?;
            let id = note.id;
            let sent = note.sent.is_some();
            let done = note.status == Status::Done;
            let number = self.number(id);
            let size = look.length(token::H_BADGE);
            let (ground, ink, edge) = if sent || done {
                (look.color(token::BG_RAISED), look.color(token::ACCENT), look.color(token::ACCENT))
            } else {
                (look.color(token::ACCENT), look.color(token::TEXT_ON_ACCENT), look.color(token::BG_BASE))
            };
            // A done note carries no number, here or in its label, so nothing a person reads counts finished work.
            let place = number.map_or("Done note".to_string(), |number| format!("Note {number}, open"));
            let name = match &note.sent {
                Some(title) => format!("{place}, sent to {title}: {}", note.said),
                None => format!("{place}: {}", note.said),
            };
            Some(labelled(
                div()
                    .id(SharedString::from(format!("pin-{id}")))
                    .occlude()
                    .absolute()
                    .left(ring.origin.x + ring.size.width - size / 2.)
                    .top(ring.origin.y - size / 2.)
                    .min_w(size)
                    .h(size)
                    .px(look.length(token::SPACE_1))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(size)
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(edge)
                    .bg(ground)
                    .text_color(ink)
                    .text_size(look.length(token::TEXT_2XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                    .cursor_pointer()
                    .focusable()
                    .tab_stop(true)
                    .focus_visible(move |style| style.border_color(focus_ring))
                    .children(number.map(|number| number.to_string()))
                    .on_click(cx.listener(move |this, _, window, cx| this.open_note(id, window, cx))),
                Role::Button,
                name,
            ))
        });
        let opened = self.opened.and_then(|id| self.note(id)).filter(|note| shown(note)).and_then(|note| {
            let ring = note.ring?;
            let id = note.id;
            let gap = look.length(token::SPACE_2);
            let number = self.number(id);
            let title = number.map_or("Done note".to_string(), |number| format!("Note {number}"));
            let open = note.status == Status::Open;
            let dispatchable = open && note.sent.is_none();
            let off = self.sending.is_some();
            let ids = vec![id];
            let again = ids.clone();
            let dropped = note.tools.clone();
            let armed = self.deleting_card;
            let delete = Btn::new("note-delete", if armed { "Press again to delete" } else { "Delete" }, Weight::Secondary)
                .draw(look, cx)
                .text_color(look.color(token::ERROR))
                .on_click(cx.listener(move |this, _, window, cx| {
                    if this.deleting_card {
                        this.remove(id, cx);
                        this.refocus(window, cx);
                    } else {
                        this.deleting_card = true;
                        cx.notify();
                    }
                }));
            let content = card(look)
                .id("note-card")
                .occlude()
                .track_focus(&self.card_focus)
                .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                    if event.keystroke.key == "escape" {
                        cx.stop_propagation();
                        this.close_card(window, cx);
                    }
                }))
                .on_drop(move |paths: &ExternalPaths, _, cx| tools::dropped(&dropped, paths, cx))
                .w(look.length(token::SPACE_8) * 14.)
                .max_h(view.height - gap * 2.)
                .overflow_y_scroll()
                .gap(look.length(token::SPACE_3))
                .p(look.length(token::SPACE_4))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .text_size(look.length(token::TEXT_XS))
                        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                        .text_color(look.color(token::ACCENT))
                        .child(glyph("map-pin", look.length(token::SPACE_3) + look.length(token::SPACE_1) / 2., look.color(token::ACCENT)))
                        .child(title),
                )
                .child(div().truncate().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_MUTED)).child(note.path.clone()))
                .child(div().text_size(look.length(token::TEXT_SM)).child(note.said.clone()))
                .child(note.tools.clone())
                .children(note.sent.clone().map(|title| {
                    div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(format!("Sent to Job {title}"))
                }))
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .children(dispatchable.then(|| {
                            sends(
                                look,
                                cx,
                                "note",
                                "Send to job",
                                off,
                                self.targets.is_empty(),
                                None,
                                Rc::clone(&self.card_at),
                                Some((&self.card_keys, [self.card_keys.0.is_focused(window), self.card_keys.1.is_focused(window)])),
                                cx.listener(move |this, _, window, cx| {
                                    let from = Rc::clone(&this.card_at);
                                    this.open_menu_by(again.clone(), &from, window, cx)
                                }),
                                cx.listener(move |this, _, _, cx| this.send(ids.clone(), None, cx)),
                            )
                        }))
                        .child(labelled(
                            Btn::new("note-status", if open { "Mark done" } else { "Reopen" }, Weight::Secondary).draw(look, cx).on_click(cx.listener(move |this, _, _, cx| {
                                let next = if this.note(id).is_some_and(|note| note.status == Status::Open) { Status::Done } else { Status::Open };
                                this.set_status(id, next, cx);
                            })),
                            Role::Button,
                            if open { "Mark done" } else { "Reopen" },
                        ))
                        .child(labelled(delete, Role::Button, if armed { "Press again to delete" } else { "Delete" })),
                );
            Some(capture::placed(ring, note.window, window, gap, look.length(token::SPACE_8) * 14., labelled(content, Role::Dialog, "Note")))
        });
        let panel = self.listing.then(|| self.panel(window, cx));
        div()
            .id("annotate")
            .absolute()
            .left(-origin.x)
            .top(-origin.y)
            .w(view.width)
            .h(view.height)
            .on_key_down(|event, window, cx| {
                if crate::keymap::traverse(event, window, cx) {
                    cx.stop_propagation();
                }
            })
            .children(self.capture.as_ref().map(|(capture, _)| capture.clone()))
            .children(pins)
            .children(opened)
            .children(panel)
            .child(self.bar(cx))
            .children(self.menu.as_ref().map(|menu| menu.dropdown.clone()))
            .into_any_element()
    }
}

impl Layer {
    /// The bar over everything: `Annotating`, the whole-window note, the panel's key, what the last
    /// send said, the split and the key. **No count of open or done notes is drawn**, so the key
    /// that opens the panel is named for what it opens.
    fn bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let sendable = !self.sendable_here().is_empty();
        let pending = self.sending.clone();
        let split = sendable.then(|| {
            sends(
                look,
                cx,
                "annotate",
                "Send to job",
                false,
                self.targets.is_empty(),
                pending.clone(),
                Rc::clone(&self.split_at),
                None,
                cx.listener(|this, _, window, cx| this.open_bar_menu(window, cx)),
                cx.listener(|this, _, _, cx| {
                    let ids = this.sendable_here();
                    this.send(ids, None, cx)
                }),
            )
        });
        let said = self.said.as_ref().map(|said| {
            // A chip as tall as the Sending one beside it, with room inside its edge; too long for the bar,
            // the Job's title is cut and the whole line is in the tooltip.
            let words = match &said.title {
                Some((lead, title, tail)) => div()
                    .flex()
                    .min_w_0()
                    .child(div().flex_none().child(lead.clone()))
                    .child(div().min_w_0().truncate().child(title.clone()))
                    .children((!tail.is_empty()).then(|| div().flex_none().child(tail.clone()))),
                None => div().min_w_0().truncate().child(said.text.clone()),
            };
            labelled(
                div()
                    .id("annotate-said")
                    .flex()
                    .items_center()
                    .min_w_0()
                    .max_w(look.length(token::SPACE_12) * 9.)
                    .h(look.length(token::H_CONTROL))
                    .px(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_LG))
                    .text_color(look.color(if said.fault { token::ERROR } else { token::FG_MUTED }))
                    .tooltip(tip(said.text.clone()))
                    .child(words),
                Role::Status,
                said.text.clone(),
            )
        });
        let shot = labelled(
            Btn::new("annotate-shoot", "Screenshot window", Weight::Secondary)
                .icon("camera")
                .draw(look, cx)
                .tooltip(tip("A note on the whole window, with the page, the open Job and Doctor's read ⌥⌘S"))
                .on_click(cx.listener(|this, _, window, cx| this.shoot(window, cx))),
            Role::Button,
            "Screenshot window",
        );
        let listing = self.listing;
        let notes = labelled(
            Btn::new("annotate-notes", "All notes", if listing { Weight::Primary } else { Weight::Secondary })
                .icon("list")
                .draw(look, cx)
                .on_click(cx.listener(|this, _, window, cx| {
                    this.listing = !this.listing;
                    if this.listing {
                        this.make_search(window, cx);
                    }
                    cx.notify();
                })),
            Role::Button,
            "All notes",
        );
        let key = chord("⌥⌘A", cx);
        let bar_at = Rc::clone(&self.bar_at);
        div().absolute().left_0().right_0().bottom(look.length(token::SPACE_3)).flex().justify_center().child(labelled(
            div()
                .id("annotate-bar")
                .occlude()
                .relative()
                .flex()
                .flex_wrap()
                .items_center()
                .justify_center()
                .gap(look.length(token::SPACE_3))
                .max_w_full()
                .px(look.length(token::SPACE_4))
                .py(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_OVERLAY))
                .shadow(look.shadows(token::SHADOW_OVERLAY))
                .text_size(look.length(token::TEXT_XS))
                .text_color(look.color(token::FG_MUTED))
                .child(canvas(move |bounds, _, _| bar_at.set(Some(bounds)), |_, _, _, _| {}).absolute().size_full())
                .child(
                    div()
                        .px(look.length(token::SPACE_2))
                        .rounded(look.length(token::RADIUS_SM))
                        .bg(look.color(token::ACCENT))
                        .text_color(look.color(token::TEXT_ON_ACCENT))
                        .font_family(mono(look))
                        .text_size(look.length(token::TEXT_2XS))
                        .line_height(look.length(token::LEADING_2XS))
                        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                        .child("Annotating"),
                )
                .child(shot)
                .child(notes)
                .children(said)
                .children(split)
                .child(div().id("annotate-key").tooltip(tip("Turns annotating off")).child(key)),
            Role::Status,
            "Annotating",
        ))
    }

    fn make_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.search.is_some() {
            return;
        }
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search notes"));
        let typed = cx.subscribe_in(&search, window, |this, search, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                this.query = search.read(cx).value().to_string();
                cx.notify();
            }
        });
        self.search = Some((search, typed));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none(_: &Path) -> bool {
        false
    }

    #[test]
    fn a_kept_preview_bundle_is_a_dev_build_and_a_shipped_one_is_not() {
        let preview = Path::new("/Users/me/Library/Application Support/Armada/builds/current/Armada.app/Contents/MacOS/Armada");
        let stamp = Path::new("/Users/me/Library/Application Support/Armada/builds/2026-11-12T132938Z-da8d647");
        let kept = |dir: &Path| dir.ends_with("builds/current");
        assert!(is_dev_build(Some(preview), Some(stamp), &none, &kept));
        let shipped = Path::new("/Applications/Armada.app/Contents/MacOS/Armada");
        assert!(!is_dev_build(Some(shipped), Some(Path::new("/")), &none, &none));
        let started_in_source = |dir: &Path| dir == Path::new("/src/armada");
        assert!(is_dev_build(Some(shipped), Some(Path::new("/src/armada/crates")), &started_in_source, &none));
        assert!(is_dev_build(Some(Path::new("/src/armada/target/debug/bridge-native")), Some(Path::new("/")), &none, &none));
    }
}
