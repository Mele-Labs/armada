//! The Jobs list, as Electron's Cockpit list (`cockpit/deck/Deck.tsx`): every Job in the Filter's
//! scope, grouped by project in the order the person placed them, a Job another started standing
//! under it. A right press opens the row's menu, a pause on a row opens its card, and the Filter
//! narrows the list by scope and by search.
//!
//! A Job's lamp and state glyph are not drawn yet: Fleet works them out as the Job's mark (#3374),
//! and copying the TypeScript rules here would make a second vocabulary.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use gpui::{
    actions, anchored, canvas, deferred, div, point, prelude::*, svg, App, Bounds, Context, Anchor, DismissEvent, Entity, Focusable,
    FocusHandle, FontWeight, KeyBinding, MouseButton, Pixels, Point, Role, SharedString, ScrollHandle, Subscription, Task,
    Window,
};
use gpui_component::input::{InputEvent, InputState};
use ipc::{SessionRecord, SessionState};

use crate::assets::icon_path;
use crate::dropdown::{under, Dropdown, Entry, Picked};
use crate::fleet::FleetConnection;
use crate::keymap::Act;
use crate::labels::labelled;
use crate::log::Note;
use crate::look::{token, ActiveLook, Look};

actions!(jobs, [RowMenu, Earlier]);

/// The list's keys the registry does not carry yet: Shift+F10 for a row's menu, as Electron binds
/// it in `Deck.tsx`, and `h` for the Job this one took over from, whose registry row waits on the look.
pub fn bind(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("shift-f10", RowMenu, Some("list")), KeyBinding::new("h", Earlier, Some("list"))]);
}

/// Which Jobs the list holds (`cockpit/deck/store.ts`, `SCOPES`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Active,
    Paused,
    Quiet,
    Ended,
    All,
}

const SCOPES: [(Scope, &str, &str); 5] = [
    (Scope::Active, "active", "Active"),
    (Scope::Paused, "paused", "Paused"),
    (Scope::Quiet, "quiet", "Quiet"),
    (Scope::Ended, "ended", "Ended"),
    (Scope::All, "all", "All"),
];

impl Scope {
    fn id(self) -> &'static str {
        SCOPES.iter().find(|(scope, ..)| *scope == self).map_or("active", |(_, id, _)| id)
    }

    fn label(self) -> &'static str {
        SCOPES.iter().find(|(scope, ..)| *scope == self).map_or("Active", |(.., label)| label)
    }

    fn of(id: &str) -> Scope {
        SCOPES.iter().find(|(_, held, _)| *held == id).map_or(Scope::Active, |(scope, ..)| *scope)
    }
}

/// A Job as its row and its card draw it.
#[derive(Clone, Debug, PartialEq)]
struct Row {
    id: SharedString,
    title: SharedString,
    project: SharedString,
    paused: bool,
    ended: bool,
    /// How many listed Jobs up the line of starters this one stands.
    depth: usize,
    /// The Job that started this one, by title and project, for the link's tooltip.
    started_by: Option<(SharedString, SharedString)>,
    slot: Option<SharedString>,
    branch: Option<SharedString>,
    pr: Option<SharedString>,
    last: Option<SharedString>,
}

/// One line of the list: a project's name over its Jobs, or a Job.
#[derive(Clone, Debug, PartialEq)]
enum Line {
    Project(SharedString),
    Job(Row),
}

fn project_of(job: &SessionRecord) -> String {
    job.manifest_id.as_ref().map_or_else(|| job.cwd.clone(), |id| id.as_str().to_string())
}

fn attached<'a>(job: &'a SessionRecord, kind: &str) -> Option<&'a str> {
    job.attachments.iter().find(|one| one.kind == kind && one.state == ipc::AttachmentState::Standing).map(|one| one.target.as_str())
}

/// The Job that started `job`, by id: `started_by` is spent the moment it is written, so any state.
fn starter_of(job: &SessionRecord) -> Option<&str> {
    job.attachments.iter().find(|one| one.kind == "started_by").map(|one| one.target.as_str())
}

/// How long ago `at` was, in Electron's short form: `now`, `4m`, `2h`, `3d`.
fn age(at: &str, now: chrono::DateTime<chrono::Utc>) -> Option<String> {
    let then = chrono::DateTime::parse_from_rfc3339(at).ok()?;
    let minutes = (now - then.with_timezone(&chrono::Utc)).num_minutes().max(0);
    Some(match minutes {
        0 => "now".into(),
        1..=59 => format!("{minutes}m"),
        60..=1439 => format!("{}h", minutes / 60),
        _ => format!("{}d", minutes / 1440),
    })
}

/// Whether `job` answers `query` (`find.ts`): `#N` and `pr N` name a pull request by its number's
/// start, `slot N` a slot exactly, and anything else a branch or the title, by a part of either.
fn matches(job: &SessionRecord, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return true;
    }
    let pr = attached(job, "pr").unwrap_or_default();
    if let Some(number) = query.strip_prefix('#').or_else(|| query.strip_prefix("pr ")) {
        return !pr.is_empty() && pr.starts_with(number.trim());
    }
    if let Some(number) = query.strip_prefix("slot ") {
        return attached(job, "slot").is_some_and(|slot| slot.rsplit(['/', '-']).next() == Some(number.trim()));
    }
    let branch = attached(job, "branch").unwrap_or_default().to_lowercase();
    branch.contains(&query) || job.title.as_deref().unwrap_or_default().to_lowercase().contains(&query)
}

