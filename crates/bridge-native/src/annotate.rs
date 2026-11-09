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
//! **Available only in a development build**, as Electron's is ([`dev_build`]).

use std::cell::Cell;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    actions, canvas, div, point, prelude::*, Anchor, App, Bounds, Context, DismissEvent, Entity, EventEmitter, FocusHandle,
    Focusable, FontWeight, KeyBinding, Pixels, Role, SharedString, Subscription, Window,
};

use crate::capture::{self, Capture, CaptureEvent, Held, Words};
use crate::dispatch::face::Offered;
use crate::dropdown::{Dropdown, Entry, Picked};
use crate::labels::labelled;
use crate::look::{token, ActiveLook};
use crate::titlebar::id_of;
use crate::titlebar::parts::{card, glyph, mono, spinner, tip};
use crate::walk::bar::chord;

actions!(annotate, [Toggle]);

/// The key context the screen that hosts the layer sets, so ⌥⌘A reaches it from anywhere on the screen.
pub const HOST: &str = "annotate-host";

/// Where the layer is kept between openings of the screen: the notes survive a toggle.
const KEEP_NOTE: &str = "annotate:note";

/// ⌥⌘A. **Electron reads it by key place in the layer itself** (`Layer.tsx:75-82`); the registry does
/// not list it, so it is restated here until it does. Bound only in a development build.
pub fn bind(cx: &mut App) {
    if dev_build() {
        cx.bind_keys([KeyBinding::new("alt-cmd-a", Toggle, Some(HOST))]);
    }
}

/// Whether this is a development build: not packaged, or a kept build run from a source folder.
///
/// Electron's gate is `!app.isPackaged || process.defaultApp` (`M/dev-build.ts`): the dev copy of the
/// app is a bundle in name, so a bundle started with a source folder still counts. The native
/// reading of the same: the executable is **packaged** when it lives inside a `.app` bundle's
/// `Contents/MacOS`; a development build is any executable that is not (cargo's `target`, a kept
/// build in a folder), **or** a bundle started with its working directory inside a source folder,
/// one that has a `.git` above it. Launched from the Dock the working directory is `/`, so a shipped
/// app is never one.
pub fn dev_build() -> bool {
    let exe = std::env::current_exe().ok();
    let cwd = std::env::current_dir().ok();
    is_dev_build(exe.as_deref(), cwd.as_deref(), &|dir: &Path| dir.join(".git").exists())
}

pub fn is_dev_build(exe: Option<&Path>, cwd: Option<&Path>, has_git: &dyn Fn(&Path) -> bool) -> bool {
    let packaged = exe.is_some_and(|exe| {
        let parts: Vec<_> = exe.components().map(|part| part.as_os_str().to_string_lossy().into_owned()).collect();
        parts.windows(3).any(|three| three[0].ends_with(".app") && three[1] == "Contents" && three[2] == "MacOS")
    });
    !packaged || cwd.is_some_and(|cwd| cwd.ancestors().any(has_git))
}

/// A note written on the screen.
struct Note {
    said: String,
    role: String,
    name: String,
    ring: Bounds<Pixels>,
    /// The window as it was when the note was held: the send burns the ring and number into it.
    #[allow(dead_code)]
    picture: Option<(Arc<image::RgbaImage>, f32)>,
    /// The Job it went to, once Fleet took it.
    sent: Option<SharedString>,
}

/// What the layer asks of the app.
#[derive(Clone, Debug, PartialEq)]
pub enum Asked {
    /// Every unsent note, to a Job; `None` starts a new held Job with the notes' words.
    Send { to: Option<Offered>, notes: Vec<String> },
}

impl EventEmitter<Asked> for Layer {}

struct Said {
    text: SharedString,
    fault: bool,
}

struct Menu {
    dropdown: Entity<Dropdown>,
    _picked: Subscription,
    _dismissed: Subscription,
}

pub struct Layer {
    on: bool,
    /// The screen the layer sits over, where aim and a box may hold.
    area: Rc<Cell<Option<Bounds<Pixels>>>>,
    capture: Option<(Entity<Capture>, Subscription)>,
    notes: Vec<Note>,
    /// The note whose card is open.
    opened: Option<usize>,
    /// The live Jobs the picker lists.
    pub targets: Vec<Offered>,
    menu: Option<Menu>,
    split_at: Rc<Cell<Option<Bounds<Pixels>>>>,
    sending: Option<SharedString>,
    said: Option<Said>,
    keys_were: Option<FocusHandle>,
}

impl Layer {
    pub fn new(area: Rc<Cell<Option<Bounds<Pixels>>>>, targets: Vec<Offered>) -> Layer {
        Layer { on: false, area, capture: None, notes: Vec::new(), opened: None, targets, menu: None, split_at: Rc::default(), sending: None, said: None, keys_were: None }
    }

    pub fn is_on(&self) -> bool {
        self.on
    }

