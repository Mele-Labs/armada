//! The Jobs list, as Electron's Cockpit list (`cockpit/deck/Deck.tsx`): every Job in the chosen
//! scope, grouped by project in the order the person placed them, a Job another started standing
//! under it. A right press opens the row's menu and a pause on a row opens its card. Where Electron
//! has a Filter button, a search field and the scope sit over every group, on the owner's word.
//!
//! A Job's lamp, state glyph and group come from Fleet as its mark (`SessionRecord::mark`), so
//! the rules that read them are not copied here. Only the glyph and the words for each state are.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use gpui::{
    actions, anchored, pulsating_between, Animation, AnimationExt, canvas, deferred, div, point, prelude::*, svg, App, Bounds, Context, Anchor, DismissEvent, Entity, Focusable,
    FocusHandle, FontWeight, KeyBinding, MouseButton, Pixels, Point, Role, SharedString, ScrollHandle, Subscription, Task,
    Window,
};
use gpui_component::input::{Input, InputEvent, InputState};
use ipc::{JobGroup, JobLamp, JobMark, JobState, SessionRecord, SessionState};

use crate::assets::icon_path;
use crate::dropdown::{under, Dropdown, Entry, Picked};
use crate::fleet::{BuildAct, FleetConnection};
use crate::job_view::{JobSide, JobView};
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
    /// Asked to carry on in a new Job and told yes: a turning ring until the new one is up.
    handing_off: bool,
    /// The earlier Job this one took the place of, which the list does not draw: a mark beside the row opens it.
    earlier: Option<SharedString>,
    /// How many listed Jobs up the line of starters this one stands.
    depth: usize,
    /// The Job that started this one, by title and project, for the link's tooltip.
    started_by: Option<(SharedString, SharedString)>,
    slot: Option<SharedString>,
    branch: Option<SharedString>,
    pr: Option<SharedString>,
    last: Option<SharedString>,
    mark: Option<JobMark>,
    /// The issue the Job works, as its chip says it, and its title for the tooltip.
    issue: Option<(SharedString, SharedString)>,
    /// The Job's workflow by name, and the step it is at, or the name again where Fleet gives none.
    flow: Option<(SharedString, SharedString)>,
    /// What the Job is doing now, or last said.
    now: Option<SharedString>,
    /// Not a Job: an idea saved to start later, drawn dashed with Start and Remove (`QueuedRow`).
    queued: Option<crate::queued::QueuedIdea>,
}