/// The lines for `jobs` in `scope`, narrowed by `query`. Live Jobs keep the order the person placed
/// them in, `order`, then Fleet's for the rest; ended ones come newest first. Each project's Jobs
/// sit under its name, projects in the order their first Job comes, and a Job started by another
/// listed in its project stands right under it. One project draws no name, as Electron's list
/// draws a head only when more than one repository is listed (`cockpit/deck/bands.ts`).
fn lines_of(jobs: &[SessionRecord], scope: Scope, query: &str, order: &[String]) -> Vec<Line> {
    let paused = |job: &SessionRecord| job.hosted.as_ref().is_some_and(|hosted| hosted.paused.is_some());
    let live = |job: &&SessionRecord| job.state == SessionState::Live;
    let placed = |job: &&SessionRecord| order.iter().position(|id| id == job.id.as_str()).unwrap_or(usize::MAX);
    let mut lives: Vec<&SessionRecord> = jobs.iter().filter(live).collect();
    lives.sort_by_key(placed);
    let mut ended: Vec<&SessionRecord> = jobs.iter().filter(|job| job.state == SessionState::Ended).collect();
    ended.sort_by(|a, b| b.ended_at.as_ref().map(|at| at.as_str()).cmp(&a.ended_at.as_ref().map(|at| at.as_str())));
    let picked: Vec<&SessionRecord> = match scope {
        Scope::Active => lives.into_iter().filter(|job| !paused(job)).collect(),
        Scope::Paused => lives.into_iter().filter(|job| paused(job)).collect(),
        // A quiet Job is one whose mod stopped answering, which Fleet's mark says. Until it does, none.
        Scope::Quiet => Vec::new(),
        Scope::Ended => ended,
        Scope::All => lives.into_iter().chain(ended).collect(),
    };
    let picked: Vec<&SessionRecord> = picked.into_iter().filter(|job| matches(job, query)).collect();
    let now = chrono::Utc::now();
    let title_of = |job: &SessionRecord| -> SharedString { job.title.clone().unwrap_or_else(|| job.id.as_str().to_string()).into() };
    let mut projects: Vec<(String, Vec<&SessionRecord>)> = Vec::new();
    for job in picked {
        let project = project_of(job);
        match projects.iter_mut().find(|(name, _)| *name == project) {
            Some((_, held)) => held.push(job),
            None => projects.push((project, vec![job])),
        }
    }
    let headed = projects.len() > 1;
    let mut lines = Vec::new();
    for (project, held) in projects {
        if headed {
            lines.push(Line::Project(project.clone().into()));
        }
        let listed = |id: &str| held.iter().any(|job| job.id.as_str() == id);
        let row_of = |job: &SessionRecord, depth: usize| Row {
            id: job.id.as_str().to_string().into(),
            title: title_of(job),
            project: project.clone().into(),
            paused: paused(job),
            ended: job.state == SessionState::Ended,
            depth,
            started_by: starter_of(job).and_then(|id| jobs.iter().find(|one| one.id.as_str() == id)).map(|one| (title_of(one), project_of(one).into())),
            slot: attached(job, "slot").map(|slot| slot.rsplit('/').next().unwrap_or(slot).to_string().into()),
            branch: attached(job, "branch").map(|branch| branch.to_string().into()),
            pr: attached(job, "pr").map(|pr| format!("#{pr}").into()),
            last: job.last_turn_at.as_ref().and_then(|at| age(at.as_str(), now)).map(Into::into),
        };
        fn under<'a>(
            parent: &str,
            depth: usize,
            held: &[&'a SessionRecord],
            row_of: &dyn Fn(&SessionRecord, usize) -> Row,
            lines: &mut Vec<Line>,
        ) {
            for child in held.iter().filter(|job| starter_of(job) == Some(parent)) {
                lines.push(Line::Job(row_of(child, depth)));
                under(child.id.as_str(), depth + 1, held, row_of, lines);
            }
        }
        for root in held.iter().filter(|job| !starter_of(job).is_some_and(listed)) {
            lines.push(Line::Job(row_of(root, 0)));
            under(root.id.as_str(), 1, &held, &row_of, &mut lines);
        }
    }
    lines
}

/// Where the list's scope and placed order are kept between runs, as Electron keeps them in its
/// `localStorage`: the scope's id on the first line, then one Job id a line.
fn deck_file() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join("Library/Application Support/Armada/native-deck"))
}

fn read_deck() -> (Scope, Vec<String>) {
    let text = deck_file().and_then(|path| std::fs::read_to_string(path).ok()).unwrap_or_default();
    let mut lines = text.lines();
    let scope = Scope::of(lines.next().unwrap_or_default());
    (scope, lines.filter(|line| !line.is_empty()).map(str::to_string).collect())
}

fn write_deck(scope: Scope, order: &[String]) {
    let Some(path) = deck_file() else { return };
    let text = std::iter::once(scope.id().to_string()).chain(order.iter().cloned()).collect::<Vec<_>>().join("\n");
    if let Err(why) = path.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|()| std::fs::write(&path, text)) {
        Note::warn("the list's order was not kept").field("why", why.to_string()).emit();
    }
}

