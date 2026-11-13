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

actions!(jobs, [RowMenu, Earlier, FoldRow, UnfoldRow]);

/// The list's keys the registry does not carry yet: Shift+F10 for a row's menu, as Electron binds
/// it in `Deck.tsx`, and `h` for the Job this one took over from, whose registry row waits on the look.
/// ← and → fold and unfold a parent's sub-Jobs; Electron's list does not fold, so they have no registry row.
pub fn bind(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("shift-f10", RowMenu, Some("list")),
        KeyBinding::new("h", Earlier, Some("list")),
        KeyBinding::new("left", FoldRow, Some("list")),
        KeyBinding::new("right", UnfoldRow, Some("list")),
    ]);
}

/// Which Jobs the list holds (`cockpit/deck/store.ts`, `SCOPES`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// The Jobs that need him now: the lamp Fleet reads as Needs, the set Needs you and the Dock count follow.
    Needs,
    Active,
    Paused,
    /// The Jobs he snoozed, and only those: Needs you and Active leave them out until the snooze clears.
    Snoozed,
    Quiet,
    Ended,
    All,
}

const SCOPES: [(Scope, &str, &str); 7] = [
    (Scope::Needs, "needs", "Needs you"),
    (Scope::Active, "active", "Active"),
    (Scope::Paused, "paused", "Paused"),
    (Scope::Snoozed, "snoozed", "Snoozed"),
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
    /// Kept quiet until this time: it works on and raises nothing to him (`snoozed_until`).
    snoozed: Option<chrono::DateTime<chrono::Local>>,
    ended: bool,
    /// Asked to carry on in a new Job and told yes: a turning ring until the new one is up.
    handing_off: bool,
    /// Set where Jobs stand under this one: whether they are folded away, and what the fold keeps in view.
    kids: Option<Kids>,
    /// The earlier Job this one took the place of, which the list does not draw: a mark beside the row opens it.
    earlier: Option<SharedString>,
    /// How many listed Jobs up the line of starters this one stands.
    depth: usize,
    /// For each starter up the line, from the top: whether its line runs on past this row to a later
    /// Job it started (Pocket's `through`).
    through: Vec<bool>,
    /// The last Job its starter started, where the starter's line ends in an elbow.
    ends: bool,
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
    /// Drawn only for its place, as the starter of a listed Job the scope leaves out: muted, never pressed.
    place: bool,
    /// The Mac the Job runs on, by id; none is this one (`crate::pairing`).
    mac: Option<SharedString>,
}

/// A parent's sub-Jobs as its row knows them.
#[derive(Clone, Debug, PartialEq)]
struct Kids {
    folded: bool,
    /// While folded: each sub-Job down the tree, one that needs him first, so a clipped strip never loses a need.
    strip: Vec<Sub>,
}

/// One state in a folded parent's strip, with the sub-Jobs that stand in it: a glyph per state, never one per Job.
#[derive(Clone, Debug, PartialEq)]
struct Sub {
    mark: JobMark,
    paused: bool,
    titles: Vec<SharedString>,
}

impl Sub {
    /// The glyph, its hue and the state's words. A Job that needs him reads Needs you whatever state asks it.
    fn look(&self) -> (&'static str, crate::look::Token, SharedString) {
        if self.paused {
            return ("circle-pause", token::FG_MUTED, "Paused".into());
        }
        let (glyph, said) = glyph_of(self.mark.state, None);
        let said = if self.mark.lamp == JobLamp::Needs { "Needs you".into() } else { said };
        (glyph, hue_of(self.mark.state), said)
    }

    /// The tooltip and the name VoiceOver reads: the state, then the sub-Jobs in it.
    fn words(&self) -> SharedString {
        format!("{}: {}", self.look().2, self.titles.join(", ")).into()
    }
}

/// Which parents are folded, and the switch that folds every one, new ones too. Kept between runs.
#[derive(Clone, Debug, Default, PartialEq)]
struct Folds {
    root_only: bool,
    parents: std::collections::BTreeSet<String>,
}

impl Folds {
    fn folds(&self, id: &str) -> bool {
        self.root_only || self.parents.contains(id)
    }

    /// An unfold under the switch ends it and keeps every other parent folded, the one way to open one parent
    /// while the rest stay shut.
    fn unfold(&mut self, id: &str, parents: &[String]) {
        if self.root_only {
            self.root_only = false;
            self.parents = parents.iter().cloned().collect();
        }
        self.parents.remove(id);
    }

    fn fold_all(&mut self, parents: &[String]) {
        self.parents = parents.iter().cloned().collect();
    }

    fn unfold_all(&mut self) {
        self.root_only = false;
        self.parents.clear();
    }

    /// The switch on its own line, then `fold <id>` for each parent.
    fn text(&self) -> String {
        let switch = self.root_only.then(|| ROOT_ONLY.to_string());
        switch.into_iter().chain(self.parents.iter().map(|id| format!("fold {id}"))).collect::<Vec<_>>().join("\n")
    }

    fn from_text(text: &str) -> Folds {
        Folds {
            root_only: text.lines().any(|line| line == ROOT_ONLY),
            parents: text.lines().filter_map(|line| line.strip_prefix("fold ")).map(str::to_string).collect(),
        }
    }
}

const ROOT_ONLY: &str = "root-only";

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
        JobState::Ended => ("power-off", "Ended"),
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

/// The repository a Job runs in, by the id it carries: the key its repository's group is held by.
fn repository_of(job: &SessionRecord) -> String {
    job.manifest_id.as_ref().map_or_else(|| job.cwd.clone(), |id| id.as_str().to_string())
}

/// What the list calls a group: his projects by id (`list_projects`), and each repository Fleet serves by
/// the id its Jobs carry, named as the title bar's picker names it.
#[derive(Clone, Debug, Default, PartialEq)]
struct Names {
    projects: HashMap<String, String>,
    repositories: HashMap<String, String>,
}

impl Names {
    fn of(served: &[ipc::RepositorySummary], projects: Option<&ipc::Projects>) -> Names {
        let named = crate::titlebar::repositories_of(served, &[]);
        let id_of = |one: &ipc::RepositorySummary| one.manifest.as_ref().map(|manifest| &manifest.id).or(one.implicit_id.as_ref()).map(|id| id.as_str().to_string());
        Names {
            projects: projects.map(|all| all.projects.iter().map(|one| (one.id.clone(), one.name.clone())).collect()).unwrap_or_default(),
            repositories: served.iter().zip(named).filter_map(|(one, named)| Some((id_of(one)?, named.name.to_string()))).collect(),
        }
    }
}

/// The group a Job stands in, as its key and its heading: the project Fleet says it was started for, by
/// that project's name, else its repository by name. A heading is never an id.
fn group_of(job: &SessionRecord, names: &Names) -> (String, String) {
    let project = job.hosted.as_ref().and_then(|hosted| hosted.project.as_ref());
    if let Some((id, name)) = project.and_then(|id| names.projects.get(id).map(|name| (id, name))) {
        return (format!("project:{id}"), name.clone());
    }
    let named = job.manifest_id.as_ref().and_then(|id| names.repositories.get(id.as_str())).cloned();
    (repository_of(job), named.unwrap_or_else(|| folder_of(&job.cwd)))
}

/// The repository's folder a Job runs in, from its working directory: a slot's checkout is under the
/// repository it was cut from, so its `.armada/slots/…` tail is not the name.
fn folder_of(cwd: &str) -> String {
    let root = cwd.split("/.armada/").next().unwrap_or(cwd);
    root.rsplit('/').find(|part| !part.is_empty()).unwrap_or(root).to_string()
}

/// What a row calls a Job Fleet has not titled yet, as ⌘T names the Job it starts.
const UNTITLED: &str = "New Job";

/// A Job's title as its row draws it: never its id.
fn title_of(job: &SessionRecord) -> SharedString {
    titled(job.title.as_deref()).into()
}

