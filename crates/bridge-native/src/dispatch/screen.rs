//! The screen a dispatch is made from: the Jobs screen with the dispatch sheet and the annotation
//! layer over it. It opens the sheet on ⌘N (`new_job`) and on what the
//! title bar asks (`Search`, and `Dispatch` from the narrow menu), and answers the sheet's and the
//! layer's asks. **A walk answers them by scene** (`scene`), the way the title bar's walk changes
//! its face: Fleet is not on the other end, so a step names the answer it wants.

use std::cell::Cell;
use std::rc::Rc;

use std::time::Duration;

use gpui::{
    canvas, div, prelude::*, AnyView, AnyWindowHandle, App, Bounds, Context, Entity, FocusHandle, Global, Pixels, SharedString,
    Subscription, Task, WeakEntity, Window,
};
use ipc::{SessionId, SessionRecord, SessionTag, TagKind};

use super::face::Face;
use super::live;
use super::scenes;
use super::sheet::{Asked, DispatchSheet};
use crate::annotate::{self, Layer, Shoot, Toggle};
use crate::jobs::JobsScreen;
use crate::keymap::Act;
use crate::terminal::shells::{Place, Start};
use crate::titlebar::{self, Asked as BarAsked, AskedSink};

pub struct DispatchScreen {
    jobs: Entity<JobsScreen>,
    /// What the sheet stands over: the Jobs screen, or the Operator's page that holds it.
    under: AnyView,
    sheet: Entity<DispatchSheet>,
    /// The New project sheet, which the title bar's switcher opens over the window.
    new_project: Entity<crate::new_project::NewProjectSheet>,
    layer: Option<Entity<Layer>>,
    /// Where the notes are kept, once something says.
    kept_in: Option<std::path::PathBuf>,
    /// The repositories Fleet serves, as the sheet's chips name them, while the window is on a live Fleet.
    served: Vec<live::Served>,
    models: Option<ipc::ModelChoices>,
    models_asked: bool,
    /// What Fleet last read of the words that tag a Job, and the reads out for the repository and for the wait.
    wait_read: Option<WaitRead>,
    read_task: Option<Task<()>>,
    wait_task: Option<Task<()>>,
    pub within: FocusHandle,
    area: Rc<Cell<Option<Bounds<Pixels>>>>,
    _subs: Vec<Subscription>,
}

/// The words and picks a Job's Start is made from.
#[derive(Clone)]
struct Begin {
    words: String,
    repository: String,
    model: Option<String>,
    effort: Option<String>,
    held: bool,
}

/// Fleet's reading of the words that tag `job`, as it came: the sentence is drawn from it and the hand over it.
struct WaitRead {
    job: String,
    words: String,
    reading: ipc::WaitReading,
}

/// How long the words stay still before the repository is read, and before the wait is.
const READ_REPOSITORY_AFTER: Duration = Duration::from_millis(250);
const READ_WAIT_AFTER: Duration = Duration::from_millis(400);

/// What the window is scoped to: a project, by Fleet's id, with the names of the repositories it holds in the
/// order its members name them, or one repository in no project, with no id. `None` on All.
pub(crate) fn scope(cx: &App) -> Option<(Option<SharedString>, Vec<SharedString>)> {
    let bar = titlebar::live(cx)?;
    let face = &bar.read(cx).face;
    let Some(project) = face.scoped.and_then(|at| face.projects.get(at)) else {
        return face.chosen.and_then(|at| face.repositories.get(at)).map(|one| (None, vec![one.name.clone()]));
    };
    let mut names: Vec<SharedString> = Vec::new();
    for member in &project.members {
        if let Some(one) = face.repositories.get(member.repository) {
            if !names.contains(&one.name) {
                names.push(one.name.clone());
            }
        }
    }
    Some((Some(project.id.clone()), names))
}

