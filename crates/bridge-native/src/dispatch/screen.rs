//! The screen a dispatch is made from: the Jobs screen with the dispatch sheet, the strip a send
//! leaves and the annotation layer over it. It opens the sheet on ⌘N (`new_job`) and on what the
//! title bar asks (`Search`, and `Dispatch` from the narrow menu), and answers the sheet's and the
//! layer's asks. **A walk answers them by scene** (`scene`), the way the title bar's walk changes
//! its face: Fleet is not on the other end, so a step names the answer it wants.

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    canvas, div, prelude::*, AnyWindowHandle, App, Bounds, Context, Entity, EventEmitter, FocusHandle, Global, KeyDownEvent, Pixels, Role, SharedString,
    Subscription, WeakEntity, Window,
};
use ipc::SessionRecord;

use super::face::{Face, Sent};
use super::scenes;
use super::sheet::{Asked, DispatchSheet};
use crate::annotate::{self, Layer, Shoot, Toggle};
use crate::jobs::JobsScreen;
use crate::keymap::Act;
use crate::labels::{labelled, LabelBook};
use crate::look::{token, ActiveLook};
use crate::titlebar::parts::{card, glyph, icon_button, kbd, mono, tip, Btn, Weight};
use crate::titlebar::{self, Asked as BarAsked, AskedSink};

/// The strip a send leaves, hung from the title bar's field.
struct Strip {
    sent: Sent,
    place: Bounds<Pixels>,
    focus: FocusHandle,
}

/// What the screen asks of the window around it: a place this screen does not draw.
#[derive(Clone, Debug, PartialEq)]
pub enum Opened {
    /// The repository's Autopilot lane on the Cockpit, from the strip's Open or its Enter.
    AutopilotLane(SharedString),
}

impl EventEmitter<Opened> for DispatchScreen {}

pub struct DispatchScreen {
    jobs: Entity<JobsScreen>,
    sheet: Entity<DispatchSheet>,
    layer: Option<Entity<Layer>>,
    /// Where the notes are kept, once something says.
    kept_in: Option<std::path::PathBuf>,
    strip: Option<Strip>,
    /// A start Fleet refused, raised as the window's toast.
    toast: Option<SharedString>,
    pub within: FocusHandle,
    area: Rc<Cell<Option<Bounds<Pixels>>>>,
    _subs: Vec<Subscription>,
}

/// The screen in front, for a walk to move between scenes.
struct Live(WeakEntity<DispatchScreen>, AnyWindowHandle);

impl Global for Live {}

pub fn live(cx: &App) -> Option<Entity<DispatchScreen>> {
    cx.try_global::<Live>().and_then(|live| live.0.upgrade())
}

/// ⌘N, ⌥A, ⌥⌘A and ⌥⌘S, from any focus: the keys are bound with no context, so a bar popup, a menu or
/// nothing at all holding the keys does not lose them. Each reaches the screen in front.
pub fn bind(cx: &mut App) {
    cx.on_action(|key: &Act, cx: &mut App| {
        let add = match key.id {
            "new_job" => "",
            "dispatch_after" => "@",
            // Not the screen's: the key goes on to whatever else listens.
            _ => return cx.propagate(),
        };
        reach(cx, move |this, window, cx| this.open_sheet(add, window, cx));
    });
    cx.on_action(|_: &Toggle, cx: &mut App| reach(cx, |this, window, cx| this.toggle_layer(window, cx)));
    cx.on_action(|_: &Shoot, cx: &mut App| reach(cx, |this, window, cx| this.shoot_layer(window, cx)));
}

/// The palette's start row: the sheet opens holding the words typed, a Job's or Autopilot's (`one-bar.tsx`).
pub fn start_with(words: String, autopilot: bool, cx: &mut App) {
    crate::drafts::keep(super::sheet::KEEP_REQUEST, &words, cx);
    reach(cx, move |this, window, cx| {
        this.open_sheet("", window, cx);
        if autopilot {
            this.sheet.update(cx, |sheet, cx| sheet.set_kind(super::face::Kind::Autopilot, window, cx));
        }
    });
}