/// What a row draws, chosen in the View group of its menu (`row-view.tsx`, `VIEW_FIELDS`), in the
/// menu's order. Electron's default is the Status and Conflicts marks.
const VIEW_FIELDS: [(&str, &str); 7] = [
    ("view:pr", "Active PRs"),
    ("view:issue", "Issues"),
    ("view:now", "Now"),
    ("view:time", "Time"),
    ("view:conflicts", "Conflicts"),
    ("view:status", "Status"),
    ("view:workflow", "Workflow"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct View([bool; 7]);

impl View {
    const DEFAULT: View = View([false, false, false, false, true, true, false]);

    fn on(self, id: &str) -> bool {
        VIEW_FIELDS.iter().position(|(field, _)| *field == id).is_some_and(|at| self.0[at])
    }

    fn flip(&mut self, id: &str) {
        if let Some(at) = VIEW_FIELDS.iter().position(|(field, _)| *field == id) {
            self.0[at] = !self.0[at];
        }
    }

    /// The menu's View group: a label and a switch for each field.
    fn entries(self) -> Vec<Entry> {
        std::iter::once(Entry::Label("View"))
            .chain(VIEW_FIELDS.iter().zip(self.0).map(|((id, label), on)| Entry::Toggle { id, label: (*label).into(), on }))
            .collect()
    }
}

/// A state's glyph and the words its tooltip says, as Electron's row draws them
/// (`SessionFrame.tsx`, `MARK`; `session-state.ts`, `jobStateOf`).
pub(crate) fn glyph_of(state: JobState, pr: Option<&str>) -> (&'static str, SharedString) {
    let (glyph, said) = match state {
        JobState::Working => ("circle-dot", "Working"),
        JobState::Running => ("square-terminal", "Running"),
        JobState::Subagent => ("split", "Subagent working"),
        JobState::Waiting | JobState::StoppedAsks => (if state == JobState::Waiting { "eye" } else { "octagon-alert" }, "Waiting on you"),
        JobState::Done => ("power", "Done"),
        JobState::Stuck => ("clock", "Stuck"),
        JobState::Failing => return ("shield-x", pr.map_or("Checks failed".into(), |pr| format!("Checks failed on {pr}").into())),
        JobState::Unstarted => ("x", "Not started"),
        JobState::Stopped => ("square", "Stopped"),
        JobState::Held => ("radar", "Held"),
        JobState::Limited => ("battery-low", "Held for the usage limit"),
        JobState::After => ("link", "Starts after another Job"),
        JobState::Draft => ("message-square-dashed", "Reading the request"),
        JobState::Blank => ("circle-dashed", "Blank: no slot, no branch"),
        JobState::Idle => ("check", "Idle"),
        JobState::Replied => ("reply", "Unread reply"),
        JobState::Quiet => ("unplug", "Not reachable"),
        JobState::Ended => ("unplug", "Ended"),
        JobState::Paused => ("circle-pause", "Paused"),
    };
    (glyph, said.into())
}

/// The hue a state's glyph takes: one per bucket (`SessionFrame.css`, `data-bucket`).
pub(crate) fn hue_of(state: JobState) -> crate::look::Token {
    match state {
        JobState::Waiting | JobState::Done | JobState::Unstarted | JobState::StoppedAsks | JobState::Stuck | JobState::Failing => {
            token::STATUS_AWAITING_REVIEW
        }
        JobState::Working | JobState::Running | JobState::Subagent => token::STATUS_RUNNING,
        JobState::Held | JobState::Limited | JobState::After | JobState::Stopped => token::STATUS_HANDED_IN,
        JobState::Draft | JobState::Blank => token::STATUS_NOT_STARTED,
        _ => token::FG_MUTED,
    }
}

/// How strongly a row's ground takes its lamp's colour; a Job waiting on you pulses from this to twice it.
const HULL: f32 = 0.06;

fn lamp_of(lamp: JobLamp) -> crate::look::Token {
    match lamp {
        JobLamp::Needs => token::STATUS_AWAITING_REVIEW,
        JobLamp::Failed => token::STATUS_COMPLETED_FAILED,
        JobLamp::Running => token::STATUS_RUNNING,
        JobLamp::Done => token::STATUS_COMPLETED_SUCCESS,
        JobLamp::Idle => token::BORDER_STRONG,
    }
}

/// Where a Job the person has not placed stands: by group, Needs you first, then the loudest
/// first inside it (`order.ts`, `GROUPS` and `LOUD`).
fn standing_of(job: &SessionRecord) -> (u8, u8) {
    let Some(mark) = job.mark else { return (4, 2) };
    let group = match mark.group {
        JobGroup::Needs => 0,
        JobGroup::Working => 1,
        JobGroup::Held => 2,
        JobGroup::Done => 3,
    };
    let loud = match mark.lamp {
        JobLamp::Failed => 0,
        JobLamp::Needs => 1,
        _ => 2,
    };
    (group, loud)
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
/// The Job's workflow and the step it is at (`row-view.tsx`, `workflowOf`): the step's words where
/// Fleet gives a step, else the workflow's name. Absent where it follows none.
fn flow_of(job: &SessionRecord) -> Option<(SharedString, SharedString)> {
    let copy = job.workflow_copy.as_ref();
    let name = copy.map(|copy| copy.name.clone()).or_else(|| job.workflow.clone())?;
    let step = job.workflow_step.and_then(|at| copy?.steps.get(at as usize)).map_or_else(|| name.clone(), |step| step.label.clone());
    Some((name.into(), step.into()))
}

/// A hand off's ledger (`a-job-hands-off-to-a-new-one.md`), as `sessions-wire.ts` reads it: `replaces` and
/// `replaced_by` are spent once written.
fn spent<'a>(job: &'a SessionRecord, kind: &str) -> Option<&'a str> {
    job.attachments.iter().find(|one| one.kind == kind && one.state == ipc::AttachmentState::Spent).map(|one| one.target.as_str())
}

/// Handing off: `handoff_starting` stands and no failure outranks it, on a Job not ended.
fn handing_off(job: &SessionRecord) -> bool {
    job.state != SessionState::Ended && attached(job, "handoff_starting").is_some() && attached(job, "handoff_failed").is_none()
}

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
/// The list's order with `id` taken out and put back before or after `target`; `None` when either is
/// not in it.
fn moved(mut placed: Vec<String>, id: &str, target: &str, after: bool) -> Option<Vec<String>> {
    let at = placed.iter().position(|one| one == id)?;
    let lifted = placed.remove(at);
    let there = placed.iter().position(|one| one == target)?;
    placed.insert(if after { there + 1 } else { there }, lifted);
    Some(placed)
}

fn lines_of(jobs: &[SessionRecord], ideas: &[crate::queued::QueuedIdea], scope: Scope, query: &str, order: &[String]) -> Vec<Line> {
    let paused = |job: &SessionRecord| job.hosted.as_ref().is_some_and(|hosted| hosted.paused.is_some());
    let quiet = |job: &SessionRecord| job.mark.is_some_and(|mark| mark.state == JobState::Quiet);
    let live = |job: &&SessionRecord| job.state == SessionState::Live;
    let placed = |job: &&SessionRecord| (order.iter().position(|id| id == job.id.as_str()).unwrap_or(usize::MAX), standing_of(job));
    let mut lives: Vec<&SessionRecord> = jobs.iter().filter(live).collect();
    lives.sort_by_key(placed);
    let mut ended: Vec<&SessionRecord> = jobs.iter().filter(|job| job.state == SessionState::Ended).collect();
    ended.sort_by(|a, b| b.ended_at.as_ref().map(|at| at.as_str()).cmp(&a.ended_at.as_ref().map(|at| at.as_str())));
    let picked: Vec<&SessionRecord> = match scope {
        Scope::Active => lives.into_iter().filter(|job| !paused(job) && !quiet(job)).collect(),
        Scope::Paused => lives.into_iter().filter(|job| paused(job)).collect(),
        Scope::Quiet => lives.into_iter().filter(|job| quiet(job)).collect(),
        Scope::Ended => ended,
        Scope::All => lives.into_iter().chain(ended).collect(),
    };
    // A Job another took the place of stands behind that one's row, as Electron's deck has it (`ship.ts`, `behind`).
    let picked: Vec<&SessionRecord> = picked.into_iter().filter(|job| spent(job, "replaced_by").is_none() && matches(job, query)).collect();
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
    // Groups stand in the order Fleet lists Jobs, so a state change never moves one.
    projects.sort_by_key(|(name, _)| jobs.iter().position(|job| project_of(job) == *name));
    // Ideas waiting to start, among the live Jobs, each under its repository's group (`bands.ts`): the group of a
    // Job in its root, else its repository's name. A group with only ideas stands after the rest.
    let waiting: Vec<&crate::queued::QueuedIdea> = match scope {
        Scope::Active | Scope::All => ideas.iter().filter(|idea| query.trim().is_empty() || idea.text.to_lowercase().contains(&query.trim().to_lowercase())).collect(),
        _ => Vec::new(),
    };
    let group_of = |idea: &crate::queued::QueuedIdea| jobs.iter().find(|job| job.cwd == idea.root).map_or_else(|| idea.repository.clone(), project_of);
    for idea in &waiting {
        let group = group_of(idea);
        if !projects.iter().any(|(name, _)| *name == group) {
            projects.push((group, Vec::new()));
        }
    }
    let headed = projects.len() > 1;
    let mut lines = Vec::new();
    for (project, held) in projects {
        if headed {
            lines.push(Line::Project(project.clone().into()));
        }
        for idea in waiting.iter().filter(|idea| group_of(idea) == project) {
            lines.push(Line::Job(Row {
                id: format!("queued:{}", idea.id).into(),
                title: idea.title().to_string().into(),
                project: project.clone().into(),
                paused: false,
                ended: false,
                handing_off: false,
                earlier: None,
                depth: 0,
                started_by: None,
                slot: None,
                branch: None,
                pr: None,
                last: None,
                mark: None,
                issue: None,
                flow: None,
                now: None,
                queued: Some((*idea).clone()),
            }));
        }
        let listed = |id: &str| held.iter().any(|job| job.id.as_str() == id);
        let row_of = |job: &SessionRecord, depth: usize| Row {
            id: job.id.as_str().to_string().into(),
            title: title_of(job),
            project: project.clone().into(),
            paused: paused(job),
            ended: job.state == SessionState::Ended,
            handing_off: handing_off(job),
            earlier: spent(job, "replaces").map(|id| id.to_string().into()),
            depth,
            started_by: starter_of(job).and_then(|id| jobs.iter().find(|one| one.id.as_str() == id)).map(|one| (title_of(one), project_of(one).into())),
            slot: attached(job, "slot").map(|slot| slot.rsplit('/').next().unwrap_or(slot).to_string().into()),
            branch: attached(job, "branch").map(|branch| branch.to_string().into()),
            pr: attached(job, "pr").map(|pr| format!("#{pr}").into()),
            last: job.last_turn_at.as_ref().and_then(|at| age(at.as_str(), now)).map(Into::into),
            mark: job.mark,
            issue: job.attachments.iter().find(|one| one.kind == "issue").map(|one| {
                let number = if one.target.chars().all(|c| c.is_ascii_digit()) { format!("#{}", one.target) } else { one.target.clone() };
                let title = one.detail.get("title").cloned().unwrap_or_default();
                (number.into(), title.into())
            }),
            flow: flow_of(job),
            now: job.activity.as_ref().and_then(|activity| activity.now.clone().or_else(|| activity.said.clone())).or_else(|| job.so_far.clone()).map(Into::into),
            queued: None,
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
pub(crate) fn deck_file() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join("Library/Application Support/Armada/native-deck"))
}

fn read_deck() -> (Scope, Vec<String>) {
    let text = deck_file().and_then(|path| std::fs::read_to_string(path).ok()).unwrap_or_default();
    let mut lines = text.lines();
    let scope = Scope::of(lines.next().unwrap_or_default());
    (scope, lines.filter(|line| !line.is_empty()).map(str::to_string).collect())
}

/// Where the row's View is kept between runs: the id of each field that is on, one a line.
fn view_file() -> Option<PathBuf> {
    deck_file().map(|deck| deck.with_file_name("native-row-view"))
}

fn read_view() -> View {
    let Some(text) = view_file().and_then(|path| std::fs::read_to_string(path).ok()) else { return View::DEFAULT };
    View(std::array::from_fn(|at| text.lines().any(|line| line == VIEW_FIELDS[at].0)))
}

fn write_view(view: View) {
    let Some(path) = view_file() else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let text = VIEW_FIELDS.iter().filter(|(id, _)| view.on(id)).map(|(id, _)| *id).collect::<Vec<_>>().join("\n");
    if let Err(why) = std::fs::write(&path, text) {
        Note::warn("the row's view was not kept").field("why", why.to_string()).emit();
    }
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
pub(crate) fn send(port: Option<u16>, path: &'static str, body: Result<String, String>, cx: &mut App) {
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

/// A command with no body, sent as [`send`] sends one. A refusal is noted, and Fleet's next state is drawn.
pub(crate) fn send_bare(port: Option<u16>, path: &'static str, cx: &mut App) {
    let Some(port) = port else {
        Note::warn("not sent: no Fleet is connected").field("route", path).emit();
        return;
    };
    cx.background_executor()
        .spawn(async move {
            let answer = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|why| why.to_string())
                .and_then(|runtime| runtime.block_on(crate::transport::post_bare(port, path)));
            match answer {
                Ok((status, _)) if (200..300).contains(&status) => {}
                Ok((status, bytes)) => Note::warn("Fleet refused an act").field("route", path).field("status", status as i64).field("said", String::from_utf8_lossy(&bytes).to_string()).emit(),
                Err(why) => Note::warn("an act did not reach Fleet").field("route", path).field("why", why).emit(),
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
    /// What each row draws.
    view: View,
    /// Live Jobs in the order the person placed them, by id.
    order: Vec<String>,
    /// Whether the scope and the order are kept between runs: not on mock data.
    remember: bool,
    /// The Job the keys are on, by id, so a list that changes under it keeps it.
    focused: Option<SharedString>,
    /// The Job opened with Enter or a press.
    opened: Option<SharedString>,
    /// Counts each pick of a Job, the open one again too, so the view puts the keys in its box.
    picks: u64,
    focus: FocusHandle,
    /// The whole panel's, which holds the list and what it opens over itself, its menus.
    within: FocusHandle,
    scroll: ScrollHandle,
    /// Each drawn row's place in the window, for a menu opened from the keyboard and for its card.
    bounds: Rc<RefCell<HashMap<SharedString, Bounds<Pixels>>>>,
    menu: Option<Open>,
    /// The scope's menu while it is open.
    filter: Option<Open>,
    /// The search field over the groups. It needs a window, so it is made on the first draw.
    search: Option<(Entity<InputState>, Subscription)>,
    /// The Job whose card is open, and the wait before one opens.
    card: Option<SharedString>,
    _card_wait: Option<Task<()>>,
    fleet: Option<Entity<FleetConnection>>,
    _watch: Option<Subscription>,
    /// Titles asked for and not yet answered, by Job id: (the title Fleet last held, the title asked for).
    /// Every reader of a Job's title reads the record, so the head, the list, the review sheet and the
    /// palette cannot disagree while Fleet's answer is pending.
    asked: HashMap<String, (Option<String>, String)>,
    /// While a row is dragged: the row under the pointer, and whether it drops after it (`reorder.ts`).
    drop: Option<(SharedString, bool)>,
    /// The row being dragged, faded where it stood.
    lifted: Option<SharedString>,
    /// The ideas waiting to start (`crate::queued`), each a row under its repository's group.
    ideas: Vec<crate::queued::QueuedIdea>,
}

/// A Job's row while it is dragged to a new place in its project.
#[derive(Clone)]
struct DraggedRow {
    id: SharedString,
    title: SharedString,
}

/// What follows the pointer while a row is dragged: the row's title on a card.
struct RowGhost(SharedString);

impl Render for RowGhost {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        div()
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::ACCENT))
            .bg(look.color(token::BG_RAISED))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            .text_size(look.length(token::TEXT_SM))
            .text_color(look.color(token::FG_DEFAULT))
            .opacity(0.9)
            .child(self.0.clone())
    }
}

impl JobsList {
    /// The list on a live Fleet: it follows the Jobs the connection holds.
    pub fn live(fleet: &Entity<FleetConnection>, cx: &mut Context<Self>) -> Self {
        // A press on a clash mark opens the other Job, as a press on its row does; `main` has nothing to open.
        let opens = cx.entity().downgrade();
        cx.set_global(crate::clash_mark::OpenHolder(Rc::new(move |holder: &ipc::ClashHolder, _window, cx| {
            if holder.kind != ipc::ClashSide::Session {
                return;
            }
            let id = SharedString::from(holder.id.clone());
            let _ = opens.update(cx, |list, cx| list.open(id, cx));
        })));
        let watch = cx.observe(fleet, |this, fleet, cx| {
            // The title bar's picker lists what Fleet serves; the bar has no Fleet of its own to watch.
            if let Some(bar) = crate::titlebar::live(cx) {
                let (usage, downshift) = (fleet.read(cx).held().plan_usage.clone(), fleet.read(cx).held().downshift.clone());
                let build = fleet.read(cx).held().build.clone();
                let link = fleet.read(cx).held().link();
                bar.update(cx, |bar, cx| {
                    bar.set_fleet(&link, cx);
                    bar.set_usage(usage.as_ref(), downshift.as_ref(), cx);
                    bar.set_build(build.as_ref(), cx);
                });
            }
            // Every row's write-clash mark reads what Fleet last said; nothing while it cannot say.
            let clashes = fleet.read(cx).held().clashes.clone().unwrap_or_default();
            crate::clash_mark::hold(clashes, cx);
            if let (Some(served), Some(bar)) = (fleet.read(cx).held().repositories.clone(), crate::titlebar::live(cx)) {
                let projects = fleet.read(cx).held().projects.clone();
                bar.update(cx, |bar, cx| {
                    bar.set_repositories(&served, cx);
                    if let Some(projects) = &projects {
                        bar.set_projects(&served, projects, cx);
                    }
                });
            }
            let mut jobs = fleet.read(cx).held().jobs.clone();
            this.hold_asked_titles(&mut jobs);
            if jobs != this.jobs {
                this.jobs = jobs;
                this.place_new();
                this.relist(cx);
            }
        });
        let (scope, order) = read_deck();
        crate::queued::begin(true, cx);
        let jobs = fleet.read(cx).held().jobs.clone();
        Self::with(jobs, scope, order, read_view(), true, Some(fleet.clone()), Some(watch), cx)
    }

    /// The list on mock Jobs, which never change, with the View as a first start has it, so a walk shows what he will.
    pub fn mock(jobs: &[SessionRecord], cx: &mut Context<Self>) -> Self {
        crate::queued::begin(false, cx);
        Self::with(jobs.to_vec(), Scope::Active, Vec::new(), View::DEFAULT, false, None, None, cx)
    }

    fn with(
        jobs: Vec<SessionRecord>,
        scope: Scope,
        order: Vec<String>,
        view: View,
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
            view,
            order,
            remember,
            focused: None,
            opened: None,
            picks: 0,
            focus: cx.focus_handle(),
            within: cx.focus_handle(),
            scroll: ScrollHandle::new(),
            bounds: Rc::default(),
            menu: None,
            filter: None,
            search: None,
            card: None,
            _card_wait: None,
            fleet,
            _watch: watch,
            asked: HashMap::new(),
            drop: None,
            lifted: None,
            ideas: crate::queued::ideas(cx),
        };
        // The ideas waiting to start stand in the list as they change, from Dispatch or a row's own acts.
        cx.observe_global::<crate::queued::Queued>(|this, cx| {
            this.ideas = crate::queued::ideas(cx);
            this.relist(cx);
        })
        .detach();
        this.place_new();
        this.lines = this.lines_now();
        this.focused = this.first_row();
        this
    }

    /// Put each live Job the order has not met at its end, in the standing order it is drawn in,
    /// so no later change of state moves it (`store.ts`, a new one is added at the end).
    fn place_new(&mut self) {
        let fresh: Vec<String> = lines_of(&self.jobs, &[], Scope::All, "", &self.order)
            .into_iter()
            .filter_map(|line| match line {
                Line::Job(one) if !one.ended && !self.order.iter().any(|id| *id == *one.id) => Some(one.id.to_string()),
                _ => None,
            })
            .collect();
        if !fresh.is_empty() {
            self.order.extend(fresh);
            self.keep();
        }
    }

    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus
    }

    /// Open a Job from outside the list, as a press on its row does: the keys land on it and its card
    /// closes. A Job the list does not hold is not opened. Returns whether it was.
    pub fn open_job(&mut self, id: &str, cx: &mut Context<Self>) -> bool {
        if !self.jobs.iter().any(|one| one.id.as_str() == id) {
            return false;
        }
        let id: SharedString = id.to_string().into();
        self.focused = Some(id.clone());
        self.opened = Some(id);
        self.card = None;
        self.picks += 1;
        cx.notify();
        true
    }

    /// How many times a Job was picked: a press, Enter, the palette or a banner.
    pub fn picks(&self) -> u64 {
        self.picks
    }

    pub fn within(&self) -> &FocusHandle {
        &self.within
    }

    /// The Job the person opened, whose room the middle of the screen draws.
    pub fn opened_id(&self) -> Option<SharedString> {
        self.opened.clone()
    }

    /// Open a Job, as a press on its row does.
    pub fn open(&mut self, id: SharedString, cx: &mut Context<Self>) {
        if self.opened.as_ref() != Some(&id) {
            self.focused = Some(id.clone());
            self.opened = Some(id);
            self.card = None;
        }
        self.picks += 1;
        cx.notify();
    }

    /// The title a rename asked for stands on the Job at once. On a live Fleet it stands until Fleet's
    /// record moves off the old title (its echo, or another rename); on mock data nothing echoes, so the
    /// record takes it for good, as Fleet's would.
    pub fn retitle(&mut self, id: &str, title: String, cx: &mut Context<Self>) {
        let Some(held) = self.jobs.iter_mut().find(|job| job.id.as_str() == id) else { return };
        if self.fleet.is_some() {
            let was = self.asked.get(id).map_or_else(|| held.title.clone(), |(was, _)| was.clone());
            self.asked.insert(id.to_string(), (was, title.clone()));
        }
        held.title = Some(title);
        self.relist(cx);
        cx.notify();
    }

    /// Fleet refused the rename: the title it holds comes back.
    pub fn untitle(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some((was, _)) = self.asked.remove(id) else { return };
        if let Some(held) = self.jobs.iter_mut().find(|job| job.id.as_str() == id) {
            held.title = was;
        }
        self.relist(cx);
        cx.notify();
    }

    /// Put the titles still waiting on Fleet over the records it sent, and let go of those it has answered.
    fn hold_asked_titles(&mut self, jobs: &mut [SessionRecord]) {
        self.asked.retain(|id, (was, asked)| match jobs.iter_mut().find(|job| job.id.as_str() == id) {
            Some(job) if job.title == *was => {
                job.title = Some(asked.clone());
                true
            }
            _ => false,
        });
    }

    pub fn record(&self, id: &str) -> Option<&SessionRecord> {
        self.jobs.iter().find(|job| job.id.as_str() == id)
    }

    /// A walk's stand-in for Fleet's yes to a queued idea's start: a Job of its words, in its repository's group,
    /// stands in the list at once and opens, as a live start's record would.
    pub fn hold_started(&mut self, idea: &crate::queued::QueuedIdea, cx: &mut Context<Self>) {
        self.hold_new(format!("started-{}", idea.id), idea.title().to_string(), &idea.repository, None, cx);
    }

    /// A walk's stand-in for Fleet's yes to a start from the sheet: a Job of the words stands in the list and opens, working.
    pub fn hold_started_words(&mut self, words: &str, repository: &str, cx: &mut Context<Self>) {
        let title = words.lines().find(|line| !line.trim().is_empty()).map_or("", str::trim).to_string();
        let mark = ipc::JobMark { state: ipc::JobState::Working, lamp: ipc::JobLamp::Running, group: ipc::JobGroup::Working };
        self.hold_new("started-from-sheet".to_string(), title, repository, Some(mark), cx);
    }

    fn hold_new(&mut self, id: String, title: String, repository: &str, mark: Option<ipc::JobMark>, cx: &mut Context<Self>) {
        let like = self.jobs.iter().find(|job| project_of(job) == repository).or_else(|| self.jobs.first());
        let Some(mut record) = like.cloned() else { return };
        record.id = ipc::SessionId::carried(id);
        record.title = Some(title);
        record.state = SessionState::Live;
        record.attachments = Vec::new();
        record.activity = None;
        record.so_far = None;
        record.hosted = None;
        record.mark = mark;
        record.ended_at = None;
        let id: SharedString = record.id.as_str().to_string().into();
        self.jobs.insert(0, record);
        self.relist(cx);
        self.open(id, cx);
    }

    /// Every Job the list holds, for the message box's `@` to name.
    pub fn records(&self) -> &[SessionRecord] {
        &self.jobs
    }

    pub fn fleet(&self) -> Option<Entity<FleetConnection>> {
        self.fleet.clone()
    }

    /// Whether this list is kept between runs: not on mock data.
    pub fn remembers(&self) -> bool {
        self.remember
    }

    fn lines_now(&self) -> Vec<Line> {
        lines_of(&self.jobs, &self.ideas, self.scope, &self.query, &self.order)
    }

    /// The keys on the first Job that needs him, as the deck's `c` puts them (`keys-model.ts`, the
    /// `needs` slot). With none, they stay where they are.
    pub fn to_first_needing(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let first = self.lines.iter().enumerate().find_map(|(at, line)| match line {
            Line::Job(row) if row.mark.is_some_and(|mark| mark.group == JobGroup::Needs) => Some((at, row.id.clone())),
            _ => None,
        });
        if let Some((at, id)) = first {
            self.focused = Some(id);
            self.scroll.scroll_to_item(at);
        }
        window.focus(&self.focus, cx);
        cx.notify();
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
                    self.to_search(window, cx);
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
        if row.depth > 0 || row.ended || row.queued.is_some() {
            return;
        }
        let peers: Vec<SharedString> =
            self.rows().filter(|one| one.project == row.project && one.depth == 0 && !one.ended).map(|one| one.id.clone()).collect();
        let Some(at) = peers.iter().position(|one| *one == id) else { return };
        let Some(over) = at.checked_add_signed(by).and_then(|to| peers.get(to)).cloned() else { return };
        // Place every live Job, so the move holds whatever the list draws next.
        let mut placed: Vec<String> =
            lines_of(&self.jobs, &[], Scope::All, "", &self.order).iter().filter_map(|line| match line {
                Line::Job(one) if !one.ended => Some(one.id.to_string()),
                _ => None,
            }).collect();
        let (Some(from), Some(to)) = (placed.iter().position(|one| *one == *id), placed.iter().position(|one| *one == *over)) else { return };
        placed.swap(from, to);
        self.order = placed;
        self.keep();
        self.relist(cx);
    }

    /// A dragged row dropped before or after another of its project's top rows (`Deck.tsx` `onMove`).
    fn place(&mut self, id: &SharedString, target: &SharedString, after: bool, cx: &mut Context<Self>) {
        self.drop = None;
        self.lifted = None;
        let peer = |id: &SharedString| self.rows().find(|row| &row.id == id).filter(|row| row.depth == 0 && !row.ended).map(|row| row.project.clone());
        let (Some(from), Some(to)) = (peer(id), peer(target)) else { return cx.notify() };
        if id == target || from != to {
            return cx.notify();
        }
        let placed: Vec<String> =
            lines_of(&self.jobs, &[], Scope::All, "", &self.order).iter().filter_map(|line| match line {
                Line::Job(one) if !one.ended => Some(one.id.to_string()),
                _ => None,
            }).collect();
        let Some(placed) = moved(placed, id, target, after) else { return };
        self.order = placed;
        self.focused = Some(id.clone());
        self.keep();
        self.relist(cx);
    }

    fn on_act(&mut self, act: &Act, window: &mut Window, cx: &mut Context<Self>) {
        if act.is("move_focus") {
            // The registry's slots are j, k, ↓, ↑: even slots go down.
            self.step(if act.slot % 2 == 0 { 1 } else { -1 }, window, cx);
        } else if act.is("open_focused") {
            // An idea's row starts it: ↵ is the press with no pointer (`QueuedRow`).
            if let Some(idea) = self.focused_idea() {
                crate::dispatch::start_queued(idea, cx);
            } else if self.focused.is_some() {
                self.opened = self.focused.clone();
                self.picks += 1;
                cx.notify();
            }
        } else if act.is("deck_search") {
            self.to_search(window, cx);
        } else if act.is("deck_reorder") {
            // ⌥↑ then ⌥↓.
            self.shove(if act.slot == 0 { -1 } else { 1 }, cx);
        } else {
            cx.propagate();
        }
    }

    /// `p` while the rows have the keys pauses or resumes the row under them (`pauseForP`).
    pub(crate) fn pause_key(&mut self, event: &gpui::KeyDownEvent, window: &Window, cx: &mut Context<Self>) -> bool {
        if event.keystroke.key != "p" || event.keystroke.modifiers.modified() || !self.focus.is_focused(window) {
            return false;
        }
        let Some(row) = self.focused.as_ref().and_then(|id| self.rows().find(|row| &row.id == id)).filter(|row| !row.ended && row.queued.is_none()) else { return false };
        let session_id = ipc::SessionId::carried(row.id.to_string());
        let body = if row.paused {
            ("/sessions/resume", ipc::encode(&ipc::ResumeSession { session_id }))
        } else {
            ("/sessions/pause", ipc::encode(&ipc::PauseSession { session_id }))
        };
        send(self.port(cx), body.0, body.1.map_err(|why| format!("{why:?}")), cx);
        true
    }

    fn on_row_menu(&mut self, _: &RowMenu, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.focused.clone() else { return };
        // An idea's row has no menu, as Electron's has none.
        if self.focused_idea().is_some() {
            return;
        }
        let Some(bounds) = self.bounds.borrow().get(&id).copied() else { return };
        self.open_menu(id, under(bounds), true, window, cx);
    }

    /// The idea under the keys, where the focused row is one.
    fn focused_idea(&self) -> Option<crate::queued::QueuedIdea> {
        self.focused.as_ref().and_then(|id| self.rows().find(|row| &row.id == id)).and_then(|row| row.queued.clone())
    }

    /// `h`: the earlier Job behind the focused row, where it took one's place (`Deck.tsx`).
    fn on_earlier(&mut self, _: &Earlier, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(earlier) = self.focused.as_ref().and_then(|id| self.rows().find(|row| &row.id == id)).and_then(|row| row.earlier.clone()) else {
            cx.propagate();
            return;
        };
        self.open(earlier, cx);
    }

    /// The row's menu (`RowMenu.tsx`), at `at`: Pause or Resume, Name it for me, Try it in your app
    /// and Close. An ended Job has none of them, and draws no menu.
    pub(crate) fn open_menu(&mut self, id: SharedString, at: Point<Pixels>, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(row) = self.rows().find(|row| row.id == id).cloned() else { return };
        if row.ended {
            return;
        }
        self.card = None;
        let (pause, label, icon) = if row.paused { ("resume", "Resume", "play") } else { ("pause", "Pause", "circle-pause") };
        let mut entries = vec![
            Entry::Item { id: pause, label: label.into(), icon: Some(icon), danger: false, key: Some("P".into()), off: None },
            Entry::Act { id: "name", label: "Name it for me".into(), icon: Some("wand-sparkles"), danger: false },
            Entry::Act { id: "teach", label: "Teach".into(), icon: Some("graduation-cap"), danger: false },
            Entry::Act { id: "try", label: "Try it in your app".into(), icon: None, danger: false },
            Entry::Act { id: "close", label: "Close".into(), icon: Some("x"), danger: true },
            Entry::Rule,
        ];
        // The View group comes after the Job's own acts, then Mod and Report, as in `RowMenu.tsx`.
        entries.extend(self.view.entries());
        let menu = cx.new(|cx| Dropdown::new("Job", entries, at, Anchor::TopLeft, from_keys, cx).at_point(window, cx));
        let job = id.clone();
        let title = row.title.clone();
        let picked = cx.subscribe(&menu, move |this, _, picked: &Picked, cx| {
            let Picked::Entry { id: act, .. } = picked;
            let port = this.port(cx);
            let session_id = ipc::SessionId::carried(job.to_string());
            let encoded = |body: Result<String, ipc::Unencodable>| body.map_err(|why| format!("{why:?}"));
            match *act {
                "pause" => send(port, "/sessions/pause", encoded(ipc::encode(&ipc::PauseSession { session_id })), cx),
                "resume" => send(port, "/sessions/resume", encoded(ipc::encode(&ipc::ResumeSession { session_id })), cx),
                "name" => send(port, "/sessions/name", encoded(ipc::encode(&ipc::NameSession { session_id })), cx),
                "teach" => crate::dispatch::screen::teach_row(title.to_string(), cx),
                "try" => {
                    let pick = BuildAct::Pick(ipc::PickFleetBuildJob { branch: None, session_id: Some(job.to_string()), picked: true });
                    match this.fleet() {
                        Some(fleet) => fleet.update(cx, |fleet, cx| fleet.build_act(pick, cx)),
                        None => Note::warn("not sent: no Fleet is connected").field("route", "/fleet/build/pick").emit(),
                    }
                }
                "close" => send(port, "/sessions/close", encoded(ipc::encode(&ipc::CloseSession { session_id })), cx),
                field if field.starts_with("view:") => {
                    this.view.flip(field);
                    if this.remember {
                        write_view(this.view);
                    }
                    cx.notify();
                }
                _ => {}
            }
        });
        let dismissed = cx.subscribe_in(&menu, window, |this, _, _: &DismissEvent, window, cx| {
            this.menu = None;
            window.focus(&this.focus, cx);
            cx.notify();
        });
        // After the press is through: the list takes focus on a mouse down, after the row opened this.
        let keys = menu.read(cx).focus_handle(cx);
        window.defer(cx, move |window, cx| window.focus(&keys, cx));
        self.menu = Some(Open { menu, _picked: picked, _dismissed: dismissed });
        cx.notify();
    }

    /// The keys to the search field over the groups.
    pub(crate) fn to_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some((search, _)) = &self.search {
            search.update(cx, |input, cx| input.focus(window, cx));
            cx.notify();
        }
    }

    /// The keys from the search field to the first Job.
    fn to_rows(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.focused = self.first_row();
        window.focus(&self.focus, cx);
        cx.notify();
    }

    fn make_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.search.is_some() {
            return;
        }
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search Jobs"));
        let typed = cx.subscribe_in(&search, window, |this, search, event: &InputEvent, window, cx| match event {
            InputEvent::Change => {
                this.query = search.read(cx).value().to_string();
                this.relist(cx);
            }
            InputEvent::PressEnter { .. } => this.to_rows(window, cx),
            _ => {}
        });
        self.search = Some((search, typed));
    }

    /// The scope's menu, under its button, the keys on the scope the list holds now.
    fn open_scope(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.filter.is_some() {
            return;
        }
        let Some(bounds) = self.bounds.borrow().get("scope").copied() else { return };
        let gap = cx.look().length(token::SPACE_1);
        let at = point(bounds.origin.x + bounds.size.width, bounds.origin.y + bounds.size.height + gap);
        let entries = SCOPES.iter().map(|(scope, id, label)| Entry::Choice { id, label: (*label).into(), on: *scope == self.scope }).collect();
        let menu = cx.new(|cx| Dropdown::new("Scope", entries, at, Anchor::TopRight, true, cx));
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
        let keys = menu.read(cx).focus_handle(cx);
        window.defer(cx, move |window, cx| window.focus(&keys, cx));
        self.filter = Some(Open { menu, _picked: picked, _dismissed: dismissed });
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

    /// A Job's row as Electron's Cockpit list draws it (`ListCard.tsx`, `row-view.tsx`): the title,
    /// the time and the corner's marks on the first line, the issue and pull request chips and the
    /// lines under, each as the View has it. Rounder and roomier than
    /// Electron's strip, on the owner's word; the lamp is a bar inset at its left.
    /// An idea saved to start later (`QueuedRow`): dashed in the not-started hue, its words' first line, its
    /// repository and model, then Start and Remove. ↵ on it starts it; a press on its words only gives it the keys.
    fn render_queued(&self, at: usize, row: &Row, idea: &crate::queued::QueuedIdea, cx: &mut Context<Self>) -> gpui::AnyElement {
        let look = cx.look();
        let tip = crate::titlebar::parts::tip;
        let after_job = at > 0 && matches!(self.lines[at - 1], Line::Job(_));
        let focused = self.focused.as_ref() == Some(&row.id);
        let hue = look.color(token::STATUS_NOT_STARTED);
        let starting = cx.try_global::<crate::queued::Queued>().is_some_and(|queued| queued.starting.contains(&idea.id));
        let mono = look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_default();
        let tag = |id: String, said: String, kind: &'static str| {
            labelled(
                div()
                    .id(SharedString::from(id))
                    .flex_none()
                    .font_family(mono.clone())
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(look.color(token::FG_MUTED))
                    .child(said.clone())
                    .tooltip(tip(kind)),
                Role::Image,
                format!("{kind} {said}"),
            )
        };
        let act = |id: String, icon: &'static str, said: &'static str, name: String, off: bool, press: Box<dyn Fn(&mut App)>| {
            labelled(
                div()
                    .id(SharedString::from(id))
                    .flex_none()
                    .size(look.length(token::H_CONTROL_SM))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(look.length(token::RADIUS_MD))
                    .when(off, |button| button.opacity(0.5))
                    .when(!off, |button| button.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))))
                    .child(svg().path(icon_path(icon)).size(look.length(token::SPACE_4)).text_color(look.color(token::FG_MUTED)))
                    .tooltip(tip(said))
                    .on_click(move |_, _, cx| {
                        cx.stop_propagation();
                        press(cx);
                    }),
                Role::Button,
                name,
            )
        };
        let (start, remove, pressed) = (idea.clone(), idea.id.clone(), row.id.clone());
        let strip = div()
            .id(SharedString::from(format!("job-{}", row.id)))
            .flex_1()
            .min_w_0()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_1))
            .pl(look.length(token::SPACE_3))
            .pr(look.length(token::SPACE_1))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_dashed()
            .border_color(hue.opacity(0.55))
            .when(focused, |strip| strip.bg(look.color(token::BG_HOVER)))
            .hover(|style| style.bg(look.color(token::BG_HOVER)))
            .child(labelled(
                div()
                    .id(SharedString::from(format!("queued-kind-{}", idea.id)))
                    .flex_none()
                    .child(svg().path(icon_path("circle-dashed")).size(look.length(token::SPACE_4)).text_color(hue))
                    .tooltip(tip("Queued")),
                Role::Image,
                "Queued",
            ))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(row.title.clone()),
            )
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .child(tag(format!("queued-repo-{}", idea.id), idea.repository.clone(), "Repository"))
                    .children(idea.model.clone().map(|model| tag(format!("queued-model-{}", idea.id), model, "Model"))),
            )
            .child(act(
                format!("queued-start-{}", idea.id),
                "play",
                "Start",
                format!("Start {}", row.title),
                starting,
                Box::new(move |cx| crate::dispatch::start_queued(start.clone(), cx)),
            ))
            .child(act(
                format!("queued-remove-{}", idea.id),
                "trash",
                "Remove",
                format!("Remove {}", row.title),
                false,
                Box::new(move |cx| crate::queued::remove(&remove, cx)),
            ))
            .on_click(cx.listener(move |this, _event, window, cx| {
                this.focused = Some(pressed.clone());
                window.focus(&this.focus, cx);
                cx.notify();
            }));
        div()
            .flex_none()
            .flex()
            .when(after_job, |line| line.mt(look.length(token::SPACE_2)))
            .child(labelled(strip, Role::Row, row.title.clone()))
            .into_any_element()
    }

    fn render_job(&self, at: usize, row: &Row, cx: &mut Context<Self>) -> gpui::AnyElement {
        if let Some(idea) = row.queued.clone() {
            return self.render_queued(at, row, &idea, cx);
        }
        let look = cx.look();
        let view = self.view;
        let after_job = at > 0 && matches!(self.lines[at - 1], Line::Job(_));
        let focused = self.focused.as_ref() == Some(&row.id);
        // The row stands for the earlier Job too: it is open when that one is.
        let opened = self.opened.as_ref() == Some(&row.id) || (row.earlier.is_some() && self.opened == row.earlier);
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
        let lamp = match row.mark {
            Some(mark) => lamp_of(mark.lamp),
            None if row.ended => token::STATUS_COMPLETED_SUCCESS,
            None => token::BORDER_STRONG,
        };
        // Needs him, and not the Job open now: the row's tint breathes (`ListCard.css`).
        let needs = row.mark.is_some_and(|mark| mark.lamp == JobLamp::Needs) && !opened;
        let state = row.mark.filter(|_| !row.paused).map(|mark| (glyph_of(mark.state, row.pr.as_deref()), hue_of(mark.state)));
        let rail = look.length(token::BORDER_WIDTH) * 2.;
        let indent = look.length(token::SPACE_4) * row.depth as f32;
        let line = look.length(token::H_CONTROL_SM);
        let pad = look.length(if row.depth > 0 { token::SPACE_1 } else { token::SPACE_2 });
        let radius = look.length(token::RADIUS_LG);
        let mark_glyph = look.length(token::SPACE_3);
        let (pressed, menu_on, hover_on) = (row.id.clone(), row.id.clone(), row.id.clone());
        let store = Rc::clone(&self.bounds);
        let kept = row.id.clone();
        let linked = row.started_by.as_ref().map(|(title, project)| -> SharedString { format!("Linked Job, started by {title} in {project}").into() });
        let meta = |look: &Look| div().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_XS));
        let tip = |said: SharedString| move |window: &mut Window, cx: &mut App| gpui_component::tooltip::Tooltip::new(said.clone()).build(window, cx);
        let chip = |id: String, text: SharedString, said: SharedString| {
            meta(look)
                .id(SharedString::from(id))
                .flex_none()
                .px(look.length(token::SPACE_1))
                .rounded(look.length(token::RADIUS_SM))
                .bg(look.color(token::FG_DEFAULT).opacity(0.08))
                .text_color(look.color(token::FG_MUTED))
                .font_family(look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_default())
                .child(text)
                .tooltip(tip(said))
        };
        let issue = row.issue.clone().filter(|_| view.on("view:issue")).map(|(number, title)| {
            let said = if title.is_empty() { format!("Issue {number}") } else { format!("Issue {number}: {title}") };
            chip(format!("issue-{}", row.id), number, said.into())
        });
        let pr = row.pr.clone().filter(|_| view.on("view:pr")).map(|pr| chip(format!("pr-{}", row.id), pr.clone(), format!("Pull request {pr}").into()));
        let flow = row.flow.clone().filter(|_| view.on("view:workflow")).map(|(name, step)| {
            meta(look)
                .id(SharedString::from(format!("flow-{}", row.id)))
                .min_w_0()
                .truncate()
                .text_color(look.color(token::FG_MUTED))
                .child(step.clone())
                .tooltip(tip(format!("{name}: {step}").into()))
        });
        let now = row.now.clone().filter(|_| view.on("view:now")).map(|now| {
            meta(look).min_w_0().truncate().text_color(look.color(if row.paused { token::FG_SUBTLE } else { token::FG_MUTED })).child(now)
        });
        let time = row.last.clone().filter(|_| view.on("view:time")).map(|last| {
            meta(look)
                .id(SharedString::from(format!("time-{}", row.id)))
                .flex_none()
                .text_color(look.color(token::FG_SUBTLE))
                .child(last)
                .tooltip(tip("Last turn".into()))
        });
        let tagged = issue.is_some() || pr.is_some();
        let under = tagged || flow.is_some() || now.is_some();
        let lamp_ink = look.color(lamp);
        // The earlier Job's mark stands in the state's slot, beside the state, as the ring does on the
        // Job it replaced: one row never carries both.
        let mut earlier_mark = row.earlier.clone().map(|earlier| {
                labelled(
                    div()
                        .id(SharedString::from(format!("earlier-{}", row.id)))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(look.length(token::SPACE_4))
                        .rounded(look.length(token::RADIUS_SM))
                        .text_color(look.color(token::FG_MUTED))
                        .hover(|style| style.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)))
                        .child(svg().path(icon_path("layers")).size(mark_glyph).text_color(look.color(token::FG_MUTED)))
                        .tooltip(tip("Earlier Job".into()))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.open(earlier.clone(), cx);
                        })),
                    Role::Button,
                    "Earlier Job",
                )
                .into_any_element()
            });
        let status_on = view.on("view:status");
        let marks = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_1))
            .when(view.on("view:conflicts"), |marks| marks.child(crate::clash_mark::ClashMark::in_row(row.id.clone())))
            // The time stands right against the state; the glyphs end at the row's edge, so they
            // line up down the list.
            .children(time)
            .when(!status_on, |marks| marks.children(earlier_mark.take()))
            .when(status_on, |marks| {
                // The paused mark in place of the state's glyph, and a hand off's ring before it.
                // Electron's wide mark (`ListCard.css`): two glyphs and the gap between, the state at its end.
                let wide = look.length(token::SPACE_4) * 2. + look.length(token::SPACE_1);
                marks.child(div().flex_none().w(wide).flex().items_center().justify_end().gap(look.length(token::SPACE_1)).when(row.handing_off, |slot| {
                    slot.child(labelled(
                        div()
                            .id(SharedString::from(format!("handoff-{}", row.id)))
                            .child(crate::titlebar::parts::spinner(SharedString::from(format!("handing-{}", row.id)), look.length(token::SPACE_4), look.color(token::FG_MUTED), cx))
                            .tooltip(tip("Handing off".into())),
                        Role::Image,
                        "Handing off",
                    ))
                }).children(earlier_mark.take()).children(state.map(|((glyph, said), hue)| {
                    labelled(
                        div()
                            .id(SharedString::from(format!("state-{}", row.id)))
                            .child(svg().path(icon_path(glyph)).size(look.length(token::SPACE_4)).text_color(look.color(hue)))
                            .tooltip(tip(said.clone())),
                        Role::Image,
                        said,
                    )
                })).when(row.paused, |slot| {
                    slot.child(labelled(
                        div()
                            .id(SharedString::from(format!("paused-{}", row.id)))
                            .child(svg().path(icon_path("circle-pause")).size(mark_glyph).text_color(look.color(token::FG_MUTED)))
                            .tooltip(tip("Paused".into())),
                        Role::Image,
                        "Paused",
                    ))
                }))
            });
        // A top row of a live Job is dragged to a new place in its project, as Electron's deck does.
        let lit = crate::clash_mark::lit(cx) == Some(row.id.as_ref());
        let movable = row.depth == 0 && !row.ended;
        let dragging = cx.has_active_drag();
        let lifted = dragging && self.lifted.as_ref() == Some(&row.id);
        let drop_here = self.drop.as_ref().filter(|(at, _)| dragging && *at == row.id).map(|(_, after)| *after);
        let dragged = DraggedRow { id: row.id.clone(), title: row.title.clone() };
        let strip = div()
            .id(SharedString::from(format!("job-{}", row.id)))
            .when(movable, |strip| strip.on_drag(dragged, |dragged, _, _, cx| cx.new(|_| RowGhost(dragged.title.clone()))))
            .when(lifted, |strip| strip.opacity(0.55))
            .relative()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .py(pad)
            .pl(look.length(token::SPACE_3))
            .pr(look.length(token::SPACE_3))
            .rounded(radius)
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(edge))
            .bg(ground)
            .when(!opened, |strip| strip.hover(|style| style.bg(look.color(token::BG_HOVER))))
            // Lit as hovered while the pointer is on a clash's link to this Job (`data-hover`).
            .when(!opened && lit, |strip| strip.bg(look.color(token::BG_HOVER)))
            .child(canvas(move |bounds, _, _| { store.borrow_mut().insert(kept, bounds); }, |_, _, _, _| {}).absolute().top_0().left_0().size_full())
            .when(needs, |strip| {
                let tint = look.color(token::STATUS_AWAITING_REVIEW);
                strip.child(div().absolute().inset_0().rounded(radius).with_animation(
                    SharedString::from(format!("needs-{}", row.id)),
                    Animation::new(look.duration(token::DURATION_PULSE)).repeat().with_easing(pulsating_between(HULL, HULL * 2.)),
                    move |tinted, at| tinted.bg(tint.opacity(at)),
                ))
            })
            // A Job pulsing for you already carries its colour, and a wash under the pulse muddies it.
            .when(!opened && !needs, |strip| strip.child(div().absolute().inset_0().rounded(radius).bg(lamp_ink.opacity(HULL))))
            // The lamp's edge down the row's left side (`ListCard.css` `border-left`).
            // Painted as the card's own rounded shape clipped to its left edge, so the edge follows the
            // corners as a CSS `border-left` does instead of a straight bar poking past them.
            .child({
                let wide = look.length(token::SPACE_1);
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, _| {
                        let edge = gpui::Bounds { origin: bounds.origin, size: gpui::size(wide, bounds.size.height) };
                        let card = gpui::Bounds { origin: bounds.origin, size: gpui::size(bounds.size.width.min(radius * 2. + wide), bounds.size.height) };
                        window.with_content_mask(Some(gpui::ContentMask { bounds: edge }), |window| {
                            window.paint_quad(gpui::fill(card, lamp_ink).corner_radii(radius));
                        });
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full()
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .h(line)
                    .children(linked.clone().map(|said| {
                        div()
                            .id(SharedString::from(format!("link-{}", row.id)))
                            .flex_none()
                            .child(svg().path(icon_path("link")).size(mark_glyph).text_color(look.color(token::FG_SUBTLE)))
                            .tooltip(tip(said))
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
                    // The time keeps the title line on every row, beside the state, by the owner's
                    // choice, where Electron moves it to the foot of a row with lines.
                    .child(marks),
            )
            .when(under, |strip| {
                strip.child(
                    div()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_1))
                        .when(tagged, |lines| lines.child(div().flex().items_center().gap(look.length(token::SPACE_1)).children(issue).children(pr)))
                        .children(flow)
                        .children(now),
                )
            })
            .cursor_pointer()
            .on_hover(cx.listener(move |this, hovered: &bool, window, cx| this.hovered(hover_on.clone(), *hovered, window, cx)))
            .on_mouse_down(MouseButton::Right, cx.listener(move |this, event: &gpui::MouseDownEvent, window, cx| {
                this.focused = Some(menu_on.clone());
                this.open_menu(menu_on.clone(), event.position, false, window, cx);
            }))
            .on_click(cx.listener(move |this, _event, window, cx| {
                this.focused = Some(pressed.clone());
                this.opened = Some(pressed.clone());
                this.card = None;
                this.picks += 1;
                window.focus(&this.focus, cx);
                cx.notify();
            }));
        let name = match &linked {
            Some(said) => format!("{}, {said}", row.title).into(),
            None => row.title.clone(),
        };
        let (over_id, drop_id) = (row.id.clone(), row.id.clone());
        let line_edge = look.length(token::BORDER_WIDTH) * 2.;
        let gap_half = look.length(token::SPACE_1) / 2. + look.length(token::BORDER_WIDTH);
        div()
            .flex_none()
            .relative()
            .flex()
            .when(after_job, |line| line.mt(look.length(token::SPACE_2)))
            .pl(indent)
            .when(movable, |line| {
                line.on_drag_move(cx.listener(move |this, event: &gpui::DragMoveEvent<DraggedRow>, _, cx| {
                    if !event.bounds.contains(&event.event.position) {
                        return;
                    }
                    let after = event.event.position.y > event.bounds.center().y;
                    let lifted = event.drag(cx).id.clone();
                    if this.drop.as_ref() != Some(&(over_id.clone(), after)) || this.lifted.as_ref() != Some(&lifted) {
                        this.drop = Some((over_id.clone(), after));
                        this.lifted = Some(lifted);
                        cx.notify();
                    }
                }))
                .on_drop(cx.listener(move |this, dragged: &DraggedRow, _, cx| {
                    let after = this.drop.as_ref().filter(|(at, _)| *at == drop_id).is_some_and(|(_, after)| *after);
                    this.place(&dragged.id, &drop_id, after, cx);
                }))
            })
            // The drop line, before or after the row under the pointer (`deck.css` `[data-drop]`).
            .children(drop_here.map(|after| {
                labelled(
                    div()
                        .id("drop-line")
                        .absolute()
                        .left(indent)
                        .right_0()
                        .h(line_edge)
                        .rounded(look.length(token::RADIUS_SM))
                        .bg(look.color(token::ACCENT))
                        .map(|bar| if after { bar.bottom(-gap_half) } else { bar.top(-gap_half) }),
                    Role::Image,
                    "Lands here",
                )
            }))
            .when(row.depth > 0, |row_line| {
                // The elbow from the Job above to the middle of this one's first line.
                row_line.child(
                    div()
                        .flex_none()
                        .w(look.length(token::SPACE_2))
                        .h(line / 2. + pad + look.length(token::BORDER_WIDTH))
                        .ml(-look.length(token::SPACE_2) - look.length(token::SPACE_1))
                        .mr(look.length(token::SPACE_1))
                        .border_l(rail)
                        .border_b(rail)
                        .rounded_bl(look.length(token::RADIUS_MD))
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
                        .when(at > 0, |head| head.mt(look.length(token::SPACE_4)))
                        .mb(look.length(token::SPACE_2))
                        .px(look.length(token::SPACE_2))
                        .border_b(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_SUBTLE))
                        .font_family(look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_default())
                        // `--cb-code`: four fifths of the smallest text.
                        .text_size(look.length(token::TEXT_2XS) * 0.8)
                        .line_height(look.length(token::LEADING_XS))
                        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                        .text_color(look.color(token::FG_MUTED))
                        .child(name.clone()),
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

    /// The bar over every group: the search field, and the scope it searches in.
    fn render_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let store = Rc::clone(&self.bounds);
        let label = self.scope.label();
        let ink = if self.scope == Scope::Active { token::FG_MUTED } else { token::ACCENT };
        let field = self.search.as_ref().map(|(search, _)| {
            labelled(
                div().id("search").key_context(crate::keymap::TEXT_BOX).flex_1().min_w_0().child(
                    Input::new(search)
                        .h(look.length(token::H_CONTROL_LG))
                        .rounded(look.length(token::RADIUS_LG))
                        .prefix(svg().path(icon_path("search")).size(look.length(token::SPACE_4)).text_color(look.color(token::FG_SUBTLE))),
                ),
                Role::SearchInput,
                "Search Jobs",
            )
        });
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_3))
            .pt(look.length(token::SPACE_3))
            .pb(look.length(token::SPACE_2))
            // ↓, Enter or Esc give the keys to the rows; Tab opens the scope.
            .capture_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| match event.keystroke.key.as_str() {
                "down" | "escape" => {
                    cx.stop_propagation();
                    this.to_rows(window, cx);
                }
                "tab" => {
                    cx.stop_propagation();
                    this.open_scope(window, cx);
                }
                _ => {}
            }))
            .children(field)
            .child(labelled(
                div()
                    .id("scope")
                    .relative()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .h(look.length(token::H_CONTROL_LG))
                    .px(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_LG))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_DEFAULT))
                    .bg(look.color(token::BG_SUNKEN))
                    .cursor_pointer()
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .text_size(look.length(token::TEXT_SM))
                    .text_color(look.color(ink))
                    .child(canvas(move |bounds, _, _| { store.borrow_mut().insert("scope".into(), bounds); }, |_, _, _, _| {}).absolute().size_full())
                    .child(label)
                    .child(svg().path(icon_path("chevron-down")).size(look.length(token::SPACE_3)).text_color(look.color(ink)))
                    .tooltip(|window, cx| gpui_component::tooltip::Tooltip::new("Which Jobs the list holds").build(window, cx))
                    .on_click(cx.listener(|this, _, window, cx| this.open_scope(window, cx))),
                Role::Button,
                label,
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
        self.make_search(window, cx);
        let lines: Vec<_> = (0..self.lines.len()).map(|at| self.render_line(at, cx)).collect();
        let bar = self.render_bar(cx);
        let card = self.render_card(cx);
        let look = cx.look();
        div()
            .track_focus(&self.within)
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .child(bar)
            .child(
                div()
                    .relative()
                    .w_full()
                    .flex_1()
                    .min_h_0()
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
                    .p(look.length(token::SPACE_3))
                    .font_family(look.font_stack(token::FONT_SANS).into_iter().next().unwrap_or_default())
                    .children(lines),
                Role::List,
                "Jobs",
            ))
                    // Rows pass under the bar: the panel's ground fades in over them at the top.
                    .child({
                        let ground = look.color(token::BG_RAISED);
                        div().absolute().top_0().left_0().right_0().h(look.length(token::SPACE_6)).bg(gpui::linear_gradient(
                            180.,
                            gpui::linear_color_stop(ground, 0.),
                            gpui::linear_color_stop(ground.opacity(0.), 1.),
                        ))
                    }),
            )
            .children(card)
            .children(self.menu.as_ref().map(|open| open.menu.clone()))
            .children(self.filter.as_ref().map(|open| open.menu.clone()))
    }
}