    /// ⌥⌘A: on, and again off. The notes stay for the next time.
    pub fn toggle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.on {
            self.on = false;
            self.capture = None;
            self.menu = None;
            self.opened = None;
            if let Some(keys) = self.keys_were.take() {
                window.focus(&keys, cx);
            }
        } else {
            self.on = true;
            self.keys_were = window.focused(cx);
            self.arm(window, cx);
        }
        cx.notify();
    }

    /// A fresh capture over the screen, with the keys on it.
    fn arm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let words = Words { save: "Save note", band: Some("New note"), card: "New note" };
        let capture = cx.new(|cx| Capture::new(Rc::clone(&self.area), words, Some(KEEP_NOTE.into()), cx));
        let watch = cx.subscribe_in(&capture, window, |this, _, event: &CaptureEvent, window, cx| match event {
            CaptureEvent::Noted(noted) => this.noted(noted, window, cx),
            CaptureEvent::Closed { .. } => this.arm(window, cx),
        });
        let focus = capture.read(cx).focus_handle(cx);
        window.defer(cx, move |window, cx| window.focus(&focus, cx));
        self.capture = Some((capture, watch));
        cx.notify();
    }

    fn noted(&mut self, noted: &capture::Noted, window: &mut Window, cx: &mut Context<Self>) {
        let (role, name) = match &noted.held {
            Held::Element(drawn) => (capture::role_word(drawn.role).unwrap_or("element").to_lowercase(), drawn.name.to_string()),
            Held::Region(_) => ("region".to_string(), "A box you drew".to_string()),
        };
        self.notes.push(Note { said: noted.said.clone(), role, name, ring: noted.held.bounds(), picture: noted.picture.clone(), sent: None });
        self.said = None;
        self.arm(window, cx);
    }

    fn unsent(&self) -> Vec<&Note> {
        self.notes.iter().filter(|note| note.sent.is_none()).collect()
    }

    /// Send every unsent note on this screen: pending in this frame, the split off.
    fn send(&mut self, to: Option<Offered>, cx: &mut Context<Self>) {
        if self.sending.is_some() || self.unsent().is_empty() {
            return;
        }
        let notes = self.unsent().iter().map(|note| note.said.clone()).collect();
        self.sending = Some(to.as_ref().map_or("Starting job".into(), |job| format!("Sending to {}", job.title).into()));
        self.said = None;
        cx.emit(Asked::Send { to, notes });
        cx.notify();
    }

    /// The Job the pending send named, for a mock Fleet that answers whoever was asked.
    pub fn asked_of(&self) -> SharedString {
        self.sending.as_ref().and_then(|word| word.strip_prefix("Sending to ").map(|title| title.to_string().into())).unwrap_or_else(|| "Dispatch sheet notes".into())
    }

    /// Fleet took the notes: each is saved with the Job it went to.
    pub fn sent(&mut self, title: &str, new_job: bool, cx: &mut Context<Self>) {
        self.sending = None;
        for note in self.notes.iter_mut().filter(|note| note.sent.is_none()) {
            note.sent = Some(title.to_string().into());
        }
        let text = if new_job { format!("Job started: {title}") } else { format!("Sent to Job {title}") };
        self.said = Some(Said { text: text.into(), fault: false });
        cx.notify();
    }

    /// Fleet refused: the notes are as they were, and the bar says why.
    pub fn refused(&mut self, title: &str, why: &str, cx: &mut Context<Self>) {
        self.sending = None;
        self.said = Some(Said { text: format!("Notes not sent to \"{title}\": {why}").into(), fault: true });
        cx.notify();
    }

    fn open_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.sending.is_some() || self.menu.is_some() {
            return;
        }
        let Some(from) = self.split_at.get() else { return };
        let entries: Vec<Entry> = self
            .targets
            .iter()
            .map(|job| Entry::Act { id: id_of(format!("job:{}", job.id)), label: job.title.clone(), icon: Some("square-terminal"), danger: false })
            .collect();
        let at = point(from.origin.x + from.size.width, from.origin.y - cx.look().length(token::SPACE_1));
        let dropdown = cx.new(|cx| Dropdown::new("Send to a Job", entries, at, Anchor::BottomRight, false, cx));
        let picked = cx.subscribe_in(&dropdown, window, |this, _, picked: &Picked, window, cx| {
            let Picked::Entry { id, .. } = picked;
            this.menu = None;
            this.refocus(window, cx);
            let job = id.strip_prefix("job:").and_then(|id| this.targets.iter().find(|one| one.id == id)).cloned();
            if job.is_some() {
                this.send(job, cx);
            }
        });
        let dismissed = cx.subscribe_in(&dropdown, window, |this, _, _: &DismissEvent, window, cx| {
            this.menu = None;
            this.refocus(window, cx);
            cx.notify();
        });
        let keys = dropdown.read(cx).focus_handle(cx);
        window.defer(cx, move |window, cx| window.focus(&keys, cx));
        self.menu = Some(Menu { dropdown, _picked: picked, _dismissed: dismissed });
        cx.notify();
    }

    /// The keys go back to the capture after a menu.
    fn refocus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some((capture, _)) = &self.capture {
            let focus = capture.read(cx).focus_handle(cx);
            window.focus(&focus, cx);
        }
    }
}