/// Mod on a point menu: a Mod Job is started, told what was pressed (`mod-job.ts` `startModJob`).
pub fn start_mod(pressed: &crate::labels::Drawn, cx: &mut App) {
    let Some(screen) = live(cx) else { return };
    let jobs = screen.read(cx).jobs.clone();
    let list = jobs.read(cx).list.clone();
    let port = list.read(cx).fleet().and_then(|fleet| fleet.read(cx).held().port());
    let name = crate::point::name_of(pressed);
    let role = crate::capture::role_word(pressed.role).unwrap_or("element");
    let note = ipc::SessionNote {
        said: String::new(),
        address: "bridge-native://jobs".into(),
        note_id: None,
        place: ipc::NotePlace {
            selector: format!("{role} \"{}\"", pressed.name),
            element: ipc::NoteElement { tag: role.to_string(), text: pressed.name.to_string(), label: Some(pressed.name.to_string()) },
            location: "bridge-native://jobs".into(),
            ..ipc::NotePlace::default()
        },
        state: ipc::NoteState::default(),
        crop: None,
        job: Vec::new(),
    };
    let start = ipc::StartModSession { title: Some(format!("Mod {name}")), model: None, effort: None, aim: None, note: Some(note), said: None, from_job: None, from_tab: None };
    crate::jobs::send(port, "/mods/session", ipc::encode(&start).map_err(|why| format!("{why:?}")), cx);
}

/// The deck's `d`: the sheet opens on the draft as it stands.
pub fn open_dispatch(cx: &mut App) {
    reach(cx, move |this, window, cx| this.open_sheet("", window, cx));
}

/// Auto Pilot on a Job's row: the sheet opens on Autopilot, the draft as it stands.
pub fn open_autopilot(cx: &mut App) {
    reach(cx, move |this, window, cx| {
        this.open_sheet("", window, cx);
        this.sheet.update(cx, |sheet, cx| sheet.set_kind(super::face::Kind::Autopilot, window, cx));
    });
}

/// Teach on a Job's row: the Teach box opens on the row's words.
pub fn teach_row(text: String, cx: &mut App) {
    reach(cx, move |this, window, cx| this.jobs.update(cx, |jobs, cx| jobs.teach_row(text, window, cx)));
}

/// Report on a point menu: the annotation layer wakes and begins a note on what is under `at`.
pub fn report_at(at: gpui::Point<Pixels>, cx: &mut App) {
    reach(cx, move |this, window, cx| {
        if let Some(layer) = &this.layer {
            layer.update(cx, |layer, cx| layer.report_at(at, window, cx));
        }
    });
}

/// Run something on the screen in front. A key's handler runs inside a window update, so the work
/// waits until it is over.
fn reach(cx: &mut App, run: impl FnOnce(&mut DispatchScreen, &mut Window, &mut Context<DispatchScreen>) + 'static) {
    let Some(live) = cx.try_global::<Live>() else { return };
    let (me, handle) = (live.0.clone(), live.1);
    cx.defer(move |cx| {
        let _ = handle.update(cx, |_, window, cx| {
            if let Some(me) = me.upgrade() {
                me.update(cx, |this, cx| run(this, window, cx));
            }
        });
    });
}

/// The layer over this screen. `seeded` reads back the notes a walk starts with, which a layer read back from
/// its directory finds there instead.
fn new_layer(area: &Rc<Cell<Option<Bounds<Pixels>>>>, targets: Vec<super::face::Offered>, seeded: bool, cx: &mut Context<DispatchScreen>) -> Entity<Layer> {
    cx.new(|cx| {
        let mut layer = Layer::new(Rc::clone(area), targets, cx);
        layer.screen = Some("Jobs".into());
        if seeded {
            layer.read_again(scenes::saved_notes(), cx);
        }
        layer
    })
}

