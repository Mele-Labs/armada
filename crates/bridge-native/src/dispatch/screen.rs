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
use ipc::{SessionRecord, SessionId};

use super::face::{Face, Sent};
use super::live;
use super::scenes;
use super::sheet::{Asked, DispatchSheet};
use crate::annotate::{self, Layer, Shoot, Toggle};
use crate::jobs::JobsScreen;
use crate::keymap::Act;
use crate::labels::{labelled, LabelBook};
use crate::look::{token, ActiveLook};
use crate::titlebar::parts::{glyph, icon_button, kbd, mono, tip, Btn, Weight};
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
    /// The repositories Fleet serves, as the sheet's chips name them, while the window is on a live Fleet.
    served: Vec<live::Served>,
    models: Option<ipc::ModelChoices>,
    models_asked: bool,
    pub within: FocusHandle,
    area: Rc<Cell<Option<Bounds<Pixels>>>>,
    _subs: Vec<Subscription>,
}

/// The project the window is scoped to, by Fleet's id, and the names of the repositories it holds in the
/// order its members name them. `None` on All.
fn scope(cx: &App) -> Option<(SharedString, Vec<SharedString>)> {
    let bar = titlebar::live(cx)?;
    let face = &bar.read(cx).face;
    let project = face.scoped.and_then(|at| face.projects.get(at))?;
    let mut names: Vec<SharedString> = Vec::new();
    for member in &project.members {
        if let Some(one) = face.repositories.get(member.repository) {
            if !names.contains(&one.name) {
                names.push(one.name.clone());
            }
        }
    }
    Some((project.id.clone(), names))
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

/// Auto Pilot on a Job's row: the sheet opens on Autopilot with the row's repository chosen, as
/// Electron picks the row's repository before Autopilot (`Deck.tsx` `onAutopilot`).
pub fn open_autopilot(repository: Option<String>, cx: &mut App) {
    reach(cx, move |this, window, cx| {
        this.open_sheet("", window, cx);
        this.sheet.update(cx, |sheet, cx| {
            sheet.set_kind(super::face::Kind::Autopilot, window, cx);
            if let Some(repository) = &repository {
                sheet.pick_repository(repository, cx);
            }
        });
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
        DispatchScreen { jobs, sheet, layer, kept_in: None, strip: None, served: Vec::new(), models: None, models_asked: false, within: cx.focus_handle(), area, _subs: subs }
    }

    /// ⌘N and its other doors: the sheet opens on the draft as it stands, and the strip goes.
    fn open_sheet(&mut self, add: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.strip = None;
        self.refresh_face(window, cx);
        self.sheet.update(cx, |sheet, cx| sheet.show(add, window, cx));
        cx.notify();
    }

    /// What the sheet asks. On a live Fleet the start is posted and the words are read; on mock data nothing is
    /// on the other end, and a walk answers by scene.
    fn sheet_asked(&mut self, asked: &Asked, window: &mut Window, cx: &mut Context<Self>) {
        if self.jobs.read(cx).list.read(cx).fleet().is_some() {
            match asked {
                Asked::ReadRepository(words) => self.read_repository(words.clone(), window, cx),
                Asked::Start { words, repository, model, effort, held } => {
                    self.start_job(words.clone(), repository, model.clone(), effort.clone(), *held, window, cx)
                }
                // The live sheet offers neither Autopilot, a wait nor the queue.
                _ => {}
            }
        }
        cx.notify();
    }

    fn fleet_port(&self, cx: &App) -> Option<u16> {
        self.jobs.read(cx).list.read(cx).fleet().and_then(|fleet| fleet.read(cx).held().port())
    }

    /// The sheet's face from what Fleet serves, each time it opens: the repositories, narrowed to the
    /// project the window is scoped to, and the Jobs the `@` menu offers.
    fn refresh_face(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(fleet) = self.jobs.read(cx).list.read(cx).fleet() else { return };
        if self.sheet.read(cx).is_open() {
            return;
        }
        let repositories = fleet.read(cx).held().repositories.clone().unwrap_or_default();
        let port = fleet.read(cx).held().port();
        self.served = live::served(&repositories);
        let keep = scope(cx).map(|(_, names)| names);
        let jobs = scenes::live(self.jobs.read(cx).list.read(cx).records());
        let face = live::face_of(&self.served, keep.as_deref(), jobs, port.is_some(), live::tuning(self.models.as_ref()));
        self.sheet.update(cx, |sheet, cx| {
            // What was picked by hand last time does not stand over a face read again.
            sheet.face.reading = super::face::Reading::Idle;
            sheet.set_face(face, window, cx);
        });
        if let (Some(port), false) = (port, self.models_asked) {
            self.models_asked = true;
            cx.spawn(async move |this, cx| {
                let answer = cx.background_executor().spawn(async move { live::call(port, "/models".into(), None) }).await;
                let _ = this.update(cx, |this, cx| match answer.and_then(|bytes| ipc::decode::<ipc::ModelChoices>("ModelChoices", &bytes).map_err(|why| why.to_string())) {
                    Ok(models) => {
                        let tuning = live::tuning(Some(&models));
                        this.models = Some(models);
                        this.sheet.update(cx, |sheet, cx| {
                            sheet.face.tuning = tuning;
                            cx.notify();
                        });
                    }
                    Err(_) => this.models_asked = false,
                });
            })
            .detach();
        }
    }

    /// The words rested: Fleet reads which repository they name, and the chip it names is chosen. Where it names
    /// none, or one the project does not hold, the sheet says so and the person picks.
    fn read_repository(&mut self, words: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(port) = self.fleet_port(cx) else { return };
        let path = format!("/repositories/of_request?q={}", crate::link::percent_encoded(&words));
        cx.spawn_in(window, async move |this, cx| {
            let answer = cx.background_executor().spawn(async move { live::call(port, path, None) }).await;
            let _ = this.update_in(cx, |this, window, cx| this.repository_read(words, answer, window, cx));
        })
        .detach();
    }

    fn repository_read(&mut self, words: String, answer: Result<Vec<u8>, String>, window: &mut Window, cx: &mut Context<Self>) {
        // Words typed since are read by their own ask.
        if self.sheet.read(cx).words(cx) != words {
            return;
        }
        let face = self.sheet.read(cx).face.clone();
        let read = answer.ok().and_then(|bytes| ipc::decode::<ipc::RequestRepository>("RequestRepository", &bytes).ok());
        let named = read
            .and_then(|read| read.manifest_id)
            .and_then(|id| self.served.iter().find(|one| one.id == id))
            .map(|one| one.name.clone())
            .filter(|name| face.chips.iter().chain(&face.more).any(|chip| chip.name == *name));
        let face = live::read(face, named.as_ref());
        self.sheet.update(cx, |sheet, cx| sheet.set_face(face, window, cx));
    }

    /// Start: the words go to Fleet with the repository's Manifest id, the model and effort picked, and the project
    /// the window is scoped to. A Job started opens; a refusal is Fleet's sentence on the sheet, which keeps its words.
    #[allow(clippy::too_many_arguments)]
    fn start_job(&mut self, words: String, repository: &str, model: Option<String>, effort: Option<String>, held: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(port) = self.fleet_port(cx) else {
            self.sheet.update(cx, |sheet, cx| sheet.refused("Fleet is not connected".into(), cx));
            return;
        };
        let Some(manifest_id) = self.served.iter().find(|one| one.name == repository).map(|one| one.id.clone()) else {
            let said = format!("Fleet serves no repository named {repository}");
            self.sheet.update(cx, |sheet, cx| sheet.refused(said.into(), cx));
            return;
        };
        let start = ipc::StartSession {
            manifest_id,
            title: None,
            model,
            effort,
            mode: None,
            fork: None,
            root: None,
            request: Some(words.clone()),
            after: None,
            held,
            project_rows: Vec::new(),
            on_your_words: None,
            project: scope(cx).map(|(id, _)| id.to_string()),
        };
        let body = ipc::encode(&start).map_err(|why| format!("{why:?}"));
        cx.spawn_in(window, async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move { live::call(port, "/sessions/start".into(), Some(body?)) })
                .await;
            let _ = this.update_in(cx, |this, window, cx| this.started(answer, words, port, window, cx));
        })
        .detach();
    }

    fn started(&mut self, answer: Result<Vec<u8>, String>, words: String, port: u16, window: &mut Window, cx: &mut Context<Self>) {
        let record = answer.and_then(|bytes| ipc::decode::<SessionRecord>("SessionRecord", &bytes).map_err(|why| why.to_string()));
        match record {
            Ok(record) => {
                let id = record.id.as_str().to_string();
                self.sheet.update(cx, |sheet, cx| {
                    sheet.answered(window, cx);
                });
                // The words are the Job's first message, as Electron's Start sends them once the Job exists.
                let body = ipc::encode(&ipc::SendSessionMessageBody {
                    message: ipc::SendSessionMessage { session_id: SessionId::carried(id.clone()), text: words, attachments: Vec::new(), mentions: Vec::new() },
                    aim: None,
                })
                .map_err(|why| format!("{why:?}"));
                cx.background_executor()
                    .spawn(async move {
                        if let Err(why) = body.and_then(|body| live::call(port, "/sessions/message".into(), Some(body))) {
                            crate::log::Note::warn("a started Job was not given its words").field("why", why).emit();
                        }
                    })
                    .detach();
                let list = self.jobs.read(cx).list.clone();
                list.update(cx, |list, cx| list.open(id.into(), cx));
            }
            Err(why) => self.sheet.update(cx, |sheet, cx| sheet.refused(why.into(), cx)),
        }
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
                    sheet.refused("Job not started: the repository has no free slot".into(), cx)
                });
            }
            "storefront" => {
                // The window is scoped to the project that holds two repositories: the picker offers those.
                if let Some(bar) = titlebar::live(cx) {
                    bar.update(cx, |bar, cx| {
                        bar.face.scoped = Some(1);
                        cx.notify();
                    });
                }
                let keep = scope(cx).map(|(_, names)| names).ok_or("the title bar holds no project to scope to")?;
                let mut face = live::narrowed(face, &keep);
                face.refused = None;
                self.sheet.update(cx, |sheet, cx| sheet.set_face(face, window, cx));
            }
            "refuse-member" => {
                // Fleet's 404 for a start whose project was removed after the window was scoped to it.
                self.sheet.update(cx, |sheet, cx| {
                    sheet.refused("there is no project storefront".into(), cx)
                });
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
            .children(self.layer.clone())
    }
}