/// The list's column, in `--space-12` as Electron's `columns.ts` sizes it: where it rests, the
/// narrowest it goes, the right column beside it, and the least the centre keeps.
const REST_LEFT: f32 = 6.;
const FLOOR_LEFT: f32 = 4.5;
const CENTRE_FLOOR: f32 = 10.;
/// What ← or → on the handle moves it by, in points (`DOCK_WIDTH_STEP`).
const HANDLE_STEP: f32 = 16.;

/// Where the list's dragged width is kept between runs, in points. No file is the resting width.
fn columns_file() -> Option<PathBuf> {
    deck_file().map(|deck| deck.with_file_name("native-deck-columns"))
}

/// The handle being dragged, and what the pointer carries while it is: nothing to draw.
struct DeckHandle;

impl Render for DeckHandle {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        gpui::Empty
    }
}

/// The thread's share of the Job beside its right column, as Electron's inner split draws it (0.62 to
/// 0.38), and the least a pane needs (`MIN_PANE_W`).
const SPLIT_ARRIVES: f32 = 0.62;
const PANE_LEAST: f32 = 160.;

/// The Jobs screen in Electron's regions (`cockpit/deck/Deck.tsx`, `.deck-grid`): the list's panel at
/// the left, the open Job's room in the middle, and the right column's panel, under the title row.
/// A handle on the list's inner edge drags it wider or narrower, as Electron's `DockHandle` does.
pub struct JobsScreen {
    pub list: Entity<JobsList>,
    /// The open Job, in the middle.
    view: Entity<JobView>,
    /// The open Job's right column: Open, Done and Deferred over Now.
    side: Entity<JobSide>,
    /// The list's width as dragged; absent, it rests at Electron's width.
    left: Option<f32>,
    handle: FocusHandle,
    /// The whole screen's, which holds the list and the handle beside it.
    within: FocusHandle,
    remember: bool,
    /// The command palette, over the whole screen.
    palette: Entity<crate::palette::Palette>,
}