/// Where ⌘T would start a Job, as the card under the sheet names it: the project the title bar is scoped
/// to, else its repository, else the place one last started. None where ⌘T cannot start one.
fn here_name(cx: &App) -> Option<SharedString> {
    if !crate::terminal::shells::starts(cx) {
        return None;
    }
    let bar = titlebar::live(cx);
    let named = bar.and_then(|bar| {
        let face = &bar.read(cx).face;
        match face.scoped.and_then(|at| face.projects.get(at)) {
            Some(project) => Some(project.name.clone()),
            None => face.chosen.and_then(|at| face.repositories.get(at)).map(|one| one.name.clone()),
        }
    });
    named.or_else(|| {
        crate::terminal::start::kept_place(cx).map(|place| match place {
            Place::Repository(name) => name.into(),
            Place::Folder(path) => std::path::Path::new(&path).file_name().map_or(path.clone(), |name| name.to_string_lossy().into_owned()).into(),
        })
    })
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
        if key.is("new_terminal_job") {
            if !crate::terminal::shells::starts(cx) {
                return;
            }
            return reach(cx, |this, window, cx| {
                let picked = this.sheet.read(cx).is_open().then(|| this.sheet.read(cx).named_repository()).flatten();
                this.new_terminal(picked, window, cx)
            });
        }
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

/// The palette's start row: the sheet opens holding the words typed (`one-bar.tsx`).
pub fn start_with(words: String, cx: &mut App) {
    crate::drafts::keep(super::sheet::KEEP_REQUEST, &words, cx);
    reach(cx, move |this, window, cx| {
        this.open_sheet("", window, cx);
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

/// A red main's Dispatch a new Job: the sheet opens on the brief, with its repository chosen
/// (`useFixChoice`, `merge-line.tsx`).
pub fn open_drafted(request: String, repository: Option<String>, cx: &mut App) {
    reach(cx, move |this, window, cx| {
        this.open_sheet(&request, window, cx);
        if let Some(repository) = &repository {
            this.sheet.update(cx, |sheet, cx| sheet.pick_repository(repository, cx));
        }
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

/// The switcher's New project: its sheet opens over the window, on what was typed last time.
pub fn open_new_project(cx: &mut App) {
    reach(cx, |this, window, cx| this.new_project.update(cx, |sheet, cx| sheet.show(window, cx)));
}

/// Run something on the screen in front. A key's handler runs inside a window update, so the work
/// waits until it is over.
/// Start a queued idea from its row in the Jobs list, through the screen that knows the repositories.
pub fn start_queued(idea: crate::queued::QueuedIdea, cx: &mut App) {
    reach(cx, move |this, window, cx| this.start_queued(idea, window, cx));
}

/// How a send of notes ended: into a Job, a new Job with the notes sent, or a dispatch the Operator holds.
enum Sent {
    Into { session: String, title: String },
    Started { session: String, title: String },
    Kept { dispatch: String, title: String },
}

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
        let face = scenes::rest(records);
        let targets = face.mentions.jobs.clone();
        let sheet = cx.new(|cx| DispatchSheet::new(face, window, cx));
        let area: Rc<Cell<Option<Bounds<Pixels>>>> = Rc::default();
        // The layer is mounted only where Electron's is: a development build.
        annotate::forget_awake(cx);
        let layer = annotate::dev_build().then(|| new_layer(&area, targets, true, cx));
        // On a live Fleet the New project sheet sends to it; a walk's has the mock's answers.
        let wire = match jobs.read(cx).list.read(cx).fleet() {
            Some(fleet) => crate::new_project::wire::Wire::Live(fleet),
            None => crate::new_project::wire::Wire::Mock(Box::new(crate::new_project::wire::Mock::new())),
        };
        let new_project = cx.new(|cx| crate::new_project::NewProjectSheet::new(wire, window, cx));
        let mut subs = vec![
            cx.subscribe_in(&sheet, window, |this, _, asked: &Asked, window, cx| this.sheet_asked(asked, window, cx)),
            cx.subscribe_in(&new_project, window, |this, _, made: &crate::new_project::Made, window, cx| this.project_made(made, window, cx)),
        ];
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
        DispatchScreen { under: jobs.clone().into(), jobs, sheet, new_project, layer, kept_in: None, served: Vec::new(), models: None, models_asked: false, wait_read: None, read_task: None, wait_task: None, within: cx.focus_handle(), area, _subs: subs }
    }

    /// The sheet over another screen that holds the Jobs screen it reads, as the Operator's page does.
    pub fn over(mut self, under: AnyView) -> Self {
        self.under = under;
        self
    }

    /// ⌘N and its other doors: the sheet opens on the draft as it stands.
    fn open_sheet(&mut self, add: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.refresh_face(window, cx);
        let here = here_name(cx);
        self.sheet.update(cx, |sheet, cx| sheet.set_here(here, cx));
        self.sheet.update(cx, |sheet, cx| sheet.show(add, window, cx));
        cx.notify();
    }

    /// What the sheet asks. On a live Fleet the acts are posted and the words are read; on mock data nothing is
    /// on the other end, and a walk answers by scene.
    fn sheet_asked(&mut self, asked: &Asked, window: &mut Window, cx: &mut Context<Self>) {
        // The card under the sheet: what ⌘T starts.
        if let Asked::JobHere(picked) = asked {
            self.new_terminal(picked.clone(), window, cx);
            return;
        }
        // Queue for later is kept here, on a live Fleet or not.
        if let Asked::Queue { words, repository, model, effort } = asked {
            self.queue_idea(words, repository, model.clone(), effort.clone(), window, cx);
            cx.notify();
            return;
        }
        if self.jobs.read(cx).list.read(cx).fleet().is_some() {
            match asked {
                Asked::ReadRepository(words) => self.read_repository(words.clone(), window, cx),
                Asked::ReadWait { job, words } => self.read_wait(job.clone(), words.clone(), window, cx),
                Asked::Start { words, repository, model, effort, held } => {
                    let begin = Begin { words: words.clone(), repository: repository.clone(), model: model.clone(), effort: effort.clone(), held: *held };
                    self.start_job(begin, window, cx)
                }
                Asked::Queue { .. } | Asked::JobHere(_) | Asked::Closed => {}
            }
        }
        cx.notify();
    }

    /// ⌘T, New Job here: a Job where the window is, else where the last one started, with no sheet. With
    /// neither, the sheet opens to be told where.
    /// A repository the sheet's picker named starts it there; none, where the title bar is.
    fn new_terminal(&mut self, picked: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        // ⌘T over the sheet does what its card does: the sheet shuts first.
        if self.sheet.read(cx).is_open() && here_name(cx).is_some() {
            self.sheet.update(cx, |sheet, cx| sheet.close(window, cx));
        }
        let here = picked.map(Place::Repository).or_else(|| scope(cx).and_then(|(_, names)| names.first().map(|name| Place::Repository(name.to_string()))));
        match here.or_else(|| crate::terminal::start::kept_place(cx)) {
            Some(place) => self.start_terminal(Start { place, size: crate::terminal::start::size(cx), project: None }, None, window, cx),
            None => self.open_sheet("", window, cx),
        }
    }

    /// A New project was made: the window goes to it, and a Job starts in its first repository as ⌘T starts one,
    /// with the line that says how its work lands over the box.
    fn project_made(&mut self, made: &crate::new_project::Made, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(bar) = titlebar::live(cx) {
            bar.update(cx, |bar, cx| bar.made_project(&made.served, &made.projects, &made.project, cx));
        }
        let Some(repository) = made.repository.clone() else { return };
        let start = Start { place: Place::Repository(repository), size: crate::terminal::start::size(cx), project: Some(made.project.clone()) };
        self.start_terminal(start, made.lands.clone(), window, cx);
    }

    /// Start Fleet's quick Job and open it on its thread, the keys in its message box. A refusal is a toast.
    /// `lands` is the line over its box, for a New project's first Job.
    fn start_terminal(&mut self, start: Start, lands: Option<SharedString>, window: &mut Window, cx: &mut Context<Self>) {
        let place = start.place.clone();
        let mut asked = start;
        // A repository goes to Fleet as its Manifest's id; a walk's has none and keeps the name.
        if let Place::Repository(name) = &place {
            if let Some(id) = self.served.iter().find(|one| one.name == *name).map(|one| one.id.clone()) {
                asked.place = Place::Repository(id.as_str().to_string());
            }
        }
        let task = crate::terminal::start::start(asked, cx);
        cx.spawn_in(window, async move |this, cx| {
            let answer = task.await;
            let _ = this.update_in(cx, |this, window, cx| {
                match answer {
                    Ok(started) => {
                        crate::terminal::start::keep_place(&place, cx);
                        if let Some(line) = lands {
                            crate::new_project::lands_on(&started.job, line, cx);
                        }
                        let list = this.jobs.read(cx).list.clone();
                        list.update(cx, |list, cx| list.open(started.job.clone().into(), cx));
                        // After the Job comes on screen, so the box that takes the keys is its own.
                        window.defer(cx, |window, cx| {
                            let Some(view) = crate::job_view::JobView::live(cx) else { return };
                            view.update(cx, |view, cx| view.onto_thread(window, cx));
                        });
                    }
                    Err(why) => crate::toasts::tell(why, cx),
                }
                cx.notify();
            });
        })
        .detach();
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
        // Each rest of the words replaces the read before it, which is let go.
        self.read_task = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(READ_REPOSITORY_AFTER).await;
            let answer = cx.background_executor().spawn(async move { live::call(port, path, None) }).await;
            let _ = this.update_in(cx, |this, window, cx| this.repository_read(words, answer, window, cx));
        }));
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

    /// The words tag a Job: Fleet's proposer reads them for the wait they name, once the words have rested, and the
    /// sentence over the request is drawn from the answer (`readingWait`, `sessions-wired.tsx`).
    fn read_wait(&mut self, job: String, words: String, window: &mut Window, cx: &mut Context<Self>) {
        if self.wait_read.as_ref().is_some_and(|read| read.job == job && read.words == words) {
            return;
        }
        self.ask_wait(job, words, READ_WAIT_AFTER, None, window, cx);
    }

    /// `POST /sessions/read_wait`. `then` is a Start waiting on the reading: a Job is not started on the
    /// defaults while the proposer is still out.
    fn ask_wait(&mut self, job: String, words: String, after: Duration, then: Option<Begin>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(port) = self.fleet_port(cx) else { return };
        let body = ipc::encode(&ipc::ReadSessionWait { session_id: SessionId::carried(job.clone()), request: words.clone() }).map_err(|why| format!("{why:?}"));
        self.wait_task = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(after).await;
            let answer = cx.background_executor().spawn(async move { live::call_model(port, "/sessions/read_wait".into(), body?) }).await;
            let _ = this.update_in(cx, |this, window, cx| this.wait_answered(job, words, answer, then, window, cx));
        }));
    }

    /// A reading that cannot be had is the defaults, as Electron holds it: the Job ends, from main, asking nothing.
    fn wait_answered(&mut self, job: String, words: String, answer: Result<Vec<u8>, String>, then: Option<Begin>, window: &mut Window, cx: &mut Context<Self>) {
        // Words typed since are read by their own ask.
        if self.sheet.read(cx).words(cx) != words {
            return;
        }
        let Some(tagged) = self.sheet.read(cx).tagged().filter(|tagged| tagged.id == job) else { return };
        let read = answer.ok().and_then(|bytes| ipc::decode::<ipc::WaitReading>("WaitReading", &bytes).ok()).unwrap_or_else(live::defaults);
        // A tag that names no condition only refers to the Job: nothing waits.
        let wait = (!read.plain).then(|| {
            let sheet = self.sheet.read(cx);
            let others = sheet.face.mentions.jobs.iter().filter(|other| other.id != tagged.id).take(4).cloned().collect();
            let was = self.wait_read.as_ref().filter(|was| was.job == job).map(|was| &was.reading);
            let hand = sheet.wait.as_ref().zip(was);
            live::wait_of(tagged, others, &read, hand)
        });
        self.wait_read = Some(WaitRead { job, words, reading: read });
        self.sheet.update(cx, |sheet, cx| sheet.set_wait(wait, cx));
        if let Some(begin) = then {
            self.start_job(begin, window, cx);
        }
        cx.notify();
    }

    /// Start: the words go to Fleet with the repository's Manifest id, the model and effort picked, the project the
    /// window is scoped to, and the wait the words name. A Job started opens; a refusal is Fleet's sentence on the
    /// sheet, which keeps its words.
    fn start_job(&mut self, begin: Begin, window: &mut Window, cx: &mut Context<Self>) {
        let Some(port) = self.fleet_port(cx) else {
            self.sheet.update(cx, |sheet, cx| sheet.refused("Fleet is not connected".into(), cx));
            return;
        };
        let Some(manifest_id) = self.served.iter().find(|one| one.name == begin.repository).map(|one| one.id.clone()) else {
            let said = format!("Fleet serves no repository named {}", begin.repository);
            self.sheet.update(cx, |sheet, cx| sheet.refused(said.into(), cx));
            return;
        };
        let tagged = self.sheet.read(cx).tagged();
        if let Some(tagged) = &tagged {
            let settled = self.wait_read.as_ref().is_some_and(|read| read.job == tagged.id && read.words == begin.words);
            if !settled {
                self.ask_wait(tagged.id.to_string(), begin.words.clone(), Duration::ZERO, Some(begin), window, cx);
                return;
            }
        }
        let plain = self.wait_read.as_ref().is_some_and(|read| read.reading.plain);
        let after = tagged.as_ref().filter(|_| !plain).and_then(|_| self.sheet.read(cx).wait.as_ref().map(live::after_of));
        let mentions: Vec<SessionTag> =
            tagged.filter(|_| plain).map(|tagged| SessionTag { kind: TagKind::Session, id: tagged.id.to_string(), title: tagged.title.to_string(), url: None }).into_iter().collect();
        // A wait's request is the brief it starts on, and nothing is sent to it.
        let send = after.is_none();
        let start = ipc::StartSession {
            manifest_id,
            title: None,
            model: begin.model,
            effort: begin.effort,
            mode: None,
            fork: None,
            root: None,
            request: Some(begin.words.clone()),
            held: begin.held && send,
            after,
            project_rows: Vec::new(),
            on_your_words: None,
            project: scope(cx).and_then(|(id, _)| id).map(|id| id.to_string()),
        };
        // His own work goes to Fleet as a dispatch: with the Operator on it may rule on it, and what follows the
        // start (his words as the first message) goes in `then`, so nothing is lost whoever starts the Job.
        let then = if send {
            vec![ipc::DispatchThen::Message(ipc::DispatchMessage { text: begin.words.clone(), attachments: Vec::new(), mentions, sketches: Vec::new() })]
        } else {
            Vec::new()
        };
        let body = ipc::encode(&ipc::DispatchJob { start, kind: None, then }).map_err(|why| format!("{why:?}"));
        let words = begin.words;
        cx.spawn_in(window, async move |this, cx| {
            let answer = cx.background_executor().spawn(async move { live::call(port, "/sessions/dispatch".into(), Some(body?)) }).await;
            let _ = this.update_in(cx, |this, window, cx| this.dispatched(answer, &words, window, cx));
        })
        .detach();
    }

    /// The repositories a queued idea is kept and started against: Fleet's, else the walks' own.
    fn served_now(&self, cx: &App) -> Vec<live::Served> {
        if self.jobs.read(cx).list.read(cx).fleet().is_some() {
            let repositories = self.jobs.read(cx).list.read(cx).fleet().and_then(|fleet| fleet.read(cx).held().repositories.clone()).unwrap_or_default();
            live::served(&repositories)
        } else {
            live::served(&crate::mock::repositories())
        }
    }

    /// Queue for later (`queueSession`): the words, the repository's root and name, the model and effort picked,
    /// saved on this machine; the sheet clears and goes, and nothing starts.
    fn queue_idea(&mut self, words: &str, repository: &str, model: Option<String>, effort: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(root) = self.served_now(cx).into_iter().find(|one| one.name == repository).map(|one| one.root) else {
            let said = format!("Fleet serves no repository named {repository}");
            self.sheet.update(cx, |sheet, cx| sheet.refused(said.into(), cx));
            return;
        };
        let idea = crate::queued::QueuedIdea { id: crate::queued::new_id(), text: words.to_string(), root, repository: repository.to_string(), model, effort };
        crate::queued::add(idea, cx);
        self.sheet.update(cx, |sheet, cx| {
            sheet.answered(window, cx);
        });
    }

    /// Start a queued idea from its row (`startingQueued`): a plain start in its repository with its model and
    /// effort, its words the brief and the first message. It leaves the queue only once the Job exists, which opens;
    /// a refusal keeps it. A second press while the first is out is not a second Job.
    fn start_queued(&mut self, idea: crate::queued::QueuedIdea, window: &mut Window, cx: &mut Context<Self>) {
        let Some(port) = self.fleet_port(cx) else {
            crate::log::Note::warn("a queued idea did not start").field("why", "Fleet is not connected").emit();
            return;
        };
        let served = self.served_now(cx);
        let Some(manifest_id) = served.iter().find(|one| one.root == idea.root).or_else(|| served.iter().find(|one| one.name == idea.repository)).map(|one| one.id.clone()) else {
            crate::log::Note::warn("a queued idea did not start").field("why", format!("Fleet serves no repository at {}", idea.root)).emit();
            return;
        };
        if !crate::queued::set_starting(&idea.id, true, cx) {
            return;
        }
        let start = ipc::StartSession {
            manifest_id,
            title: None,
            model: idea.model.clone(),
            effort: idea.effort.clone(),
            mode: None,
            fork: None,
            root: None,
            request: Some(idea.text.clone()),
            held: false,
            after: None,
            project_rows: Vec::new(),
            on_your_words: None,
            project: scope(cx).and_then(|(id, _)| id).map(|id| id.to_string()),
        };
        // His idea is his work: it goes as a dispatch, unheld, its words the first message, answered as the sheet's Start is.
        let then = vec![ipc::DispatchThen::Message(ipc::DispatchMessage { text: idea.text.clone(), attachments: Vec::new(), mentions: Vec::new(), sketches: Vec::new() })];
        let body = ipc::encode(&ipc::DispatchJob { start, kind: None, then }).map_err(|why| format!("{why:?}"));
        cx.spawn_in(window, async move |this, cx| {
            let answer = cx.background_executor().spawn(async move { live::call(port, "/sessions/dispatch".into(), Some(body?)) }).await;
            let _ = this.update_in(cx, |this, _window, cx| {
                match answer.and_then(|bytes| ipc::decode::<ipc::JobDispatched>("JobDispatched", &bytes).map_err(|why| why.to_string())) {
                    // Fleet took it, whoever starts the Job: it leaves the queue. A Job started opens; with the Operator
                    // holding it, or keeping it as an idea, he stays where he was and the toast says what became of it.
                    Ok(done) => {
                        crate::queued::remove(&idea.id, cx);
                        match done.session {
                            Some(record) => {
                                let list = this.jobs.read(cx).list.clone();
                                list.update(cx, |list, cx| list.open(record.id.as_str().to_string().into(), cx));
                            }
                            None => {
                                if let Some(id) = done.operator.as_deref().or(done.parked.as_deref()) {
                                    crate::toasts::dispatch_held(id, &idea.text, cx);
                                }
                            }
                        }
                    }
                    Err(why) => {
                        crate::queued::set_starting(&idea.id, false, cx);
                        crate::log::Note::warn("a queued idea did not start").field("why", why).emit();
                    }
                }
            });
        })
        .detach();
    }

    /// Fleet answered the dispatch. A Job started opens; with the Operator holding it, or keeping it as an idea, the
    /// sheet goes and he stays where he was: the toast `operator.dispatched` raises says what became of it.
    fn dispatched(&mut self, answer: Result<Vec<u8>, String>, words: &str, window: &mut Window, cx: &mut Context<Self>) {
        let done = answer.and_then(|bytes| ipc::decode::<ipc::JobDispatched>("JobDispatched", &bytes).map_err(|why| why.to_string()));
        match done {
            Ok(ipc::JobDispatched { session: Some(record), .. }) => {
                let id = record.id.as_str().to_string();
                self.sheet.update(cx, |sheet, cx| {
                    sheet.answered(window, cx);
                });
                let list = self.jobs.read(cx).list.clone();
                list.update(cx, |list, cx| list.open(id.into(), cx));
            }
            Ok(held) => {
                if let Some(id) = held.operator.as_deref().or(held.parked.as_deref()) {
                    crate::toasts::dispatch_held(id, words, cx);
                }
                self.sheet.update(cx, |sheet, cx| {
                    sheet.answered(window, cx);
                });
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

    /// The live app's notes: the walk's made-up ones go, and the layer keeps its notes where Classic keeps
    /// them (`.armada/annotations` in the tree this build came from), so either app lists the other's.
    pub fn live_notes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(layer) = self.layer.clone() else { return };
        layer.update(cx, |layer, cx| layer.forget_seeded(cx));
        match crate::annotate::store::dir() {
            Some(dir) => self.keep_notes(dir, window, cx),
            None => crate::toasts::tell("Notes are not saved: no repository found for this build", cx),
        }
        // The picker lists the live Jobs Fleet hosts, as Classic's does, kept as the list changes.
        let list = self.jobs.read(cx).list.clone();
        let targets_of = |records: &[SessionRecord]| -> Vec<super::face::Offered> {
            records
                .iter()
                .filter(|record| record.state == ipc::SessionState::Live && (record.hosted.is_some() || record.terminal.is_some()))
                .map(|record| {
                    let id = record.id.as_str().to_string();
                    let title = crate::jobs::titled(record.title.as_deref());
                    super::face::Offered { id: id.into(), title: title.into() }
                })
                .collect()
        };
        let targets = targets_of(list.read(cx).records());
        layer.update(cx, |layer, cx| layer.set_targets(targets, cx));
        let watched = layer.clone();
        self._subs.push(cx.observe(&list, move |_, list, cx| {
            let targets = targets_of(list.read(cx).records());
            watched.update(cx, |layer, cx| layer.set_targets(targets, cx));
        }));
        self._subs.push(cx.subscribe_in(&layer, window, |this, layer, asked: &annotate::Asked, window, cx| this.send_notes(layer.clone(), asked, window, cx)));
    }

    /// A send from the layer, posted to the live Fleet: into the Job picked, or a new held Job started with the
    /// notes' words, as Classic's. The Job opens once Fleet took them; a refusal is said in the bar and a toast.
    fn send_notes(&mut self, layer: Entity<Layer>, asked: &annotate::Asked, window: &mut Window, cx: &mut Context<Self>) {
        let annotate::Asked::Send { to, notes } = asked;
        let Some(port) = self.fleet_port(cx) else {
            let title = to.as_ref().map_or_else(|| "a new Job".to_string(), |job| job.title.to_string());
            layer.update(cx, |layer, cx| layer.refused(&title, "Fleet is not connected", cx));
            return;
        };
        let manifest = match to {
            Some(_) => None,
            None => {
                let repositories = self.jobs.read(cx).list.read(cx).fleet().and_then(|fleet| fleet.read(cx).held().repositories.clone()).unwrap_or_default();
                let served = live::served(&repositories);
                let dir = crate::annotate::store::dir();
                let here = served.iter().find(|one| dir.as_ref().is_some_and(|dir| dir.starts_with(&one.root))).or(served.first());
                match here {
                    Some(one) => Some(one.id.clone()),
                    None => {
                        layer.update(cx, |layer, cx| layer.refused("a new Job", "Fleet serves no repository to start it in", cx));
                        return;
                    }
                }
            }
        };
        let look = crate::look::ActiveLook::look(&**cx);
        let bytes = |name| {
            let rgba = look.color(name).to_rgb();
            [rgba.r, rgba.g, rgba.b, rgba.a].map(|part| (part * 255.).round() as u8)
        };
        let ink = annotate::live::Ink(bytes(crate::look::token::ACCENT), bytes(crate::look::token::TEXT_ON_ACCENT));
        let outgoing = layer.read(cx).outgoing();
        let to = to.clone();
        let words = notes.join("\n");
        let list = self.jobs.read(cx).list.clone();
        let task = cx.background_spawn(async move {
            match (to, manifest) {
                (Some(job), _) => {
                    let title = job.title.to_string();
                    annotate::live::deliver(port, job.id.as_ref(), outgoing, ink).map_err(|why| (title.clone(), why))?;
                    Ok::<_, (String, String)>(Sent::Into { session: job.id.to_string(), title })
                }
                (None, Some(manifest)) => match annotate::live::dispatch(port, manifest, words, outgoing, ink).map_err(|why| ("a new Job".to_string(), why))? {
                    annotate::live::Dispatched::Started { id, title } => Ok(Sent::Started { session: id, title }),
                    annotate::live::Dispatched::Kept { id, title } => Ok(Sent::Kept { dispatch: id, title }),
                },
                (None, None) => Err(("a new Job".to_string(), "no repository".to_string())),
            }
        });
        cx.spawn_in(window, async move |_, cx| {
            let answer = task.await;
            let _ = cx.update(|_, cx| match answer {
                Ok(Sent::Into { session, title }) => {
                    layer.update(cx, |layer, cx| {
                        layer.sending_to(session.clone().into());
                        layer.sent(&title, false, cx);
                    });
                    list.update(cx, |list, cx| list.open(session.into(), cx));
                }
                Ok(Sent::Started { session, title }) => {
                    layer.update(cx, |layer, cx| {
                        layer.sending_to(session.clone().into());
                        layer.sent(&title, true, cx);
                    });
                    list.update(cx, |list, cx| list.open(session.into(), cx));
                }
                // The Operator holds the work: the notes are sent, and he stays where he was; the toast says what became of it.
                Ok(Sent::Kept { dispatch, title }) => {
                    crate::toasts::dispatch_held(&dispatch, &title, cx);
                    layer.update(cx, |layer, cx| layer.sent(&title, true, cx));
                }
                Err((title, why)) => {
                    layer.update(cx, |layer, cx| layer.refused(&title, &why, cx));
                    crate::toasts::tell(format!("Notes not sent to \"{title}\": {why}"), cx);
                }
            });
        })
        .detach();
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
            "answer-start" => self.sheet.update(cx, |sheet, cx| sheet.answered(window, cx)),
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
            "waiting-live" => {
                // What a live Fleet reads of a tag: a task of the Job's plan, from its PR branch.
                let tagged = self.sheet.read(cx).tagged().or_else(|| face.mentions.jobs.first().cloned()).ok_or("no Job to wait on")?;
                let others = face.mentions.jobs.iter().filter(|other| other.id != tagged.id).take(4).cloned().collect();
                let read = ipc::WaitReading {
                    condition: ipc::StartCondition::TaskDone { task: "t3".into() },
                    base: ipc::StartBase::Branch,
                    needs: "the schema change and the seed data".into(),
                    plain: false,
                };
                let wait = live::wait_of(tagged, others, &read, None);
                // A reading follows words typed, and typing has cleared any refusal before it.
                self.sheet.update(cx, |sheet, cx| {
                    sheet.face.refused = None;
                    sheet.set_wait(Some(wait), cx)
                });
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
            "answer-start-job" | "answer-draft" => {
                // Fleet's yes to a start from the sheet: a Start runs at once and its Job opens working, a Draft opens
                // held on its start card. The sheet goes with the words it held.
                let held = name == "answer-draft";
                let (words, repository) = self.sheet.read(cx).started(cx).ok_or("the sheet holds no start to answer")?;
                self.sheet.update(cx, |sheet, cx| sheet.answered(window, cx));
                let list = self.jobs.read(cx).list.clone();
                list.update(cx, |list, cx| list.hold_started_words(&words, &repository, held, cx));
            }
            "answer-operator-started" | "answer-operator-handed" | "answer-operator-parked" => {
                // Fleet's answer to a Start with the Operator on is `operator`, and the sheet goes with him where he was.
                // What the Operator did then arrives as `operator.dispatched`, which is said as a toast.
                let (words, repository) = self.sheet.read(cx).started(cx).ok_or("the sheet holds no start to answer")?;
                self.sheet.update(cx, |sheet, cx| sheet.answered(window, cx));
                let list = self.jobs.read(cx).list.clone();
                let (way, job) = match name {
                    "answer-operator-started" => {
                        let made = list.update(cx, |list, cx| list.operator_started(&words, &repository, cx));
                        (ipc::operator_board::DispatchWay::Started, made.ok_or("the list holds no Job to stand the new one beside")?)
                    }
                    "answer-operator-handed" => {
                        let live = "started-by-operator".to_string();
                        let title = list.read(cx).title_of(&live).ok_or("no Job was started for the work to be handed to")?;
                        (ipc::operator_board::DispatchWay::Handed, (live, title))
                    }
                    _ => (ipc::operator_board::DispatchWay::Parked, (String::new(), words.lines().next().unwrap_or_default().to_string())),
                };
                let kept = way == ipc::operator_board::DispatchWay::Parked;
                crate::toasts::dispatched(
                    ipc::operator_board::OperatorDispatched {
                        dispatch_id: if kept { "idea:walk".into() } else { "start:dispatch:walk".into() },
                        manifest_id: repository,
                        way,
                        session_id: (!kept).then(|| SessionId::carried(job.0)),
                        title: Some(job.1),
                    },
                    cx,
                );
            }
            "queued-started" => {
                // Fleet's yes to the first queued idea's start: it leaves the queue and its Job opens.
                let idea = crate::queued::ideas(cx).into_iter().next().ok_or("no idea is queued")?;
                crate::queued::remove(&idea.id, cx);
                let list = self.jobs.read(cx).list.clone();
                list.update(cx, |list, cx| list.hold_started(&idea, cx));
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
        let area = Rc::clone(&self.area);
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
            .child(self.under.clone())
            .child(canvas(move |bounds, _, _| area.set(Some(bounds)), |_, _, _, _| {}).absolute().top_0().left_0().size_full())
            .children(sheet)
            // Drawn always: it draws nothing while shut, so its own opening and closing need no redraw here.
            .child(self.new_project.clone())
            // Over every overlay the screen draws, Settings among them, so a note can be made on it.
            .children(self.layer.clone().map(|layer| gpui::deferred(layer).with_priority(annotate::OVER)))
    }
}