/// A row's act, sent to Fleet off the main thread. With no Fleet to ask, as on mock data, it is
/// only noted.
fn send(port: Option<u16>, path: &'static str, body: Result<String, String>, cx: &mut App) {
    let Some(port) = port else {
        Note::warn("not sent: no Fleet is connected").field("route", path).emit();
        return;
    };
    cx.background_executor()
        .spawn(async move {
            let answer = match body {
                Ok(body) => tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|why| why.to_string())
                    .and_then(|runtime| runtime.block_on(crate::transport::post(port, path, body))),
                Err(why) => Err(why),
            };
            match answer {
                Ok((status, _)) if (200..300).contains(&status) => {}
                Ok((status, bytes)) => Note::warn("Fleet refused a row's act")
                    .field("route", path)
                    .field("status", status as i64)
                    .field("said", String::from_utf8_lossy(&bytes).to_string())
                    .emit(),
                Err(why) => Note::warn("a row's act did not reach Fleet").field("route", path).field("why", why).emit(),
            }
        })
        .detach();
}

/// A menu that is open, and what listens to it.
struct Open {
    menu: Entity<Dropdown>,
    _picked: Subscription,
    _dismissed: Subscription,
}

pub struct JobsList {
    jobs: Vec<SessionRecord>,
    lines: Vec<Line>,
    scope: Scope,
    query: String,
    /// Live Jobs in the order the person placed them, by id.
    order: Vec<String>,
    /// Whether the scope and the order are kept between runs: not on mock data.
    remember: bool,
    /// The Job the keys are on, by id, so a list that changes under it keeps it.
    focused: Option<SharedString>,
    /// The Job opened with Enter or a press.
    opened: Option<SharedString>,
    focus: FocusHandle,
    /// The whole panel's, which holds the list and what it opens over itself, its menus.
    within: FocusHandle,
    scroll: ScrollHandle,
    /// Each drawn row's place in the window, for a menu opened from the keyboard and for its card.
    bounds: Rc<RefCell<HashMap<SharedString, Bounds<Pixels>>>>,
    menu: Option<Open>,
    filter: Option<(Open, Entity<InputState>, Subscription)>,
    /// The Job whose card is open, and the wait before one opens.
    card: Option<SharedString>,
    _card_wait: Option<Task<()>>,
    fleet: Option<Entity<FleetConnection>>,
    _watch: Option<Subscription>,
}

impl JobsList {
    /// The list on a live Fleet: it follows the Jobs the connection holds.
    pub fn live(fleet: &Entity<FleetConnection>, cx: &mut Context<Self>) -> Self {
        let watch = cx.observe(fleet, |this, fleet, cx| {
            let jobs = fleet.read(cx).held().jobs.clone();
            if jobs != this.jobs {
                this.jobs = jobs;
                this.relist(cx);
            }
        });
        let (scope, order) = read_deck();
        let jobs = fleet.read(cx).held().jobs.clone();
        Self::with(jobs, scope, order, true, Some(fleet.clone()), Some(watch), cx)
    }

    /// The list on mock Jobs, which never change.
    pub fn mock(jobs: &[SessionRecord], cx: &mut Context<Self>) -> Self {
        Self::with(jobs.to_vec(), Scope::Active, Vec::new(), false, None, None, cx)
    }