impl Render for Layer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.on {
            return div().into_any_element();
        }
        let look = cx.look();
        let pins = self.notes.iter().enumerate().map(|(at, note)| {
            let sent = note.sent.is_some();
            let size = look.length(token::H_BADGE);
            let (ground, ink, edge) = if sent {
                (look.color(token::BG_RAISED), look.color(token::ACCENT), look.color(token::ACCENT))
            } else {
                (look.color(token::ACCENT), look.color(token::TEXT_ON_ACCENT), look.color(token::BG_BASE))
            };
            let name = if sent { format!("Note {}, sent: {}", at + 1, note.said) } else { format!("Note {}, open: {}", at + 1, note.said) };
            labelled(
                div()
                    .id(SharedString::from(format!("pin-{at}")))
                    .occlude()
                    .absolute()
                    .left(note.ring.origin.x + note.ring.size.width - size / 2.)
                    .top(note.ring.origin.y - size / 2.)
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
                    .child((at + 1).to_string())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.opened = if this.opened == Some(at) { None } else { Some(at) };
                        cx.notify();
                    })),
                Role::Button,
                name,
            )
        });
        let opened = self.opened.and_then(|at| self.notes.get(at).map(|note| (at, note))).map(|(at, note)| {
            let gap = look.length(token::SPACE_2);
            labelled(
                card(look)
                    .id("note-card")
                    .occlude()
                    .absolute()
                    .left(note.ring.origin.x)
                    .top(note.ring.origin.y + note.ring.size.height + gap)
                    .w(look.length(token::SPACE_8) * 10.)
                    .gap(look.length(token::SPACE_2))
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
                            .child(format!("Note {}", at + 1)),
                    )
                    .child(div().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_MUTED)).child(format!("{} · {}", note.role, note.name)))
                    .child(div().text_size(look.length(token::TEXT_SM)).child(note.said.clone()))
                    .children(note.sent.clone().map(|title| {
                        div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(format!("Sent to Job {title}"))
                    })),
                Role::Dialog,
                "Note",
            )
        });
        div()
            .id("annotate")
            .absolute()
            .inset_0()
            .children(self.capture.as_ref().map(|(capture, _)| capture.clone()))
            .children(pins)
            .children(opened)
            .child(self.bar(cx))
            .children(self.menu.as_ref().map(|menu| menu.dropdown.clone()))
            .into_any_element()
    }
}

impl Layer {
    /// The bar over everything: `Annotating`, what the last send said, the split and the key.
    fn bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let sendable = !self.unsent().is_empty();
        let pending = self.sending.clone();
        let split = sendable.then(|| {
            let accent = look.color(token::ACCENT);
            let ink = look.color(token::TEXT_ON_ACCENT);
            let off = pending.is_some();
            let split_at = Rc::clone(&self.split_at);
            let part = |id: &'static str| {
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
            div()
                .relative()
                .flex()
                .gap(look.length(token::BORDER_WIDTH))
                .child(canvas(move |bounds, _, _| split_at.set(Some(bounds)), |_, _, _, _| {}).absolute().size_full())
                .child(labelled(
                    part("annotate-start")
                        .px(look.length(token::SPACE_4))
                        .gap(look.length(token::SPACE_2))
                        .rounded_l(look.length(token::RADIUS_LG))
                        .text_size(look.length(token::TEXT_XS))
                        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                        .children(off.then(|| spinner("annotate-sending", look.length(token::SPACE_4), ink, cx)))
                        .child(pending.clone().map_or(SharedString::from("Start job"), |word| word))
                        .when(!off, |part| part.on_click(cx.listener(|this, _, _, cx| this.send(None, cx)))),
                    Role::Button,
                    pending.clone().map_or(SharedString::from("Start job"), |word| word),
                ))
                .child(labelled(
                    part("annotate-send-to")
                        .w(look.length(token::H_CONTROL))
                        .rounded_r(look.length(token::RADIUS_LG))
                        .child(glyph("chevron-down", look.length(token::SPACE_3), ink))
                        .when(!off, |part| part.on_click(cx.listener(|this, _, window, cx| this.open_picker(window, cx)))),
                    Role::Button,
                    "Send to a Job",
                ))
        });
        let said = self.said.as_ref().map(|said| {
            labelled(
                div()
                    .id("annotate-said")
                    .text_color(look.color(if said.fault { token::ERROR } else { token::FG_MUTED }))
                    .child(said.text.clone()),
                Role::Status,
                said.text.clone(),
            )
        });
        let key = chord("⌥⌘A", cx);
        div().absolute().left_0().right_0().bottom(look.length(token::SPACE_3)).flex().justify_center().child(labelled(
            div()
                .id("annotate-bar")
                .occlude()
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
                .children(said)
                .children(split)
                .child(div().id("annotate-key").tooltip(tip("Turns annotating off")).child(key)),
            Role::Status,
            "Annotating",
        ))
    }
}