impl JobsScreen {
    pub fn new(
        list: Entity<JobsList>,
        threads: Option<HashMap<String, ipc::JobThread>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let remember = list.read(cx).remember;
        let view = cx.new(|cx| JobView::new(list.clone(), threads, window, cx));
        let side = cx.new(|cx| JobSide::new(view.clone(), cx));
        // A zoom or a closed pane changes which halves of the split this screen draws.
        let shows = std::cell::Cell::new(view.read(cx).split_shows());
        cx.observe(&view, move |_, view, cx| {
            if shows.replace(view.read(cx).split_shows()) != shows.get() {
                cx.notify();
            }
        })
        .detach();
        let left = remember
            .then(|| columns_file().and_then(|path| std::fs::read_to_string(path).ok()).and_then(|text| text.trim().parse::<f32>().ok()))
            .flatten();
        let palette = cx.new(|cx| crate::palette::Palette::new(list.clone(), cx));
        // The bar's search, and ⌘K where no screen took it, open this palette.
        let handle = window.window_handle();
        let opens = palette.downgrade();
        let fleet_of = list.downgrade();
        cx.set_global(crate::titlebar::AskedSink(Rc::new(move |asked: &crate::titlebar::Asked, cx: &mut App| {
            // The bar's build switcher: each press is Fleet's build act, followed until the report settles.
            let act = match asked {
                crate::titlebar::Asked::ChangeBuild { build, adopt } => Some(BuildAct::Change(ipc::ChangeFleetBuild { build: *build, adopt: *adopt })),
                crate::titlebar::Asked::Revert => Some(BuildAct::Revert),
                crate::titlebar::Asked::Keep => Some(BuildAct::Keep),
                crate::titlebar::Asked::Swap => Some(BuildAct::Swap),
                crate::titlebar::Asked::Unpick(branch) => Some(BuildAct::Pick(ipc::PickFleetBuildJob { branch: Some(branch.clone()), session_id: None, picked: false })),
                _ => None,
            };
            if let Some(act) = act {
                match fleet_of.upgrade().and_then(|list| list.read(cx).fleet()) {
                    Some(fleet) => fleet.update(cx, |fleet, cx| fleet.build_act(act, cx)),
                    None => Note::warn("not sent: no Fleet is connected").field("route", "/fleet/build").emit(),
                }
                return;
            }
            // Restart Fleet goes to launchd, never to Fleet: the Fleet it restarts is the one the runtime file names.
            if *asked == crate::titlebar::Asked::RestartFleet {
                let Some(fleet) = fleet_of.upgrade().and_then(|list| list.read(cx).fleet()) else {
                    Note::warn("not restarted: this window has no Fleet connection").emit();
                    return;
                };
                let pid = match &fleet.read(cx).held().presence {
                    Some((crate::runtime_file::Presence::Running(file), _)) => Some(file.pid),
                    _ => None,
                };
                let ran = cx.background_executor().spawn(async move { crate::restart_fleet::restart(pid) });
                cx.spawn(async move |cx| {
                    if let Err((why, detail)) = ran.await {
                        Note::warn("Fleet was not restarted").field("why", why.word()).field("detail", detail.as_str()).emit();
                        let _ = cx.update(|cx| {
                            if let Some(bar) = crate::titlebar::live(cx) {
                                bar.update(cx, |bar, cx| bar.restart_failed(why, detail, cx));
                            }
                        });
                    }
                })
                .detach();
                return;
            }
            // The step down's acts, each its own command; Choose settles it, as Electron's does before the model setting.
            if let crate::titlebar::Asked::Remedy(remedy) = asked {
                let port = fleet_of.upgrade().and_then(|list| list.read(cx).fleet()).and_then(|fleet| fleet.read(cx).held().port());
                let path = match remedy {
                    crate::titlebar::Remedy::Accept => "/model_downshift/accept",
                    crate::titlebar::Remedy::Restore => "/model_downshift/restore",
                    crate::titlebar::Remedy::Keep => "/model_downshift/keep",
                    crate::titlebar::Remedy::Choose => "/model_downshift/settle",
                };
                send_bare(port, path, cx);
                return;
            }
            if *asked != crate::titlebar::Asked::Palette {
                return;
            }
            let opens = opens.clone();
            let _ = handle.update(cx, move |_, window, cx| {
                if let Some(palette) = opens.upgrade() {
                    palette.update(cx, |palette, cx| palette.open(window, cx));
                }
            });
        })));
        JobsScreen {
            list,
            view,
            side,
            left,
            handle: cx.focus_handle(),
            within: cx.focus_handle(), remember, palette }
    }