    fn with(
        jobs: Vec<SessionRecord>,
        scope: Scope,
        order: Vec<String>,
        remember: bool,
        fleet: Option<Entity<FleetConnection>>,
        watch: Option<Subscription>,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut this = JobsList {
            jobs,
            lines: Vec::new(),
            scope,
            query: String::new(),
            order,
            remember,
            focused: None,
            opened: None,
            focus: cx.focus_handle(),
            within: cx.focus_handle(),
            scroll: ScrollHandle::new(),
            bounds: Rc::default(),
            menu: None,
            filter: None,
            card: None,
            _card_wait: None,
            fleet,
            _watch: watch,
        };
        this.lines = this.lines_now();
        this.focused = this.first_row();
        this
    }

    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus
    }

    pub fn within(&self) -> &FocusHandle {
        &self.within
    }

    fn lines_now(&self) -> Vec<Line> {
        lines_of(&self.jobs, self.scope, &self.query, &self.order)
    }

    fn first_row(&self) -> Option<SharedString> {
        self.rows().next().map(|row| row.id.clone())
    }

    fn rows(&self) -> impl Iterator<Item = &Row> {
        self.lines.iter().filter_map(|line| match line {
            Line::Job(row) => Some(row),
            Line::Project(_) => None,
        })
    }

    /// Draw the list again from the Jobs, the scope, the search and the order.
    fn relist(&mut self, cx: &mut Context<Self>) {
        let lines = self.lines_now();
        if lines != self.lines {
            self.lines = lines;
            if !self.focused.as_ref().is_some_and(|id| self.at_of(id).is_some()) {
                self.focused = self.first_row();
            }
            cx.notify();
        }
    }

    fn keep(&self) {
        if self.remember {
            write_deck(self.scope, &self.order);
        }
    }

    fn row_at(&self, at: usize) -> Option<&Row> {
        match self.lines.get(at)? {
            Line::Job(row) => Some(row),
            Line::Project(_) => None,
        }
    }

    fn at_of(&self, id: &SharedString) -> Option<usize> {
        self.lines.iter().position(|line| matches!(line, Line::Job(row) if &row.id == id))
    }

    fn port(&self, cx: &App) -> Option<u16> {
        self.fleet.as_ref().and_then(|fleet| fleet.read(cx).held().port())
    }

    /// Move the keys one Job down (`by` 1) or up (`by` -1), past project names. Up from the first
    /// Job opens the Filter, as Electron's list does.
    fn step(&mut self, by: isize, window: &mut Window, cx: &mut Context<Self>) {
        let from = self.focused.as_ref().and_then(|id| self.at_of(id));
        let mut at = from.map_or(if by > 0 { -1 } else { self.lines.len() as isize }, |at| at as isize);
        loop {
            at += by;
            if at < 0 || at >= self.lines.len() as isize {
                if by < 0 && from.is_some() {
                    self.open_filter(window, cx);
                }
                return;
            }
            if let Some(id) = self.row_at(at as usize).map(|row| row.id.clone()) {
                self.focused = Some(id);
                self.scroll.scroll_to_item(at as usize);
                cx.notify();
                return;
            }
        }
    }

    /// Move the focused Job past its neighbour in its project (`by` 1 down, -1 up). A Job standing
    /// under another moves with it and is not moved alone, as in Electron (`reorder.ts`).
    fn shove(&mut self, by: isize, cx: &mut Context<Self>) {
        let Some(id) = self.focused.clone() else { return };
        let Some(row) = self.rows().find(|row| row.id == id).cloned() else { return };
        if row.depth > 0 || row.ended {
            return;
        }
        let peers: Vec<SharedString> =
            self.rows().filter(|one| one.project == row.project && one.depth == 0 && !one.ended).map(|one| one.id.clone()).collect();
        let Some(at) = peers.iter().position(|one| *one == id) else { return };
        let Some(over) = at.checked_add_signed(by).and_then(|to| peers.get(to)).cloned() else { return };
        // Place every live Job, so the move holds whatever the list draws next.
        let mut placed: Vec<String> =
            lines_of(&self.jobs, Scope::All, "", &self.order).iter().filter_map(|line| match line {
                Line::Job(one) if !one.ended => Some(one.id.to_string()),
                _ => None,
            }).collect();
        let (Some(from), Some(to)) = (placed.iter().position(|one| *one == *id), placed.iter().position(|one| *one == *over)) else { return };
        placed.swap(from, to);
        self.order = placed;
        self.keep();
        self.relist(cx);
    }

    fn on_act(&mut self, act: &Act, window: &mut Window, cx: &mut Context<Self>) {
        if act.is("move_focus") {
            // The registry's slots are j, k, ↓, ↑: even slots go down.
            self.step(if act.slot % 2 == 0 { 1 } else { -1 }, window, cx);
        } else if act.is("open_focused") {
            if self.opened != self.focused {
                self.opened = self.focused.clone();
                cx.notify();
            }
        } else if act.is("deck_search") {
            self.open_filter(window, cx);
        } else if act.is("deck_reorder") {
            // ⌥↑ then ⌥↓.
            self.shove(if act.slot == 0 { -1 } else { 1 }, cx);
        } else {
            cx.propagate();
        }
    }

    fn on_row_menu(&mut self, _: &RowMenu, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.focused.clone() else { return };
        let Some(bounds) = self.bounds.borrow().get(&id).copied() else { return };
        self.open_menu(id, under(bounds), true, window, cx);
    }

    fn on_earlier(&mut self, _: &Earlier, _window: &mut Window, cx: &mut Context<Self>) {
        // A Job another took the place of is `behind` it, and Fleet does not say so on the record
        // yet; until it does, `h` has nowhere to go.
        cx.propagate();
    }

    /// The row's menu (`RowMenu.tsx`), at `at`: Pause or Resume, Name it for me, Try it in your app
    /// and Close. An ended Job has none of them, and draws no menu.
    fn open_menu(&mut self, id: SharedString, at: Point<Pixels>, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(row) = self.rows().find(|row| row.id == id).cloned() else { return };
        if row.ended {
            return;
        }
        self.card = None;
        let entries = vec![
            if row.paused {
                Entry::Act { id: "resume", label: "Resume".into(), icon: Some("play"), danger: false }
            } else {
                Entry::Act { id: "pause", label: "Pause".into(), icon: Some("circle-pause"), danger: false }
            },
            Entry::Act { id: "name", label: "Name it for me".into(), icon: Some("wand-sparkles"), danger: false },
            Entry::Act { id: "try", label: "Try it in your app".into(), icon: None, danger: false },
            Entry::Act { id: "close", label: "Close".into(), icon: Some("x"), danger: true },
        ];
        let menu = cx.new(|cx| Dropdown::new("Job", entries, at, Anchor::TopLeft, from_keys, cx));
        let job = id.clone();
        let picked = cx.subscribe(&menu, move |this, _, picked: &Picked, cx| {
            let Picked::Entry { id: act, .. } = picked;
            let port = this.port(cx);
            let session_id = ipc::SessionId::carried(job.to_string());
            let encoded = |body: Result<String, ipc::Unencodable>| body.map_err(|why| format!("{why:?}"));
            match *act {
                "pause" => send(port, "/sessions/pause", encoded(ipc::encode(&ipc::PauseSession { session_id })), cx),
                "resume" => send(port, "/sessions/resume", encoded(ipc::encode(&ipc::ResumeSession { session_id })), cx),
                "name" => send(port, "/sessions/name", encoded(ipc::encode(&ipc::NameSession { session_id })), cx),
                "try" => send(
                    port,
                    "/fleet/build/pick",
                    encoded(ipc::encode(&ipc::PickFleetBuildJob { branch: None, session_id: Some(job.to_string()), picked: true })),
                    cx,
                ),
                "close" => send(port, "/sessions/close", encoded(ipc::encode(&ipc::CloseSession { session_id })), cx),
                _ => {}
            }
        });
        let dismissed = cx.subscribe_in(&menu, window, |this, _, _: &DismissEvent, window, cx| {
            this.menu = None;
            window.focus(&this.focus, cx);
            cx.notify();
        });
        menu.update(cx, |menu, cx| window.focus(&menu.focus_handle(cx), cx));
        self.menu = Some(Open { menu, _picked: picked, _dismissed: dismissed });
        cx.notify();
    }

    /// Whether the Filter narrows the list: a scope other than Active, or a search.
    fn narrowed(&self) -> bool {
        self.scope != Scope::Active || !self.query.trim().is_empty()
    }

    /// What the Filter's tooltip says it narrows to (`Deck.tsx`, `Filter`).
    fn filter_said(&self) -> SharedString {
        let cut = |text: &str, at: usize| {
            if text.chars().count() > at { format!("{}…", text.chars().take(at).collect::<String>()) } else { text.to_string() }
        };
        let query = self.query.trim();
        match (self.scope, query.is_empty()) {
            (Scope::Active, true) => "Filter".into(),
            (Scope::Active, false) => format!("Filter: {}", cut(query, 14)).into(),
            (scope, true) => format!("Filter: {}", scope.label()).into(),
            (scope, false) => format!("Filter: {} · {}", scope.label(), cut(query, 8)).into(),
        }
    }

    fn filter_entries(&self) -> Vec<Entry> {
        SCOPES.iter().map(|(scope, id, label)| Entry::Choice { id, label: (*label).into(), on: *scope == self.scope }).collect()
    }

    /// The Filter's menu, under its button: the search field, which takes the keys, over the five scopes.
    fn open_filter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.filter.is_some() {
            return;
        }
        let Some(bounds) = self.bounds.borrow().get("filter").copied() else { return };
        let gap = cx.look().length(token::SPACE_1);
        let at = point(bounds.origin.x + bounds.size.width, bounds.origin.y + bounds.size.height + gap);
        let query = self.query.clone();
        let search = cx.new(|cx| InputState::new(window, cx).default_value(query));
        let entries = self.filter_entries();
        let menu = cx.new(|cx| Dropdown::new("Filter", entries, at, Anchor::TopRight, false, cx).with_search(search.clone()));
        let typed = cx.subscribe_in(&search, window, |this, search, event: &InputEvent, window, cx| match event {
            InputEvent::Change => {
                this.query = search.read(cx).value().to_string();
                this.relist(cx);
            }
            // Enter in the field shuts the menu and puts the keys on the first Job.
            InputEvent::PressEnter { .. } => {
                this.filter = None;
                this.focused = this.first_row();
                window.focus(&this.focus, cx);
                cx.notify();
            }
            _ => {}
        });
        let picked = cx.subscribe(&menu, |this, _, picked: &Picked, cx| {
            let Picked::Entry { id, .. } = picked;
            this.scope = Scope::of(id);
            this.keep();
            this.relist(cx);
        });
        let dismissed = cx.subscribe_in(&menu, window, |this, _, _: &DismissEvent, window, cx| {
            this.filter = None;
            window.focus(&this.focus, cx);
            cx.notify();
        });
        search.update(cx, |input, cx| input.focus(window, cx));
        self.filter = Some((Open { menu, _picked: picked, _dismissed: dismissed }, search, typed));
        cx.notify();
    }

    /// The pointer rests on a row (`hovered`) or left it: a card opens after the tooltip's wait, at
    /// once while another is open, and shuts as the pointer leaves.
    fn hovered(&mut self, id: SharedString, hovered: bool, window: &mut Window, cx: &mut Context<Self>) {
        if !hovered {
            if self.card.as_ref() == Some(&id) {
                self.card = None;
                cx.notify();
            }
            self._card_wait = None;
            return;
        }
        if self.menu.is_some() {
            return;
        }
        let wait = cx.look().duration(token::TOOLTIP_DELAY);
        if self.card.is_some() || wait.is_zero() {
            self.card = Some(id);
            cx.notify();
            return;
        }
        self._card_wait = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(wait).await;
            let _ = this.update(cx, |this, cx| {
                this.card = Some(id);
                cx.notify();
            });
        }));
    }

    /// A Job's row as Electron's Cockpit list draws it (`console-bridge.css`): a strip with a lamp
    /// bar at its left and a status slot at its right, indented under the Job that started it.
    fn render_job(&self, at: usize, row: &Row, cx: &mut Context<Self>) -> gpui::AnyElement {
        let look = cx.look();
        let after_job = at > 0 && matches!(self.lines[at - 1], Line::Job(_));
        let focused = self.focused.as_ref() == Some(&row.id);
        let opened = self.opened.as_ref() == Some(&row.id);
        let edge = if opened { token::ACCENT } else { token::BORDER_SUBTLE };
        let ground = if opened {
            look.color(token::ACCENT_MUTED)
        } else if focused {
            look.color(token::BG_HOVER)
        } else {
            look.color(token::BG_SUNKEN)
        };
        let weight = if opened { token::WEIGHT_HEADING } else { token::WEIGHT_BODY };
        let ink = if row.paused || row.ended { token::FG_MUTED } else { token::FG_DEFAULT };
        // An ended Job's lamp is done's; the rest wait on Fleet's mark.
        let lamp = if row.ended { token::STATUS_COMPLETED_SUCCESS } else { token::BORDER_STRONG };
        let rail = look.length(token::BORDER_WIDTH) * 2.;
        let indent = look.length(token::SPACE_4) * row.depth as f32;
        let tall = if row.depth > 0 { token::SPACE_2 } else { token::SPACE_3 };
        let mark_glyph = look.length(token::SPACE_3);
        let (pressed, menu_on, hover_on) = (row.id.clone(), row.id.clone(), row.id.clone());
        let store = Rc::clone(&self.bounds);
        let kept = row.id.clone();
        let linked = row.started_by.as_ref().map(|(title, project)| -> SharedString { format!("Linked Job, started by {title} in {project}").into() });
        let strip = div()
            .id(SharedString::from(format!("job-{}", row.id)))
            .relative()
            .flex_1()
            .min_w_0()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .h(look.length(token::SPACE_6) + look.length(tall))
            .pl(look.length(token::SPACE_3))
            .pr(look.length(token::SPACE_2))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(edge))
            .bg(ground)
            .when(!opened, |strip| strip.hover(|style| style.bg(look.color(token::BG_HOVER))))
            .child(canvas(move |bounds, _, _| { store.borrow_mut().insert(kept, bounds); }, |_, _, _, _| {}).absolute().size_full())
            .child(div().absolute().top_0().bottom_0().left_0().w(look.length(token::SPACE_1)).bg(look.color(lamp)))
            .children(linked.clone().map(|said| {
                div()
                    .id(SharedString::from(format!("link-{}", row.id)))
                    .flex_none()
                    .child(svg().path(icon_path("link")).size(mark_glyph).text_color(look.color(token::FG_SUBTLE)))
                    .tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(said.clone()).build(window, cx))
            }))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .text_color(look.color(ink))
                    .font_weight(FontWeight(look.number(weight)))
                    .child(row.title.clone()),
            )
            .child(
                // The wide status slot: the paused mark now, the state glyph once Fleet sends the mark.
                div().flex_none().flex().justify_end().w(look.length(token::SPACE_4) * 2. + look.length(token::SPACE_1)).when(row.paused, |slot| {
                    slot.child(labelled(
                        div()
                            .id(SharedString::from(format!("paused-{}", row.id)))
                            .child(svg().path(icon_path("circle-pause")).size(mark_glyph).text_color(look.color(token::FG_MUTED)))
                            .tooltip(|window, cx| gpui_component::tooltip::Tooltip::new("Paused").build(window, cx)),
                        Role::Image,
                        "Paused",
                    ))
                }),
            )
            .on_hover(cx.listener(move |this, hovered: &bool, window, cx| this.hovered(hover_on.clone(), *hovered, window, cx)))
            .on_mouse_down(MouseButton::Right, cx.listener(move |this, event: &gpui::MouseDownEvent, window, cx| {
                this.focused = Some(menu_on.clone());
                this.open_menu(menu_on.clone(), event.position, false, window, cx);
            }))
            .on_click(cx.listener(move |this, _event, window, cx| {
                this.focused = Some(pressed.clone());
                this.opened = Some(pressed.clone());
                this.card = None;
                window.focus(&this.focus, cx);
                cx.notify();
            }));
        let name = match &linked {
            Some(said) => format!("{}, {said}", row.title).into(),
            None => row.title.clone(),
        };
        div()
            .flex_none()
            .flex()
            .when(after_job, |line| line.mt(look.length(token::SPACE_1)))
            .pl(indent)
            .when(row.depth > 0, |line| {
                // The elbow from the Job above: a square rail, its start and bottom edges drawn.
                line.child(
                    div()
                        .flex_none()
                        .w(look.length(token::SPACE_2))
                        .h(look.length(token::SPACE_6) / 2. + look.length(token::SPACE_1))
                        .ml(-look.length(token::SPACE_2) - look.length(token::SPACE_1))
                        .mr(look.length(token::SPACE_1))
                        .border_l(rail)
                        .border_b(rail)
                        .border_color(look.color(token::BORDER_DEFAULT)),
                )
            })
            .child(labelled(strip, Role::Row, name))
            .into_any_element()
    }

    fn render_line(&self, at: usize, cx: &mut Context<Self>) -> gpui::AnyElement {
        match &self.lines[at] {
            Line::Project(name) => {
                let look = cx.look();
                labelled(
                    div()
                        .id(("project", at))
                        .flex_none()
                        .flex()
                        .items_center()
                        .h(look.length(token::SPACE_6))
                        .when(at > 0, |head| head.mt(look.length(token::SPACE_3)))
                        .mb(look.length(token::SPACE_1))
                        .px(look.length(token::SPACE_2))
                        .border_b(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_SUBTLE))
                        .font_family(look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_default())
                        // `--cb-code`: four fifths of the smallest text.
                        .text_size(look.length(token::TEXT_2XS) * 0.8)
                        .line_height(look.length(token::LEADING_XS))
                        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                        .text_color(look.color(token::FG_MUTED))
                        .child(name.to_uppercase()),
                    Role::Heading,
                    name.clone(),
                )
                .into_any_element()
            }
            Line::Job(row) => {
                let row = row.clone();
                self.render_job(at, &row, cx)
            }
        }
    }

    /// The Filter's chip at the panel's top right corner (`deck.css`, `.deck-band`), shown while the
    /// pointer is over the panel or the keys are in it.
    fn render_filter(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let shown = self.filter.is_some() || self.focus.contains_focused(window, cx);
        let said = self.filter_said();
        let store = Rc::clone(&self.bounds);
        let ink = if self.narrowed() { token::ACCENT } else { token::FG_MUTED };
        div()
            .absolute()
            .top(look.length(token::SPACE_1))
            .right(look.length(token::SPACE_1))
            .px(look.length(token::SPACE_1))
            .rounded(look.length(token::RADIUS_MD))
            .bg(look.color(token::BG_SUNKEN))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .when(!shown, |chip| chip.opacity(0.).group_hover("jobs-panel", |style| style.opacity(1.)))
            .child(labelled(
                div()
                    .id("filter")
                    .relative()
                    .size(look.length(token::H_CONTROL_SM))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(look.length(token::RADIUS_MD))
                    .cursor_pointer()
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .child(canvas(move |bounds, _, _| { store.borrow_mut().insert("filter".into(), bounds); }, |_, _, _, _| {}).absolute().size_full())
                    .child(svg().path(icon_path("funnel")).size(look.length(token::SPACE_4)).text_color(look.color(ink)))
                    .tooltip({
                        let said = said.clone();
                        move |window, cx| gpui_component::tooltip::Tooltip::new(said.clone()).build(window, cx)
                    })
                    .on_click(cx.listener(|this, _, window, cx| this.open_filter(window, cx))),
                Role::Button,
                said,
            ))
    }

    /// The open Job's card beside its row (`inspect-cards.tsx`, `SubjectCard`).
    fn render_card(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let id = self.card.as_ref()?;
        let row = self.rows().find(|row| &row.id == id)?.clone();
        let bounds = self.bounds.borrow().get(id).copied()?;
        let look = cx.look();
        let mono = look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_default();
        let facts: Vec<(&str, SharedString, bool)> = [
            row.slot.clone().map(|slot| ("Bay", slot, true)),
            row.pr.clone().map(|pr| ("Pull request", pr, true)),
            row.branch.clone().map(|branch| ("Branch", branch, true)),
            row.last.clone().map(|last| ("Last", last, true)),
        ]
        .into_iter()
        .flatten()
        .collect();
        let small = |look: &Look| div().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS));
        let key = |look: &Look, keys: &'static str, said: &'static str| {
            small(look)
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .text_color(look.color(token::FG_MUTED))
                .child(div().font_family(look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_default()).child(keys))
                .child(said)
        };
        let gap = look.length(token::SPACE_2);
        let card = div()
            .w(look.length(token::W_MENU))
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .p(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_t(look.length(token::BORDER_WIDTH) * 2.)
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_OVERLAY))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            .font_family(look.font_stack(token::FONT_SANS).into_iter().next().unwrap_or_default())
            .child(
                small(look)
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_1))
                    .text_color(look.color(token::FG_MUTED))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .child(svg().path(icon_path("square-terminal")).size(look.length(token::SPACE_3)).text_color(look.color(token::FG_MUTED)))
                    .child("JOB")
                    .child(div().flex_1())
                    .child(div().font_family(mono.clone()).font_weight(FontWeight(look.number(token::WEIGHT_BODY))).text_color(look.color(token::FG_SUBTLE)).child(row.project.clone())),
            )
            .child(
                div()
                    .text_size(look.length(token::TEXT_SM))
                    .line_height(look.length(token::LEADING_SM))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(row.title.clone()),
            )
            .when(row.paused, |card| {
                card.child(
                    small(look)
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .px(look.length(token::SPACE_2))
                        .py(look.length(token::SPACE_1))
                        .rounded(look.length(token::RADIUS_MD))
                        .bg(look.color(token::FG_MUTED).opacity(0.14))
                        .text_color(look.color(token::FG_MUTED))
                        .child(svg().path(icon_path("circle-pause")).size(look.length(token::SPACE_4)).text_color(look.color(token::FG_MUTED)))
                        .child("Paused"),
                )
            })
            .when(!facts.is_empty(), |card| {
                card.child(div().flex().flex_col().gap(look.length(token::BORDER_WIDTH) * 2.).children(facts.into_iter().map(|(name, value, code)| {
                    small(look)
                        .flex()
                        .gap(look.length(token::SPACE_3))
                        .child(div().flex_none().text_color(look.color(token::FG_MUTED)).child(name))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_right()
                                .text_color(look.color(token::FG_DEFAULT))
                                .when(code, |value| value.font_family(mono.clone()))
                                .child(value),
                        )
                })))
            })
            .child(
                div()
                    .flex()
                    .gap(look.length(token::SPACE_3))
                    .pt(look.length(token::SPACE_2))
                    .border_t(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .child(key(look, "Enter", "Open"))
                    .child(key(look, "Esc", "Close")),
            );
        Some(deferred(
            anchored()
                .position(point(bounds.origin.x + bounds.size.width + gap, bounds.origin.y))
                .snap_to_window_with_margin(look.length(token::SPACE_1))
                .child(labelled(div().id("job-card").child(card), Role::Tooltip, row.title.clone())),
        ))
    }
}