impl DispatchScreen {
    pub fn new(jobs: Entity<JobsScreen>, records: &[SessionRecord], window: &mut Window, cx: &mut Context<Self>) -> Self {
        crate::drafts::forget_all(cx);
        let face = scenes::rest(records);
        let targets = face.mentions.jobs.clone();
        let sheet = cx.new(|cx| DispatchSheet::new(face, window, cx));
        let area: Rc<Cell<Option<Bounds<Pixels>>>> = Rc::default();
        // The layer is mounted only where Electron's is: a development build.
        annotate::forget_awake(cx);
        let layer = annotate::dev_build().then(|| new_layer(&area, targets, true, cx));
        let mut subs = vec![cx.subscribe_in(&sheet, window, |this, _, asked: &Asked, window, cx| this.sheet_asked(asked, window, cx))];
        if let Some(layer) = &layer {
            subs.push(cx.observe(layer, |_, _, cx| cx.notify()));
        }
        cx.set_global(Live(cx.entity().downgrade(), window.window_handle()));
        // What the title bar asks: the narrow menu's Dispatch opens the sheet. The rest goes to the sink this
        // one stands in front of, the Jobs screen's, which opens its palette on search.
        let handle = window.window_handle();
        let me = cx.entity().downgrade();
        let behind = cx.try_global::<AskedSink>().cloned();
        cx.set_global(AskedSink(Rc::new(move |asked: &BarAsked, cx: &mut App| {
            if matches!(asked, BarAsked::Dispatch) {
                let me = me.clone();
                let _ = handle.update(cx, move |_, window, cx| {
                    if let Some(me) = me.upgrade() {
                        me.update(cx, |this, cx| this.open_sheet("", window, cx));
                    }
                });
            } else if let Some(behind) = &behind {
                (behind.0)(asked, cx);
            }
        })));
        DispatchScreen { jobs, sheet, layer, kept_in: None, strip: None, toast: None, within: cx.focus_handle(), area, _subs: subs }
    }

    /// ⌘N and its other doors: the sheet opens on the draft as it stands, and the strip goes.
    fn open_sheet(&mut self, add: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.strip = None;
        self.toast = None;
        self.sheet.update(cx, |sheet, cx| sheet.show(add, window, cx));
        cx.notify();
    }

    fn sheet_asked(&mut self, _asked: &Asked, _window: &mut Window, cx: &mut Context<Self>) {
        // A start that was refused clears the toast when the next one begins.
        self.toast = None;
        cx.notify();
    }

    /// Keep the layer's notes in `dir`: what stands there is read, and each note is written as it is made.
    pub fn keep_notes(&mut self, dir: std::path::PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.kept_in = Some(dir.clone());
        if let Some(layer) = &self.layer {
            layer.update(cx, |layer, cx| layer.keep_in(dir, window, cx));
        }
    }