    pub fn within(&self) -> &FocusHandle {
        &self.within
    }

    /// The room the three columns share: the window less its padding and the gaps between them.
    fn room(window: &Window, look: &Look) -> f32 {
        let frame = f32::from(look.length(token::SPACE_3)) * 2. + f32::from(look.length(token::GAP_PANEL)) * 2.;
        f32::from(window.viewport_size().width) - frame
    }

    /// The narrowest and widest the list may be in this window, so the centre keeps its floor. The Job's
    /// own panes are the centre and there is no right column, as Electron's `session` layout draws it
    /// (`NO_RIGHT`): the list may take its room and gap.
    fn range(&self, window: &Window, cx: &App) -> (f32, f32) {
        let look = cx.look();
        let unit = f32::from(look.length(token::SPACE_12));
        let least = unit * FLOOR_LEFT;
        let beside = -f32::from(look.length(token::GAP_PANEL));
        (least, (Self::room(window, look) - beside - unit * CENTRE_FLOOR).max(least))
    }

    fn width(&self, window: &Window, cx: &App) -> f32 {
        let (least, most) = self.range(window, cx);
        self.left.unwrap_or(f32::from(cx.look().length(token::SPACE_12)) * REST_LEFT).clamp(least, most)
    }

    fn resize(&mut self, to: Option<f32>, window: &Window, cx: &mut Context<Self>) {
        let (least, most) = self.range(window, cx);
        self.left = to.map(|to| to.clamp(least, most));
        if self.remember {
            if let Some(path) = columns_file() {
                let kept = match self.left {
                    Some(left) => std::fs::write(&path, format!("{left:.0}")),
                    None => std::fs::remove_file(&path).or_else(|why| if why.kind() == std::io::ErrorKind::NotFound { Ok(()) } else { Err(why) }),
                };
                if let Err(why) = kept {
                    Note::warn("the list's width was not kept").field("why", why.to_string()).emit();
                }
            }
        }
        cx.notify();
    }