impl Render for JobsList {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let lines: Vec<_> = (0..self.lines.len()).map(|at| self.render_line(at, cx)).collect();
        let filter = self.render_filter(window, cx);
        let card = self.render_card(cx);
        let look = cx.look();
        div()
            .track_focus(&self.within)
            .relative()
            .size_full()
            .group("jobs-panel")
            .child(labelled(
                div()
                    .id("jobs")
                    .key_context("list")
                    .track_focus(&self.focus)
                    .on_action(cx.listener(Self::on_act))
                    .on_action(cx.listener(Self::on_row_menu))
                    .on_action(cx.listener(Self::on_earlier))
                    .track_scroll(&self.scroll)
                    .overflow_y_scroll()
                    .size_full()
                    .flex()
                    .flex_col()
                    .p(look.length(token::SPACE_2))
                    .font_family(look.font_stack(token::FONT_SANS).into_iter().next().unwrap_or_default())
                    .children(lines),
                Role::List,
                "Jobs",
            ))
            .child(filter)
            .children(card)
            .children(self.menu.as_ref().map(|open| open.menu.clone()))
            .children(self.filter.as_ref().map(|(open, ..)| open.menu.clone()))
    }
}

/// The Jobs screen in Electron's regions (`cockpit/deck/Deck.tsx`, `.deck-grid`): the list's panel at
/// the left, the open Job's room in the middle, and the right column's panel, under the title row.
pub struct JobsScreen {
    pub list: Entity<JobsList>,
}