/// A Job's title wherever it is named, New Job until Fleet titles it: never its id.
pub fn titled(title: Option<&str>) -> String {
    title.map(str::trim).filter(|title| !title.is_empty()).unwrap_or(UNTITLED).to_string()
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

fn lines_of(jobs: &[SessionRecord], ideas: &[crate::queued::QueuedIdea], scope: Scope, query: &str, order: &[String], names: &Names) -> Vec<Line> {
    let paused = |job: &SessionRecord| job.hosted.as_ref().is_some_and(|hosted| hosted.paused.is_some());
    let quiet = |job: &SessionRecord| job.mark.is_some_and(|mark| mark.state == JobState::Quiet);
    // Fleet clears `snoozed_until` when the snooze runs out, so one that is set is still ahead (`Held::snoozed`).
    let snoozed = |job: &SessionRecord| job.snoozed_until.is_some();
    let live = |job: &&SessionRecord| job.state == SessionState::Live;
    let placed = |job: &&SessionRecord| (order.iter().position(|id| id == job.id.as_str()).unwrap_or(usize::MAX), standing_of(job));
    let mut lives: Vec<&SessionRecord> = jobs.iter().filter(live).collect();
    lives.sort_by_key(placed);
    let mut ended: Vec<&SessionRecord> = jobs.iter().filter(|job| job.state == SessionState::Ended).collect();
    ended.sort_by(|a, b| b.ended_at.as_ref().map(|at| at.as_str()).cmp(&a.ended_at.as_ref().map(|at| at.as_str())));
    let needs = |job: &SessionRecord| job.mark.is_some_and(|mark| mark.lamp == JobLamp::Needs);
    let picked: Vec<&SessionRecord> = match scope {
        Scope::Needs => lives.iter().copied().filter(|job| needs(job) && !snoozed(job)).collect(),
        Scope::Active => lives.iter().copied().filter(|job| !paused(job) && !quiet(job) && !snoozed(job)).collect(),
        Scope::Paused => lives.iter().copied().filter(|job| paused(job)).collect(),
        Scope::Snoozed => lives.iter().copied().filter(|job| snoozed(job)).collect(),
        Scope::Quiet => lives.iter().copied().filter(|job| quiet(job)).collect(),
        Scope::Ended => ended,
        Scope::All => lives.iter().copied().chain(ended).collect(),
    };
    // A Job another took the place of stands behind that one's row, as Electron's deck has it (`ship.ts`, `behind`).
    let picked: Vec<&SessionRecord> = picked.into_iter().filter(|job| spent(job, "replaced_by").is_none() && matches(job, query)).collect();
    // In every scope a listed Job stands under its live starters; one the scope leaves out, a paused or
    // snoozed starter in Active or a calm one in Needs you, comes in for its place alone.
    let mut places: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for job in &picked {
        let mut up = starter_of(job);
        while let Some(id) = up.filter(|id| !picked.iter().any(|one| one.id.as_str() == *id) && !places.contains(id)) {
            let Some(starter) = lives.iter().find(|one| one.id.as_str() == id) else { break };
            places.insert(starter.id.as_str());
            up = starter_of(starter);
        }
    }
    let picked: Vec<&SessionRecord> = if places.is_empty() {
        picked
    } else {
        let mut kept: Vec<&SessionRecord> =
            lives.iter().copied().filter(|job| places.contains(job.id.as_str()) || picked.iter().any(|one| one.id == job.id)).collect();
        kept.extend(picked.iter().copied().filter(|job| !lives.iter().any(|one| one.id == job.id)));
        kept
    };
    let now = chrono::Utc::now();
    // Each group by its key, its heading and its Jobs.
    let mut projects: Vec<(String, String, Vec<&SessionRecord>)> = Vec::new();
    for job in picked {
        let (key, name) = group_of(job, names);
        match projects.iter_mut().find(|(held, ..)| *held == key) {
            Some((.., held)) => held.push(job),
            None => projects.push((key, name, vec![job])),
        }
    }
    // Groups stand in the order Fleet lists Jobs, so a state change never moves one.
    projects.sort_by_key(|(key, ..)| jobs.iter().position(|job| group_of(job, names).0 == *key));
    // Ideas waiting to start, among the live Jobs, each under its repository's group (`bands.ts`): the group of a
    // Job in its root, else its repository's name. A group with only ideas stands after the rest.
    let waiting: Vec<&crate::queued::QueuedIdea> = match scope {
        Scope::Active | Scope::All => ideas.iter().filter(|idea| query.trim().is_empty() || idea.text.to_lowercase().contains(&query.trim().to_lowercase())).collect(),
        _ => Vec::new(),
    };
    let idea_group = |idea: &crate::queued::QueuedIdea| {
        jobs.iter().find(|job| job.cwd == idea.root).map_or_else(|| (idea.repository.clone(), idea.repository.clone()), |job| group_of(job, names))
    };
    for idea in &waiting {
        let (key, name) = idea_group(idea);
        if !projects.iter().any(|(held, ..)| *held == key) {
            projects.push((key, name, Vec::new()));
        }
    }
    let headed = projects.len() > 1;
    let mut lines = Vec::new();
    for (key, project, held) in projects {
        if headed {
            lines.push(Line::Project(project.clone().into()));
        }
        for idea in waiting.iter().filter(|idea| idea_group(idea).0 == key) {
            lines.push(Line::Job(Row {
                id: format!("queued:{}", idea.id).into(),
                title: idea.title().to_string().into(),
                project: project.clone().into(),
                paused: false,
                snoozed: None,
                ended: false,
                handing_off: false,
                kids: None,
                earlier: None,
                depth: 0,
                through: Vec::new(),
                ends: true,
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
                place: false,
                mac: None,
            }));
        }
        let listed = |id: &str| held.iter().any(|job| job.id.as_str() == id);
        let row_of = |job: &SessionRecord, depth: usize, through: &[bool], last: bool| Row {
            id: job.id.as_str().to_string().into(),
            title: title_of(job),
            project: project.clone().into(),
            paused: paused(job),
            snoozed: job.snoozed_until.as_ref().and_then(crate::snooze::local_of),
            ended: job.state == SessionState::Ended,
            handing_off: handing_off(job),
            kids: None,
            earlier: spent(job, "replaces").map(|id| id.to_string().into()),
            depth,
            through: through.to_vec(),
            ends: last,
            started_by: starter_of(job).and_then(|id| jobs.iter().find(|one| one.id.as_str() == id)).map(|one| (title_of(one), group_of(one, names).1.into())),
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
            place: places.contains(job.id.as_str()),
            mac: job.mac.clone().map(Into::into),
        };
        fn under<'a>(
            parent: &str,
            depth: usize,
            through: &[bool],
            held: &[&'a SessionRecord],
            row_of: &dyn Fn(&SessionRecord, usize, &[bool], bool) -> Row,
            lines: &mut Vec<Line>,
        ) {
            let children: Vec<&&SessionRecord> = held.iter().filter(|job| starter_of(job) == Some(parent)).collect();
            for (at, child) in children.iter().enumerate() {
                let last = at + 1 == children.len();
                lines.push(Line::Job(row_of(child, depth, through, last)));
                let on: Vec<bool> = through.iter().copied().chain([!last]).collect();
                under(child.id.as_str(), depth + 1, &on, held, row_of, lines);
            }
        }
        for root in held.iter().filter(|job| !starter_of(job).is_some_and(listed)) {
            lines.push(Line::Job(row_of(root, 0, &[], true)));
            under(root.id.as_str(), 1, &[], &held, &row_of, &mut lines);
        }
    }
    lines
}

/// `lines` with each parent's sub-Jobs folded away where `folds` shuts it, and its row told what stands under
/// it: a folded parent keeps their glyphs, one that needs him first, so folding hides no need. A parent drawn only
/// for its place never folds, since the Jobs under it are why it is there.
fn fold_lines(lines: Vec<Line>, folds: &Folds) -> Vec<Line> {
    let mut out = Vec::with_capacity(lines.len());
    let mut at = 0;
    while at < lines.len() {
        let Line::Job(row) = &lines[at] else {
            out.push(lines[at].clone());
            at += 1;
            continue;
        };
        let below = lines[at + 1..].iter().take_while(|line| matches!(line, Line::Job(down) if down.depth > row.depth)).count();
        at += 1;
        if row.place || row.queued.is_some() || below == 0 {
            out.push(Line::Job(row.clone()));
            continue;
        }
        let folded = folds.folds(row.id.as_ref());
        let mut row = row.clone();
        let mut strip: Vec<Sub> = Vec::new();
        if folded {
            for line in &lines[at..at + below] {
                let Line::Job(sub) = line else { continue };
                let Some(mark) = sub.mark.filter(|_| !sub.place) else { continue };
                let held = strip.iter_mut().find(|one| one.paused == sub.paused && (sub.paused || one.mark.state == mark.state));
                match held {
                    Some(one) => one.titles.push(sub.title.clone()),
                    None => strip.push(Sub { mark, paused: sub.paused, titles: vec![sub.title.clone()] }),
                }
            }
            // Needs you first, then failed, then running, then the rest as they stand.
            strip.sort_by_key(|sub| match sub.mark.lamp {
                JobLamp::Needs => 0,
                JobLamp::Failed => 1,
                JobLamp::Running => 2,
                _ => 3,
            });
            at += below;
        }
        row.kids = Some(Kids { folded, strip });
        out.push(Line::Job(row));
    }
    out
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

/// Where the folded parents and the Root Jobs only switch are kept between runs, beside the list's order.
fn folds_file() -> Option<PathBuf> {
    deck_file().map(|deck| deck.with_file_name("native-folds"))
}

impl Folds {
    /// What `path` holds; nothing folded where it holds none or cannot be read.
    fn load(path: Option<&std::path::Path>) -> Folds {
        path.and_then(|path| std::fs::read_to_string(path).ok()).map(|text| Folds::from_text(&text)).unwrap_or_default()
    }

    fn save(&self, path: Option<&std::path::Path>) {
        let Some(path) = path else { return };
        if let Err(why) = path.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|()| std::fs::write(path, self.text())) {
            Note::warn("the folded Jobs were not kept").field("why", why.to_string()).emit();
        }
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
    /// The parents whose sub-Jobs are folded away, and the switch that folds every one.
    folds: Folds,
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
    /// The Job that was open when the last run ended, opened once Fleet lists it (`job_view::place`).
    resume: Option<String>,
    /// Counts each pick of a Job, the open one again too, so the view puts the keys in its box.
    picks: u64,
    /// The Jobs a walk's stand-in for Fleet started: their threads are empty, as Fleet answers for a Job with nothing in it.
    made: Vec<String>,
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
    /// The row whose quiet glyph the pointer is on: its tooltip stands alone, so the row's card does not open over it.
    quiet_over: Option<SharedString>,
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
    /// What each group is called: his projects and the repositories Fleet serves, by name.
    names: Names,
    /// Jobs `session.removed` took away that the open Job's screen has not let go of yet.
    gone: Vec<SharedString>,
    _removed: Option<Subscription>,
}

/// The list a walk plays on, so a scene can send it what Fleet would.
struct MockList(gpui::WeakEntity<JobsList>);

impl gpui::Global for MockList {}

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
                let offers = fleet.read(cx).held().offers.clone();
                let disk = fleet.read(cx).held().disk.clone();
                let link = fleet.read(cx).held().link();
                bar.update(cx, |bar, cx| {
                    bar.set_fleet(&link, cx);
                    bar.set_usage(usage.as_ref(), downshift.as_ref(), cx);
                    bar.set_build(build.as_ref(), cx);
                    bar.set_offers(offers.as_ref(), cx);
                    bar.set_disk(disk.as_ref(), cx);
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
            let held = fleet.read(cx).held();
            let names = Names::of(held.repositories.as_deref().unwrap_or_default(), held.projects.as_ref());
            let mut jobs = held.jobs.clone();
            this.hold_asked_titles(&mut jobs);
            if jobs != this.jobs || names != this.names {
                this.jobs = jobs;
                this.names = names;
                this.place_new();
                this.relist(cx);
            }
            if this.resume_open() {
                cx.notify();
            }
        });
        let (scope, order) = read_deck();
        crate::queued::begin(true, cx);
        let held = fleet.read(cx).held();
        let names = Names::of(held.repositories.as_deref().unwrap_or_default(), held.projects.as_ref());
        let jobs = held.jobs.clone();
        Self::with(jobs, names, scope, order, read_view(), true, Some(fleet.clone()), Some(watch), cx)
    }

    /// The list on mock Jobs, which never change, with the View as a first start has it, so a walk shows what he will.
    pub fn mock(jobs: &[SessionRecord], cx: &mut Context<Self>) -> Self {
        crate::queued::begin(false, cx);
        let names = Names::of(&crate::mock::repositories(), Some(&crate::mock::projects()));
        Self::with(jobs.to_vec(), names, Scope::Active, Vec::new(), View::DEFAULT, false, None, None, cx)
    }

    fn with(
        jobs: Vec<SessionRecord>,
        names: Names,
        scope: Scope,
        order: Vec<String>,
        view: View,
        remember: bool,
        fleet: Option<Entity<FleetConnection>>,
        watch: Option<Subscription>,
        cx: &mut Context<Self>,
    ) -> Self {
        let fleet_held = fleet.is_some();
        let mut this = JobsList {
            jobs,
            lines: Vec::new(),
            scope,
            query: String::new(),
            folds: if remember { Folds::load(folds_file().as_deref()) } else { Folds::default() },
            view,
            order,
            remember,
            focused: None,
            opened: None,
            resume: if remember { crate::job_view::place_kept_open() } else { None },
            picks: 0,
            made: Vec::new(),
            focus: cx.focus_handle(),
            within: cx.focus_handle(),
            scroll: ScrollHandle::new(),
            bounds: Rc::default(),
            menu: None,
            filter: None,
            search: None,
            card: None,
            quiet_over: None,
            _card_wait: None,
            fleet,
            _watch: watch,
            asked: HashMap::new(),
            drop: None,
            lifted: None,
            ideas: crate::queued::ideas(cx),
            names,
            gone: Vec::new(),
            _removed: None,
        };
        if fleet_held {
            this._removed = this.fleet.clone().map(|fleet| cx.subscribe(&fleet, |this, _, crate::fleet::Removed(ids), cx| this.gone(ids, cx)));
        } else {
            cx.set_global(MockList(cx.weak_entity()));
        }
        // The ideas waiting to start stand in the list as they change, from Dispatch or a row's own acts.
        cx.observe_global::<crate::queued::Queued>(|this, cx| {
            this.ideas = crate::queued::ideas(cx);
            this.relist(cx);
        })
        .detach();
        this.place_new();
        this.lines = this.folded(this.lines_now());
        this.focused = this.first_row();
        this
    }

    /// `session.removed`: each row goes, and the Job open or focused on it is let go of, as the list does for a row
    /// Fleet no longer serves. The open Job's screen takes the ids ([`Self::take_gone`]) to drop its tabs and page.
    pub fn gone(&mut self, ids: &[String], cx: &mut Context<Self>) {
        let named = |id: &str| ids.iter().any(|gone| gone == id);
        self.jobs.retain(|job| !named(job.id.as_str()));
        self.order.retain(|id| !named(id));
        self.keep();
        if self.opened.as_ref().is_some_and(|id| named(id)) {
            self.opened = None;
        }
        if self.card.as_ref().is_some_and(|id| named(id)) {
            self.card = None;
        }
        self.gone.extend(ids.iter().map(|id| SharedString::from(id.clone())));
        self.lines = self.lines_now();
        if !self.focused.as_ref().is_some_and(|id| self.at_of(id).is_some()) {
            self.focused = self.first_row();
        }
        cx.notify();
    }

    /// A walk's `session.removed` for these Jobs, sent to the list it plays on.
    pub fn mock_gone(ids: &[String], cx: &mut App) -> Result<(), String> {
        let list = cx.try_global::<MockList>().and_then(|held| held.0.upgrade()).ok_or("no Jobs list is drawn yet")?;
        list.update(cx, |list, cx| list.gone(ids, cx));
        Ok(())
    }

    /// The Jobs taken away since the screen last asked, once.
    pub fn take_gone(&mut self) -> Vec<SharedString> {
        std::mem::take(&mut self.gone)
    }

    /// Put each live Job the order has not met at its end, in the standing order it is drawn in,
    /// so no later change of state moves it (`store.ts`, a new one is added at the end).
    fn place_new(&mut self) {
        let fresh: Vec<String> = lines_of(&self.jobs, &[], Scope::All, "", &self.order, &self.names)
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

    /// Open the Job kept open at the last run once Fleet lists Jobs: nothing where it lists none yet, and the
    /// wish is let go where the Job is gone or another was opened first. True where it opened one.
    fn resume_open(&mut self) -> bool {
        if self.resume.is_none() || self.jobs.is_empty() {
            return false;
        }
        let Some(id) = self.resume.take() else { return false };
        if self.opened.is_some() || !self.jobs.iter().any(|job| job.id.as_str() == id) {
            return false;
        }
        let id = SharedString::from(id);
        self.focused = Some(id.clone());
        self.opened = Some(id);
        true
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

    /// A walk's stand-in for Fleet's yes to a start from the sheet: a Job of the words stands in the list and opens,
    /// working at once, or held on its start card where the person pressed Draft.
    pub fn hold_started_words(&mut self, words: &str, repository: &str, held: bool, cx: &mut Context<Self>) {
        let title = words.lines().find(|line| !line.trim().is_empty()).map_or("", str::trim).to_string();
        let mark = if held {
            ipc::JobMark { state: ipc::JobState::Draft, lamp: ipc::JobLamp::Idle, group: ipc::JobGroup::Working }
        } else {
            ipc::JobMark { state: ipc::JobState::Working, lamp: ipc::JobLamp::Running, group: ipc::JobGroup::Working }
        };
        let id = if held { "drafted-from-sheet" } else { "started-from-sheet" };
        self.hold_new(id.to_string(), title, repository, Some(mark), cx);
        if held {
            self.hold_on_card(id, words, cx);
        }
    }

    /// The Job held on its start card, as Fleet's record carries one: the first message, the proposer's read workflow and the usual landing.
    fn hold_on_card(&mut self, id: &str, words: &str, cx: &mut Context<Self>) {
        let hosted = self.jobs.iter().find_map(|job| job.hosted.clone());
        let Some(job) = self.jobs.iter_mut().find(|job| job.id.as_str() == id) else { return };
        job.hosted = hosted.map(|mut hosted| {
            hosted.turn = ipc::SessionTurn::Idle;
            hosted.asked = None;
            hosted.paused = None;
            hosted.start_failed = None;
            hosted.running = false;
            hosted.start = Some(ipc::StartHeld {
                asks_delivery: true,
                picked: Some(ipc::DeliveryPicked { choice: ipc::DeliveryChoice::AutoLand, from: ipc::DeliveryPickedFrom::Usual }),
                message: Some(words.to_string()),
                queued: false,
            });
            hosted
        });
        job.workflow = Some("feature".into());
        job.proposal = Some(ipc::SessionProposal { reading: false, picked: vec![ipc::ProposedField::Workflow] });
        self.relist(cx);
        cx.notify();
    }

    /// A walk's stand-in for Fleet's yes to Start on a held Job: the card goes and the Job is working.
    pub fn began(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(job) = self.jobs.iter_mut().find(|job| job.id.as_str() == id) else { return };
        if let Some(hosted) = job.hosted.as_mut() {
            hosted.start = None;
        }
        job.mark = Some(ipc::JobMark { state: ipc::JobState::Working, lamp: ipc::JobLamp::Running, group: ipc::JobGroup::Working });
        self.relist(cx);
        cx.notify();
    }

    fn hold_new(&mut self, id: String, title: String, repository: &str, mark: Option<ipc::JobMark>, cx: &mut Context<Self>) {
        if let Some(id) = self.stand_new(id, title, repository, mark, cx) {
            self.open(id, cx);
        }
    }

    /// A walk's stand-in for the Operator starting the Job a dispatch became: it stands in its repository's group,
    /// working, and does not open. Its id and its title.
    pub fn operator_started(&mut self, words: &str, repository: &str, cx: &mut Context<Self>) -> Option<(String, String)> {
        let title = words.lines().find(|line| !line.trim().is_empty()).map_or("", str::trim).to_string();
        let mark = ipc::JobMark { state: ipc::JobState::Working, lamp: ipc::JobLamp::Running, group: ipc::JobGroup::Working };
        let id = self.stand_new("started-by-operator".to_string(), title.clone(), repository, Some(mark), cx)?;
        Some((id.to_string(), title))
    }

    /// The title of a Job the list holds.
    pub fn title_of(&self, id: &str) -> Option<String> {
        self.record(id).and_then(|job| job.title.clone())
    }

    /// A Job stands in the list in its repository's group, as a start's record would, and is not opened.
    fn stand_new(&mut self, id: String, title: String, repository: &str, mark: Option<ipc::JobMark>, cx: &mut Context<Self>) -> Option<SharedString> {
        let like = self.jobs.iter().find(|job| repository_of(job) == repository).or_else(|| self.jobs.first());
        let mut record = like.cloned()?;
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
        self.made.push(id.to_string());
        self.jobs.insert(0, record);
        self.relist(cx);
        Some(id)
    }

    /// Whether a walk's stand-in for Fleet started this Job.
    pub fn made_here(&self, id: &str) -> bool {
        self.made.iter().any(|made| made == id)
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
        lines_of(&self.jobs, &self.ideas, self.scope, &self.query, &self.order, &self.names)
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

    /// The rows a press or a key reaches: a starter drawn for its place is not one.
    fn rows(&self) -> impl Iterator<Item = &Row> {
        self.lines.iter().filter_map(|line| match line {
            Line::Job(row) if !row.place => Some(row),
            Line::Job(_) | Line::Project(_) => None,
        })
    }

    /// Draw the list again from the Jobs, the scope, the search and the order.
    fn relist(&mut self, cx: &mut Context<Self>) {
        let lines = self.folded(self.lines_now());
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

    /// `lines` with the parents this list has folded shut.
    fn folded(&self, lines: Vec<Line>) -> Vec<Line> {
        fold_lines(lines, &self.folds)
    }

    /// Every Job another is listed under, by id: the parents a fold reaches.
    fn parent_ids(&self) -> Vec<String> {
        let mut parents: Vec<String> =
            self.jobs.iter().filter_map(starter_of).filter(|id| self.jobs.iter().any(|one| one.id.as_str() == *id)).map(str::to_string).collect();
        parents.sort();
        parents.dedup();
        parents
    }

    /// Whether any parent stands, whether the switch is on, and whether every parent is folded.
    pub(crate) fn fold_facts(&self) -> (bool, bool, bool) {
        let parents = self.parent_ids();
        let every = !parents.is_empty() && (self.folds.root_only || parents.iter().all(|id| self.folds.parents.contains(id)));
        (!parents.is_empty(), self.folds.root_only, every)
    }

    /// The nearest listed Job above `id` up the line of starters.
    fn shown_starter(&self, id: &str) -> Option<SharedString> {
        let mut up = self.jobs.iter().find(|job| job.id.as_str() == id).and_then(starter_of);
        while let Some(parent) = up {
            let parent: SharedString = parent.to_string().into();
            if self.at_of(&parent).is_some() {
                return Some(parent);
            }
            up = self.jobs.iter().find(|job| job.id.as_str() == parent.as_ref()).and_then(starter_of);
        }
        None
    }

    /// Change what is folded, keep it, and draw the list again. The keys on a Job that folded away go to the
    /// parent it stood under.
    fn change_folds(&mut self, change: impl FnOnce(&mut Folds, &[String]), cx: &mut Context<Self>) {
        let parents = self.parent_ids();
        let before = self.folds.clone();
        change(&mut self.folds, &parents);
        if self.folds == before {
            return;
        }
        if self.remember {
            if !self.jobs.is_empty() {
                let jobs = &self.jobs;
                self.folds.parents.retain(|id| jobs.iter().any(|job| job.id.as_str() == id));
            }
            self.folds.save(folds_file().as_deref());
        }
        let held = self.focused.clone();
        self.relist(cx);
        if let Some(id) = held.filter(|id| self.at_of(id).is_none()) {
            if let Some(parent) = self.shown_starter(&id) {
                self.focused = Some(parent);
            }
        }
        cx.notify();
    }

    /// Fold or unfold one parent's sub-Jobs.
    pub(crate) fn set_row_fold(&mut self, id: &str, fold: bool, cx: &mut Context<Self>) {
        let id = id.to_string();
        self.change_folds(
            move |folds, parents| {
                if !fold {
                    folds.unfold(&id, parents);
                } else if !folds.root_only {
                    folds.parents.insert(id);
                }
            },
            cx,
        );
    }

    /// Fold or unfold every parent's sub-Jobs.
    pub(crate) fn set_all_folds(&mut self, fold: bool, cx: &mut Context<Self>) {
        self.change_folds(|folds, parents| if fold { folds.fold_all(parents) } else { folds.unfold_all() }, cx);
    }

    /// The Root Jobs only switch.
    pub(crate) fn set_root_only(&mut self, on: bool, cx: &mut Context<Self>) {
        self.change_folds(|folds, _| folds.root_only = on, cx);
    }

    /// A palette command: `fold-all`, `unfold-all` or `root-only`.
    pub(crate) fn fold_command(&mut self, command: &str, cx: &mut Context<Self>) {
        match command {
            "fold-all" => self.set_all_folds(true, cx),
            "unfold-all" => self.set_all_folds(false, cx),
            "root-only" => self.set_root_only(!self.folds.root_only, cx),
            _ => {}
        }
    }

    /// ← folds the focused parent and → unfolds it; on any other row the key is left for others.
    fn fold_focused(&mut self, fold: bool, cx: &mut Context<Self>) {
        let held = self.focused.as_ref().and_then(|id| self.rows().find(|row| &row.id == id)).filter(|row| row.kids.as_ref().is_some_and(|kids| kids.folded != fold)).map(|row| row.id.clone());
        match held {
            Some(id) => self.set_row_fold(&id, fold, cx),
            None => cx.propagate(),
        }
    }

    fn on_fold_row(&mut self, _: &FoldRow, _: &mut Window, cx: &mut Context<Self>) {
        self.fold_focused(true, cx);
    }

    fn on_unfold_row(&mut self, _: &UnfoldRow, _: &mut Window, cx: &mut Context<Self>) {
        self.fold_focused(false, cx);
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
            lines_of(&self.jobs, &[], Scope::All, "", &self.order, &self.names).iter().filter_map(|line| match line {
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
            lines_of(&self.jobs, &[], Scope::All, "", &self.order, &self.names).iter().filter_map(|line| match line {
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
        // A Job on another Mac is paused there: the key does nothing here.
        if self.record(&row.id).is_some_and(|job| !crate::pairing::Here::acts_here(job)) {
            return true;
        }
        let session_id = ipc::SessionId::carried(row.id.to_string());
        let body = if row.paused {
            ("/sessions/resume", ipc::encode(&ipc::ResumeSession { session_id }))
        } else {
            ("/sessions/pause", ipc::encode(&ipc::PauseSession { session_id }))
        };
        send(self.port(cx), body.0, body.1.map_err(|why| format!("{why:?}")), cx);
        true
    }

    /// `z` while the rows have the keys opens the presets for the row under them, or wakes it where it is
    /// snoozed, as `p` pauses or resumes.
    pub(crate) fn snooze_key(&mut self, event: &gpui::KeyDownEvent, window: &Window, cx: &mut Context<Self>) -> bool {
        if event.keystroke.key != "z" || event.keystroke.modifiers.modified() || !self.focus.is_focused(window) {
            return false;
        }
        let Some(row) = self.focused.as_ref().and_then(|id| self.rows().find(|row| &row.id == id)).filter(|row| !row.ended && row.queued.is_none()).cloned() else { return false };
        if row.snoozed.is_some() {
            self.snooze(&row.id, None, cx);
        } else {
            let at = self.bounds.borrow().get(&row.id).copied().map(under);
            crate::dispatch::screen::snooze_ask(row.id, row.title, at, cx);
        }
        true
    }

    /// Keep a Job quiet until `until`, or wake it now with none. A live Fleet is asked, and the row follows
    /// the record it publishes; a refusal is said in a toast. On mock data there is no Fleet, so the record
    /// the list holds takes the change, as a walk's stand-in for Fleet's yes.
    pub fn snooze(&mut self, id: &str, until: Option<chrono::DateTime<chrono::Utc>>, cx: &mut Context<Self>) {
        let at = until.map(crate::snooze::instant_of);
        if self.fleet.is_none() {
            if let Some(job) = self.jobs.iter_mut().find(|job| job.id.as_str() == id) {
                job.snoozed_until = at;
            }
            self.relist(cx);
            cx.notify();
            return;
        }
        crate::snooze::post(self.port(cx), crate::snooze::body(id, until), cx);
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

    /// The row's menu (`RowMenu.tsx`), at `at`: Pause or Resume, Snooze or Wake now, Name it for me, Try it in your app
    /// and Close. An ended Job has none of them, and draws no menu.
    pub(crate) fn open_menu(&mut self, id: SharedString, at: Point<Pixels>, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(row) = self.rows().find(|row| row.id == id).cloned() else { return };
        if row.ended {
            return;
        }
        self.card = None;
        let (pause, label, icon) = if row.paused { ("resume", "Resume", "play") } else { ("pause", "Pause", "circle-pause") };
        // A Job on another Mac is read here: its acts are shown, off, with why (`crate::pairing`).
        let off: Option<SharedString> = self.record(&id).and_then(|job| crate::pairing::refusal(job, cx)).map(Into::into);
        let act = |id: &'static str, label: &str, icon: Option<&'static str>, danger: bool| match &off {
            Some(why) => Entry::Item { id, label: label.to_string().into(), icon, danger, key: None, off: Some(why.clone()) },
            None => Entry::Act { id, label: label.to_string().into(), icon, danger },
        };
        let mut entries = vec![
            Entry::Item { id: pause, label: label.into(), icon: Some(icon), danger: false, key: Some("P".into()), off: off.clone() },
            match crate::snooze::menu_item(row.snoozed.is_some()) {
                Entry::Item { id, label, icon, danger, key, .. } => Entry::Item { id, label, icon, danger, key, off: off.clone() },
                other => other,
            },
            act("name", "Name it for me", Some("wand-sparkles"), false),
            Entry::Act { id: "teach", label: "Teach".into(), icon: Some("graduation-cap"), danger: false },
            act("try", "Try it in your app", None, false),
            act("close", "Close", Some("x"), true),
            Entry::Rule,
        ];
        // The View group comes after the Job's own acts, then Mod and Report, as in `RowMenu.tsx`.
        let (any_parents, _, all_folded) = self.fold_facts();
        let row_folded = row.kids.as_ref().map(|kids| kids.folded);
        let all_unfold = row_folded.unwrap_or(all_folded);
        if any_parents {
            entries.extend(Self::fold_entries(row_folded, all_unfold));
        }
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
                "snooze" => crate::dispatch::screen::snooze_ask(job.clone(), title.clone(), Some(at), cx),
                "wake" => this.snooze(&job, None, cx),
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
                "fold" => this.set_row_fold(&job, !row_folded.unwrap_or(false), cx),
                "fold_all" => this.set_all_folds(!all_unfold, cx),
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

    /// The row menu's fold group: this parent's sub-Jobs where it has some, and every parent's, with the
    /// unfold words while the row is folded.
    fn fold_entries(row_folded: Option<bool>, all_unfold: bool) -> Vec<Entry> {
        let mut entries = Vec::new();
        if let Some(folded) = row_folded {
            let label = if folded { "Unfold sub-Jobs" } else { "Fold sub-Jobs" };
            entries.push(Entry::Act { id: "fold", label: label.into(), icon: None, danger: false });
        }
        let label = if all_unfold { "Unfold all sub-Jobs" } else { "Fold all sub-Jobs" };
        entries.push(Entry::Act { id: "fold_all", label: label.into(), icon: None, danger: false });
        entries.push(Entry::Rule);
        entries
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
        let mut entries: Vec<Entry> = SCOPES.iter().map(|(scope, id, label)| Entry::Choice { id, label: (*label).into(), on: *scope == self.scope }).collect();
        entries.push(Entry::Rule);
        entries.push(Entry::Toggle { id: "root_only", label: "Root Jobs only".into(), on: self.folds.root_only });
        let menu = cx.new(|cx| Dropdown::new("Scope", entries, at, Anchor::TopRight, true, cx));
        let picked = cx.subscribe(&menu, |this, _, picked: &Picked, cx| {
            let Picked::Entry { id, .. } = picked;
            if *id == "root_only" {
                this.set_root_only(!this.folds.root_only, cx);
                return;
            }
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
        if self.menu.is_some() || self.quiet_over.as_ref() == Some(&id) {
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

    /// The pointer is on a row's quiet glyph or left it. On it, a card that is open or waiting goes, and none opens
    /// until the pointer is back on the row from outside it.
    fn quiet_hovered(&mut self, id: SharedString, on: bool, cx: &mut Context<Self>) {
        if on {
            if self.card.as_ref() == Some(&id) {
                self.card = None;
            }
            self._card_wait = None;
            self.quiet_over = Some(id);
        } else if self.quiet_over.as_ref() == Some(&id) {
            self.quiet_over = None;
        }
        cx.notify();
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
        let weight = if opened { token::WEIGHT_HEADING } else { token::WEIGHT_BODY };
        let ink = if row.paused || row.ended || row.place { token::FG_MUTED } else { token::FG_DEFAULT };
        let lamp = match row.mark {
            Some(mark) => lamp_of(mark.lamp),
            None if row.ended => token::STATUS_COMPLETED_SUCCESS,
            None => token::BORDER_STRONG,
        };
        // Needs him, and not the Job open now: the row's tint breathes (`ListCard.css`).
        // A snoozed Job is quiet to the eye as well: its edge stays amber and nothing moves.
        let needs = row.mark.is_some_and(|mark| mark.lamp == JobLamp::Needs) && !opened && row.snoozed.is_none();
        let state = row.mark.filter(|_| !row.paused).map(|mark| (glyph_of(mark.state, row.pr.as_deref()), hue_of(mark.state)));
        let rail = look.length(token::BORDER_WIDTH) * 2.;
        // Pocket's tree (`pocket.css`, `.pk-guide`): each level steps in by space-6.
        let step_in = look.length(token::SPACE_6);
        let indent = step_in * row.depth as f32;
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
        // A folded parent keeps its sub-Jobs' glyphs in a strip, one that needs him first: folding hides no need.
        let strip_of = row.kids.as_ref().filter(|kids| kids.folded && !kids.strip.is_empty()).map(|kids| {
            div()
                .flex_none()
                .max_w(look.length(token::SPACE_8) * 5.)
                .overflow_hidden()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .children(kids.strip.iter().enumerate().map(|(at, sub)| {
                    let (glyph, hue, _) = sub.look();
                    let said = sub.words();
                    labelled(
                        div()
                            .id(SharedString::from(format!("sub-{}-{at}", row.id)))
                            .flex_none()
                            .child(svg().path(icon_path(glyph)).size(mark_glyph).text_color(look.color(hue)))
                            .tooltip(tip(said.clone())),
                        Role::Image,
                        said,
                    )
                }))
        });
        // The chevron column is on every row, so titles line up down the list; only a parent draws its chevron.
        let column = look.length(token::SPACE_4);
        let chevron = match row.kids.as_ref().map(|kids| kids.folded) {
            Some(folded) => {
                let (name, said) = if folded { ("Unfold sub-Jobs", "Unfold sub-Jobs. \u{2325} unfolds every parent") } else { ("Fold sub-Jobs", "Fold sub-Jobs. \u{2325} folds every parent") };
                let id = row.id.clone();
                labelled(
                    div()
                        .id(SharedString::from(format!("fold-{}", row.id)))
                        .flex_none()
                        .size(column)
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(look.length(token::RADIUS_SM))
                        .hover(|style| style.bg(look.color(token::BG_HOVER)))
                        .child(svg().path(icon_path(if folded { "chevron-right" } else { "chevron-down" })).size(mark_glyph).text_color(look.color(token::FG_MUTED)))
                        .tooltip(tip(said.into()))
                        .on_click(cx.listener(move |this, event: &gpui::ClickEvent, window, cx| {
                            cx.stop_propagation();
                            this.focused = Some(id.clone());
                            window.focus(&this.focus, cx);
                            if event.modifiers().alt {
                                this.set_all_folds(!folded, cx);
                            } else {
                                this.set_row_fold(&id, !folded, cx);
                            }
                            cx.notify();
                        })),
                    Role::Button,
                    format!("{name} of {}", row.title),
                )
                .into_any_element()
            }
            None => div().flex_none().w(column).into_any_element(),
        };
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
        // Quiet until a time: the Job works on and raises nothing to him. Beside the state, never in place of it.
        let quiet_id = row.id.clone();
        let quiet = row.snoozed.map(|until| {
            let said: SharedString = crate::snooze::quiet_until(until, crate::snooze::now(), crate::settings::time::held()).into();
            labelled(
                div()
                    .id(SharedString::from(format!("snoozed-{}", row.id)))
                    .on_hover(cx.listener(move |this, on: &bool, _, cx| this.quiet_hovered(quiet_id.clone(), *on, cx)))
                    .child(svg().path(icon_path("bell-off")).size(mark_glyph).text_color(look.color(token::FG_MUTED)))
                    .tooltip(tip(said.clone())),
                Role::Image,
                said,
            )
        });
        let status_on = view.on("view:status");
        let marks = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_1))
            .children(strip_of)
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
                }).children(quiet).children(earlier_mark.take()).children(state.map(|((glyph, said), hue)| {
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
        // Once a second Mac is listed, every row keeps a column for the Mac it runs on, so the chips line up.
        let mac_column = crate::pairing::shows_chips(crate::pairing::macs(cx)).then(|| {
            let chip = crate::pairing::chip(row.mac.as_deref(), cx).map(|chip| crate::pairing::pill(look, SharedString::from(format!("mac-{}", row.id)), &chip));
            div().flex_none().w(look.length(token::SPACE_12) * 1.25).flex().items_center().children(chip)
        });
        // A top row of a live Job is dragged to a new place in its project, as Electron's deck does.
        let lit = crate::clash_mark::lit(cx) == Some(row.id.as_ref());
        let movable = row.depth == 0 && !row.ended;
        let dragging = cx.has_active_drag();
        let lifted = dragging && self.lifted.as_ref() == Some(&row.id);
        let drop_here = self.drop.as_ref().filter(|(at, _)| dragging && *at == row.id).map(|(_, after)| *after);
        let dragged = DraggedRow { id: row.id.clone(), title: row.title.clone() };
        let strip = row_card(look, SharedString::from(format!("job-{}", row.id)), opened, focused, !row.place, pad)
            .when(movable, |strip| strip.on_drag(dragged, |dragged, _, _, cx| cx.new(|_| RowGhost(dragged.title.clone()))))
            .when(lifted, |strip| strip.opacity(0.55))
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
            .when(!opened && !needs, |strip| strip.child(row_wash(look, lamp_ink)))
            .child(row_edge(look, lamp_ink))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .h(line)
                    .child(chevron)
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
                    .children(mac_column)
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
            .when(!row.place, |strip| {
                strip
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
                    }))
            });
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
                // A starter's line runs down the gutter from its row to the last Job it started, from the
                // top of the gap above, and turns into this row at its title (Pocket's `.pk-guide`).
                let gap = if after_job { look.length(token::SPACE_2) } else { gpui::px(0.) };
                let inset = look.length(token::SPACE_3);
                let guide = look.color(token::BORDER_STRONG);
                let runs = |column: usize| div().absolute().top(-gap).bottom_0().left(step_in * column as f32 + inset).w(rail).bg(guide);
                row_line
                    .children(row.through.iter().enumerate().filter(|(_, on)| **on).map(|(column, _)| runs(column)))
                    .when(!row.ends, |row_line| row_line.child(runs(row.depth - 1)))
                    .child(
                        div()
                            .absolute()
                            .top(-gap)
                            .left(step_in * (row.depth - 1) as f32 + inset)
                            .w(step_in - inset)
                            .h(gap + look.length(token::BORDER_WIDTH) + pad + line / 2. + rail / 2.)
                            .border_l(rail)
                            .border_b(rail)
                            .rounded_bl(look.length(token::RADIUS_MD))
                            .border_color(guide),
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
                    group_line(look, at == 0).id(("project", at)).child(name.clone()),
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
                div().id("search").key_context(crate::keymap::TEXT_BOX).flex_1().min_w_0().child(list_search(search, look)),
                Role::SearchInput,
                "Search Jobs",
            )
        });
        list_bar(look)
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
                list_control(look)
                    .id("scope")
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
                    .on_action(cx.listener(Self::on_fold_row))
                    .on_action(cx.listener(Self::on_unfold_row))
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
            // Skip on the Refresh row goes to Fleet, which keeps the row skipped.
            if let crate::titlebar::Asked::SkipOffers(id) = asked {
                match fleet_of.upgrade().and_then(|list| list.read(cx).fleet()) {
                    Some(fleet) => fleet.update(cx, |fleet, cx| fleet.skip_offers(id.clone(), cx)),
                    None => Note::warn("not sent: no Fleet is connected").field("route", "/fleet/build/offers/skip").emit(),
                }
                return;
            }
            // The bar's build switcher: each press is Fleet's build act, followed until the report settles.
            let act = match asked {
                crate::titlebar::Asked::ChangeBuild { build, adopt } => Some(BuildAct::Change(ipc::ChangeFleetBuild { build: *build, adopt: *adopt })),
                crate::titlebar::Asked::Revert => Some(BuildAct::Revert),
                crate::titlebar::Asked::Keep => Some(BuildAct::Keep),
                crate::titlebar::Asked::Swap => Some(BuildAct::Swap),
                crate::titlebar::Asked::Unpick(branch) => Some(BuildAct::Pick(ipc::PickFleetBuildJob { branch: Some(branch.clone()), session_id: None, picked: false })),
                crate::titlebar::Asked::PickOrigin(branch) => Some(BuildAct::Pick(ipc::PickFleetBuildJob { branch: Some(branch.clone()), session_id: None, picked: true })),
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
            // The Fleet popover opened: the disk is read again. A walk has no Fleet and reads nothing.
            if *asked == crate::titlebar::Asked::ReadDisk {
                if let Some(fleet) = fleet_of.upgrade().and_then(|list| list.read(cx).fleet()) {
                    fleet.update(cx, |fleet, cx| fleet.read_disk(cx));
                }
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
            // The switcher's New project: its sheet opens over the window.
            if *asked == crate::titlebar::Asked::NewProject {
                crate::dispatch::screen::open_new_project(cx);
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

/// A row's card in a list (`ListCard.css`), as the Jobs list draws it and every list of rows on a panel
/// shares: its spacing, edge, ground, hover and the open row's accent. `pad` is its height above and below.
pub(crate) fn row_card(look: &Look, id: SharedString, opened: bool, focused: bool, hover: bool, pad: Pixels) -> gpui::Stateful<gpui::Div> {
    let edge = if opened { token::ACCENT } else { token::BORDER_SUBTLE };
    let ground = if opened {
        token::ACCENT_MUTED
    } else if focused {
        token::BG_HOVER
    } else {
        token::BG_SUNKEN
    };
    let hovered = look.color(token::BG_HOVER);
    let radius = look.length(token::RADIUS_LG);
    div()
        .id(id)
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
        .bg(look.color(ground))
        .when(hover && !opened, |card| card.hover(move |style| style.bg(hovered)))
}

/// A row's lamp as a faint wash over its card's ground.
pub(crate) fn row_wash(look: &Look, lamp: gpui::Hsla) -> gpui::Div {
    div().absolute().inset_0().rounded(look.length(token::RADIUS_LG)).bg(lamp.opacity(HULL))
}

/// The lamp's edge down a row's left side (`ListCard.css` `border-left`).
/// Painted as the card's own rounded shape clipped to its left edge, so the edge follows the
/// corners as a CSS `border-left` does instead of a straight bar poking past them.
pub(crate) fn row_edge(look: &Look, lamp: gpui::Hsla) -> impl IntoElement {
    let wide = look.length(token::SPACE_1);
    let radius = look.length(token::RADIUS_LG);
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let edge = gpui::Bounds { origin: bounds.origin, size: gpui::size(wide, bounds.size.height) };
            let card = gpui::Bounds { origin: bounds.origin, size: gpui::size(bounds.size.width.min(radius * 2. + wide), bounds.size.height) };
            window.with_content_mask(Some(gpui::ContentMask { bounds: edge }), |window| {
                window.paint_quad(gpui::fill(card, lamp).corner_radii(radius));
            });
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// A group's heading in a list, as the Jobs list heads a project: small mono words over a rule.
/// The first group sits at the list's top; each after it keeps a gap above.
pub(crate) fn group_line(look: &Look, first: bool) -> gpui::Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .h(look.length(token::SPACE_6))
        .when(!first, |head| head.mt(look.length(token::SPACE_4)))
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
}

/// The bar over a list: its search field, then the controls beside it.
pub(crate) fn list_bar(look: &Look) -> gpui::Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_2))
        .px(look.length(token::SPACE_3))
        .pt(look.length(token::SPACE_3))
        .pb(look.length(token::SPACE_2))
}

/// The search field of a list's bar.
pub(crate) fn list_search(state: &Entity<InputState>, look: &Look) -> Input {
    Input::new(state)
        .h(look.length(token::H_CONTROL_LG))
        .rounded(look.length(token::RADIUS_LG))
        .prefix(svg().path(icon_path("search")).size(look.length(token::SPACE_4)).text_color(look.color(token::FG_SUBTLE)))
}

/// A control beside a list's search field, at the field's height: the Jobs list's scope.
pub(crate) fn list_control(look: &Look) -> gpui::Div {
    let hovered = look.color(token::BG_HOVER);
    div()
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
        .hover(move |style| style.bg(hovered))
        .text_size(look.length(token::TEXT_SM))
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
                    || this.list.update(cx, |list, cx| list.snooze_key(event, window, cx))
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
            // A layout that hides the merge line's panel hides this queue, its one counterpart (`Deck.tsx`).
            .when(!crate::settings::layout::hidden("dashboard.panels", "merge-line"), |page| page.child(crate::merge_strip::strip(&self.list, window, cx)))
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

    #[gpui::test]
    fn the_job_kept_open_opens_once_listed_and_a_gone_one_leaves_the_list_as_it_was(cx: &mut gpui::TestAppContext) {
        let all = jobs();
        let kept = all[1].id.as_str().to_string();
        let list = cx.new(|cx| JobsList::mock(&all, cx));
        list.update(cx, |list, _| {
            list.resume = Some(kept.clone());
            assert!(list.resume_open());
            assert_eq!(list.opened.as_ref().map(|id| id.as_ref()), Some(kept.as_str()));
            assert!(list.resume.is_none(), "it is opened once");
        });
        let list = cx.new(|cx| JobsList::mock(&all, cx));
        list.update(cx, |list, _| {
            list.resume = Some("a-job-fleet-no-longer-lists".into());
            assert!(!list.resume_open());
            assert!(list.opened.is_none(), "a gone Job opens nothing and says nothing");
            assert!(list.resume.is_none(), "and is let go");
        });
        let list = cx.new(|cx| JobsList::mock(&[], cx));
        list.update(cx, |list, _| {
            list.resume = Some(kept.clone());
            assert!(!list.resume_open());
            assert_eq!(list.resume.as_deref(), Some(kept.as_str()), "it waits for Fleet's Jobs");
        });
    }

    #[test]
    fn ended_jobs_leave_the_list_and_each_project_heads_its_own() {
        let lines = lines_of(&jobs(), &[], Scope::Active, "", &[], &Names::default());
        assert_eq!(lines[0], Line::Project("armada".into()));
        assert!(!titles(&lines).iter().any(|title| title.contains("ended")));
        let projects: Vec<_> = lines.iter().filter_map(|line| match line { Line::Project(name) => Some(name.clone()), _ => None }).collect();
        assert_eq!(projects, vec![SharedString::from("armada"), "sample-web".into(), "sample-api".into()]);
    }

    fn headings(lines: &[Line]) -> Vec<String> {
        lines.iter().filter_map(|line| match line { Line::Project(name) => Some(name.to_string()), Line::Job(_) => None }).collect()
    }

    #[test]
    fn a_job_stands_under_its_projects_name_else_its_repositorys_never_an_id() {
        let all = crate::mock::jobs("jobs-projects").expect("the fixture decodes");
        let names = Names::of(&crate::mock::repositories(), Some(&crate::mock::projects()));
        let lines = lines_of(&all, &[], Scope::Active, "", &[], &names);
        assert_eq!(headings(&lines), ["Armada", "Storefront", "sample-api"]);
        // His Chariot: a folder with no Manifest, which Fleet gives an id of its own, and a Job in a slot cut from it.
        let mut chariot = all[0].clone();
        chariot.manifest_id = Some(ipc::ManifestId::carried("chariot-6e8ee791"));
        chariot.cwd = "/Users/user/Development/chariot/.armada/slots/slot-2".into();
        let mut projects = crate::mock::projects();
        projects.projects[1].id = "0193".into();
        projects.projects[1].name = "Chariot".into();
        chariot.hosted.as_mut().expect("started for a project").project = Some("0193".into());
        let names = Names::of(&[], Some(&projects));
        assert_eq!(group_of(&chariot, &names).1, "Chariot");
        chariot.hosted = None;
        assert_eq!(group_of(&chariot, &names).1, "chariot", "in no project, the repository's folder names it");
        let served: ipc::RepositoryList = ipc::decode(
            "repositories",
            br#"{"repositories":[{"root":"/Users/user/Development/chariot","records_root":"/r","implicit_id":"chariot-6e8ee791"}]}"#,
        )
        .expect("repositories");
        assert_eq!(group_of(&chariot, &Names::of(&served.repositories, None)).1, "chariot", "a served repository by its name");
    }

    #[test]
    fn a_job_with_no_title_reads_new_job_never_its_id() {
        let all = crate::mock::jobs("jobs-projects").expect("the fixture decodes");
        let lines = lines_of(&all, &[], Scope::All, "", &[], &Names::default());
        let untitled = lines.iter().find_map(|line| match line { Line::Job(row) if row.id.as_ref() == "mock-06" => Some(row.title.to_string()), _ => None });
        assert_eq!(untitled.as_deref(), Some(UNTITLED));
        let mut blank = all[0].clone();
        blank.title = Some("  ".into());
        assert_eq!(title_of(&blank).as_ref(), UNTITLED);
        assert!(lines.iter().all(|line| match line { Line::Job(row) => row.title != row.id, Line::Project(_) => true }));
    }

    #[test]
    fn one_project_draws_its_jobs_with_no_head() {
        let armada: Vec<_> = jobs().into_iter().filter(|job| job.manifest_id.as_ref().is_some_and(|id| id.as_str() == "armada")).collect();
        let lines = lines_of(&armada, &[], Scope::Active, "", &[], &Names::default());
        assert!(!lines.is_empty());
        assert!(lines.iter().all(|line| matches!(line, Line::Job(_))));
    }

    #[gpui::test]
    fn an_ended_job_the_palette_opens_stays_opened_in_every_scope_whether_or_not_the_scope_lists_it(cx: &mut gpui::TestAppContext) {
        let all = jobs();
        let ended = all.iter().find(|job| job.state == ipc::SessionState::Ended).expect("an ended Job").id.as_str().to_string();
        for (scope, _, _) in SCOPES {
            let list = cx.new(|cx| JobsList::mock(&all, cx));
            list.update(cx, |list, cx| {
                list.scope = scope;
                list.relist(cx);
                list.open(ended.clone().into(), cx);
                list.relist(cx);
            });
            list.read_with(cx, |list, _| {
                assert_eq!(list.opened_id().as_deref(), Some(ended.as_str()), "{scope:?}");
                assert!(list.record(&ended).is_some(), "{scope:?}: the Job view reads the record, not the row");
                let listed = list.lines.iter().any(|line| matches!(line, Line::Job(row) if row.id.as_ref() == ended.as_str()));
                assert_eq!(listed, matches!(scope, Scope::Ended | Scope::All), "{scope:?}");
            });
        }
    }

    #[test]
    fn the_placed_order_holds_and_a_search_narrows_by_title() {
        let all = jobs();
        let second = titles(&lines_of(&all, &[], Scope::Active, "", &[], &Names::default()))[1].clone();
        let id = all.iter().find(|job| job.title.as_deref() == Some(second.as_str())).map(|job| job.id.as_str().to_string()).expect("listed");
        assert_eq!(titles(&lines_of(&all, &[], Scope::Active, "", &[id], &Names::default()))[0], second);
        let found = titles(&lines_of(&all, &[], Scope::Active, &second[..6].to_uppercase(), &[], &Names::default()));
        assert!(found.contains(&second) && found.iter().all(|title| title.to_lowercase().contains(&second[..6].to_lowercase())));
    }

    #[test]
    fn jobs_not_yet_placed_stand_needs_first_and_the_loudest_first() {
        let lines = lines_of(&jobs(), &[], Scope::Active, "", &[], &Names::default());
        let web: Vec<_> = titles(&lines).into_iter().filter(|title| ["Checkout", "Flaky", "Upgrade", "Dark"].iter().any(|word| title.starts_with(word))).collect();
        assert_eq!(web, ["Checkout page loads twice", "Flaky login test", "Upgrade the router", "Dark mode for the settings page"]);
        // A placed Job keeps its place whatever its state.
        let dark = jobs().into_iter().find(|job| job.title.as_deref() == Some("Dark mode for the settings page")).expect("listed");
        let placed = titles(&lines_of(&jobs(), &[], Scope::Active, "", &[dark.id.as_str().to_string()], &Names::default()));
        assert_eq!(placed.iter().find(|title| web.contains(title)).map(String::as_str), Some("Dark mode for the settings page"));
    }

    /// The fixture with a Job that needs him and a working one snoozed, and those two ids.
    fn with_two_snoozed() -> (Vec<SessionRecord>, [String; 2]) {
        let all = jobs();
        let live = |job: &&SessionRecord| job.state == ipc::SessionState::Live && job.hosted.as_ref().is_none_or(|hosted| hosted.paused.is_none());
        let needs = |job: &SessionRecord| job.mark.is_some_and(|mark| mark.lamp == JobLamp::Needs);
        let asking = all.iter().filter(live).find(|job| needs(job)).expect("a Job that needs him").id.as_str().to_string();
        let working = all
            .iter()
            .filter(live)
            .find(|job| !needs(job) && job.mark.is_some_and(|mark| mark.state != JobState::Quiet))
            .expect("a working Job")
            .id
            .as_str()
            .to_string();
        let ids = [asking, working];
        let all = all
            .into_iter()
            .map(|mut job| {
                if ids.iter().any(|id| id == job.id.as_str()) {
                    job.snoozed_until = Some(ipc::Instant::carried("2026-11-13T16:30:00.000Z"));
                }
                job
            })
            .collect();
        (all, ids)
    }

    /// The ids `scope` lists as Jobs of its own, leaving out a starter drawn only for its place.
    fn listed(jobs: &[SessionRecord], scope: Scope) -> Vec<String> {
        lines_of(jobs, &[], scope, "", &[], &Names::default())
            .iter()
            .filter_map(|line| match line { Line::Job(row) if !row.place => Some(row.id.to_string()), _ => None })
            .collect()
    }

    #[test]
    fn a_snoozed_job_is_not_in_needs_you() {
        let (all, [asking, _]) = with_two_snoozed();
        assert!(!listed(&all, Scope::Needs).contains(&asking));
    }

    #[test]
    fn a_snoozed_job_is_not_in_active() {
        let (all, [asking, working]) = with_two_snoozed();
        let active = listed(&all, Scope::Active);
        assert!(!active.contains(&asking) && !active.contains(&working), "{active:?}");
    }

    #[test]
    fn the_snoozed_scope_lists_exactly_the_snoozed_jobs() {
        let (all, mut ids) = with_two_snoozed();
        let mut snoozed = listed(&all, Scope::Snoozed);
        snoozed.sort();
        ids.sort();
        assert_eq!(snoozed, ids);
    }

    #[test]
    fn all_keeps_the_snoozed_jobs() {
        let (all, [asking, working]) = with_two_snoozed();
        let every = listed(&all, Scope::All);
        assert!(every.contains(&asking) && every.contains(&working), "{every:?}");
    }

    #[test]
    fn a_snoozed_starter_comes_into_active_for_its_place_alone() {
        let tree: Vec<SessionRecord> = crate::mock::jobs("jobs-tree")
            .expect("the fixture decodes")
            .into_iter()
            .map(|mut job| {
                if job.id.as_str() == "t-land" {
                    job.snoozed_until = Some(ipc::Instant::carried("2026-11-13T16:30:00.000Z"));
                }
                job
            })
            .collect();
        let land = lines_of(&tree, &[], Scope::Active, "", &[], &Names::default())
            .iter()
            .find_map(|line| match line { Line::Job(row) if row.id.as_ref() == "t-land" => Some(row.place), _ => None });
        assert_eq!(land, Some(true));
    }

    #[test]
    fn a_quiet_job_leaves_active_for_the_quiet_scope() {
        let quiet = titles(&lines_of(&jobs(), &[], Scope::Quiet, "", &[], &Names::default()));
        assert_eq!(quiet, ["Retry webhooks on a 503"]);
        assert!(!titles(&lines_of(&jobs(), &[], Scope::Active, "", &[], &Names::default())).contains(&quiet[0]));
    }

    #[test]
    fn needs_you_holds_the_jobs_that_need_him_with_their_starters_for_place() {
        let tree = crate::mock::jobs("jobs-tree").expect("the fixture decodes");
        let lines = lines_of(&tree, &[], Scope::Needs, "", &[], &Names::default());
        let rows: Vec<(String, usize, bool)> =
            lines.iter().filter_map(|line| match line { Line::Job(row) => Some((row.title.to_string(), row.depth, row.place)), Line::Project(_) => None }).collect();
        assert_eq!(
            rows,
            [
                ("Jobs across home and work Macs".to_string(), 0, false),
                ("Land the native stack".to_string(), 0, true),
                ("Fix the walk at 820".to_string(), 1, true),
                ("Re-take the shots".to_string(), 2, false),
            ]
        );
        let calm: Vec<SessionRecord> = tree.iter().cloned().map(|mut job| { job.mark = None; job }).collect();
        assert!(lines_of(&calm, &[], Scope::Needs, "", &[], &Names::default()).is_empty(), "nothing needing him draws nothing, not even a head");
        // A paused starter stays out of Active but keeps its place over the Job it started.
        let paused: Vec<SessionRecord> = tree
            .into_iter()
            .map(|mut job| {
                if job.id.as_str() == "t-walk" {
                    if let Some(hosted) = job.hosted.as_mut() {
                        hosted.paused = Some(serde_json::from_str(r#"{"at":"2026-11-08T20:00:00Z","resuming":false}"#).expect("a pause decodes"));
                    }
                }
                job
            })
            .collect();
        let active = lines_of(&paused, &[], Scope::Active, "", &[], &Names::default());
        let walk = active.iter().find_map(|line| match line { Line::Job(row) if row.title.as_ref() == "Fix the walk at 820" => Some((row.depth, row.place)), _ => None });
        let shots = active.iter().find_map(|line| match line { Line::Job(row) if row.title.as_ref() == "Re-take the shots" => Some(row.depth), _ => None });
        assert_eq!((walk, shots), (Some((1, true)), Some(2)));
    }

    #[test]
    fn an_ended_job_does_not_wear_the_quiet_jobs_glyph() {
        assert_eq!(glyph_of(JobState::Quiet, None), ("unplug", "Not reachable".into()));
        assert_eq!(glyph_of(JobState::Ended, None), ("power-off", "Ended".into()));
    }

    #[test]
    fn the_ended_scope_holds_only_ended_jobs() {
        let ended = titles(&lines_of(&jobs(), &[], Scope::Ended, "", &[], &Names::default()));
        assert!(!ended.is_empty() && ended.iter().all(|title| title.contains("ended")), "{ended:?}");
    }

    #[test]
    fn a_row_carries_its_issue_its_workflow_step_and_what_it_is_doing() {
        let lines = lines_of(&jobs(), &[], Scope::Active, "", &[], &Names::default());
        let row = |title: &str| {
            lines.iter().find_map(|line| match line {
                Line::Job(row) if row.title == title => Some(row.clone()),
                _ => None,
            }).expect(title)
        };
        let first = row("Fix the merge line freshness chip");
        assert_eq!(first.issue, Some(("#3290".into(), "Merge line freshness chip stays green when stale".into())));
        // The list's scenarios follow no workflow; the workflow walk's do, its first at Build.
        let flows = lines_of(&crate::mock::jobs("job-workflow").expect("the fixture decodes"), &[], Scope::Active, "", &[], &Names::default());
        let flow = flows.iter().find_map(|line| match line {
            Line::Job(row) if row.title == "Fix the merge line freshness chip" => Some(row.flow.clone()),
            _ => None,
        });
        assert_eq!(flow, Some(Some(("Ship a screen".into(), "Build".into()))));
        assert_eq!(first.flow, None);
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

    fn tree() -> Vec<SessionRecord> {
        crate::mock::jobs("jobs-tree").expect("the fixture decodes")
    }

    fn drawn(list: &JobsList) -> Vec<String> {
        list.rows().map(|row| row.id.to_string()).collect()
    }

    fn shut(parents: &[&str]) -> Folds {
        Folds { root_only: false, parents: parents.iter().map(|id| id.to_string()).collect() }
    }

    #[test]
    fn a_folded_parent_draws_none_of_its_sub_jobs_and_its_siblings_keep_theirs() {
        let open = fold_lines(lines_of(&tree(), &[], Scope::Active, "", &[], &Names::default()), &Folds::default());
        assert_eq!(titles(&open).len(), 8);
        let folded = fold_lines(lines_of(&tree(), &[], Scope::Active, "", &[], &Names::default()), &shut(&["t-land"]));
        let titles = titles(&folded);
        assert!(titles.contains(&"Land the native stack".to_string()) && titles.contains(&"Pair the work Mac".to_string()));
        for gone in ["Fix the walk at 820", "Re-take the shots", "Write the docs row"] {
            assert!(!titles.contains(&gone.to_string()), "{gone} is folded away");
        }
        let kids = folded.iter().find_map(|line| match line {
            Line::Job(row) if row.id.as_ref() == "t-land" => row.kids.clone(),
            _ => None,
        });
        assert_eq!(kids.map(|kids| kids.folded), Some(true));
        // A Job with nothing under it has no fold to draw.
        let chip = folded.iter().find_map(|line| match line {
            Line::Job(row) if row.id.as_ref() == "t-chip" => Some(row.kids.clone()),
            _ => None,
        });
        assert_eq!(chip, Some(None));
    }

    fn strip_of(jobs: &[SessionRecord], parent: &str) -> Vec<Sub> {
        let folded = fold_lines(lines_of(jobs, &[], Scope::Active, "", &[], &Names::default()), &shut(&[parent]));
        folded
            .iter()
            .find_map(|line| match line {
                Line::Job(row) if row.id.as_ref() == parent => row.kids.as_ref().map(|kids| kids.strip.clone()),
                _ => None,
            })
            .expect("a folded parent has a strip")
    }

    #[test]
    fn a_folded_parent_draws_one_glyph_per_state_needs_you_first_and_names_the_jobs_in_it() {
        let strip = strip_of(&tree(), "t-land");
        // The need first, the failure next, then running. The sub-Job two down is in it too.
        let words: Vec<String> = strip.iter().map(|sub| sub.words().to_string()).collect();
        assert_eq!(words, ["Needs you: Re-take the shots", "Checks failed: Write the docs row", "Working: Fix the walk at 820"]);
        assert_eq!(strip[0].mark.lamp, JobLamp::Needs);
        assert_eq!((strip[0].look().0, strip[0].look().1), ("eye", token::STATUS_AWAITING_REVIEW));
        // Open, a parent keeps no strip.
        let open = fold_lines(lines_of(&tree(), &[], Scope::Active, "", &[], &Names::default()), &Folds::default());
        let kept = open.iter().find_map(|line| match line {
            Line::Job(row) if row.id.as_ref() == "t-land" => row.kids.clone(),
            _ => None,
        });
        assert_eq!(kept.map(|kids| (kids.folded, kids.strip.len())), Some((false, 0)));
    }

    #[test]
    fn sub_jobs_in_one_state_share_a_glyph_whose_tooltip_names_every_one_of_them() {
        let mut jobs = tree();
        let mut twin = jobs.iter().find(|job| job.id.as_str() == "t-pair").cloned().expect("a working sub-Job");
        twin.id = ipc::SessionId::carried("t-pair-2".to_string());
        twin.title = Some("Pair the second Mac".to_string());
        jobs.push(twin);
        let strip = strip_of(&jobs, "t-macs");
        // Three sub-Jobs, two states: two glyphs, not three.
        assert_eq!(strip.len(), 2);
        let working = &strip[0];
        assert_eq!(working.titles.len(), 2);
        assert_eq!(working.words().as_ref(), "Working: Pair the work Mac, Pair the second Mac");
        assert_eq!(strip[1].words().as_ref(), "Subagent working: Read Fleet on the home Mac");
        // Each state is drawn once, however many Jobs stand in it.
        let states: Vec<_> = strip.iter().map(|sub| sub.mark.state).collect();
        assert_ne!(states[0], states[1]);
    }

    #[test]
    fn a_parent_drawn_for_its_place_never_folds() {
        let lines = fold_lines(lines_of(&tree(), &[], Scope::Needs, "", &[], &Names::default()), &Folds { root_only: true, parents: Default::default() });
        let place = lines.iter().find_map(|line| match line {
            Line::Job(row) if row.id.as_ref() == "t-walk" => Some((row.place, row.kids.clone())),
            _ => None,
        });
        assert_eq!(place, Some((true, None)));
        assert!(titles(&lines).contains(&"Re-take the shots".to_string()), "the Job that needs him stays drawn under its starters for place");
    }

    #[gpui::test]
    fn one_parent_folds_and_unfolds_from_its_row(cx: &mut gpui::TestAppContext) {
        let list = cx.new(|cx| JobsList::mock(&tree(), cx));
        list.update(cx, |list, cx| list.set_row_fold("t-land", true, cx));
        list.read_with(cx, |list, _| {
            let rows = drawn(list);
            assert!(rows.contains(&"t-land".to_string()) && !rows.contains(&"t-walk".to_string()) && rows.contains(&"t-pair".to_string()));
        });
        list.update(cx, |list, cx| list.set_row_fold("t-land", false, cx));
        list.read_with(cx, |list, _| assert_eq!(drawn(list).len(), 8));
    }

    #[gpui::test]
    fn left_and_right_fold_and_unfold_the_parent_the_keys_are_on(cx: &mut gpui::TestAppContext) {
        let list = cx.new(|cx| JobsList::mock(&tree(), cx));
        list.update(cx, |list, cx| {
            list.focused = Some("t-land".into());
            list.fold_focused(false, cx);
        });
        // → on an open parent changes nothing.
        list.read_with(cx, |list, _| assert_eq!(drawn(list).len(), 8));
        list.update(cx, |list, cx| list.fold_focused(true, cx));
        list.read_with(cx, |list, _| {
            assert!(!drawn(list).contains(&"t-walk".to_string()));
            assert_eq!(list.focused.as_deref(), Some("t-land"));
        });
        list.update(cx, |list, cx| list.fold_focused(false, cx));
        list.read_with(cx, |list, _| assert!(drawn(list).contains(&"t-shots".to_string())));
        // A Job with nothing under it is left alone.
        list.update(cx, |list, cx| {
            list.focused = Some("t-chip".into());
            list.fold_focused(true, cx);
        });
        list.read_with(cx, |list, _| assert_eq!(drawn(list).len(), 8));
    }

    #[gpui::test]
    fn the_keys_follow_a_job_that_folds_away_to_the_parent_that_holds_it(cx: &mut gpui::TestAppContext) {
        let list = cx.new(|cx| JobsList::mock(&tree(), cx));
        list.update(cx, |list, cx| {
            list.focused = Some("t-shots".into());
            list.set_row_fold("t-land", true, cx);
        });
        list.read_with(cx, |list, _| assert_eq!(list.focused.as_deref(), Some("t-land")));
    }

    #[gpui::test]
    fn fold_all_and_unfold_all_reach_every_parent_at_every_depth(cx: &mut gpui::TestAppContext) {
        let list = cx.new(|cx| JobsList::mock(&tree(), cx));
        list.update(cx, |list, cx| list.set_all_folds(true, cx));
        list.read_with(cx, |list, _| {
            assert_eq!(drawn(list), ["t-macs", "t-land", "t-chip"]);
            assert_eq!(list.fold_facts(), (true, false, true));
        });
        list.update(cx, |list, cx| list.set_all_folds(false, cx));
        list.read_with(cx, |list, _| {
            assert_eq!(drawn(list).len(), 8);
            assert_eq!(list.fold_facts(), (true, false, false));
        });
    }

    #[gpui::test]
    fn root_jobs_only_folds_every_parent_and_an_unfold_ends_it_keeping_the_rest_folded(cx: &mut gpui::TestAppContext) {
        let list = cx.new(|cx| JobsList::mock(&tree(), cx));
        list.update(cx, |list, cx| list.set_root_only(true, cx));
        list.read_with(cx, |list, _| {
            assert_eq!(drawn(list), ["t-macs", "t-land", "t-chip"]);
            assert!(list.fold_facts().1);
        });
        // Opening one parent ends the switch and leaves every other parent folded.
        list.update(cx, |list, cx| list.set_row_fold("t-land", false, cx));
        list.read_with(cx, |list, _| {
            assert!(!list.fold_facts().1);
            let rows = drawn(list);
            assert!(rows.contains(&"t-walk".to_string()) && rows.contains(&"t-docs".to_string()));
            assert!(!rows.contains(&"t-shots".to_string()) && !rows.contains(&"t-pair".to_string()));
        });
        list.update(cx, |list, cx| list.set_root_only(false, cx));
        list.update(cx, |list, cx| list.fold_command("root-only", cx));
        list.read_with(cx, |list, _| assert!(list.fold_facts().1));
        list.update(cx, |list, cx| list.fold_command("unfold-all", cx));
        list.read_with(cx, |list, _| assert_eq!((drawn(list).len(), list.fold_facts().1), (8, false)));
    }

    #[test]
    fn the_fold_state_survives_a_save_and_a_load() {
        let path = std::env::temp_dir().join(format!("armada-folds-test-{}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        assert_eq!(Folds::load(Some(&path)), Folds::default(), "nothing kept, nothing folded");
        let folds = Folds { root_only: true, parents: ["t-land".to_string(), "t-walk".to_string()].into_iter().collect() };
        folds.save(Some(&path));
        assert_eq!(Folds::load(Some(&path)), folds);
        let per_parent = shut(&["t-macs"]);
        per_parent.save(Some(&path));
        assert_eq!(Folds::load(Some(&path)), per_parent, "the switch off is kept off");
        let _ = std::fs::remove_file(&path);
    }
}