    /// Teach from a list row: the open Job's Teach box, holding what the row said.
    pub(crate) fn teach_row(&mut self, text: String, window: &mut Window, cx: &mut Context<Self>) {
        self.view.update(cx, |view, cx| view.teach_row(text, window, cx));
    }

    /// `l` puts the keys in the list (`deck_panel`'s first slot), `c` on the first Job that needs
    /// him, `d` opens the dispatch sheet, and `/` the list's search, from anywhere on the deck.
    /// The bays, the queue, the flight controls, the timeline and the Field are not drawn, so `a`,
    /// `b`, `q`, `f`, `t` and `e` reach nothing.
    fn on_act(&mut self, act: &Act, window: &mut Window, cx: &mut Context<Self>) {
        if act.is("deck_panel") && act.slot == 0 {
            let focus = self.list.read(cx).focus_handle().clone();
            window.focus(&focus, cx);
        } else if act.is("deck_panel") && act.slot == 2 {
            self.list.update(cx, |list, cx| list.to_first_needing(window, cx));
        } else if act.is("deck_panel") && act.slot == 6 {
            crate::dispatch::screen::open_dispatch(cx);
        } else if act.is("deck_search") {
            self.list.update(cx, |list, cx| list.to_search(window, cx));
        } else if act.is("command_palette") {
            self.palette.update(cx, |palette, cx| palette.open(window, cx));
        } else if act.is("message_job") {
            // ⌥M from anywhere on the deck puts the caret in the open Job's box.
            self.view.update(cx, |view, cx| view.to_composer(window, cx));
        } else {
            cx.propagate();
        }
    }