impl JobsScreen {
    /// `l` puts the keys in the list (`deck_panel`'s first slot); the other panels are not drawn yet.
    fn on_act(&mut self, act: &Act, window: &mut Window, cx: &mut Context<Self>) {
        if act.is("deck_panel") && act.slot == 0 {
            let focus = self.list.read(cx).focus_handle().clone();
            window.focus(&focus, cx);
        } else {
            cx.propagate();
        }
    }
}

/// A deck panel's frame (`.deck-panel`): the raised ground, a heavier top edge.
fn panel(look: &Look) -> gpui::Div {
    div()
        .h_full()
        .flex_none()
        .bg(look.color(token::BG_RAISED))
        .border(look.length(token::BORDER_WIDTH))
        .border_t(look.length(token::BORDER_WIDTH) * 2.)
        .border_color(look.color(token::BORDER_DEFAULT))
}

impl Render for JobsScreen {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let column = look.length(token::SPACE_12);
        div()
            .key_context("deck")
            .on_action(cx.listener(Self::on_act))
            .size_full()
            .flex()
            .flex_col()
            .bg(look.color(token::BG_BASE))
            .child(crate::chrome::title_row(look))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .gap(look.length(token::GAP_PANEL))
                    .p(look.length(token::SPACE_3))
                    .child(panel(look).w(column * 6.).child(self.list.clone()))
                    .child(div().flex_1())
                    .child(panel(look).w(column * 5.75)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jobs() -> Vec<SessionRecord> {
        crate::mock::jobs("jobs-many").expect("the fixture decodes")
    }

    fn titles(lines: &[Line]) -> Vec<String> {
        lines.iter().filter_map(|line| match line { Line::Job(row) => Some(row.title.to_string()), Line::Project(_) => None }).collect()
    }

    #[test]
    fn ended_jobs_leave_the_list_and_each_project_heads_its_own() {
        let lines = lines_of(&jobs(), Scope::Active, "", &[]);
        assert_eq!(lines[0], Line::Project("armada".into()));
        assert!(!titles(&lines).iter().any(|title| title.contains("ended")));
        let projects: Vec<_> = lines.iter().filter_map(|line| match line { Line::Project(name) => Some(name.clone()), _ => None }).collect();
        assert_eq!(projects, vec![SharedString::from("armada"), "sample-web".into(), "sample-api".into()]);
    }

    #[test]
    fn one_project_draws_its_jobs_with_no_head() {
        let armada: Vec<_> = jobs().into_iter().filter(|job| job.manifest_id.as_ref().is_some_and(|id| id.as_str() == "armada")).collect();
        let lines = lines_of(&armada, Scope::Active, "", &[]);
        assert!(!lines.is_empty());
        assert!(lines.iter().all(|line| matches!(line, Line::Job(_))));
    }

    #[test]
    fn the_placed_order_holds_and_a_search_narrows_by_title() {
        let all = jobs();
        let second = titles(&lines_of(&all, Scope::Active, "", &[]))[1].clone();
        let id = all.iter().find(|job| job.title.as_deref() == Some(second.as_str())).map(|job| job.id.as_str().to_string()).expect("listed");
        assert_eq!(titles(&lines_of(&all, Scope::Active, "", &[id]))[0], second);
        let found = titles(&lines_of(&all, Scope::Active, &second[..6].to_uppercase(), &[]));
        assert!(found.contains(&second) && found.iter().all(|title| title.to_lowercase().contains(&second[..6].to_lowercase())));
    }

    #[test]
    fn the_ended_scope_holds_only_ended_jobs() {
        let ended = titles(&lines_of(&jobs(), Scope::Ended, "", &[]));
        assert!(!ended.is_empty() && ended.iter().all(|title| title.contains("ended")), "{ended:?}");
    }
}