    fn toggle_layer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(layer) = &self.layer {
            layer.update(cx, |layer, cx| layer.toggle(window, cx));
        }
    }

    fn shoot_layer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(layer) = &self.layer {
            layer.update(cx, |layer, cx| layer.shoot(window, cx));
        }
    }

    // -- the strip -----------------------------------------------------------------------------

    fn show_strip(&mut self, sent: Sent, window: &mut Window, cx: &mut Context<Self>) {
        let bar = titlebar::live(cx);
        let places = bar.as_ref().is_some_and(|bar| bar.read(cx).face.drawn_places().next().is_some());
        let name = titlebar::TitleBar::search_name(places);
        let search = crate::labels::role_named("button").and_then(|role| {
            cx.try_global::<LabelBook>()
                .and_then(|book| book.find(window.window_handle().window_id(), role, &name))
                .map(|drawn| drawn.bounds)
        });
        // The strip is drawn inside this screen, the search is read in the window's own space.
        let origin = self.area.get().map(|area| area.origin).unwrap_or_default();
        let mut place = search.unwrap_or_default();
        // Beside the places the trigger is an icon: the strip hangs over the whole centre instead, as wide as the field was alone.
        if let Some(centre) = bar.as_ref().filter(|_| places).and_then(|bar| bar.read(cx).centre_bounds()) {
            let wide = cx.look().length(token::PALETTE_WIDTH).max(centre.size.width);
            place.origin.x = centre.origin.x + (centre.size.width - wide) / 2.;
            place.size.width = wide;
        }
        place.origin = place.origin - origin;
        let focus = cx.focus_handle();
        self.strip = Some(Strip { sent, place, focus: focus.clone() });
        // The cursor lands on the strip when it appears.
        window.defer(cx, move |window, cx| window.focus(&focus, cx));
        cx.notify();
    }

    fn dismiss_strip(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.strip = None;
        let keys = self.jobs.read(cx).list.read(cx).focus_handle().clone();
        window.focus(&keys, cx);
        cx.notify();
    }

    /// Open on the strip, or Enter on it: the lane of the repository the Autopilot runs in is asked for, and
    /// the strip, which said what was sent, goes.
    fn open_lane(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(strip) = &self.strip else { return };
        cx.emit(Opened::AutopilotLane(strip.sent.root.clone()));
        self.dismiss_strip(window, cx);
    }

    fn strip_keys(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = &event.keystroke;
        if key.key == "escape" {
            self.dismiss_strip(window, cx);
        } else if key.key == "enter" {
            self.open_lane(window, cx);
        } else if let Some(typed) = key.key_char.as_ref().filter(|typed| typed.chars().count() == 1 && *typed != " ") {
            if !key.modifiers.platform && !key.modifiers.control && !key.modifiers.alt {
                let typed = typed.clone();
                self.open_sheet(&typed, window, cx);
            }
        } else {
            return;
        }
        cx.stop_propagation();
    }

    // -- scenes ----------------------------------------------------------------------------------

    /// Move the screen to a named scene: Fleet's answer to a send, or a state of the sheet's face. A
    /// scene changes data and presses nothing.
    pub fn scene(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        let face = self.sheet.read(cx).face.clone();
        let change: Option<fn(Face) -> Face> = match name {
            "found" => Some(scenes::found),
            "unresolved" => Some(scenes::unresolved),
            "scoped" => Some(scenes::scoped),
            "many" => Some(scenes::many),
            "offline" => Some(scenes::offline),
            "online" => Some(scenes::online),
            "autopilot-read" => Some(scenes::autopilot_read),
            "autopilot-refused" => Some(scenes::autopilot_refused),
            _ => None,
        };
        if name == "waiting" {
            let tagged = self.sheet.read(cx).tagged();
            self.sheet.update(cx, |sheet, cx| sheet.set_face(scenes::waiting(face, tagged), window, cx));
            return Ok(());
        }
        if let Some(change) = change {
            self.sheet.update(cx, |sheet, cx| sheet.set_face(change(face), window, cx));
            // The window tells one story: the title bar's Fleet dot goes with the sheet's.
            match name {
                "offline" => titlebar::scenes::apply("fleet-unreachable", cx)?,
                "online" => titlebar::scenes::apply("fleet-running", cx)?,
                _ => {}
            }
            return Ok(());
        }
        match name {
            "attach" => self.sheet.update(cx, |sheet, cx| sheet.attach("pasted-snippet.txt", cx)),
            "answer-start" | "answer-autopilot" => {
                let sent = self.sheet.update(cx, |sheet, cx| sheet.answered(window, cx));
                if let Some(sent) = sent {
                    self.show_strip(sent, window, cx);
                }
            }
            "refuse-start" => {
                // Fleet answered, so it is running again.
                titlebar::scenes::apply("fleet-running", cx)?;
                self.sheet.update(cx, |sheet, cx| {
                    let face = scenes::online(sheet.face.clone());
                    sheet.set_face(face, window, cx);
                    sheet.refused(cx)
                });
                self.toast = Some("Job not started: the repository has no free slot".into());
            }
            "layer-reload" => {
                // What a window reloaded does: the layer is built again, reads the notes back from where they were
                // kept, and is on again where it was on.
                let dir = std::env::temp_dir().join(format!("bridge-native-walk-notes-{}", std::process::id()));
                if self.kept_in.is_none() {
                    let _ = std::fs::remove_dir_all(&dir);
                    self.keep_notes(dir.clone(), window, cx);
                }
                if let Some(layer) = &self.layer {
                    layer.update(cx, |layer, cx| layer.round_trip_now(cx));
                }
                let targets = self.sheet.read(cx).face.mentions.jobs.clone();
                let layer = new_layer(&self.area, targets, false, cx);
                self._subs.push(cx.observe(&layer, |_, _, cx| cx.notify()));
                self.layer = Some(layer.clone());
                layer.update(cx, |layer, cx| {
                    layer.keep_in(dir, window, cx);
                    layer.round_trip_now(cx);
                });
            }
            "layer-picture" => {
                let layer = self.layer.clone().ok_or("the annotation layer is not in this build")?;
                layer.update(cx, |layer, cx| layer.picture_arrives(cx));
            }
            "layer-sent" | "layer-started" | "layer-refused" => {
                let layer = self.layer.clone().ok_or("the annotation layer is not in this build")?;
                layer.update(cx, |layer, cx| {
                    let title = layer.asked_of();
                    match name {
                        "layer-sent" => layer.sent(&title, false, cx),
                        "layer-started" => layer.sent(&title, true, cx),
                        _ => layer.refused(&title, "The Job has ended", cx),
                    }
                });
            }
            other => return Err(format!("no dispatch scene named {other}")),
        }
        cx.notify();
        Ok(())
    }
}