    /// The handle on the list's inner edge (`dock-handle.tsx`): a quiet line that shows on hover,
    /// focus or drag; ← and → move it, a double press puts the list back at its resting width.
    fn render_handle(&self, width: f32, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (least, most) = self.range(window, cx);
        let look = cx.look();
        let line = look.color(token::BORDER_DEFAULT);
        let focused = self.handle.is_focused(window);
        labelled(
            div()
                .id("deck-handle")
                .track_focus(&self.handle)
                .absolute()
                .top_0()
                .bottom_0()
                .right(-look.length(token::GAP_PANEL) / 2. - look.length(token::SPACE_1))
                .w(look.length(token::SPACE_2))
                .flex()
                .justify_center()
                .cursor(gpui::CursorStyle::ResizeLeftRight)
                .child(
                    div()
                        .h_full()
                        .w(look.length(token::BORDER_WIDTH) * 2.)
                        .rounded_full()
                        .when(focused, |mark| mark.bg(line))
                        .group_hover("deck-handle", move |style| style.bg(line)),
                )
                .group("deck-handle")
                .on_drag(DeckHandle, |_, _, _, cx| cx.new(|_| DeckHandle))
                .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| {
                    window.focus(&this.handle, cx);
                    cx.notify();
                }))
                .on_click(cx.listener(|this, event: &gpui::ClickEvent, window, cx| {
                    if event.click_count() == 2 {
                        this.resize(None, window, cx);
                    }
                }))
                .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                    let by = match event.keystroke.key.as_str() {
                        "left" => -HANDLE_STEP,
                        "right" => HANDLE_STEP,
                        _ => return,
                    };
                    cx.stop_propagation();
                    this.resize(Some((width + by).clamp(least, most)), window, cx);
                })),
            Role::Splitter,
            "Resize the Jobs list",
        )
    }
}