impl Render for DispatchScreen {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let area = Rc::clone(&self.area);
        let strip = self.strip.as_ref().map(|strip| {
            let sent = &strip.sent;
            let place = strip.place;
            let tipped = sent.kind.word();
            labelled(
                div()
                    .id("dispatch-strip")
                    .track_focus(&strip.focus)
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| this.strip_keys(event, window, cx)))
                    .occlude()
                    .absolute()
                    .left(place.origin.x)
                    .top(place.origin.y)
                    .w(place.size.width)
                    .h(place.size.height)
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .pl(look.length(token::SPACE_4))
                    .pr(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_LG))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_DEFAULT))
                    .focus(|style| style.border_color(look.color(token::ACCENT)))
                    .bg(look.color(token::BG_RAISED))
                    .shadow(look.shadows(token::SHADOW_CARD))
                    .text_color(look.color(token::FG_MUTED))
                    .font_family(mono(look))
                    .text_size(look.length(token::TEXT_XS))
                    .child(div().id("strip-mark").flex_none().tooltip(tip(tipped)).child(glyph(
                        sent.kind.glyph(),
                        look.length(token::SPACE_4),
                        look.color(token::ACCENT),
                    )))
                    .child(div().flex_1().min_w_0().truncate().text_color(look.color(token::FG_DEFAULT)).child(sent.title.clone()))
                    .child(kbd("↵", look).opacity(if strip.focus.is_focused(_window) { 1. } else { 0. }))
                    .child(labelled(
                        Btn::new("strip-open", "Open", Weight::Secondary).draw(look, cx).on_click(cx.listener(|this, _, window, cx| {
                            cx.stop_propagation();
                            this.open_lane(window, cx);
                        })),
                        Role::Button,
                        "Open",
                    ))
                    .child(labelled(
                        icon_button("strip-dismiss", "x", look.color(token::FG_MUTED), look)
                            .tooltip(tip("Dismiss"))
                            .on_click(cx.listener(|this, _, window, cx| {
                                cx.stop_propagation();
                                this.dismiss_strip(window, cx);
                            })),
                        Role::Button,
                        "Dismiss",
                    )),
                Role::Group,
                "Last dispatch",
            )
        });
        let toast = self.toast.clone().map(|text| {
            labelled(
                card(look)
                    .id("dispatch-toast")
                    .absolute()
                    .right(look.length(token::SPACE_6))
                    .bottom(look.length(token::SPACE_6))
                    .max_w(look.length(token::W_TOAST))
                    .p(look.length(token::SPACE_4))
                    .border_color(look.color(token::ERROR_EDGE))
                    .text_size(look.length(token::TEXT_SM))
                    .child(text.clone()),
                Role::Alert,
                text,
            )
        });
        let sheet = self.sheet.read(cx).is_open().then(|| self.sheet.clone());
        div()
            .id("dispatch-screen")
            .track_focus(&self.within)
            // Tab, for what nothing under the screen took: the annotation layer's pins are among the stops.
            .on_key_down(cx.listener(|_, event: &gpui::KeyDownEvent, window, cx| {
                if crate::keymap::traverse(event, window, cx) {
                    cx.stop_propagation();
                }
            }))
            .relative()
            .size_full()
            .child(self.jobs.clone())
            .child(canvas(move |bounds, _, _| area.set(Some(bounds)), |_, _, _, _| {}).absolute().top_0().left_0().size_full())
            .children(strip)
            .children(sheet)
            .children(toast)
            .children(self.layer.clone())
    }
}