/// A deck panel's frame (`.deck-panel`): the raised ground, a heavier top edge.
fn panel(look: &Look) -> gpui::Div {
    div()
        .h_full()
        .flex_none()
        .bg(look.color(token::BG_RAISED))
        .rounded(look.length(token::RADIUS_LG))
        .border(look.length(token::BORDER_WIDTH))
        .border_t(look.length(token::BORDER_WIDTH) * 2.)
        .border_color(look.color(token::BORDER_DEFAULT))
}

impl Render for JobsScreen {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let width = self.width(window, cx);
        let handle = self.render_handle(width, window, cx);
        let title = crate::chrome::title_row(window, cx);
        let look = cx.look();
        let pad = look.length(token::SPACE_3);
        let look = cx.look();
        // A zoomed pane covers the Job's own area, right of the list (`zoom.ts`); a closed pane gives its room up.
        let (thread, side) = self.view.read(cx).split_shows();
        div()
            .key_context("deck")
            .track_focus(&self.within)
            .on_action(cx.listener(Self::on_act))
            // A chord the person gave a palette row runs that row from anywhere on the screen; Tab reaches every tab stop.
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                if this.palette.update(cx, |palette, cx| palette.hotkey(&event.keystroke, window, cx))
                    || this.list.update(cx, |list, cx| list.pause_key(event, window, cx))
                    || this.view.update(cx, |view, cx| view.tile_key(event, window, cx))
                    || crate::keymap::traverse(event, window, cx)
                {
                    cx.stop_propagation();
                }
            }))
            .size_full()
            .flex()
            .flex_col()
            .bg(look.color(token::BG_BASE))
            .child(title)
            .child(
                div()
                    .id("deck-grid")
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .gap(look.length(token::GAP_PANEL))
                    .p(pad)
                    // The list's edge follows the pointer while its handle is dragged.
                    .on_drag_move(cx.listener(move |this, event: &gpui::DragMoveEvent<DeckHandle>, window, cx| {
                        let to = f32::from(event.event.position.x - event.bounds.origin.x - pad);
                        this.resize(Some(to), window, cx);
                    }))
                    .child(div().relative().h_full().flex_none().child(panel(look).w(gpui::px(width)).child(self.list.clone())).child(handle))
                    .child(
                        // The Job's own split, as Electron's tree draws it: the thread and the right column, 0.62 to 0.38.
                        div()
                            .id("job-split")
                            .h_full()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .when(thread && side, |split| split.gap(look.length(token::GAP_PANEL)))
                            .child(
                                // Covered by a zoomed pane of the column, the thread's place keeps no room but stays drawn,
                                // so the sheets and dialogs the Job view lays over the window still show.
                                panel(look)
                                    .min_w_0()
                                    .overflow_hidden()
                                    .map(|place| match (thread, side) {
                                        (true, true) => place.flex_grow(SPLIT_ARRIVES * 100.).flex_shrink(1.).flex_basis(gpui::px(0.)).min_w(gpui::px(PANE_LEAST)),
                                        (true, false) => place.flex_1(),
                                        (false, _) => place.flex_none().w_0().border_0(),
                                    })
                                    .child(self.view.clone()),
                            )
                            .when(side, |split| {
                                split.child(
                                    div()
                                        .h_full()
                                        .min_w_0()
                                        .map(|column| if thread { column.flex_grow((1. - SPLIT_ARRIVES) * 100.).flex_shrink(1.).flex_basis(gpui::px(0.)).min_w(gpui::px(PANE_LEAST)) } else { column.flex_1() })
                                        .child(self.side.clone()),
                                )
                            }),
                    ),
            )
            .child(crate::merge_strip::strip(&self.list, window, cx))
            .child(self.palette.clone())
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn a_dropped_row_stands_before_or_after_the_one_it_was_dropped_on() {
        let order = |ids: &[&str]| ids.iter().map(|id| id.to_string()).collect::<Vec<_>>();
        let list = order(&["a", "b", "c", "d"]);
        assert_eq!(moved(list.clone(), "d", "b", false), Some(order(&["a", "d", "b", "c"])));
        assert_eq!(moved(list.clone(), "a", "c", true), Some(order(&["b", "c", "a", "d"])));
        assert_eq!(moved(list.clone(), "b", "d", true), Some(order(&["a", "c", "d", "b"])));
        assert_eq!(moved(list, "x", "b", false), None);
    }

    use super::*;

    fn jobs() -> Vec<SessionRecord> {
        crate::mock::jobs("jobs-many").expect("the fixture decodes")
    }

    fn titles(lines: &[Line]) -> Vec<String> {
        lines.iter().filter_map(|line| match line { Line::Job(row) => Some(row.title.to_string()), Line::Project(_) => None }).collect()
    }

    #[gpui::test]
    fn a_job_opened_from_outside_is_the_one_the_keys_are_on(cx: &mut gpui::TestAppContext) {
        let all = jobs();
        let live = all.iter().find(|job| job.state == ipc::SessionState::Live).expect("a live Job").id.as_str().to_string();
        let list = cx.new(|cx| JobsList::mock(&all, cx));
        assert!(list.update(cx, |list, cx| list.open_job(&live, cx)));
        list.read_with(cx, |list, _| {
            assert_eq!(list.opened.as_ref().map(|id| id.as_ref()), Some(live.as_str()));
            assert_eq!(list.focused, list.opened);
        });
        assert!(!list.update(cx, |list, cx| list.open_job("no-such-job", cx)), "a Job the list does not hold is not opened");
        list.read_with(cx, |list, _| assert_eq!(list.opened.as_ref().map(|id| id.as_ref()), Some(live.as_str())));
    }

    #[test]
    fn ended_jobs_leave_the_list_and_each_project_heads_its_own() {
        let lines = lines_of(&jobs(), &[], Scope::Active, "", &[]);
        assert_eq!(lines[0], Line::Project("armada".into()));
        assert!(!titles(&lines).iter().any(|title| title.contains("ended")));
        let projects: Vec<_> = lines.iter().filter_map(|line| match line { Line::Project(name) => Some(name.clone()), _ => None }).collect();
        assert_eq!(projects, vec![SharedString::from("armada"), "sample-web".into(), "sample-api".into()]);
    }

    #[test]
    fn one_project_draws_its_jobs_with_no_head() {
        let armada: Vec<_> = jobs().into_iter().filter(|job| job.manifest_id.as_ref().is_some_and(|id| id.as_str() == "armada")).collect();
        let lines = lines_of(&armada, &[], Scope::Active, "", &[]);
        assert!(!lines.is_empty());
        assert!(lines.iter().all(|line| matches!(line, Line::Job(_))));
    }

    #[test]
    fn the_placed_order_holds_and_a_search_narrows_by_title() {
        let all = jobs();
        let second = titles(&lines_of(&all, &[], Scope::Active, "", &[]))[1].clone();
        let id = all.iter().find(|job| job.title.as_deref() == Some(second.as_str())).map(|job| job.id.as_str().to_string()).expect("listed");
        assert_eq!(titles(&lines_of(&all, &[], Scope::Active, "", &[id]))[0], second);
        let found = titles(&lines_of(&all, &[], Scope::Active, &second[..6].to_uppercase(), &[]));
        assert!(found.contains(&second) && found.iter().all(|title| title.to_lowercase().contains(&second[..6].to_lowercase())));
    }

    #[test]
    fn jobs_not_yet_placed_stand_needs_first_and_the_loudest_first() {
        let lines = lines_of(&jobs(), &[], Scope::Active, "", &[]);
        let web: Vec<_> = titles(&lines).into_iter().filter(|title| ["Checkout", "Flaky", "Upgrade", "Dark"].iter().any(|word| title.starts_with(word))).collect();
        assert_eq!(web, ["Checkout page loads twice", "Flaky login test", "Upgrade the router", "Dark mode for the settings page"]);
        // A placed Job keeps its place whatever its state.
        let dark = jobs().into_iter().find(|job| job.title.as_deref() == Some("Dark mode for the settings page")).expect("listed");
        let placed = titles(&lines_of(&jobs(), &[], Scope::Active, "", &[dark.id.as_str().to_string()]));
        assert_eq!(placed.iter().find(|title| web.contains(title)).map(String::as_str), Some("Dark mode for the settings page"));
    }

    #[test]
    fn a_quiet_job_leaves_active_for_the_quiet_scope() {
        let quiet = titles(&lines_of(&jobs(), &[], Scope::Quiet, "", &[]));
        assert_eq!(quiet, ["Retry webhooks on a 503"]);
        assert!(!titles(&lines_of(&jobs(), &[], Scope::Active, "", &[])).contains(&quiet[0]));
    }

    #[test]
    fn the_ended_scope_holds_only_ended_jobs() {
        let ended = titles(&lines_of(&jobs(), &[], Scope::Ended, "", &[]));
        assert!(!ended.is_empty() && ended.iter().all(|title| title.contains("ended")), "{ended:?}");
    }

    #[test]
    fn a_row_carries_its_issue_its_workflow_step_and_what_it_is_doing() {
        let lines = lines_of(&jobs(), &[], Scope::Active, "", &[]);
        let row = |title: &str| {
            lines.iter().find_map(|line| match line {
                Line::Job(row) if row.title == title => Some(row.clone()),
                _ => None,
            }).expect(title)
        };
        let first = row("Fix the merge line freshness chip");
        assert_eq!(first.issue, Some(("#3290".into(), "Merge line freshness chip stays green when stale".into())));
        assert_eq!(first.flow, Some(("Feature".into(), "Walk".into())));
        assert_eq!(first.now.as_deref(), Some("Asked which chip colour the stale state takes"));
        // What it is doing now comes before what it last summed up.
        assert_eq!(row("Native Bridge foundation").now.as_deref(), Some("Editing jobs.rs"));
    }

    #[test]
    fn the_view_group_lists_every_field_and_a_switch_flips_one() {
        let mut view = View::DEFAULT;
        assert!(view.on("view:status") && view.on("view:conflicts") && !view.on("view:now"));
        view.flip("view:now");
        assert!(view.on("view:now"));
        let entries = view.entries();
        assert!(matches!(entries[0], Entry::Label("View")));
        assert_eq!(entries.len(), VIEW_FIELDS.len() + 1);
    }
}
