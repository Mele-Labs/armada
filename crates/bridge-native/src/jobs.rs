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
use ipc::{JobGroup, JobLamp, JobMark, JobState, SessionRecord, SessionState, WaitingSource};

use crate::assets::icon_path;
use crate::dropdown::{under, Dropdown, Entry, Picked, Row as MenuRow};
use crate::fleet::{BuildAct, FleetConnection};
use crate::job_view::JobView;
use crate::keymap::Act;
use crate::labels::labelled;
use crate::log::Note;
use crate::look::{token, ActiveLook, Look};

pub(crate) mod judging;

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

    /// The words the view menu's Show group and its button use: `All Jobs` for the Jobs the list holds at rest.
    fn show_label(self) -> &'static str {
        SHOWS.iter().find(|(scope, ..)| *scope == self).map_or("All Jobs", |(.., label)| label)
    }

    fn of(id: &str) -> Scope {
        SCOPES.iter().find(|(_, held, _)| *held == id).map_or(Scope::Active, |(scope, ..)| *scope)
    }
}

/// What the view menu's Show group offers, in its order. `All Jobs` is the scope the list opens on (`Scope::Active`);
/// the old `All` scope, which also held every ended Job, is no longer a choice, and a deck file that holds it reads as `All Jobs`.
const SHOWS: [(Scope, &str, &str); 6] = [
    (Scope::Active, "active", "All Jobs"),
    (Scope::Needs, "needs", "Needs you only"),
    (Scope::Paused, "paused", "Paused"),
    (Scope::Snoozed, "snoozed", "Snoozed"),
    (Scope::Quiet, "quiet", "Quiet"),
    (Scope::Ended, "ended", "Ended"),
];

/// How the list is grouped, chosen in the view menu's Group part and kept between runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Group {
    /// Headed by the state a Job stands in: Needs you, Ready to try, Working, Waiting on ci.
    #[default]
    State,
    /// Headed by the project, as the list was before the view button.
    Project,
    /// No headings: every Job in one list, by title.
    List,
}

const GROUPS: [(Group, &str, &str); 3] = [(Group::State, "state", "By state"), (Group::Project, "project", "By project"), (Group::List, "list", "One list")];

impl Group {
    fn id(self) -> &'static str {
        GROUPS.iter().find(|(group, ..)| *group == self).map_or("state", |(_, id, _)| id)
    }

    fn label(self) -> &'static str {
        GROUPS.iter().find(|(group, ..)| *group == self).map_or("By state", |(.., label)| label)
    }

    /// The group an id names; By state for one this build does not know.
    fn of(id: &str) -> Group {
        GROUPS.iter().find(|(_, held, _)| *held == id).map_or(Group::State, |(group, ..)| *group)
    }
}

/// The view menu: the Group part, then the Show part, then the switch that folds every parent.
fn view_entries(scope: Scope, group: Group, root_only: bool, hide_done: bool) -> Vec<Entry> {
    let mut entries = vec![Entry::Label("Group")];
    entries.extend(GROUPS.iter().map(|(held, id, label)| Entry::Choice { id, label: (*label).into(), on: *held == group }));
    entries.push(Entry::Rule);
    entries.push(Entry::Label("Show"));
    entries.extend(SHOWS.iter().map(|(held, id, label)| Entry::Choice { id, label: (*label).into(), on: *held == scope }));
    entries.push(Entry::Rule);
    entries.push(Entry::Toggle { id: "done_jobs", label: "Done Jobs".into(), on: !hide_done });
    entries.push(Entry::Toggle { id: "root_only", label: "Root Jobs only".into(), on: root_only });
    entries
}

/// The ink of the view button and its chevron: quiet while every Job is shown, Needs you's amber once the list is narrowed.
fn view_ink(scope: Scope) -> crate::look::Token {
    if scope == Scope::Active { token::FG_MUTED } else { token::STATUS_AWAITING_REVIEW }
}

/// The words on the view button: the group alone while every Job is shown, else what is shown first (`Needs you · By state`).
fn view_label(scope: Scope, group: Group) -> String {
    let shown = match scope {
        Scope::Needs => "Needs you",
        other => other.show_label(),
    };
    if scope == Scope::Active { group.label().to_string() } else { format!("{shown} \u{b7} {}", group.label()) }
}

mod asleep_mac;
#[cfg(test)]
mod prioritise_tests;
#[cfg(test)]
mod ending_tests;

/// An End he pressed that Fleet has not yet served: what the record held before it, when he pressed it, and the
/// Mac it ends on, by name, where that is not this one.
struct Ending {
    was: (SessionState, Option<ipc::Instant>, Option<JobMark>),
    at: ipc::Instant,
    on: Option<String>,
}

impl Ending {
    /// `job` as it reads while its End waits: ended, at the press, in the Done group.
    fn hold(&self, job: &mut SessionRecord) {
        job.state = SessionState::Ended;
        job.ended_at = Some(self.at.clone());
        job.mark = job.mark.map(|_| JobMark { state: JobState::Ended, lamp: JobLamp::Idle, group: JobGroup::Done });
    }

    /// The row's line until Fleet serves the End.
    fn said(&self) -> SharedString {
        match &self.on {
            Some(mac) => format!("Ending on {mac}").into(),
            None => "Ending".into(),
        }
    }
}

/// A Job as its row and its card draw it.
#[derive(Clone, Debug, PartialEq)]
struct Row {
    id: SharedString,
    title: SharedString,
    project: SharedString,
    paused: bool,
    /// The person's one prioritised Job (`SessionRecord.prioritised`): its builds run first on a loaded machine.
    prioritised: bool,
    /// Paused and kept quiet until this time, when Fleet resumes it (`snoozed_until`).
    snoozed: Option<chrono::DateTime<chrono::Local>>,
    /// Waiting for a worktree slot, as its glyph's tooltip says it (`slot_wait`).
    slot_wait: Option<SharedString>,
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
    /// The Job's steps as bars, the step it is on and how long it has been on it.
    track: Option<crate::job_view::column_strip::Track>,
    /// What the Job is doing now, or last said.
    now: Option<SharedString>,
    /// Not a Job: an idea saved to start later, drawn dashed with Start and Remove (`QueuedRow`).
    queued: Option<crate::queued::QueuedIdea>,
    /// Drawn only for its place, as the starter of a listed Job the scope leaves out: muted, never pressed.
    place: bool,
    /// The Mac the Job runs on, by id; none is this one (`crate::pairing`).
    mac: Option<SharedString>,
    /// The Mac the Job is moving to, by id, while its move waits for the turn to end (`crate::move_to_mac`).
    moving_to: Option<SharedString>,
    /// The Job went to the other Mac and ended here: its row there stands in for this one.
    moved: bool,
    /// A reply answers a question of his he has not read (`answer_unread`): a dot in his hue.
    answer: bool,
    /// The Job runs on a Mac that cannot be reached: it stands under that Mac's fold, dimmed, and pulls at him nowhere.
    asleep: bool,
    /// He pressed End and Fleet still serves the Job live: drawn ended, with this line in place of what it is doing.
    ending: Option<SharedString>,
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
    /// The Done Jobs switch, off: Jobs that stand under Done are left out. Shown is the default, so only the off state is kept.
    hide_done: bool,
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
        let done = self.hide_done.then(|| HIDE_DONE.to_string());
        switch.into_iter().chain(done).chain(self.parents.iter().map(|id| format!("fold {id}"))).collect::<Vec<_>>().join("\n")
    }

    fn from_text(text: &str) -> Folds {
        Folds {
            root_only: text.lines().any(|line| line == ROOT_ONLY),
            hide_done: text.lines().any(|line| line == HIDE_DONE),
            parents: text.lines().filter_map(|line| line.strip_prefix("fold ")).map(str::to_string).collect(),
        }
    }
}

const ROOT_ONLY: &str = "root-only";
const HIDE_DONE: &str = "hide-done";

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
    const DEFAULT: View = View([false, false, false, false, true, true, true]);

    fn on(self, id: &str) -> bool {
        VIEW_FIELDS.iter().position(|(field, _)| *field == id).is_some_and(|at| self.0[at])
    }

    fn flip(&mut self, id: &str) {
        if let Some(at) = VIEW_FIELDS.iter().position(|(field, _)| *field == id) {
            self.0[at] = !self.0[at];
        }
    }

    /// The menu's Job Card row: the switches for each field open beside it, and stay open as they are flipped.
    fn entries(self) -> Vec<Entry> {
        let switches = VIEW_FIELDS.iter().zip(self.0).map(|((id, label), on)| Entry::Row(MenuRow::new(id, *label).ticked(on).keeps_open())).collect();
        vec![Entry::Row(MenuRow::new("job_card", "Job Card").icon("sliders-horizontal").keeps_open().acts_on_arrival(switches))]
    }
}

/// A state's glyph and the words its tooltip says, as Electron's row draws them
/// (`SessionFrame.tsx`, `MARK`; `session-state.ts`, `jobStateOf`).
pub(crate) fn glyph_of(state: JobState, pr: Option<&str>) -> (&'static str, SharedString) {
    let (glyph, said) = match state {
        JobState::Working => ("circle-dot", "Working"),
        JobState::Running => ("square-terminal", "Running"),
        JobState::Subagent => ("split", "Subagent working"),
        JobState::Waiting | JobState::Ready | JobState::StoppedAsks => (if state == JobState::StoppedAsks { "octagon-alert" } else { "eye" }, "Waiting on you"),
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
        JobState::Waiting | JobState::Ready | JobState::Done | JobState::Unstarted | JobState::StoppedAsks | JobState::Stuck | JobState::Failing => {
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
        JobLamp::Needs | JobLamp::Ready => token::STATUS_AWAITING_REVIEW,
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
        JobGroup::Needs | JobGroup::Ready => 0,
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

/// Where a Job stands, as By state heads it. The first four are the groups he reads first, in his order: Needs you, Ready to
/// try right under it, then Working and Waiting on ci; a Job that fits none of them goes to the group after, and ideas saved
/// for later stand last. The order is the derive's, so a variant moves a group by where it is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Stand {
    Needs,
    Ready,
    Working,
    Ci,
    /// Held for a slot, a limit or another Job, or stopped: nothing runs and nothing asks him.
    Held,
    /// Alive and quiet: its turn ended with nothing for him.
    Idle,
    /// Its work is done and handed in, and nothing is owed on it: the Done Jobs switch shows or hides the group.
    Done,
    Ended,
    Saved,
}

impl Stand {
    fn words(self) -> &'static str {
        match self {
            Stand::Needs => "Needs you",
            Stand::Working => "Working",
            Stand::Ci => "Waiting on ci",
            Stand::Ready => "Ready to try",
            Stand::Held => "Held",
            Stand::Idle => "Idle",
            Stand::Done => "Done",
            Stand::Ended => "Ended",
            Stand::Saved => "Saved for later",
        }
    }

    /// The hue of the group's dot and words, as the approved page draws it.
    fn hue(self) -> crate::look::Token {
        match self {
            Stand::Needs => token::STATUS_AWAITING_REVIEW,
            Stand::Working => token::STATUS_RUNNING,
            Stand::Ci => token::STATUS_NOT_STARTED,
            Stand::Ready => token::STATUS_COMPLETED_SUCCESS,
            Stand::Held => token::STATUS_HANDED_IN,
            Stand::Idle | Stand::Done | Stand::Ended | Stand::Saved => token::FG_MUTED,
        }
    }
}

/// The state group a Job stands in, from Fleet's mark. A Job whose only open item is a Ready to try stands there, not
/// under Needs you; one with a question or a failure stays under Needs you. A Job with a pull request whose checks
/// still run, and no work of its own going, is Waiting on ci. Anything else follows Fleet's group for it.
fn stand_in(job: &SessionRecord) -> Stand {
    if job.state == SessionState::Ended || crate::move_to_mac::moved(job) {
        return Stand::Ended;
    }
    let Some(mark) = mark_of(job) else { return Stand::Idle };
    let open = || job.waiting_for.iter().filter(|item| item.source != WaitingSource::Done);
    // Fleet marks a Job whose every owed item is built work for him to look at Ready; an older Fleet marks it Needs, and
    // the items say the same.
    if mark.lamp == JobLamp::Ready
        || mark.group == JobGroup::Ready
        || (mark.lamp == JobLamp::Needs && open().next().is_some() && open().all(|item| item.source == WaitingSource::ReadyToTry))
    {
        return Stand::Ready;
    }
    if matches!(mark.lamp, JobLamp::Needs | JobLamp::Failed) {
        return Stand::Needs;
    }
    let checking = job.attachments.iter().any(|one| {
        one.kind == "pr"
            && one.state == ipc::AttachmentState::Standing
            && one.detail.get("state").map(String::as_str) != Some("merged")
            && one.detail.get("checks").map(String::as_str) == Some("pending")
    });
    if checking && mark.lamp != JobLamp::Running {
        return Stand::Ci;
    }
    // A Job Fleet marks Done has its own group, apart from the quiet ones Fleet files with it.
    if mark.state == JobState::Done {
        return Stand::Done;
    }
    match mark.group {
        JobGroup::Needs => Stand::Needs,
        JobGroup::Ready => Stand::Ready,
        JobGroup::Working => Stand::Working,
        JobGroup::Held => Stand::Held,
        JobGroup::Done => Stand::Idle,
    }
}

/// Where a Job stands as By state words it, and the hue its group wears: what a chip for the Job elsewhere is drawn in.
pub(crate) fn stand_of(job: &SessionRecord) -> (&'static str, crate::look::Token) {
    let stand = stand_in(job);
    (stand.words(), stand.hue())
}

/// The groups By state leads with, in their order, each with its hue: the key to the colours a Job's chip wears elsewhere.
pub(crate) fn legend() -> [(&'static str, crate::look::Token); 4] {
    [Stand::Needs, Stand::Ready, Stand::Working, Stand::Ci].map(|stand| (stand.words(), stand.hue()))
}

/// A group's heading: its words, how many Jobs stand under it, and, by state, which state.
#[derive(Clone, Debug, PartialEq)]
struct Head {
    name: SharedString,
    count: usize,
    stand: Option<Stand>,
}

/// One line of the list: a group's heading over its Jobs, or a Job.
#[derive(Clone, Debug, PartialEq)]
enum Line {
    Group(Head),
    Job(Row),
    /// A Mac that is asleep, as a fold over its Jobs, after every group.
    Asleep(asleep_mac::Shelf),
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
    /// A copy of another Mac's project, by id, to this Mac's own project of the same name: **one project on two
    /// Macs is one group**, merged or not, as the switcher lists only this Mac's own.
    same: HashMap<String, String>,
    /// Every workflow Fleet holds, which a row's step bars are read from.
    flows: Vec<ipc::WorkflowSummary>,
}

impl Names {
    /// The same names, with the workflows a row's steps are read from.
    fn with_flows(mut self, flows: Option<&[ipc::WorkflowSummary]>) -> Names {
        self.flows = flows.map(<[_]>::to_vec).unwrap_or_default();
        self
    }

    fn of(served: &[ipc::RepositorySummary], projects: Option<&ipc::Projects>) -> Names {
        let named = crate::titlebar::repositories_of(served, &[]);
        let id_of = |one: &ipc::RepositorySummary| one.manifest.as_ref().map(|manifest| &manifest.id).or(one.implicit_id.as_ref()).map(|id| id.as_str().to_string());
        let all = projects.map(|all| all.projects.as_slice()).unwrap_or_default();
        let own = |name: &str| all.iter().find(|one| one.mac.is_none() && one.name == name).map(|one| one.id.clone());
        Names {
            projects: all.iter().map(|one| (one.id.clone(), one.name.clone())).collect(),
            repositories: served.iter().zip(named).filter_map(|(one, named)| Some((id_of(one)?, named.name.to_string()))).collect(),
            same: all.iter().filter(|one| one.mac.is_some()).filter_map(|one| Some((one.id.clone(), own(&one.name)?))).collect(),
            flows: Vec::new(),
        }
    }

    /// The project a Job counts in: its own id, or this Mac's project that a copy of the same name stands for.
    fn project_of<'a>(&'a self, job: &'a SessionRecord) -> Option<&'a str> {
        let id = job.hosted.as_ref().and_then(|hosted| hosted.project.as_deref())?;
        Some(self.same.get(id).map_or(id, String::as_str))
    }
}

/// The group a Job stands in, as its key and its heading: the project Fleet says it was started for, by
/// that project's name, else its repository by name. A heading is never an id.
fn group_of(job: &SessionRecord, names: &Names) -> (String, String) {
    let project = names.project_of(job);
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

/// Fleet's mark, save that an idle Job with work still going past its turn reads as running it: idle would say nothing runs.
pub(crate) fn mark_of(job: &SessionRecord) -> Option<JobMark> {
    let mark = job.mark?;
    let Some(first) = job.running.first().filter(|_| mark.state == JobState::Idle && mark.lamp == JobLamp::Idle) else { return Some(mark) };
    let state = if first.kind == ipc::RunningKind::Subagent { JobState::Subagent } else { JobState::Running };
    Some(JobMark { state, lamp: JobLamp::Running, ..mark })
}

/// A row's second line, as plain text. While the Job waits on him: the first thing it waits on him for, named
/// by its kind where its words do not. Otherwise what it is doing now, a background task still going, or what it last summed up.

pub(crate) fn now_of(job: &SessionRecord) -> Option<String> {
    let waits = job.mark.is_some_and(|mark| mark.lamp == JobLamp::Needs);
    let waiting = job.waiting_for.iter().find(|item| item.source != WaitingSource::Done).filter(|_| waits).map(|item| {
        let kind = match item.source {
            WaitingSource::Walk => "Walk",
            WaitingSource::Sketch => "Sketch",
            WaitingSource::Draft => "Draft",
            WaitingSource::Document => "Document",
            _ => "",
        };
        let named = kind.is_empty() || item.text.to_lowercase().starts_with(&kind.to_lowercase());
        if named { item.text.clone() } else { format!("{kind}: {}", item.text) }
    });
    let running = job.running.first().filter(|_| !waits).map(|one| one.what.clone());
    let line = waiting.or(running).or_else(|| job.activity.as_ref().and_then(|activity| activity.now.clone().or_else(|| activity.said.clone())).or_else(|| job.so_far.clone()))?;
    Some(crate::job_view::plain_of(&line)).filter(|line| !line.is_empty())
}

fn lines_of(jobs: &[SessionRecord], ideas: &[crate::queued::QueuedIdea], scope: Scope, query: &str, order: &[String], names: &Names) -> Vec<Line> {
    lines_within(jobs, ideas, scope, query, order, names, None)
}

/// [`lines_of`] narrowed to the Jobs started for `project`, by Fleet's id, when one is given: the scope and the
/// search still apply within it. A starter in another project is not drawn for its place, and an idea stands only
/// where a Job of the project stands in its root.
fn lines_within(jobs: &[SessionRecord], ideas: &[crate::queued::QueuedIdea], scope: Scope, query: &str, order: &[String], names: &Names, project: Option<&str>) -> Vec<Line> {
    lines_shelving(jobs, ideas, scope, query, order, names, project, &[])
}

/// [`lines_within`] with the Macs whose Jobs cannot be reached, asleep or on another build: a Job on one leaves its group for that Mac's fold at the foot of the
/// list, and is left out of Needs you, since it asks nothing of him here.
#[allow(clippy::too_many_arguments)]
fn lines_shelving(jobs: &[SessionRecord], ideas: &[crate::queued::QueuedIdea], scope: Scope, query: &str, order: &[String], names: &Names, project: Option<&str>, asleep: &[crate::pairing::Unreachable]) -> Vec<Line> {
    lines_grouped(jobs, ideas, scope, query, order, names, project, asleep, Group::Project, false)
}

/// [`lines_shelving`] grouped as `group` says: under the state each Job stands in, under its project, or in one list by
/// title. Within a group the Jobs keep the places `order` gives them, so one that changes state leaves its group and
/// lands in the other at its own place, and nothing else moves.
#[allow(clippy::too_many_arguments)]
fn lines_grouped(
    jobs: &[SessionRecord],
    ideas: &[crate::queued::QueuedIdea],
    scope: Scope,
    query: &str,
    order: &[String],
    names: &Names,
    project: Option<&str>,
    asleep: &[crate::pairing::Unreachable],
    group: Group,
    hide_done: bool,
) -> Vec<Line> {
    let asleep_on = |job: &SessionRecord| job.mac.as_deref().and_then(|id| asleep.iter().find(|one| one.id == id));
    // A copy of another Mac's project the window is scoped to stands for this Mac's project of its name, where it has one.
    let project = project.map(|id| names.same.get(id).map_or(id, String::as_str));
    let of_project = |job: &&SessionRecord| project.is_none_or(|id| names.project_of(job) == Some(id));
    let paused = |job: &SessionRecord| job.hosted.as_ref().is_some_and(|hosted| hosted.paused.is_some());
    let quiet = |job: &SessionRecord| job.mark.is_some_and(|mark| mark.state == JobState::Quiet);
    // Fleet clears `snoozed_until` when the snooze runs out, so one that is set is still ahead (`Held::snoozed`).
    let snoozed = |job: &SessionRecord| job.snoozed_until.is_some();
    // A Job that went to the other Mac stands among the live ones until its row there takes its place.
    let live = |job: &&SessionRecord| job.state == SessionState::Live || crate::move_to_mac::moved(job);
    let mut at: HashMap<&str, usize> = HashMap::new();
    for (place, id) in order.iter().enumerate() {
        at.entry(id.as_str()).or_insert(place);
    }
    let placed = |job: &&SessionRecord| (at.get(job.id.as_str()).copied().unwrap_or(usize::MAX), standing_of(job));
    let mut lives: Vec<&SessionRecord> = jobs.iter().filter(live).filter(of_project).collect();
    lives.sort_by_key(placed);
    let mut ended: Vec<&SessionRecord> = jobs.iter().filter(|job| job.state == SessionState::Ended && !crate::move_to_mac::moved(job)).filter(of_project).collect();
    ended.sort_by(|a, b| b.ended_at.as_ref().map(|at| at.as_str()).cmp(&a.ended_at.as_ref().map(|at| at.as_str())));
    let needs = |job: &SessionRecord| job.mark.is_some_and(|mark| mark.lamp == JobLamp::Needs);
    let picked: Vec<&SessionRecord> = match scope {
        Scope::Needs => lives.iter().copied().filter(|job| needs(job) && !snoozed(job) && asleep_on(job).is_none()).collect(),
        Scope::Active => lives.iter().copied().filter(|job| !paused(job) && !quiet(job) && !snoozed(job)).collect(),
        // A snoozed Job is paused as well, and listed under Snoozed alone.
        Scope::Paused => lives.iter().copied().filter(|job| paused(job) && !snoozed(job)).collect(),
        Scope::Snoozed => lives.iter().copied().filter(|job| snoozed(job)).collect(),
        Scope::Quiet => lives.iter().copied().filter(|job| quiet(job)).collect(),
        Scope::Ended => ended,
        Scope::All => lives.iter().copied().chain(ended).collect(),
    };
    // A Job another took the place of stands behind that one's row, as Electron's deck has it (`ship.ts`, `behind`).
    let picked: Vec<&SessionRecord> = picked.into_iter().filter(|job| spent(job, "replaced_by").is_none() && matches(job, query)).collect();
    // With the Done Jobs switch off, the Jobs Fleet marks Done are left out in every grouping.
    let picked: Vec<&SessionRecord> = picked.into_iter().filter(|job| !hide_done || stand_in(job) != Stand::Done).collect();
    // Jobs on a Mac that is asleep stand apart, under its fold.
    let (sleeping, picked): (Vec<&SessionRecord>, Vec<&SessionRecord>) = picked.into_iter().partition(|job| asleep_on(job).is_some());
    // In every scope a listed Job stands under its live starters; one the scope leaves out, a paused or
    // snoozed starter in Active or a calm one in Needs you, comes in for its place alone.
    let mut places: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for job in &picked {
        let mut up = starter_of(job);
        while let Some(id) = up.filter(|id| !picked.iter().any(|one| one.id.as_str() == *id) && !places.contains(id)) {
            let Some(starter) = lives.iter().find(|one| one.id.as_str() == id && asleep_on(one).is_none()) else { break };
            places.insert(starter.id.as_str());
            up = starter_of(starter);
        }
    }
    // By state a Job and its starter may stand in different groups, so no starter is drawn only for its place.
    if group == Group::State {
        places.clear();
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
    // Each group by its key, its words, the state it stands for where it has one, and its Jobs.
    let mut projects: Vec<(String, String, Option<Stand>, Vec<&SessionRecord>)> = Vec::new();
    match group {
        Group::Project => {
            for job in picked {
                let (key, name) = group_of(job, names);
                match projects.iter_mut().find(|(held, ..)| *held == key) {
                    Some((.., held)) => held.push(job),
                    None => projects.push((key, name, None, vec![job])),
                }
            }
            // Groups stand in the order Fleet lists Jobs, so a state change never moves one.
            projects.sort_by_key(|(key, ..)| jobs.iter().position(|job| group_of(job, names).0 == *key));
        }
        Group::State => {
            for job in picked {
                let stand = stand_in(job);
                match projects.iter_mut().find(|(_, _, held, _)| *held == Some(stand)) {
                    Some((.., held)) => held.push(job),
                    None => projects.push((format!("stand:{}", stand.words()), stand.words().to_string(), Some(stand), vec![job])),
                }
            }
            projects.sort_by_key(|(_, _, stand, _)| *stand);
        }
        Group::List => {
            let mut all = picked;
            all.sort_by_cached_key(|job| (title_of(job).to_lowercase(), job.id.as_str().to_string()));
            projects.push(("list".to_string(), String::new(), None, all));
        }
    }
    // Ideas waiting to start, among the live Jobs, each under its repository's group (`bands.ts`): the group of a
    // Job in its root, else its repository's name. A group with only ideas stands after the rest. By state they stand
    // together under Saved for later, after every state.
    let waiting: Vec<&crate::queued::QueuedIdea> = match scope {
        Scope::Active | Scope::All => ideas.iter().filter(|idea| query.trim().is_empty() || idea.text.to_lowercase().contains(&query.trim().to_lowercase())).collect(),
        _ => Vec::new(),
    };
    let idea_group = |idea: &crate::queued::QueuedIdea| {
        jobs.iter().find(|job| job.cwd == idea.root).map_or_else(|| (idea.repository.clone(), idea.repository.clone()), |job| group_of(job, names))
    };
    let mut waiting: Vec<&crate::queued::QueuedIdea> = waiting.into_iter().filter(|idea| project.is_none_or(|id| idea_group(idea).0 == format!("project:{id}"))).collect();
    if group == Group::List {
        waiting.sort_by_cached_key(|idea| idea.title().to_string().to_lowercase());
    }
    let idea_key = |idea: &crate::queued::QueuedIdea| match group {
        Group::Project => idea_group(idea).0,
        Group::State => format!("stand:{}", Stand::Saved.words()),
        Group::List => "list".to_string(),
    };
    for idea in &waiting {
        let key = idea_key(idea);
        if !projects.iter().any(|(held, ..)| *held == key) {
            match group {
                Group::Project => projects.push((key, idea_group(idea).1, None, Vec::new())),
                Group::State => projects.push((key, Stand::Saved.words().to_string(), Some(Stand::Saved), Vec::new())),
                Group::List => projects.push((key, String::new(), None, Vec::new())),
            }
        }
    }
    if group == Group::State {
        projects.sort_by_key(|(_, _, stand, _)| *stand);
    }
    let headed = match group {
        Group::Project => projects.len() > 1,
        Group::State => true,
        Group::List => false,
    };
    projects.extend(asleep_mac::groups(&sleeping, asleep, names).into_iter().map(|(key, name, held)| (key, name, None, held)));
    let mut lines = Vec::new();
    for (key, name, stand, held) in projects {
        if !asleep_mac::open_line(&key, asleep, &mut lines) && headed {
            // Its Jobs but for a starter drawn only for its place, and its ideas.
            let count = held.iter().filter(|job| !places.contains(job.id.as_str())).count() + waiting.iter().filter(|idea| idea_key(idea) == key).count();
            lines.push(Line::Group(Head { name: name.clone().into(), count, stand }));
        }
        for idea in waiting.iter().filter(|idea| idea_key(idea) == key) {
            lines.push(Line::Job(Row {
                id: format!("queued:{}", idea.id).into(),
                title: idea.title().to_string().into(),
                project: idea_group(idea).1.into(),
                paused: false,
                prioritised: false,
                snoozed: None,
                slot_wait: None,
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
                track: None,
                now: None,
                queued: Some((*idea).clone()),
                place: false,
                mac: None,
                moving_to: None,
                moved: false,
                answer: false,
                asleep: false,
                ending: None,
            }));
        }
        let listed = |id: &str| held.iter().any(|job| job.id.as_str() == id);
        let row_of = |job: &SessionRecord, depth: usize, through: &[bool], last: bool| Row {
            id: job.id.as_str().to_string().into(),
            title: title_of(job),
            project: group_of(job, names).1.into(),
            paused: paused(job),
            prioritised: job.prioritised && job.state != SessionState::Ended,
            snoozed: job.snoozed_until.as_ref().and_then(crate::snooze::local_of),
            slot_wait: job.slot_wait.as_ref().map(|wait| crate::slot_wait::said(wait).into()),
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
            // A Job that went to the other Mac is ended here, as Fleet's mark for it says.
            mark: if crate::move_to_mac::moved(job) { job.mark.map(|_| JobMark { state: JobState::Ended, lamp: JobLamp::Idle, group: JobGroup::Done }) } else { mark_of(job) },
            issue: job.attachments.iter().find(|one| one.kind == "issue").map(|one| {
                let number = if one.target.chars().all(|c| c.is_ascii_digit()) { format!("#{}", one.target) } else { one.target.clone() };
                let title = one.detail.get("title").cloned().unwrap_or_default();
                (number.into(), title.into())
            }),
            flow: flow_of(job),
            track: crate::job_view::column_strip::track_of(job, &names.flows, chrono::Utc::now()),
            now: now_of(job).map(Into::into),
            queued: None,
            place: places.contains(job.id.as_str()),
            mac: job.mac.clone().map(Into::into),
            moving_to: crate::move_to_mac::moving_to(job).map(|id| id.to_string().into()),
            moved: crate::move_to_mac::moved(job),
            answer: job.answer_unread,
            asleep: asleep_on(job).is_some(),
            ending: None,
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
                let Some(mark) = sub.mark.filter(|_| !sub.place && !sub.asleep) else { continue };
                let held = strip.iter_mut().find(|one| one.paused == sub.paused && (sub.paused || one.mark.state == mark.state));
                match held {
                    Some(one) => one.titles.push(sub.title.clone()),
                    None => strip.push(Sub { mark, paused: sub.paused, titles: vec![sub.title.clone()] }),
                }
            }
            // Needs you first, then failed, then running, then the rest as they stand.
            strip.sort_by_key(|sub| match sub.mark.lamp {
                JobLamp::Needs | JobLamp::Ready => 0,
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

/// The key a deck file's group line starts with, after the scope's line: `group=state`.
const GROUP_KEY: &str = "group=";

fn read_deck() -> (Scope, Group, Vec<String>) {
    parse_deck(&deck_file().and_then(|path| std::fs::read_to_string(path).ok()).unwrap_or_default())
}

/// What a deck file holds: the scope on its first line, a `group=` line anywhere after it, and a Job id on each other line.
/// A file from before the view button has no group line and reads as By state, and its scope `all`, which the view menu no longer
/// offers, reads as All Jobs.
fn parse_deck(text: &str) -> (Scope, Group, Vec<String>) {
    let mut lines = text.lines();
    let scope = match Scope::of(lines.next().unwrap_or_default()) {
        Scope::All => Scope::Active,
        held => held,
    };
    let rest: Vec<&str> = lines.filter(|line| !line.is_empty()).collect();
    let group = rest.iter().find_map(|line| line.strip_prefix(GROUP_KEY)).map_or(Group::default(), Group::of);
    (scope, group, rest.into_iter().filter(|line| !line.starts_with(GROUP_KEY)).map(str::to_string).collect())
}

fn deck_text(scope: Scope, group: Group, order: &[String]) -> String {
    [scope.id().to_string(), format!("{GROUP_KEY}{}", group.id())].into_iter().chain(order.iter().cloned()).collect::<Vec<_>>().join("\n")
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

fn write_deck(scope: Scope, group: Group, order: &[String]) {
    let Some(path) = deck_file() else { return };
    let text = deck_text(scope, group, order);
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

/// The open Job was left for the Cockpit's page: by Esc, by a second press on its row, or by the Cockpit's own place.
pub struct Left;

impl gpui::EventEmitter<Left> for JobsList {}

pub struct JobsList {
    jobs: Vec<SessionRecord>,
    /// The Operator's own session, which is never a row: the Operator's page opens it as a Job's view does.
    operator: Option<SessionRecord>,
    lines: Vec<Line>,
    scope: Scope,
    /// How the Jobs are grouped: by state, by project, or in one list.
    group: Group,
    query: String,
    /// The parents whose sub-Jobs are folded away, and the switch that folds every one.
    folds: Folds,
    /// The Macs that are asleep, whose Jobs stand under a fold at the foot of the list.
    asleep: Vec<crate::pairing::Unreachable>,
    /// The folds of those Macs that are open, by Mac id. Held for the window's life; a Mac that wakes drops out.
    asleep_open: std::collections::BTreeSet<String>,
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
    /// The Cockpit draws the Operator's page where no Job is open, so a Job can be left and has a home to leave to.
    cockpit: bool,
    /// The Job the page opened without a pick, for a card it draws from the Job view: it is open but is no Job the person opened.
    /// Held with the picks as they stood, so a pick of that very row since is a real open.
    lent: Option<(SharedString, u64)>,
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
    /// Jobs he pressed End on that Fleet still serves live, by id. The record reads ended at once, as `asked` holds a
    /// title, until Fleet serves it ended or refuses the End; a Job on a sleeping Mac stays so until that Mac wakes.
    ending: HashMap<String, Ending>,
    /// While a row is dragged: the row under the pointer, and whether it drops after it (`reorder.ts`).
    drop: Option<(SharedString, bool)>,
    /// The row being dragged, faded where it stood.
    lifted: Option<SharedString>,
    /// The ideas waiting to start (`crate::queued`), each a row under its repository's group.
    ideas: Vec<crate::queued::QueuedIdea>,
    /// What each group is called: his projects and the repositories Fleet serves, by name.
    names: Names,
    /// The project the title bar is scoped to, by Fleet's id, as it said at its last pick: the list draws only
    /// that project's Jobs. The bar's face holds the scope; this is read from it by [`Self::follow_scope`] and set nowhere else.
    project: Option<String>,
    /// Jobs `session.removed` took away that the open Job's screen has not let go of yet.
    gone: Vec<SharedString>,
    _removed: Option<Subscription>,
    /// The Jobs selected to be acted on together, by id, in the order they were added.
    selected: Vec<SharedString>,
    /// The row a ⇧-click or ⇧↓ selects from: the last one clicked, toggled or extended from.
    anchor: Option<SharedString>,
}

/// How the Jobs list on mock Jobs is grouped while a walk plays. A walk about the view button opens by state, as a first start does;
/// every other walk was written against the list grouped by project, and opens in it.
pub struct WalkGroup(pub Group);

impl gpui::Global for WalkGroup {}

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
                let notes = fleet.read(cx).held().build_notes.clone();
                let disk = fleet.read(cx).held().disk.clone();
                let link = fleet.read(cx).held().link();
                let asked = fleet.read(cx).held().reviews_unopened();
                let moved = bar.update(cx, |bar, cx| bar.set_build_notes(notes.as_ref(), cx));
                if let (true, Some(notes)) = (moved, notes) {
                    crate::dispatch::screen::hold_build_notes(notes, cx);
                }
                bar.update(cx, |bar, cx| {
                    bar.set_fleet(&link, cx);
                    bar.set_usage(usage.as_ref(), downshift.as_ref(), cx);
                    bar.set_build(build.as_ref(), cx);
                    bar.set_offers(offers.as_ref(), cx);
                    bar.set_disk(disk.as_ref(), cx);
                    bar.set_figure("reviews", asked, cx);
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
            // A project Fleet no longer lists takes the bar back to All, which the list follows.
            this.follow_scope(cx);
            let held = fleet.read(cx).held();
            let names = Names::of(held.repositories.as_deref().unwrap_or_default(), held.projects.as_ref()).with_flows(held.workflows.as_deref());
            // The Operator's own session is never a Job in the list: its page is the Operator's. Compared before it is
            // cloned: Fleet moves several times a second and the list holds hundreds of Jobs.
            let unmoved = names == this.names && this.holds_already(held.jobs.iter().filter(|job| !job.operator));
            let mut jobs: Vec<SessionRecord> = if unmoved { Vec::new() } else { held.jobs.iter().filter(|job| !job.operator).cloned().collect() };
            let operator = crate::operator::session_of(&held.jobs).cloned();
            if unmoved {
                // A rename Fleet now serves is let go, as the hold below lets it go.
                this.asked.retain(|id, (was, _)| held.jobs.iter().any(|job| job.id.as_str() == id && job.title == *was));
            } else {
                this.hold_asked_titles(&mut jobs);
                this.hold_ending(&mut jobs);
            }
            if !unmoved && (jobs != this.jobs || names != this.names) {
                this.jobs = jobs;
                this.names = names;
                this.place_new();
                this.relist(cx);
            }
            if operator != this.operator {
                this.operator = operator;
                // Only the Operator's session opened over the Jobs screen has anything drawn from this record.
                if this.operator.as_ref().is_some_and(|record| this.opened.as_deref() == Some(record.id.as_str())) {
                    cx.notify();
                }
            }
            if this.resume_open() {
                cx.notify();
            }
        });
        let (scope, group, order) = read_deck();
        crate::queued::begin(true, cx);
        let held = fleet.read(cx).held();
        let names = Names::of(held.repositories.as_deref().unwrap_or_default(), held.projects.as_ref()).with_flows(held.workflows.as_deref());
        let jobs = held.jobs.iter().filter(|job| !job.operator).cloned().collect();
        Self::with(jobs, names, scope, group, order, read_view(), true, Some(fleet.clone()), Some(watch), cx)
    }

    /// The list on mock Jobs, which never change, with the View as a first start has it, so a walk shows what he will.
    pub fn mock(jobs: &[SessionRecord], cx: &mut Context<Self>) -> Self {
        crate::queued::begin(false, cx);
        let names = Names::of(&crate::mock::repositories(), Some(&crate::mock::projects())).with_flows(Some(crate::mock::workflows().as_slice()));
        let group = cx.try_global::<WalkGroup>().map_or(Group::default(), |held| held.0);
        Self::with(jobs.to_vec(), names, Scope::Active, group, Vec::new(), View::DEFAULT, false, None, None, cx)
    }

    /// The mock Jobs over a Fleet connection, so a test reaches the paths a live Fleet takes.
    #[cfg(test)]
    pub fn mock_over(jobs: &[SessionRecord], fleet: Entity<FleetConnection>, cx: &mut Context<Self>) -> Self {
        let names = Names::of(&crate::mock::repositories(), Some(&crate::mock::projects()));
        Self::with(jobs.to_vec(), names, Scope::Active, Group::default(), Vec::new(), View::DEFAULT, false, Some(fleet), None, cx)
    }

    fn with(
        jobs: Vec<SessionRecord>,
        names: Names,
        scope: Scope,
        group: Group,
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
            group,
            query: String::new(),
            folds: if remember { Folds::load(folds_file().as_deref()) } else { Folds::default() },
            asleep: crate::pairing::unreachable_of(crate::pairing::macs(cx)),
            asleep_open: std::collections::BTreeSet::new(),
            view,
            order,
            remember,
            focused: None,
            opened: None,
            resume: if remember { crate::job_view::place_kept_open() } else { None },
            picks: 0,
            cockpit: false,
            lent: None,
            operator: None,
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
            ending: HashMap::new(),
            drop: None,
            lifted: None,
            ideas: crate::queued::ideas(cx),
            names,
            project: None,
            gone: Vec::new(),
            _removed: None,
            selected: Vec::new(),
            anchor: None,
        };
        if fleet_held {
            this._removed = this.fleet.clone().map(|fleet| cx.subscribe(&fleet, |this, _, crate::fleet::Removed(ids), cx| this.gone(ids, cx)));
        } else {
            cx.set_global(MockList(cx.weak_entity()));
        }
        // A Mac that falls asleep takes its Jobs to the foot of the list, and one that wakes gives them back.
        crate::pairing::observe(cx, |this: &mut Self, cx| this.follow_macs(cx)).detach();
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
        self.forget_unlisted_selection();
        if !self.focused.as_ref().is_some_and(|id| self.at_of(id).is_some()) {
            self.focused = self.first_row();
        }
        cx.notify();
    }

    /// A walk's Job Fleet has just started: it joins the list the walk plays on, as the event stream would bring it,
    /// where the list does not hold it yet.
    pub fn mock_started(record: SessionRecord, cx: &mut App) {
        let Some(list) = cx.try_global::<MockList>().and_then(|held| held.0.upgrade()) else { return };
        list.update(cx, |list, cx| {
            if list.jobs.iter().any(|one| one.id == record.id) {
                return;
            }
            list.jobs.push(record);
            list.place_new();
            list.relist(cx);
        });
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
        // Most moves bring no new Job: each live one already has its place, and the lines need not be drawn up.
        let placed: std::collections::HashSet<&str> = self.order.iter().map(String::as_str).collect();
        let live = |job: &&SessionRecord| job.state == SessionState::Live || crate::move_to_mac::moved(job);
        if self.jobs.iter().filter(live).filter(|job| spent(job, "replaced_by").is_none()).all(|job| placed.contains(job.id.as_str())) {
            return;
        }
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
        if self.record(id).is_none() {
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

    /// Every record as Fleet sends them anew, for a test that moves one under an open view.
    #[cfg(test)]
    pub fn replace_records(&mut self, jobs: Vec<SessionRecord>, cx: &mut Context<Self>) {
        self.jobs = jobs;
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
    /// Whether `served` is the list held, as [`Self::hold_asked_titles`] would leave it: a title a rename was asked
    /// for reads as asked while Fleet still serves the one it replaced.
    fn holds_already<'a>(&self, served: impl Iterator<Item = &'a SessionRecord>) -> bool {
        let mut held = self.jobs.iter();
        for one in served {
            let Some(mine) = held.next() else { return false };
            if one == mine {
                continue;
            }
            let mut held = one.clone();
            if let Some((_, asked)) = self.asked.get(one.id.as_str()).filter(|(was, _)| one.title == *was) {
                held.title = Some(asked.clone());
            }
            if let Some(ending) = self.ending.get(one.id.as_str()).filter(|_| one.state != SessionState::Ended) {
                ending.hold(&mut held);
            }
            if held != *mine {
                return false;
            }
        }
        held.next().is_none()
    }

    /// Put the Ends still waiting on Fleet over the records it sent, and let go of those it now serves ended.
    fn hold_ending(&mut self, jobs: &mut [SessionRecord]) {
        self.ending.retain(|id, ending| match jobs.iter_mut().find(|job| job.id.as_str() == id) {
            Some(job) if job.state != SessionState::Ended => {
                ending.hold(job);
                true
            }
            _ => false,
        });
    }

    /// He pressed End: the Job reads ended at once, everywhere its record is read, while Fleet ends it in the
    /// background. On mock data nothing echoes, so the record takes it for good, as Fleet's would.
    pub fn end_now(&mut self, id: &str, cx: &mut Context<Self>) {
        let macs = crate::pairing::macs(cx).to_vec();
        let Some(held) = self.jobs.iter_mut().find(|job| job.id.as_str() == id) else { return };
        if held.state == SessionState::Ended {
            return;
        }
        let on = held.mac.as_deref().map(|mac| macs.iter().find(|one| one.id == mac).map_or("another Mac", |one| one.name.as_str()).to_string());
        let ending = Ending { was: (held.state, held.ended_at.clone(), held.mark), at: ipc::Instant::carried(chrono::Utc::now().to_rfc3339()), on };
        ending.hold(held);
        if self.fleet.is_some() {
            self.ending.entry(id.to_string()).or_insert(ending);
        }
        self.relist(cx);
        cx.notify();
    }

    /// Fleet refused the End: the Job stands as Fleet holds it.
    pub fn unend(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(ending) = self.ending.remove(id) else { return };
        if let Some(held) = self.jobs.iter_mut().find(|job| job.id.as_str() == id) {
            (held.state, held.ended_at, held.mark) = ending.was;
        }
        self.relist(cx);
        cx.notify();
    }

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
        self.jobs.iter().find(|job| job.id.as_str() == id).or_else(|| self.operator.as_ref().filter(|job| job.id.as_str() == id))
    }

    /// The Operator's session as the Operator's page read it, held so its Job view has a record to draw. A live Fleet's own
    /// record replaces it as it changes; the walk's has none.
    pub fn hold_operator(&mut self, record: SessionRecord) {
        self.operator = Some(record);
    }

    /// The Operator's own session, which the list holds apart from its Jobs.
    pub fn operator_record(&self) -> Option<&SessionRecord> {
        self.operator.as_ref()
    }

    /// The Cockpit has the Operator's page for a home: the Job area draws it where no Job is open.
    pub fn set_cockpit(&mut self, on: bool) {
        self.cockpit = on;
    }

    /// Whether the Cockpit draws its page and no Job: none is open, or the open one is only lent to the page for a card.
    pub fn at_home(&self) -> bool {
        self.cockpit
            && match &self.opened {
                None => true,
                Some(open) => self.lent.as_ref().is_some_and(|(lent, picks)| lent == open && *picks == self.picks),
            }
    }

    /// Whether Esc and a second press on the open row have a Job to leave.
    pub fn closable(&self) -> bool {
        self.cockpit && self.opened.is_some() && !self.at_home()
    }

    /// Leave the open Job for the Cockpit's page. Says so to whoever follows the list, which puts the keys and the page's cards right.
    pub fn close_job(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.closable() {
            return false;
        }
        self.opened = None;
        self.card = None;
        self.lent = None;
        cx.emit(Left);
        cx.notify();
        true
    }

    /// The Operator's session is shut again: the Job that was open before it is open once more.
    pub fn leave_operator(&mut self, was: Option<SharedString>, cx: &mut Context<Self>) {
        self.open_quietly(was, cx);
    }

    /// A Job opens as no pick does: the Job's box does not take the keys. For a place over the Jobs screen that draws one of its Job's parts.
    pub fn open_quietly(&mut self, id: Option<SharedString>, cx: &mut Context<Self>) {
        self.opened = id.clone();
        self.focused = id;
        self.card = None;
        // The Review panel opens the Job it was lent, quietly again: that stays a loan.
        if self.lent.as_ref().map(|(lent, _)| lent) != self.opened.as_ref() {
            self.lent = None;
        }
        cx.notify();
    }

    /// A Job opens quietly and is lent: in the Cockpit the page stays in front of it.
    pub fn lend(&mut self, id: SharedString, cx: &mut Context<Self>) {
        self.open_quietly(Some(id.clone()), cx);
        self.lent = Some((id, self.picks));
    }

    /// A test's stand-in for a Fleet read that changes one record.
    #[cfg(test)]
    pub(crate) fn record_mut(&mut self, id: &str) -> Option<&mut SessionRecord> {
        self.jobs.iter_mut().find(|job| job.id.as_str() == id)
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
        let mut lines = lines_grouped(&self.jobs, &self.ideas, self.scope, &self.query, &self.order, &self.names, self.project.as_deref(), &self.asleep, self.group, self.folds.hide_done);
        if !self.ending.is_empty() {
            for line in lines.iter_mut() {
                if let Line::Job(row) = line {
                    row.ending = self.ending.get(row.id.as_ref()).map(Ending::said);
                }
            }
        }
        lines
    }

    /// The title bar was scoped to a project, or back to All or a repository: the list takes the project the
    /// bar's face names and draws only its Jobs. The scope of the list (Active, Needs you and the rest) holds within it.
    pub fn follow_scope(&mut self, cx: &mut Context<Self>) {
        let project = crate::titlebar::live(cx).and_then(|bar| bar.read(cx).face.scoped_project().map(str::to_string));
        if project != self.project {
            self.project = project;
            self.relist(cx);
        }
    }

    /// The keys on the first Job that needs him, as the deck's `c` puts them (`keys-model.ts`, the
    /// `needs` slot). With none, they stay where they are.
    pub fn to_first_needing(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let first = self.lines.iter().enumerate().find_map(|(at, line)| match line {
            Line::Job(row) if !row.asleep && row.mark.is_some_and(|mark| mark.group == JobGroup::Needs) => Some((at, row.id.clone())),
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
            Line::Job(_) | Line::Group(_) | Line::Asleep(_) => None,
        })
    }

    /// Draw the list again from the Jobs, the scope, the search and the order.
    fn relist(&mut self, cx: &mut Context<Self>) {
        let lines = self.folded(self.lines_now());
        if lines != self.lines {
            self.lines = lines;
            self.forget_unlisted_selection();
            if !self.focused.as_ref().is_some_and(|id| self.at_of(id).is_some()) {
                self.focused = self.first_row();
            }
            cx.notify();
        }
    }

    fn keep(&self) {
        if self.remember {
            write_deck(self.scope, self.group, &self.order);
        }
    }

    /// `lines` with the parents this list has folded shut.
    fn folded(&self, lines: Vec<Line>) -> Vec<Line> {
        asleep_mac::shelve_open(fold_lines(lines, &self.folds), &self.asleep_open)
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

    /// The Done Jobs switch: off leaves the Jobs that stand under Done out of the list.
    pub(crate) fn set_hide_done(&mut self, hide: bool, cx: &mut Context<Self>) {
        self.change_folds(|folds, _| folds.hide_done = hide, cx);
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
        if let Some((mac, open)) = self.focused_shelf().filter(|(_, open)| *open == fold) {
            self.set_shelf(&mac, !open, cx);
            return;
        }
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

    /// What the keys can land on at this line: a Job, or the fold of a Mac asleep, by the id they carry.
    fn stop_at(&self, at: usize) -> Option<SharedString> {
        match self.lines.get(at)? {
            Line::Job(row) => Some(row.id.clone()),
            Line::Asleep(shelf) => Some(shelf.id()),
            Line::Group(_) => None,
        }
    }

    fn at_of(&self, id: &SharedString) -> Option<usize> {
        (0..self.lines.len()).find(|at| self.stop_at(*at).as_ref() == Some(id))
    }

    fn port(&self, cx: &App) -> Option<u16> {
        self.fleet.as_ref().and_then(|fleet| fleet.read(cx).held().port())
    }

    /// Move the keys one Job down (`by` 1) or up (`by` -1), past project names. Up from the first
    /// Job opens the Filter, as Electron's list does.
    fn step(&mut self, by: isize, window: &mut Window, cx: &mut Context<Self>) {
        match self.next_row(by) {
            Some((at, id)) => {
                self.focused = Some(id);
                // Keys that only move the keys let go of the row a ⇧-click would have ranged from.
                self.anchor = None;
                self.scroll.scroll_to_item(at);
                crate::ui_sounds::moved();
                cx.notify();
            }
            None if by < 0 && self.focused.as_ref().is_some_and(|id| self.at_of(id).is_some()) => {
                crate::ui_sounds::moved();
                self.to_search(window, cx)
            }
            None => {}
        }
    }

    /// The row `by` places from the focused one in the order drawn, past project names, with its place in the lines.
    fn next_row(&self, by: isize) -> Option<(usize, SharedString)> {
        let from = self.focused.as_ref().and_then(|id| self.at_of(id));
        let mut at = from.map_or(if by > 0 { -1 } else { self.lines.len() as isize }, |at| at as isize);
        loop {
            at += by;
            if at < 0 || at >= self.lines.len() as isize {
                return None;
            }
            if let Some(id) = self.stop_at(at as usize) {
                return Some((at as usize, id));
            }
        }
    }

    /// Whether a Job can be selected: a row of a Job, never an idea waiting to start or a project's name.
    fn selectable(&self, id: &str) -> bool {
        self.rows().any(|row| row.id.as_ref() == id && row.queued.is_none())
    }

    /// The Jobs selected, in the order they were added.
    #[cfg(test)]
    pub fn selected(&self) -> &[SharedString] {
        &self.selected
    }

    /// Let go of selected Jobs the list no longer draws.
    fn forget_unlisted_selection(&mut self) {
        let held = std::mem::take(&mut self.selected);
        self.selected = held.into_iter().filter(|id| self.selectable(id)).collect();
        if self.anchor.as_ref().is_some_and(|id| !self.selectable(id)) {
            self.anchor = None;
        }
    }

    /// Add the Job to the selection, or take it out: a ⌘-click, or Space or `x` on the focused row.
    pub fn toggle_selected(&mut self, id: &str, cx: &mut Context<Self>) {
        if !self.selectable(id) {
            return;
        }
        match self.selected.iter().position(|one| one.as_ref() == id) {
            Some(at) => {
                self.selected.remove(at);
            }
            None => self.selected.push(id.to_string().into()),
        }
        self.anchor = Some(id.to_string().into());
        cx.notify();
    }

    /// A ⇧-click: the Jobs from the anchor to this one, in the order drawn, are the selection. The anchor is the
    /// last row clicked or toggled, else the row the keys are on.
    pub fn select_range(&mut self, to: &str, cx: &mut Context<Self>) {
        let drawn: Vec<SharedString> = self.rows().filter(|row| row.queued.is_none()).map(|row| row.id.clone()).collect();
        let place = |id: &str| drawn.iter().position(|one| one.as_ref() == id);
        let from = self.anchor.clone().or_else(|| self.focused.clone());
        let (Some(a), Some(b)) = (from.as_deref().and_then(place), place(to)) else { return };
        self.selected = drawn[a.min(b)..=a.max(b)].to_vec();
        self.anchor = from;
        cx.notify();
    }

    /// Let go of the whole selection.
    pub fn clear_selection(&mut self, cx: &mut Context<Self>) {
        if self.selected.is_empty() && self.anchor.is_none() {
            return;
        }
        self.selected.clear();
        self.anchor = None;
        cx.notify();
    }

    /// ⇧↓ and ⇧↑: select the row the keys are on, move them, and select the row they land on. Moving back over a
    /// row already selected lets go of the one left.
    fn extend_selection(&mut self, by: isize, cx: &mut Context<Self>) {
        let Some(from) = self.focused.clone() else { return };
        let Some((at, next)) = self.next_row(by) else { return };
        let had = self.selected.contains(&from);
        if had && self.selected.contains(&next) {
            self.selected.retain(|id| *id != from);
        } else {
            if !had && self.selectable(&from) {
                self.selected.push(from.clone());
            }
            if self.selectable(&next) {
                self.selected.push(next.clone());
            }
        }
        self.anchor.get_or_insert(from);
        self.focused = Some(next);
        self.scroll.scroll_to_item(at);
        crate::ui_sounds::moved();
        cx.notify();
    }

    /// ⇧↓ and ⇧↑ while the rows have the keys.
    pub(crate) fn select_key(&mut self, event: &gpui::KeyDownEvent, window: &Window, cx: &mut Context<Self>) -> bool {
        let modifiers = event.keystroke.modifiers;
        let by = match event.keystroke.key.as_str() {
            "down" => 1,
            "up" => -1,
            _ => return false,
        };
        if !modifiers.shift || modifiers.control || modifiers.alt || modifiers.platform || !self.focus.is_focused(window) {
            return false;
        }
        self.extend_selection(by, cx);
        true
    }

    /// The top Jobs standing in the same group as `id`, in the order they are drawn: the Jobs it can be moved past. A group is
    /// whatever lies between two headings, or the whole list where none is drawn. Ideas, ended Jobs and Jobs under another are no peers.
    fn peers_of(&self, id: &SharedString) -> Vec<SharedString> {
        let mut section = 0;
        let mut held: Vec<(usize, &Row)> = Vec::new();
        for line in &self.lines {
            match line {
                Line::Group(_) | Line::Asleep(_) => section += 1,
                Line::Job(row) if !row.place && row.depth == 0 && !row.ended && row.queued.is_none() => held.push((section, row)),
                Line::Job(_) => {}
            }
        }
        let Some(mine) = held.iter().find(|(_, row)| row.id == *id).map(|(section, _)| *section) else { return Vec::new() };
        held.into_iter().filter(|(section, _)| *section == mine).map(|(_, row)| row.id.clone()).collect()
    }

    /// The placed order with every live Job the list draws in it: the order as it stands, and any Job it has not met after
    /// them, so a move holds whatever the list draws next and no Job that was not moved changes its place.
    fn placed_order(&self) -> Vec<String> {
        let mut placed = self.order.clone();
        let known: std::collections::HashSet<String> = placed.iter().cloned().collect();
        placed.extend(self.rows().filter(|row| !row.ended && row.queued.is_none() && !known.contains(row.id.as_ref())).map(|row| row.id.to_string()));
        placed
    }

    /// Move the focused Job past its neighbour in its group (`by` 1 down, -1 up). A Job standing
    /// under another moves with it and is not moved alone, as in Electron (`reorder.ts`).
    fn shove(&mut self, by: isize, cx: &mut Context<Self>) {
        let Some(id) = self.focused.clone() else { return };
        let Some(row) = self.rows().find(|row| row.id == id).cloned() else { return };
        if row.depth > 0 || row.ended || row.queued.is_some() {
            return;
        }
        let peers = self.peers_of(&id);
        let Some(at) = peers.iter().position(|one| *one == id) else { return };
        let Some(over) = at.checked_add_signed(by).and_then(|to| peers.get(to)).cloned() else { return };
        let mut placed = self.placed_order();
        let (Some(from), Some(to)) = (placed.iter().position(|one| *one == *id), placed.iter().position(|one| *one == *over)) else { return };
        placed.swap(from, to);
        self.order = placed;
        self.keep();
        self.relist(cx);
    }

    /// A dragged row dropped before or after another of its group's top rows (`Deck.tsx` `onMove`).
    fn place(&mut self, id: &SharedString, target: &SharedString, after: bool, cx: &mut Context<Self>) {
        self.drop = None;
        self.lifted = None;
        if id == target || !self.peers_of(id).contains(target) {
            return cx.notify();
        }
        let Some(placed) = moved(self.placed_order(), id, target, after) else { return };
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
            } else if let Some((mac, open)) = self.focused_shelf() {
                self.set_shelf(&mac, !open, cx);
            } else if self.focused.is_some() {
                self.opened = self.focused.clone();
                self.picks += 1;
                cx.notify();
            }
        } else if act.is("move_to_mac") {
            // `M` on the row under the keys, as the menu's Move to the other Mac or Bring here.
            match self.focused.clone().filter(|id| self.rows().any(|row| &row.id == id && row.queued.is_none())) {
                Some(id) => self.move_to_mac(&id, cx),
                None => cx.propagate(),
            }
        } else if act.is("move_to_project") {
            // `P` on the row under the keys, or on the selection where that row is in it.
            match self.focused.clone().filter(|id| self.selectable(id)) {
                Some(id) => self.pick_project(&id, None, window, cx),
                None => cx.propagate(),
            }
        } else if act.is("deck_select") {
            // Space or `x` selects the row under the keys, or takes it out of the selection.
            match (self.focused.clone().filter(|id| self.selectable(id)), self.focused_shelf()) {
                (Some(id), _) => self.toggle_selected(&id, cx),
                (None, Some((mac, open))) => self.set_shelf(&mac, !open, cx),
                (None, None) => cx.propagate(),
            }
        } else if act.is("close") && !self.selected.is_empty() {
            // Esc lets go of the selection while there is one, and otherwise means what it means everywhere.
            self.clear_selection(cx);
        } else if act.is("job_judge") {
            // `⌥⌘L` on the row under the keys, as the menu's Judge.
            match self.focused.clone().filter(|id| self.rows().any(|row| &row.id == id && row.queued.is_none() && !row.ended)) {
                Some(id) => self.judge_job(&id, window, cx),
                None => cx.propagate(),
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
                // Fleet pauses a Job it snoozes, and Wake now resumes it.
                if let Some(hosted) = job.hosted.as_mut() {
                    hosted.paused = at.is_some().then(|| ipc::SessionPaused { at: crate::snooze::instant_of(crate::snooze::now().with_timezone(&chrono::Utc)), resuming: false });
                }
                job.snoozed_until = at;
            }
            self.relist(cx);
            cx.notify();
            return;
        }
        crate::snooze::post(self.port(cx), crate::snooze::body(id, until), cx);
    }

    /// Move the Job to the other Mac, or bring it here where that Mac holds it. A live Fleet is asked, and the row
    /// follows the record it publishes; a refusal is said in a toast in Fleet's words. A Job the act is not on for
    /// does nothing, and one left off says why. On mock data there is no Fleet, so the record the list holds takes
    /// the mark, as a walk's stand-in for Fleet's yes; [`Self::mock_move_settled`] says how it ended.
    pub fn move_to_mac(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(offer) = self.record(id).and_then(|job| crate::move_to_mac::offer(job, crate::pairing::macs(cx))) else { return };
        if let Some(why) = offer.off {
            crate::toasts::tell(why, cx);
            return;
        }
        if self.fleet.is_some() {
            crate::move_to_mac::post(self.port(cx), crate::move_to_mac::body(id, offer.to.as_deref()), cx);
            return;
        }
        if let Some(why) = crate::move_to_mac::mock_refusal(id) {
            crate::toasts::tell(why, cx);
            return;
        }
        let to = offer.to.or_else(|| crate::move_to_mac::this_mac(crate::pairing::macs(cx)).map(str::to_string));
        if let Some(hosted) = self.jobs.iter_mut().find(|job| job.id.as_str() == id).and_then(|job| job.hosted.as_mut()) {
            hosted.moving_to_mac = to;
        }
        self.relist(cx);
        cx.notify();
    }

    /// The row menu's Close: the Job reads ended at once and Fleet ends it in the background, on the Mac that holds
    /// it. A refusal puts the row back as Fleet holds it and is said in a toast in Fleet's words.
    pub fn end_row(&mut self, id: &str, cx: &mut Context<Self>) {
        let port = self.port(cx);
        self.end_now(id, cx);
        let Some(port) = port.filter(|_| self.fleet.is_some()) else { return };
        let body = ipc::encode(&ipc::CloseSession { session_id: ipc::SessionId::carried(id.to_string()) });
        let id = id.to_string();
        cx.spawn(async move |this, cx| {
            let answer = match body {
                Ok(body) => {
                    cx.background_executor()
                        .spawn(async move {
                            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                            runtime.block_on(crate::transport::post(port, "/sessions/close", body))
                        })
                        .await
                }
                Err(why) => Err(format!("{why:?}")),
            };
            let said = match answer {
                Ok((status, _)) if (200..300).contains(&status) => return,
                Ok((status, bytes)) => crate::snooze::refusal_said(status, &bytes),
                Err(why) => why,
            };
            let _ = this.update(cx, |this, cx| {
                this.unend(&id, cx);
                crate::toasts::tell(said, cx);
            });
        })
        .detach();
    }

    /// The Jobs an act on this row reaches: the whole selection where the row is in it, else the row alone.
    pub fn reached_by(&self, id: &str) -> Vec<SharedString> {
        if self.selected.iter().any(|one| one.as_ref() == id) {
            self.selected.clone()
        } else {
            vec![id.to_string().into()]
        }
    }

    /// Move the Jobs into a project of his, or out to none. A live Fleet is asked, and the rows follow the records it
    /// publishes; a refusal is said in a toast in Fleet's words. On mock data there is no Fleet, so the records the
    /// list holds take the project, as a walk's stand-in for Fleet's yes, and the rows regroup. A move sent lets go of
    /// the selection.
    pub fn move_to_project(&mut self, ids: Vec<SharedString>, project: Option<String>, cx: &mut Context<Self>) {
        let ids: Vec<SharedString> = ids.into_iter().filter(|id| self.record(id).is_some_and(crate::move_to_project::movable)).collect();
        if ids.is_empty() {
            return;
        }
        self.clear_selection(cx);
        if self.fleet.is_some() {
            crate::move_to_project::post(self.port(cx), crate::move_to_project::body(&ids, project.as_deref()), cx);
            return;
        }
        for job in self.jobs.iter_mut().filter(|job| ids.iter().any(|id| id.as_ref() == job.id.as_str())) {
            if let Some(hosted) = job.hosted.as_mut() {
                hosted.project = project.clone();
            }
        }
        self.relist(cx);
        cx.notify();
    }

    /// Open the project picker for the Jobs an act on this row reaches, under the row, or in the window's upper middle
    /// where the row is not drawn. `back` is where the keys go when it shuts, else the list.
    pub fn pick_project(&mut self, id: &str, back: Option<FocusHandle>, window: &mut Window, cx: &mut Context<Self>) {
        let at = self.row_under(id);
        self.open_project_picker(self.reached_by(id), at, true, back, window, cx);
    }

    /// Where a menu for this row hangs: its middle, at its bottom edge, where the row is drawn.
    pub fn row_under(&self, id: &str) -> Option<Point<Pixels>> {
        self.bounds.borrow().get(id).copied().map(under)
    }

    /// The project picker for these Jobs: his projects with their repositories under the name, the copies of another
    /// Mac's projects, then No project, the row they all stand in ticked. A Job on another Mac, which Fleet refuses, says why in a toast and opens nothing.
    /// `at` is where its corner sits, or none for the window's upper middle.
    pub fn open_project_picker(&mut self, ids: Vec<SharedString>, at: Option<Point<Pixels>>, from_keys: bool, back: Option<FocusHandle>, window: &mut Window, cx: &mut Context<Self>) {
        let jobs: Vec<&SessionRecord> = ids.iter().filter_map(|id| self.record(id)).filter(|job| crate::move_to_project::movable(job)).collect();
        if jobs.is_empty() {
            return;
        }
        if let Some(why) = jobs.iter().find_map(|job| crate::pairing::refusal_to(job, crate::move_to_project::ROUTE, cx)) {
            crate::toasts::tell(why, cx);
            return;
        }
        let standing = crate::move_to_project::standing(jobs.iter().copied());
        let entries = crate::move_to_project::entries(&crate::move_to_project::projects(self.fleet.is_none(), cx), &crate::move_to_project::copies(cx), &standing);
        let ids: Vec<SharedString> = jobs.iter().map(|job| SharedString::from(job.id.as_str().to_string())).collect();
        let (at, corner) = match at {
            Some(at) => (at, Anchor::TopLeft),
            None => {
                let window_size = window.viewport_size();
                (point(window_size.width / 2., window_size.height / 4.), Anchor::TopCenter)
            }
        };
        self.card = None;
        let menu = cx.new(|cx| Dropdown::new("Project", entries, at, corner, from_keys, cx));
        let picked = cx.subscribe(&menu, move |this, _, picked: &Picked, cx| {
            let Picked::Entry { id: row, .. } = picked;
            if let Some(project) = crate::move_to_project::project_of(row) {
                this.move_to_project(ids.clone(), project, cx);
            }
        });
        let dismissed = cx.subscribe_in(&menu, window, move |this, shut, _: &DismissEvent, window, cx| {
            // Only the picker that shut gives the keys back: another menu may have opened as this one was picked.
            if this.menu.as_ref().is_some_and(|open| open.menu == *shut) {
                this.menu = None;
            }
            match &back {
                Some(handle) => window.focus(handle, cx),
                None => window.focus(&this.focus, cx),
            }
            cx.notify();
        });
        let keys = menu.read(cx).focus_handle(cx);
        window.defer(cx, move |window, cx| window.focus(&keys, cx));
        self.menu = Some(Open { menu, _picked: picked, _dismissed: dismissed });
        cx.notify();
    }

    /// A walk's stand-in for how a move ends. `Ended` is the Mac the Job left ending it, with `moved_to_mac`; `Landed`
    /// is the other Mac's row of the same id standing in its place, or, for a Job brought here, this Mac's row
    /// replacing the one held there.
    pub fn mock_move_settled(landing: bool, cx: &mut App) -> Result<(), String> {
        let list = cx.try_global::<MockList>().and_then(|held| held.0.upgrade()).ok_or("no Jobs list is drawn yet")?;
        let macs = crate::pairing::macs(cx).to_vec();
        list.update(cx, |list, cx| {
            for job in list.jobs.iter_mut() {
                let moving = crate::move_to_mac::moving_to(job).map(str::to_string);
                if !landing {
                    // Only a Job of this Mac ends here: one brought from the other Mac ends there, out of sight.
                    if let (Some(_), None) = (&moving, &job.mac) {
                        job.state = SessionState::Ended;
                        job.end_reason = Some(crate::move_to_mac::ENDED_AS.into());
                        job.ended_at = Some(ipc::Instant::carried(chrono::Utc::now().to_rfc3339()));
                        if let Some(hosted) = job.hosted.as_mut() {
                            hosted.moving_to_mac = None;
                        }
                    }
                    continue;
                }
                let target = moving.or_else(|| crate::move_to_mac::moved(job).then(|| crate::move_to_mac::went_to(&macs).map(str::to_string)).flatten());
                let Some(target) = target else { continue };
                job.state = SessionState::Live;
                job.end_reason = None;
                job.ended_at = None;
                job.mac = (crate::move_to_mac::this_mac(&macs) != Some(target.as_str())).then_some(target);
                if let Some(hosted) = job.hosted.as_mut() {
                    hosted.moving_to_mac = None;
                }
            }
            list.relist(cx);
            cx.notify();
        });
        Ok(())
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
            // Fleet moves the mark, or clears it where this Job holds it, and refuses an ended Job (`fleet.session_not_prioritisable`).
            act("prioritise", if row.prioritised { "Stop prioritising" } else { "Prioritise" }, Some("arrow-up-to-line"), false),
            match crate::snooze::menu_item(row.snoozed.is_some()) {
                Entry::Item { id, label, icon, danger, key, .. } => Entry::Item { id, label, icon, danger, key, off: off.clone() },
                other => other,
            },
            act("name", "Name it for me", Some("wand-sparkles"), false),
            Entry::Act { id: "teach", label: "Teach".into(), icon: Some("graduation-cap"), danger: false },
            act("try", "Try it in your app", Some("flask-conical"), false),
            act("judge", "Judge", Some("scale"), false),
            // Fleet carries Close to the Mac that holds the Job, so it stays on.
            match self.record(&id).and_then(|job| crate::pairing::refusal_to(job, "/sessions/close", cx)) {
                Some(why) => Entry::Item { id: "close", label: "Close".into(), icon: Some("x"), danger: true, key: None, off: Some(why.into()) },
                None => Entry::Act { id: "close", label: "Close".into(), icon: Some("x"), danger: true },
            },
            Entry::Rule,
        ];
        // Fleet pauses a Job it snoozes and Wake now resumes it, so a snoozed Job offers no Resume of its own.
        if row.snoozed.is_some() {
            entries.remove(0);
        }
        // Move to the other Mac, or bring the Job here, under Snooze: the Job's own state acts stand together.
        let mut after = if row.snoozed.is_some() { 2 } else { 3 };
        if let Some(offer) = self.record(&id).and_then(|job| crate::move_to_mac::offer(job, crate::pairing::macs(cx))) {
            entries.insert(after, Entry::Item { id: "move", label: offer.label.into(), icon: Some("arrow-right-left"), danger: false, key: Some("M".into()), off: offer.off.map(Into::into) });
            after += 1;
        }
        // Move to a project, after Move to Mac: it reaches the whole selection where this row is in it.
        if let Some(job) = self.record(&id).filter(|job| crate::move_to_project::movable(job)) {
            let off = crate::pairing::refusal_to(job, crate::move_to_project::ROUTE, cx).map(Into::into);
            entries.insert(after, Entry::Item { id: "move_project", label: "Move to project".into(), icon: Some("folder-input"), danger: false, key: Some("\u{21e7}P".into()), off });
        }
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
        let picked = cx.subscribe_in(&menu, window, move |this, _, picked: &Picked, window, cx| {
            let Picked::Entry { id: act, .. } = picked;
            let port = this.port(cx);
            let session_id = ipc::SessionId::carried(job.to_string());
            let encoded = |body: Result<String, ipc::Unencodable>| body.map_err(|why| format!("{why:?}"));
            match *act {
                "pause" => send(port, "/sessions/pause", encoded(ipc::encode(&ipc::PauseSession { session_id })), cx),
                "resume" => send(port, "/sessions/resume", encoded(ipc::encode(&ipc::ResumeSession { session_id })), cx),
                "prioritise" => send(port, "/sessions/prioritise", encoded(ipc::encode(&ipc::PrioritiseSession { session_id })), cx),
                "snooze" => crate::dispatch::screen::snooze_ask(job.clone(), title.clone(), Some(at), cx),
                "wake" => this.snooze(&job, None, cx),
                "move" => this.move_to_mac(&job, cx),
                // After this menu has shut, which would otherwise take the picker's place as it goes.
                "move_project" => {
                    let job = job.clone();
                    cx.defer_in(window, move |this, window, cx| {
                        let ids = this.reached_by(&job);
                        this.open_project_picker(ids, Some(at), from_keys, None, window, cx);
                    });
                }
                "judge" => this.judge_job(&job, window, cx),
                "name" => send(port, "/sessions/name", encoded(ipc::encode(&ipc::NameSession { session_id })), cx),
                "teach" => crate::dispatch::screen::teach_row(title.to_string(), cx),
                "try" => {
                    let pick = BuildAct::Pick(ipc::PickFleetBuildJob { branch: None, session_id: Some(job.to_string()), picked: true });
                    match this.fleet() {
                        Some(fleet) => fleet.update(cx, |fleet, cx| fleet.build_act(pick, cx)),
                        None => Note::warn("not sent: no Fleet is connected").field("route", "/fleet/build/pick").emit(),
                    }
                }
                "close" => this.end_row(&job, cx),
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
            let icon = if folded { "unfold-vertical" } else { "fold-vertical" };
            entries.push(Entry::Act { id: "fold", label: label.into(), icon: Some(icon), danger: false });
        }
        let label = if all_unfold { "Unfold all sub-Jobs" } else { "Fold all sub-Jobs" };
        let icon = if all_unfold { "chevrons-up-down" } else { "chevrons-down-up" };
        entries.push(Entry::Act { id: "fold_all", label: label.into(), icon: Some(icon), danger: false });
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

    /// The view menu, under its button, the keys on the group and the choice of Jobs the list holds now.
    fn open_view(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.filter.is_some() {
            return;
        }
        let Some(bounds) = self.bounds.borrow().get("view").copied() else { return };
        let gap = cx.look().length(token::SPACE_1);
        let at = point(bounds.origin.x + bounds.size.width, bounds.origin.y + bounds.size.height + gap);
        let entries = view_entries(self.scope, self.group, self.folds.root_only, self.folds.hide_done);
        let menu = cx.new(|cx| Dropdown::new("View", entries, at, Anchor::TopRight, true, cx));
        let picked = cx.subscribe(&menu, |this, _, picked: &Picked, cx| {
            let Picked::Entry { id, .. } = picked;
            this.pick_view(id, cx);
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

    /// A choice in the view menu: a group, a choice of Jobs to show, or the switch that folds every parent.
    fn pick_view(&mut self, id: &str, cx: &mut Context<Self>) {
        if id == "root_only" {
            self.set_root_only(!self.folds.root_only, cx);
        } else if id == "done_jobs" {
            self.set_hide_done(!self.folds.hide_done, cx);
        } else if let Some((group, ..)) = GROUPS.iter().find(|(_, held, _)| *held == id) {
            self.group = *group;
            self.keep();
            self.relist(cx);
        } else if let Some((scope, ..)) = SHOWS.iter().find(|(_, held, _)| *held == id) {
            self.scope = *scope;
            self.keep();
            self.relist(cx);
        }
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
        let ink = if row.paused || row.ended || row.place || row.asleep { token::FG_MUTED } else { token::FG_DEFAULT };
        let lamp = match row.mark {
            _ if row.asleep => token::BORDER_STRONG,
            Some(mark) => lamp_of(mark.lamp),
            None if row.ended => token::STATUS_COMPLETED_SUCCESS,
            None => token::BORDER_STRONG,
        };
        // Needs him, and not the Job open now: the row's tint breathes (`ListCard.css`).
        // A snoozed Job is quiet to the eye as well: its edge stays amber and nothing moves.
        let needs = row.mark.is_some_and(|mark| mark.lamp == JobLamp::Needs) && !opened && row.snoozed.is_none() && !row.asleep;
        let state = row.mark.filter(|_| !row.paused && !row.asleep).map(|mark| (glyph_of(mark.state, row.pr.as_deref()), hue_of(mark.state)));
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
        // The workflow as a chip, its steps as bars and the step with how long the Job has been on it; a dashed chip where the Job follows none.
        let flow = view.on("view:workflow").then(|| {
            let line = meta(look).id(SharedString::from(format!("flow-{}", row.id))).flex().items_center().gap(look.length(token::SPACE_2)).min_w_0();
            match row.flow.clone() {
                Some((name, step)) => {
                    let said: SharedString = format!("{name}: {step}").into();
                    let step_words = match row.track.as_ref().and_then(|track| track.time.clone()) {
                        Some(time) => format!("{step} \u{b7} {time}"),
                        None => step.to_string(),
                    };
                    // A step with no move for longer than its usual time reads in amber, and its tooltip gives Fleet's cause.
                    let cause = row.track.as_ref().and_then(|track| track.stalled.clone());
                    let pips_said: SharedString = match &cause {
                        Some(cause) => format!("{said}: {cause}").into(),
                        None => said.clone(),
                    };
                    line.child(chip(format!("flow-chip-{}", row.id), name, said.clone()))
                        .children(row.track.as_ref().filter(|track| !track.pips.is_empty()).map(|track| crate::job_view::column_strip::pips_bar(look, SharedString::from(format!("flow-pips-{}", row.id)), &track.pips, pips_said)))
                        .child(
                            div()
                                .id(SharedString::from(format!("flow-step-{}", row.id)))
                                .min_w_0()
                                .truncate()
                                .text_color(look.color(if cause.is_some() { token::STATUS_AWAITING_REVIEW } else { token::FG_MUTED }))
                                .child(SharedString::from(step_words))
                                .when_some(cause, |words, cause| words.tooltip(tip(cause.into()))),
                        )
                }
                None => line.child(
                    meta(look)
                        .id(SharedString::from(format!("flow-none-{}", row.id)))
                        .flex_none()
                        .px(look.length(token::SPACE_1))
                        .rounded(look.length(token::RADIUS_SM))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_dashed()
                        .border_color(look.color(token::BORDER_DEFAULT))
                        .text_color(look.color(token::FG_SUBTLE))
                        .child("No workflow")
                        .tooltip(tip("No workflow".into())),
                ),
            }
        });
        let now = row.ending.clone().or_else(|| row.now.clone().filter(|_| view.on("view:now"))).map(|now| {
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
        // Waiting for a slot asks him nothing: a quiet glyph beside the state, its place in the tooltip.
        let slot_wait = row.slot_wait.clone().map(|said| {
            labelled(
                div()
                    .id(SharedString::from(format!("slot-wait-{}", row.id)))
                    .child(svg().path(icon_path("hard-drive")).size(mark_glyph).text_color(look.color(token::FG_MUTED)))
                    .tooltip(tip(said.clone())),
                Role::Image,
                said,
            )
        });
        let status_on = view.on("view:status");
        // Once a second Mac is listed, a row wears a dot in the colour of the Mac it runs on, right before its state glyph.
        let dotted = crate::pairing::shows_chips(crate::pairing::macs(cx));
        // A Job gone to the other Mac wears that Mac's dot, and says so where its state is.
        let going: Option<SharedString> = row.moved.then(|| crate::move_to_mac::went_to(crate::pairing::macs(cx)).map(|id| SharedString::from(id.to_string()))).flatten();
        let going_name = going.as_deref().and_then(|id| crate::pairing::chip(Some(id), cx)).map(|chip| chip.name);
        let moved_said: Option<SharedString> = row.moved.then_some(going_name.as_ref()).flatten().map(|name| format!("Moved to {name}").into());
        // A Job on this Mac wears no mark. One on another Mac, or gone to it, wears a muted laptop glyph; the Mac's name is in
        // the row's hover card. Only a Job on its way keeps the Mac's dot, in its from → to mark.
        let mut mac_dot = if row.moving_to.is_some() {
            dotted
                .then(|| crate::pairing::chip(going.as_deref().or(row.mac.as_deref()), cx))
                .flatten()
                .map(|chip| crate::pairing::dot(look, SharedString::from(format!("mac-{}", row.id)), &chip))
        } else {
            let away = going.as_deref().or(row.mac.as_deref());
            dotted.then(|| crate::pairing::chip(away, cx)).flatten().filter(|_| away.is_some()).map(|chip| {
                labelled(
                    div()
                        .id(SharedString::from(format!("mac-{}", row.id)))
                        .flex_none()
                        .child(svg().path(icon_path("laptop")).size(mark_glyph).text_color(look.color(token::FG_SUBTLE))),
                    Role::Image,
                    SharedString::from(format!("On {}", chip.name)),
                )
                .into_any_element()
            })
        };
        // A Job on its way keeps its place and wears an amber edge. At the row's end stand its Mac's dot, a muted arrow, the other
        // Mac's dot and the amber word Moving, so the title keeps the width and the row stays one line. Where it goes is the mark's
        // tooltip and its name.
        let on_its_way = row.moving_to.as_ref().map(|target| {
            let macs = crate::pairing::macs(cx);
            let away = row.mac.is_some();
            // A Job held from the other Mac that is moving here names this Mac by an id the list may not hold.
            let to = crate::pairing::chip_of(macs, Some(target)).or_else(|| away.then(|| crate::pairing::chip_of(macs, None)).flatten());
            let words = moving_mark_said(macs, target, away);
            labelled(
                meta(look)
                    .id(SharedString::from(format!("moving-{}", row.id)))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_1))
                    .children(mac_dot.take())
                    .child(div().flex_none().text_color(look.color(token::FG_SUBTLE)).child("\u{2192}"))
                    .children(to.map(|chip| crate::pairing::dot(look, SharedString::from(format!("mac-to-{}", row.id)), &chip)))
                    .child(div().flex_none().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(look.color(token::STATUS_AWAITING_REVIEW)).child("Moving"))
                    .tooltip(tip(words.clone())),
                Role::Status,
                words,
            )
        });
        let lamp_ink = if on_its_way.is_some() { look.color(token::STATUS_AWAITING_REVIEW) } else { lamp_ink };
        // A row with a time keeps the slot's fixed width so marks line up down the list; one without lets the slot shrink to its glyphs.
        let timed = time.is_some();
        let marks = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_1))
            .children(self.judge_mark(&row.id, &row.title, look, cx))
            .children(strip_of)
            .when(view.on("view:conflicts"), |marks| marks.child(crate::clash_mark::ClashMark::in_row(row.id.clone())))
            // The time stands right against the state; the glyphs end at the row's edge, so they
            // line up down the list.
            .children(time)
            .when(!status_on, |marks| marks.children(earlier_mark.take()).children(mac_dot.take()))
            .when(status_on, |marks| {
                // The paused mark in place of the state's glyph, and a hand off's ring before it.
                // Electron's wide mark (`ListCard.css`): two glyphs and the gap between, the state at its end.
                let wide = look.length(token::SPACE_4) * 2. + look.length(token::SPACE_1);
                // The dot and its gap are the slot's to hold once a second Mac is listed.
                let wide = if dotted { wide + look.length(token::SPACE_2) + look.length(token::SPACE_1) } else { wide };
                marks.child(div().flex_none().when(timed || !dotted, |slot| slot.w(wide)).flex().items_center().justify_end().gap(look.length(token::SPACE_1)).when(row.handing_off, |slot| {
                    slot.child(labelled(
                        div()
                            .id(SharedString::from(format!("handoff-{}", row.id)))
                            .child(crate::titlebar::parts::spinner(SharedString::from(format!("handing-{}", row.id)), look.length(token::SPACE_4), look.color(token::FG_MUTED), cx))
                            .tooltip(tip("Handing off".into())),
                        Role::Image,
                        "Handing off",
                    ))
                }).when(row.prioritised, |slot| {
                    slot.child(labelled(
                        div()
                            .id(SharedString::from(format!("prioritised-{}", row.id)))
                            .child(svg().path(icon_path("arrow-up-to-line")).size(mark_glyph).text_color(look.color(token::ACCENT)))
                            .tooltip(tip("Prioritised: its builds run first when the machine is loaded".into())),
                        Role::Image,
                        "Prioritised",
                    ))
                }).children(quiet).children(slot_wait).children(earlier_mark.take()).children(mac_dot.take()).children(state.map(|((glyph, said), hue)| {
                    let said = moved_said.clone().unwrap_or(said);
                    labelled(
                        div()
                            .id(SharedString::from(format!("state-{}", row.id)))
                            .child(svg().path(icon_path(glyph)).size(look.length(token::SPACE_4)).text_color(look.color(hue)))
                            .tooltip(tip(said.clone())),
                        Role::Image,
                        said,
                    )
                })).when(row.paused && row.snoozed.is_none(), |slot| {
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
        // A selected row wears an accent edge over a faint accent fill: the open row's fill is the stronger one.
        let selected = self.selected.contains(&row.id);
        let stalled = row.track.as_ref().is_some_and(|track| track.stalled.is_some());
        let strip = row_card(look, SharedString::from(format!("job-{}", row.id)), opened, focused, !row.place && !selected, pad)
            .when(stalled && !opened && !selected, |strip| stalled_wash(strip, look))
            .when(selected && !opened, |strip| strip.border_color(look.color(token::ACCENT)).bg(look.color(token::ACCENT_FAINT)))
            .when(movable, |strip| strip.on_drag(dragged, |dragged, _, _, cx| cx.new(|_| RowGhost(dragged.title.clone()))))
            .when(lifted, |strip| strip.opacity(0.55))
            .when(row.asleep, |strip| strip.opacity(0.6))
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
            .when(!opened && !selected && !needs, |strip| strip.child(row_wash(look, lamp_ink)))
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
                    .children((row.answer && !row.asleep).then(|| {
                        let said = "An answer to your question is waiting";
                        labelled(div().id(SharedString::from(format!("answer-{}", row.id))).flex_none().size(look.length(token::SPACE_2)).rounded_full().bg(look.color(token::YOU)).tooltip(tip(said.into())), Role::Image, said)
                    }))
                    .children(on_its_way)
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
                    .on_click(cx.listener(move |this, event: &gpui::ClickEvent, window, cx| {
                        this.focused = Some(pressed.clone());
                        window.focus(&this.focus, cx);
                        let keys = event.modifiers();
                        // ⌘ toggles the row in the selection and ⇧ selects the range to it: neither opens the Job.
                        if keys.platform {
                            this.toggle_selected(&pressed, cx);
                        } else if keys.shift {
                            this.select_range(&pressed, cx);
                        } else if this.closable() && this.opened.as_ref() == Some(&pressed) {
                            // The Cockpit: the open Job's row pressed again puts the Operator back.
                            this.clear_selection(cx);
                            this.anchor = Some(pressed.clone());
                            this.close_job(cx);
                        } else {
                            // A plain press lets go of the selection, opens the Job, and is where a ⇧-click ranges from.
                            this.clear_selection(cx);
                            this.anchor = Some(pressed.clone());
                            this.opened = Some(pressed.clone());
                            this.card = None;
                            this.picks += 1;
                        }
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
            Line::Group(head) => {
                let look = cx.look();
                let hue = head.stand.map_or(token::STATUS_NOT_STARTED, Stand::hue);
                let (name, count) = (head.name.clone(), head.count);
                let said = name.clone();
                // The dot and the words take the state's hue; the count after them is quiet, at the right edge.
                let dot = div()
                    .id(("group-dot", at))
                    .flex_none()
                    .size(look.length(token::SPACE_2))
                    .rounded_full()
                    .bg(look.color(hue))
                    .tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(said.clone()).build(window, cx));
                labelled(
                    group_line(look, at == 0)
                        .id(("project", at))
                        .gap(look.length(token::SPACE_2))
                        .when(head.stand.is_some(), |line| line.text_color(look.color(hue)))
                        .child(dot)
                        .child(name.clone())
                        .child(div().ml_auto().text_color(look.color(token::FG_MUTED)).child(count.to_string())),
                    Role::Heading,
                    name,
                )
                .into_any_element()
            }
            Line::Job(row) => {
                let row = row.clone();
                self.render_job(at, &row, cx)
            }
            Line::Asleep(shelf) => {
                let shelf = shelf.clone();
                self.render_shelf(at, &shelf, cx)
            }
        }
    }

    /// The bar over every group: the search field, and the scope it searches in.
    fn render_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let store = Rc::clone(&self.bounds);
        let label = SharedString::from(view_label(self.scope, self.group));
        let ink = view_ink(self.scope);
        let field = self.search.as_ref().map(|(search, _)| {
            labelled(
                div().id("search").key_context(crate::keymap::TEXT_BOX).flex_1().min_w_0().child(list_search(search, look)),
                Role::SearchInput,
                "Search Jobs",
            )
        });
        list_bar(look)
            // ↓, Enter or Esc give the keys to the rows; Tab opens the view menu.
            .capture_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| match event.keystroke.key.as_str() {
                "down" | "escape" => {
                    cx.stop_propagation();
                    this.to_rows(window, cx);
                }
                "tab" => {
                    cx.stop_propagation();
                    this.open_view(window, cx);
                }
                _ => {}
            }))
            .children(field)
            .child(labelled(
                list_control(look)
                    .id("view")
                    .text_color(look.color(ink))
                    // Needs you's accent rings the button while the list is narrowed, as the approved page draws it.
                    .when(self.scope != Scope::Active, |button| button.border_color(look.color(ink)))
                    .child(canvas(move |bounds, _, _| { store.borrow_mut().insert("view".into(), bounds); }, |_, _, _, _| {}).absolute().size_full())
                    .child(label.clone())
                    .child(svg().path(icon_path("chevron-down")).size(look.length(token::SPACE_3)).text_color(look.color(ink)))
                    .tooltip(|window, cx| gpui_component::tooltip::Tooltip::new("How the list is grouped, and which Jobs it shows").build(window, cx))
                    .on_click(cx.listener(|this, _, window, cx| this.open_view(window, cx))),
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
        let mac = crate::pairing::chip(row.mac.as_deref(), cx);
        let going = row.moving_to.as_deref().and_then(|id| crate::pairing::chip(Some(id), cx)).map(|chip| crate::pairing::Chip { name: format!("Moving to {}", chip.name), ..chip });
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
            .children(mac.map(|chip| small(look).text_color(look.color(token::FG_DEFAULT)).child(crate::pairing::card_line(look, &chip))))
            .children(going.map(|chip| small(look).text_color(look.color(token::FG_DEFAULT)).child(crate::pairing::card_line(look, &chip))))
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

/// The Jobs screen in Electron's regions (`cockpit/deck/Deck.tsx`, `.deck-grid`): the list's panel at
/// the left, the open Job's room in the middle, and the right column's panel, under the title row.
/// A handle on the list's inner edge drags it wider or narrower, as Electron's `DockHandle` does.
pub struct JobsScreen {
    pub list: Entity<JobsList>,
    /// The open Job, in the middle.
    view: Entity<JobView>,
    /// The list's width as dragged; absent, it rests at Electron's width.
    left: Option<f32>,
    handle: FocusHandle,
    /// The whole screen's, which holds the list and the handle beside it.
    within: FocusHandle,
    remember: bool,
    /// The command palette, over the whole screen.
    palette: Entity<crate::palette::Palette>,
    /// The open Job is drawn from gpui's cache (`job_area`): what it wrote when last drawn whole, and its count of draws.
    kept: Rc<RefCell<crate::labels::KeptView>>,
    drawn: Rc<std::cell::Cell<u64>>,
    /// The Cockpit's home: the Operator's page, drawn on the right while no Job is open.
    home: Option<Entity<crate::operator::page::OperatorPageView>>,
    /// The floor plan that stands in the page's place while the Operator is off, and the redraw it asks for when that changes.
    floor: Option<(Entity<crate::features::floor_home::Floor>, Subscription)>,
    _drawn_with: Subscription,
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
        let left = remember
            .then(|| columns_file().and_then(|path| std::fs::read_to_string(path).ok()).and_then(|text| text.trim().parse::<f32>().ok()))
            .flatten();
        let palette = cx.new(|cx| crate::palette::Palette::new(list.clone(), cx));
        // The bar's search, and ⌘K where no screen took it, open this palette.
        let handle = window.window_handle();
        let opens = palette.downgrade();
        let fleet_of = list.downgrade();
        cx.set_global(crate::titlebar::AskedSink(Rc::new(move |asked: &crate::titlebar::Asked, cx: &mut App| {
            // A project or a repository picked: the list follows the scope the bar now holds. The bar is being updated
            // while it hands this on, so the list reads it once that is done.
            if matches!(asked, crate::titlebar::Asked::Project(_) | crate::titlebar::Asked::Repository(_)) {
                if let Some(list) = fleet_of.upgrade() {
                    cx.defer(move |cx| list.update(cx, |list, cx| list.follow_scope(cx)));
                }
                return;
            }
            // Skip on the Refresh row goes to Fleet, which keeps the row skipped.
            if let crate::titlebar::Asked::SkipOffers(id) = asked {
                match fleet_of.upgrade().and_then(|list| list.read(cx).fleet()) {
                    Some(fleet) => fleet.update(cx, |fleet, cx| fleet.skip_offers(id.clone(), cx)),
                    None => Note::warn("not sent: no Fleet is connected").field("route", "/fleet/build/offers/skip").emit(),
                }
                return;
            }
            // The crash panel started a Job to fix the crash: it opens.
            if let crate::titlebar::Asked::OpenJob(id) = asked {
                if let Some(list) = fleet_of.upgrade() {
                    let id = id.clone();
                    cx.defer(move |cx| list.update(cx, |list, cx| list.open(id.into(), cx)));
                }
                return;
            }
            // The bar's build switcher: each press is Fleet's build act, followed until the report settles.
            let act = match asked {
                crate::titlebar::Asked::ChangeBuild { build, clear } => Some(BuildAct::Change(ipc::ChangeFleetBuild { build: *build, adopt: false, clear: *clear })),
                crate::titlebar::Asked::RefreshBoth => Some(crate::pairing::refresh_act(crate::pairing::macs(cx))),
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
            // The switcher's In this build: the sheet of the running build's notes opens over the window.
            if *asked == crate::titlebar::Asked::BuildNotes {
                crate::dispatch::screen::open_build_notes(cx);
                return;
            }
            // The switcher's New project: its sheet opens over the window.
            if *asked == crate::titlebar::Asked::NewProject {
                crate::dispatch::screen::open_new_project(cx);
                return;
            }
            if let crate::titlebar::Asked::ProjectMembers(at) = asked {
                crate::dispatch::screen::open_project_members(*at, cx);
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
        let drawn = view.read(cx).drawn();
        // What moves this screen moves the open Job, which is otherwise drawn from its cache.
        let drawn_with = cx.observe_self(|this, cx| this.view.update(cx, |_, cx| cx.notify()));
        JobsScreen {
            list,
            view,
            left,
            handle: cx.focus_handle(),
            within: cx.focus_handle(), remember, palette, kept: Rc::default(), drawn, home: None, floor: None, _drawn_with: drawn_with }
    }

    pub fn within(&self) -> &FocusHandle {
        &self.within
    }

    /// The open Job, drawn from gpui's cache so a pulse on a list row does not draw it again each frame.
    /// While an overlay stands it is drawn whole, since the cache cannot put an overlay's labels back.
    fn job_area(&self, window: &Window, cx: &App) -> gpui::AnyElement {
        let overlaid = cx.try_global::<crate::labels::LabelBook>().is_some_and(|book| book.overlaid(window.window_handle().window_id()));
        if overlaid {
            return self.view.clone().into_any_element();
        }
        crate::labels::kept(self.view.clone().cached(gpui::StyleRefinement::default().size_full()), self.kept.clone(), self.drawn.clone()).into_any_element()
    }

    /// The Operator's page becomes the right side while no Job is open: this screen is the Cockpit.
    pub(crate) fn set_home(&mut self, page: Entity<crate::operator::page::OperatorPageView>, cx: &mut Context<Self>) {
        self.home = Some(page);
        self.list.update(cx, |list, _| list.set_cockpit(true));
        cx.notify();
    }

    /// The floor plan stands where the page does while the Operator is off.
    pub(crate) fn set_floor(&mut self, floor: Entity<crate::features::floor_home::Floor>, cx: &mut Context<Self>) {
        let sub = cx.subscribe(&floor, |_, _, _: &crate::features::floor_home::Moved, cx| cx.notify());
        self.floor = Some((floor, sub));
        cx.notify();
    }

    /// Leave the open Job for the Operator's page, with the keys on the page. False where no Job is open or there is no page to go to.
    pub(crate) fn leave_job(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if !self.list.update(cx, |list, cx| list.close_job(cx)) {
            return false;
        }
        if let Some(home) = &self.home {
            let keys = home.read(cx).focus.clone();
            window.focus(&keys, cx);
        }
        true
    }

    /// ⌘J: on the page the caret goes to the Operator's box; with a Job open it is that Job's chat, as ever.
    pub(crate) fn operator_key(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.home.clone().filter(|_| self.list.read(cx).at_home()) {
            Some(home) => home.update(cx, |page, cx| page.focus_box(window, cx)),
            None => {
                let act = Act { id: "operator", slot: 0, digit: None };
                self.view.update(cx, |view, cx| view.side_chat_act(&act, window, cx));
            }
        }
    }

    /// Esc, where nothing in a Job's view, a menu or a text box took it, leaves the Job.
    fn leave_key(&mut self, event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let key = &event.keystroke;
        if key.key != "escape" || key.modifiers.modified() {
            return false;
        }
        if window.context_stack().iter().any(|context| context.contains(crate::keymap::TEXT_BOX)) {
            return false;
        }
        self.leave_job(window, cx)
    }

    /// The right side where no Job is open: the page, with the Job view under it unseen where a card or the Review panel needs it.
    fn home_area(&self, page: Entity<crate::operator::page::OperatorPageView>, cx: &App) -> gpui::AnyElement {
        let hosts = self.list.read(cx).opened.is_some() || self.view.read(cx).review_up();
        div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .children(hosts.then(|| div().absolute().size_full().invisible().child(self.view.clone())))
            .child(match self.floor.as_ref().filter(|(floor, _)| floor.read(cx).is_up(cx)) {
                Some((floor, _)) => div().flex_1().min_h_0().child(floor.clone()),
                None => div().flex_1().min_h_0().child(page),
            })
            .into_any_element()
    }

    /// The palette, for the screen this one sits in to draw while another place is in front.
    pub(crate) fn palette(&self) -> &Entity<crate::palette::Palette> {
        &self.palette
    }

    /// The Job view, for the screen this one sits in to draw under its own while the Review panel it holds is up there.
    pub(crate) fn view(&self) -> &Entity<JobView> {
        &self.view
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
            // The review panel is over the list: it shuts first, so the keys land where they are drawn.
            self.view.update(cx, |view, cx| view.close_review(window, cx));
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
        } else if self.palette.read(cx).is_open() {
            cx.propagate();
        } else if act.is("operator") && self.home.is_some() && self.list.read(cx).at_home() {
            // ⌘J on the Cockpit with no Job open puts the caret in the Operator's box.
            self.operator_key(window, cx);
        } else if self.view.update(cx, |view, cx| view.side_chat_act(act, window, cx)) {
            // ⌘J with a Job open is that Job's side chat; with none it is the Operator's, as the registry has it.
        } else if self.view.update(cx, |view, cx| view.pane_act(act, window, cx)) {
            // A key that arranges the open Job's panes, from wherever on the deck the keys are.
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

/// A stalled row's amber is a wash over its ground. Its edge stays the list's own, so the ring is the open row's alone.
fn stalled_wash(card: gpui::Stateful<gpui::Div>, look: &Look) -> gpui::Stateful<gpui::Div> {
    card.bg(look.color(token::BG_SUNKEN).blend(look.color(token::STATUS_AWAITING_REVIEW).opacity(0.10)))
}

/// What the mark at the end of a moving row says, and its tooltip: where the Job goes. A Job held from the other Mac that
/// is moving here names this Mac, by an id the list may not hold.
fn moving_mark_said(macs: &[ipc::Mac], target: &str, held_away: bool) -> SharedString {
    crate::job_view::moving_said(&crate::job_view::name_of(macs, target, held_away)).into()
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
        div()
            .key_context("deck")
            .track_focus(&self.within)
            .on_action(cx.listener(Self::on_act))
            // A chord the person gave a palette row runs that row from anywhere on the screen; Tab reaches every tab stop.
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                if this.palette.update(cx, |palette, cx| palette.hotkey(&event.keystroke, window, cx))
                    || this.leave_key(event, window, cx)
                    || this.list.update(cx, |list, cx| list.pause_key(event, window, cx))
                    || this.list.update(cx, |list, cx| list.snooze_key(event, window, cx))
                    || this.list.update(cx, |list, cx| list.select_key(event, window, cx))
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
            .children(crate::pairing::frozen_band(look, crate::pairing::macs(cx), self.list.read(cx).port(cx), cx))
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
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
                        // The Job's own area: its panes, as the Job's arrangement lays them out (`job_view/place_draw.rs`).
                        .child(
                            div()
                                .id("job-split")
                                .relative()
                                .h_full()
                                .flex_1()
                                .min_w_0()
                                .child(match self.home.clone().filter(|_| self.list.read(cx).at_home()) {
                                    Some(page) => self.home_area(page, cx),
                                    None => self.job_area(window, cx),
                                }),
                        ),
                )
                // A layout that hides the merge line's panel hides this queue, its one counterpart (`Deck.tsx`).
                .when(!crate::settings::layout::hidden("dashboard.panels", "merge-line"), |page| page.child(crate::merge_strip::strip(&self.list, window, cx)))
                    ,
            )
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
        lines.iter().filter_map(|line| match line { Line::Job(row) => Some(row.title.to_string()), _ => None }).collect()
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
        assert!(matches!(&lines[0], Line::Group(head) if head.name == "armada"));
        assert!(!titles(&lines).iter().any(|title| title.contains("ended")));
        let projects: Vec<_> = lines.iter().filter_map(|line| match line { Line::Group(head) => Some(head.name.clone()), _ => None }).collect();
        assert_eq!(projects, vec![SharedString::from("armada"), "sample-web".into(), "sample-api".into()]);
    }

    fn headings(lines: &[Line]) -> Vec<String> {
        lines.iter().filter_map(|line| match line { Line::Group(head) => Some(head.name.to_string()), _ => None }).collect()
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
        assert!(lines.iter().all(|line| match line { Line::Job(row) => row.title != row.id, _ => true }));
    }

    #[test]
    fn one_project_draws_its_jobs_with_no_head() {
        let armada: Vec<_> = jobs().into_iter().filter(|job| job.manifest_id.as_ref().is_some_and(|id| id.as_str() == "armada")).collect();
        let lines = lines_of(&armada, &[], Scope::Active, "", &[], &Names::default());
        assert!(!lines.is_empty());
        assert!(lines.iter().all(|line| matches!(line, Line::Job(_))));
    }

    /// The Jobs a list draws, by id and in order of id: a list stands its Jobs by what they need, which these do not test.
    fn ids(lines: &[Line]) -> Vec<String> {
        let mut ids: Vec<String> = lines.iter().filter_map(|line| match line { Line::Job(row) => Some(row.id.to_string()), _ => None }).collect();
        ids.sort();
        ids
    }

    fn in_projects() -> (Vec<SessionRecord>, Names) {
        (crate::mock::jobs("jobs-projects").expect("the fixture decodes"), Names::of(&crate::mock::repositories(), Some(&crate::mock::projects())))
    }

    #[test]
    fn a_project_picked_keeps_its_jobs_and_all_brings_every_job_back() {
        let (all, names) = in_projects();
        let every = ids(&lines_within(&all, &[], Scope::Active, "", &[], &names, None));
        assert_eq!(every, ids(&lines_of(&all, &[], Scope::Active, "", &[], &names)), "no project is the list as it was");
        assert!(every.contains(&"mock-01".to_string()) && every.contains(&"mock-07".to_string()) && every.contains(&"mock-11".to_string()));
        let storefront = lines_within(&all, &[], Scope::Active, "", &[], &names, Some("storefront"));
        assert_eq!(ids(&storefront), ["mock-07", "mock-08", "mock-09", "mock-10"]);
        assert!(headings(&storefront).is_empty(), "one project draws no head: the bar's trigger names it");
        let armada = ids(&lines_within(&all, &[], Scope::Active, "", &[], &names, Some("armada")));
        assert!(armada.iter().all(|id| ["mock-01", "mock-02", "mock-03", "mock-04", "mock-06"].contains(&id.as_str())), "{armada:?}");
        assert!(!armada.is_empty());
        // sample-api's Jobs were started for no project: a project keeps none of them, and one Fleet does not list keeps nothing.
        assert!(ids(&lines_within(&all, &[], Scope::All, "", &[], &names, Some("ledger"))).is_empty());
        assert!(!ids(&lines_within(&all, &[], Scope::All, "", &[], &names, Some("storefront"))).iter().any(|id| id == "mock-11"));
    }

    #[test]
    fn a_job_in_another_macs_copy_of_a_project_stands_in_this_macs_project_of_that_name() {
        let all = crate::mock::jobs("jobs-projects").expect("the fixture decodes");
        let mut projects = crate::mock::projects();
        let own = projects.projects.iter().find(|one| one.id == "armada").expect("Armada is this Mac's").clone();
        projects.projects.push(ipc::Project { id: "theirs".into(), mac: Some("kiavi".into()), ..own.clone() });
        let names = Names::of(&crate::mock::repositories(), Some(&projects));
        let here = all.iter().find(|job| job.hosted.as_ref().and_then(|hosted| hosted.project.as_deref()) == Some("armada")).expect("a Job in Armada").clone();
        let mut there = here.clone();
        there.id = ipc::SessionId::carried("kiavi-01".to_string());
        there.mac = Some("kiavi".into());
        there.hosted.as_mut().expect("hosted").project = Some("theirs".into());
        assert_eq!(group_of(&there, &names), group_of(&here, &names), "one group for one project on two Macs");
        let lines = lines_of(&[here.clone(), there.clone()], &[], Scope::All, "", &[], &names);
        assert!(headings(&lines).len() <= 1, "{:?}", headings(&lines));
        assert!(ids(&lines_within(&[here, there], &[], Scope::All, "", &[], &names, Some("armada"))).contains(&"kiavi-01".to_string()), "picking Armada keeps the other Mac's Jobs");
        // A copy with no project of its name here is its own group.
        let mut alone = crate::mock::projects();
        alone.projects.push(ipc::Project { id: "theirs".into(), name: "Kiavi only".into(), mac: Some("kiavi".into()), ..own });
        let lone = Names::of(&crate::mock::repositories(), Some(&alone));
        let mut ledger = all[0].clone();
        ledger.hosted.as_mut().expect("hosted").project = Some("theirs".into());
        assert_eq!(group_of(&ledger, &lone), ("project:theirs".to_string(), "Kiavi only".to_string()));
    }

    #[test]
    fn scoped_to_a_copy_the_list_draws_the_jobs_in_it_and_those_in_this_macs_project_of_its_name() {
        let all = crate::mock::jobs("jobs-projects").expect("the fixture decodes");
        let mut projects = crate::mock::projects();
        let own = projects.projects.iter().find(|one| one.id == "armada").expect("Armada is this Mac's").clone();
        projects.projects.push(ipc::Project { id: "theirs".into(), mac: Some("kiavi".into()), ..own.clone() });
        let names = Names::of(&crate::mock::repositories(), Some(&projects));
        let here = all.iter().find(|job| job.hosted.as_ref().and_then(|hosted| hosted.project.as_deref()) == Some("armada")).expect("a Job in Armada").clone();
        let mut there = here.clone();
        there.id = ipc::SessionId::carried("kiavi-01".to_string());
        there.mac = Some("kiavi".into());
        there.hosted.as_mut().expect("hosted").project = Some("theirs".into());
        let listed = ids(&lines_within(&[here.clone(), there], &[], Scope::All, "", &[], &names, Some("theirs")));
        assert!(listed.contains(&"kiavi-01".to_string()) && listed.contains(&here.id.as_str().to_string()), "the other Mac's Job and his own, in one project: {listed:?}");
        // A copy with no project of its name here: the Jobs in it alone.
        let mut alone = crate::mock::projects();
        alone.projects.push(ipc::Project { id: "theirs".into(), name: "Kiavi only".into(), mac: Some("kiavi".into()), ..own });
        let lone = Names::of(&crate::mock::repositories(), Some(&alone));
        let mut inside = all[0].clone();
        inside.hosted.as_mut().expect("hosted").project = Some("theirs".into());
        let listed = ids(&lines_within(&[inside.clone(), all[1].clone()], &[], Scope::All, "", &[], &lone, Some("theirs")));
        assert_eq!(listed, vec![inside.id.as_str().to_string()]);
    }

    #[test]
    fn the_scope_and_the_search_hold_within_the_project() {
        let (all, names) = in_projects();
        let within = |scope, query: &str, project| ids(&lines_within(&all, &[], scope, query, &[], &names, project));
        assert!(within(Scope::Needs, "", None).contains(&"mock-01".to_string()));
        assert_eq!(within(Scope::Needs, "", Some("storefront")), ["mock-10"], "mock-07 fails, which is not a question for him");
        assert!(within(Scope::Paused, "", Some("storefront")).is_empty(), "the Job paused is Armada's");
        assert_eq!(within(Scope::Paused, "", Some("armada")), ["mock-05"]);
        assert_eq!(within(Scope::Ended, "", Some("armada")), ["mock-15"]);
        assert!(within(Scope::Ended, "", Some("storefront")).is_empty());
        let searched = within(Scope::Active, "dark mode", Some("storefront"));
        assert_eq!(searched, ["mock-08"]);
        assert!(within(Scope::Active, "dark mode", Some("armada")).is_empty());
    }

    #[test]
    fn a_job_started_by_one_in_another_project_does_not_bring_its_starter_in_for_its_place() {
        let (mut all, names) = in_projects();
        // A Storefront Job started by an Armada one.
        let raw = r#"{"kind":"started_by","target":"mock-01","state":"spent","since":"2026-11-12T09:30:00Z","changed_at":"2026-11-12T09:30:00Z"}"#;
        let link = ipc::decode::<ipc::Attachment>("Attachment", raw.as_bytes()).expect("an attachment");
        all.iter_mut().find(|job| job.id.as_str() == "mock-08").expect("a Storefront Job").attachments.push(link);
        let ids_of = |project| ids(&lines_within(&all, &[], Scope::Active, "", &[], &names, project));
        let whole = ids_of(None);
        assert!(whole.iter().position(|id| id == "mock-08") > whole.iter().position(|id| id == "mock-01"), "under its starter on All");
        assert_eq!(ids_of(Some("storefront")), ["mock-07", "mock-08", "mock-09", "mock-10"], "alone in its project");
    }

    #[gpui::test]
    fn the_list_follows_the_scope_the_title_bar_holds(cx: &mut gpui::TestAppContext) {
        let (all, _) = in_projects();
        cx.update(crate::init_app);
        let (bar, cx) = cx.add_window_view(|window, cx| crate::titlebar::TitleBar::new(window, cx));
        let list = cx.update(|_, cx| cx.new(|cx| JobsList::mock(&all, cx)));
        let shown = |list: &Entity<JobsList>, cx: &mut gpui::VisualTestContext| list.read_with(cx, |list, _| ids(&list.lines));
        let on_all = shown(&list, cx);
        assert!(on_all.contains(&"mock-01".to_string()) && on_all.contains(&"mock-07".to_string()), "a bar at rest scoped to a project filters nothing until a pick");
        // Picking Storefront: the bar's face says so and the list follows when asked.
        let mut face = crate::titlebar::scenes::rest();
        face.scoped = Some(1);
        bar.update(cx, |bar, _| bar.reset(face.clone()));
        list.update(cx, |list, cx| list.follow_scope(cx));
        assert_eq!(shown(&list, cx), ["mock-07", "mock-08", "mock-09", "mock-10"]);
        // Back to All.
        face.scoped = None;
        bar.update(cx, |bar, _| bar.reset(face.clone()));
        list.update(cx, |list, cx| list.follow_scope(cx));
        assert_eq!(shown(&list, cx), on_all);
        // One repository is no project.
        face.chosen = Some(1);
        bar.update(cx, |bar, _| bar.reset(face));
        list.update(cx, |list, cx| list.follow_scope(cx));
        assert_eq!(shown(&list, cx), on_all);
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
                    // Fleet pauses a Job it snoozes.
                    if let Some(hosted) = job.hosted.as_mut() {
                        hosted.paused = Some(ipc::SessionPaused { at: ipc::Instant::carried("2026-11-13T14:00:00.000Z"), resuming: false });
                    }
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
    fn a_snoozed_job_is_not_in_paused() {
        let (all, [asking, working]) = with_two_snoozed();
        let paused = listed(&all, Scope::Paused);
        assert!(!paused.contains(&asking) && !paused.contains(&working), "{paused:?}");
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
            lines.iter().filter_map(|line| match line { Line::Job(row) => Some((row.title.to_string(), row.depth, row.place)), _ => None }).collect();
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
        // Waiting on him, a row says the first thing it waits on him for, in the Job's order, not what it last did.
        assert_eq!(first.now.as_deref(), Some("Which colour does the stale chip take?"));
        // What it is doing now comes before what it last summed up.
        assert_eq!(row("Native Bridge foundation").now.as_deref(), Some("Editing jobs.rs"));
    }

    /// A row on a step that stands past its usual time carries Fleet's cause for the amber and its tooltip; a row inside its time carries none.
    #[test]
    fn a_stalled_row_that_is_not_open_keeps_the_lists_own_edge() {
        let look = Look::dark();
        let card = |stalled: bool| {
            let card = row_card(&look, "job-x".into(), false, false, true, look.length(token::SPACE_2));
            if stalled { stalled_wash(card, &look) } else { card }
        };
        let (mut plain, mut stalled) = (card(false), card(true));
        assert_eq!(stalled.style().border_color, plain.style().border_color, "no amber ring beside the open row's");
        assert_eq!(stalled.style().border_color, Some(look.color(token::BORDER_SUBTLE)), "the edge is the quiet one");
        assert_ne!(stalled.style().background, plain.style().background, "the wash still marks it");
    }

    #[test]
    fn a_row_on_a_step_that_is_not_moving_carries_fleets_cause() {
        let names = Names { flows: crate::mock::workflows(), ..Names::default() };
        let lines = lines_of(&crate::mock::jobs("job-workflow").expect("the fixture decodes"), &[], Scope::Active, "", &[], &names);
        let cause = |title: &str| {
            lines
                .iter()
                .find_map(|line| match line {
                    Line::Job(row) if row.title == title => Some(row.track.as_ref().and_then(|track| track.stalled.clone())),
                    _ => None,
                })
                .expect(title)
        };
        assert!(cause("Cart total rounds the wrong way").is_some_and(|cause| cause.contains("plan")), "the stuck bug's row says why");
        assert_eq!(cause("Show the stale chip in the Jobs list"), None, "a step inside its time stays as it was");
    }

    #[test]
    fn a_waiting_row_names_what_it_waits_on_him_for_in_plain_words() {
        let jobs = crate::mock::jobs("jobs-many").expect("the fixture decodes");
        let mut job = jobs.into_iter().find(|job| job.id.as_str() == "mock-01").expect("mock-01 waits on him");
        let item = |id: &str, text: &str, source: &str| -> ipc::WaitingItem {
            serde_json::from_value(serde_json::json!({"id": id, "text": text, "since": "2026-11-09T14:00:00Z", "source": source})).expect("an item decodes")
        };
        job.waiting_for = vec![item("done:1", "Done", "done"), item("walk:u", "Macs and Models layout", "walk")];
        assert_eq!(now_of(&job).as_deref(), Some("Walk: Macs and Models layout"), "a walk is named as one; Done is no wait");
        job.waiting_for = vec![item("walk:u", "Walk the chip states", "walk")];
        assert_eq!(now_of(&job).as_deref(), Some("Walk the chip states"), "words that already say walk are not said twice");
        job.waiting_for = vec![item("sketch:t", "Tail layout", "sketch")];
        assert_eq!(now_of(&job).as_deref(), Some("Sketch: Tail layout"));
        job.waiting_for = vec![item("agent-1", "Land [#3680](https://github.com/x/y/pull/3680) with `gh pr merge`?", "agent")];
        assert_eq!(now_of(&job).as_deref(), Some("Land #3680 with gh pr merge?"), "markdown is drawn as plain words");
        // Not waiting on him: what it is doing, still in plain words.
        job.mark = job.mark.map(|mark| JobMark { lamp: JobLamp::Running, ..mark });
        job.activity = serde_json::from_value(serde_json::json!({"now": "Running `gh pr view`"})).expect("activity decodes");
        assert_eq!(now_of(&job).as_deref(), Some("Running gh pr view"));
    }

    #[test]
    fn an_idle_job_with_a_background_task_reads_as_running_it() {
        let jobs = crate::mock::jobs("jobs-many").expect("the fixture decodes");
        let mut job = jobs.into_iter().find(|job| job.id.as_str() == "mock-01").expect("mock-01");
        job.waiting_for.clear();
        job.mark = Some(JobMark { state: JobState::Idle, lamp: JobLamp::Idle, group: JobGroup::Held });
        job.activity = serde_json::from_value(serde_json::json!({"said": "Still waiting on the breakdown"})).expect("activity decodes");
        assert_eq!(mark_of(&job).map(|mark| mark.state), Some(JobState::Idle), "nothing runs: idle");
        job.running = vec![ipc::RunningWork { kind: ipc::RunningKind::BackgroundTask, what: "Break down develop folder by project".into(), since: ipc::Instant::carried("2026-11-16T16:40:10Z") }];
        let mark = mark_of(&job).expect("a mark");
        assert_eq!((mark.state, mark.lamp, mark.group), (JobState::Running, JobLamp::Running, JobGroup::Held));
        assert_eq!(glyph_of(mark.state, None).1.as_ref(), "Running");
        assert_eq!(now_of(&job).as_deref(), Some("Break down develop folder by project"));
        let rows = lines_of(std::slice::from_ref(&job), &[], Scope::All, "", &[], &Names::default());
        let row = rows.iter().find_map(|line| match line { Line::Job(row) if row.id.as_ref() == "mock-01" => Some(row), _ => None }).expect("its row");
        assert_eq!(row.mark.map(|mark| mark.state), Some(JobState::Running));
        assert_eq!(row.now.as_deref(), Some("Break down develop folder by project"));
    }

    #[test]
    fn the_view_group_lists_every_field_and_a_switch_flips_one() {
        let mut view = View::DEFAULT;
        assert!(view.on("view:status") && view.on("view:conflicts") && !view.on("view:now"));
        view.flip("view:now");
        assert!(view.on("view:now"));
        let entries = view.entries();
        let [Entry::Row(card)] = entries.as_slice() else { panic!("the main menu holds one Job Card row") };
        assert_eq!(card.label.as_ref(), "Job Card");
        assert!(card.icon.is_some());
        let ticks: Vec<_> = card.acts().iter().map(|entry| match entry { Entry::Row(row) => (row.id, row.ticked), _ => panic!("a switch is a row") }).collect();
        assert_eq!(ticks.len(), VIEW_FIELDS.len());
        assert_eq!(ticks.iter().find(|(id, _)| *id == "view:now"), Some(&("view:now", Some(true))));
    }

    fn tree() -> Vec<SessionRecord> {
        crate::mock::jobs("jobs-tree").expect("the fixture decodes")
    }

    /// The tree fixture drawn by project, where a Job stays under its starter whatever state either is in.
    fn tree_list(cx: &mut gpui::TestAppContext) -> Entity<JobsList> {
        let list = cx.new(|cx| JobsList::mock(&tree(), cx));
        list.update(cx, |list, cx| list.pick_view("project", cx));
        list
    }

    fn drawn(list: &JobsList) -> Vec<String> {
        list.rows().map(|row| row.id.to_string()).collect()
    }

    fn shut(parents: &[&str]) -> Folds {
        Folds { root_only: false, hide_done: false, parents: parents.iter().map(|id| id.to_string()).collect() }
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
        let lines = fold_lines(lines_of(&tree(), &[], Scope::Needs, "", &[], &Names::default()), &Folds { root_only: true, hide_done: false, parents: Default::default() });
        let place = lines.iter().find_map(|line| match line {
            Line::Job(row) if row.id.as_ref() == "t-walk" => Some((row.place, row.kids.clone())),
            _ => None,
        });
        assert_eq!(place, Some((true, None)));
        assert!(titles(&lines).contains(&"Re-take the shots".to_string()), "the Job that needs him stays drawn under its starters for place");
    }

    #[gpui::test]
    fn one_parent_folds_and_unfolds_from_its_row(cx: &mut gpui::TestAppContext) {
        let list = tree_list(cx);
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
        let list = tree_list(cx);
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
        let list = tree_list(cx);
        list.update(cx, |list, cx| {
            list.focused = Some("t-shots".into());
            list.set_row_fold("t-land", true, cx);
        });
        list.read_with(cx, |list, _| assert_eq!(list.focused.as_deref(), Some("t-land")));
    }

    #[gpui::test]
    fn fold_all_and_unfold_all_reach_every_parent_at_every_depth(cx: &mut gpui::TestAppContext) {
        let list = tree_list(cx);
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
        let list = tree_list(cx);
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
        let folds = Folds { root_only: true, hide_done: false, parents: ["t-land".to_string(), "t-walk".to_string()].into_iter().collect() };
        folds.save(Some(&path));
        assert_eq!(Folds::load(Some(&path)), folds);
        let per_parent = shut(&["t-macs"]);
        per_parent.save(Some(&path));
        assert_eq!(Folds::load(Some(&path)), per_parent, "the switch off is kept off");
        let _ = std::fs::remove_file(&path);
    }

    fn row_of(all: &[SessionRecord], id: &str) -> Option<Row> {
        lines_of(all, &[], Scope::Active, "", &[], &Names::default()).into_iter().find_map(|line| match line {
            Line::Job(row) if row.id.as_ref() == id => Some(row),
            _ => None,
        })
    }

    #[test]
    fn a_job_moving_to_the_other_mac_says_so_on_its_row_and_one_that_went_stays_listed_as_moved() {
        let mut all = crate::mock::jobs("jobs-screens").expect("the fixture decodes");
        let at = all.iter().position(|job| job.id.as_str() == "mock-02").expect("a hosted Job");
        let quiet = row_of(&all, "mock-02").expect("listed");
        assert_eq!((quiet.moving_to.as_deref(), quiet.moved), (None, false));
        all[at].hosted.as_mut().expect("hosted").moving_to_mac = Some("air".into());
        let moving = row_of(&all, "mock-02").expect("listed while it moves");
        assert_eq!((moving.moving_to.as_deref(), moving.moved), (Some("air"), false));
        all[at].hosted.as_mut().expect("hosted").moving_to_mac = None;
        all[at].state = SessionState::Ended;
        all[at].end_reason = Some("merged".into());
        assert!(row_of(&all, "mock-02").is_none(), "an ended Job leaves the active list");
        all[at].end_reason = Some(crate::move_to_mac::ENDED_AS.into());
        let gone = row_of(&all, "mock-02").expect("a Job that went to the other Mac stays, ended as moved");
        assert!(gone.moved);
    }

    #[test]
    fn the_mark_at_a_moving_rows_end_names_where_the_job_goes() {
        let macs = crate::pairing::mock_macs(true);
        assert_eq!(moving_mark_said(&macs, "air", false).as_ref(), "Moving to Air", "a Job this Mac holds goes to the other Mac");
        assert_eq!(moving_mark_said(&macs, "an-id-the-list-lacks", true).as_ref(), "Moving to Studio", "a Job held away that comes here names this Mac");
    }

    #[test]
    fn a_moving_row_is_dotted_from_its_mac_to_the_other_in_colours_that_read_apart() {
        let macs = crate::pairing::mock_macs(true);
        let from = crate::pairing::chip_of(&macs, None).expect("this Mac's chip");
        let to = crate::pairing::chip_of(&macs, Some("air")).expect("the other Mac's chip");
        assert_eq!((from.name.as_str(), to.name.as_str()), ("Studio", "Air"));
        assert_ne!(from.colour, to.colour);
    }

    #[gpui::test]
    fn the_mock_refuses_a_move_once_when_armed_and_the_next_press_marks_the_job_moving(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| crate::pairing::scene("air-awake", cx).expect("the scene is named"));
        let list = cx.new(|cx| JobsList::mock(&crate::mock::jobs("jobs-screens").expect("the fixture decodes"), cx));
        let moving = |list: &Entity<JobsList>, cx: &mut gpui::TestAppContext| list.read_with(cx, |list, _| list.record("mock-02").and_then(|job| crate::move_to_mac::moving_to(job).map(str::to_string)));
        assert_eq!(moving(&list, cx), None);
        crate::move_to_mac::refuse_next(true);
        list.update(cx, |list, cx| list.move_to_mac("mock-02", cx));
        assert_eq!(moving(&list, cx), None, "refused: nothing is marked");
        list.update(cx, |list, cx| list.move_to_mac("mock-02", cx));
        assert_eq!(moving(&list, cx).as_deref(), Some("air"), "the refusal was taken once, so this press goes through");
        list.update(cx, |list, cx| list.move_to_mac("mock-02", cx));
        assert_eq!(moving(&list, cx).as_deref(), Some("air"), "a Job already moving is not moved again");
    }

    #[gpui::test]
    fn a_move_left_off_while_the_other_mac_sleeps_marks_nothing(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| crate::pairing::scene("two-macs", cx).expect("the scene is named"));
        let list = cx.new(|cx| JobsList::mock(&crate::mock::jobs("jobs-screens").expect("the fixture decodes"), cx));
        list.update(cx, |list, cx| list.move_to_mac("mock-02", cx));
        list.read_with(cx, |list, _| assert_eq!(list.record("mock-02").and_then(crate::move_to_mac::moving_to), None));
    }

    /// The heading a Job stands under: the last project's name above its row.
    fn heading_of(lines: &[Line], id: &str) -> Option<String> {
        let mut heading = None;
        for line in lines {
            match line {
                Line::Group(head) => heading = Some(head.name.to_string()),
                Line::Job(row) if row.id.as_ref() == id => return heading,
                Line::Job(_) | Line::Asleep(_) => {}
            }
        }
        None
    }

    fn filter_jobs() -> Vec<SessionRecord> {
        crate::mock::jobs("project-filter").expect("the fixture decodes")
    }

    #[test]
    fn a_job_whose_project_changes_lands_under_the_new_projects_heading() {
        let mut all = filter_jobs();
        let names = Names::of(&crate::mock::repositories(), Some(&crate::mock::projects()));
        let under = |all: &[SessionRecord]| heading_of(&lines_of(all, &[], Scope::Active, "", &[], &names), "mock-01");
        let at = all.iter().position(|job| job.id.as_str() == "mock-01").expect("an Armada Job");
        assert_eq!(under(&all).as_deref(), Some("Armada"));
        all[at].hosted.as_mut().expect("hosted").project = Some("storefront".into());
        assert_eq!(under(&all).as_deref(), Some("Storefront"), "the project's own name, never its id");
        all[at].hosted.as_mut().expect("hosted").project = None;
        let repository = names.repositories.get("armada").cloned();
        assert_eq!(under(&all), repository, "in no project it stands under its repository");
        assert_ne!(under(&all).as_deref(), Some("Storefront"));
    }

    #[gpui::test]
    fn a_move_on_mock_data_regroups_the_list_and_lets_go_of_the_selection(cx: &mut gpui::TestAppContext) {
        let list = cx.new(|cx| JobsList::mock(&filter_jobs(), cx));
        list.update(cx, |list, cx| list.pick_view("project", cx));
        let heading = |list: &Entity<JobsList>, id: &str, cx: &mut gpui::TestAppContext| list.read_with(cx, |list, _| heading_of(&list.lines, id));
        assert_eq!(heading(&list, "mock-01", cx).as_deref(), Some("Armada"));
        list.update(cx, |list, cx| {
            list.toggle_selected("mock-01", cx);
            list.toggle_selected("mock-02", cx);
            let ids = list.reached_by("mock-02");
            assert_eq!(ids.len(), 2, "the row is in the selection, so the whole selection moves");
            list.move_to_project(ids, Some("storefront".into()), cx);
        });
        for id in ["mock-01", "mock-02"] {
            assert_eq!(heading(&list, id, cx).as_deref(), Some("Storefront"), "{id}");
        }
        assert_eq!(heading(&list, "mock-03", cx).as_deref(), Some("Armada"), "a Job not moved stays");
        list.read_with(cx, |list, _| assert!(list.selected().is_empty(), "a move sent lets go of the selection"));
        list.update(cx, |list, cx| list.move_to_project(vec!["mock-01".into()], None, cx));
        assert_ne!(heading(&list, "mock-01", cx).as_deref(), Some("Storefront"), "out to none");
        assert_eq!(heading(&list, "mock-02", cx).as_deref(), Some("Storefront"));
    }

    /// The Jobs the list draws, in order.
    fn drawn_ids(list: &Entity<JobsList>, cx: &mut gpui::TestAppContext) -> Vec<String> {
        list.read_with(cx, |list, _| list.rows().map(|row| row.id.to_string()).collect())
    }

    fn selected(list: &Entity<JobsList>, cx: &mut gpui::TestAppContext) -> Vec<String> {
        list.read_with(cx, |list, _| list.selected().iter().map(|id| id.to_string()).collect())
    }

    #[gpui::test]
    fn a_command_press_toggles_a_row_and_a_shift_press_selects_the_range_from_the_anchor(cx: &mut gpui::TestAppContext) {
        let list = cx.new(|cx| JobsList::mock(&filter_jobs(), cx));
        let rows = drawn_ids(&list, cx);
        assert!(rows.len() > 5);
        list.update(cx, |list, cx| list.toggle_selected(&rows[1], cx));
        list.update(cx, |list, cx| list.toggle_selected(&rows[3], cx));
        assert_eq!(selected(&list, cx), [rows[1].clone(), rows[3].clone()], "in the order added");
        list.update(cx, |list, cx| list.toggle_selected(&rows[1], cx));
        assert_eq!(selected(&list, cx), [rows[3].clone()], "toggled off");
        // The anchor is the row last toggled: rows[1]. A shift press ranges from it, whichever way it goes.
        list.update(cx, |list, cx| list.select_range(&rows[4], cx));
        assert_eq!(selected(&list, cx), rows[1..=4], "from the anchor down, in drawn order");
        list.update(cx, |list, cx| list.select_range(&rows[0], cx));
        assert_eq!(selected(&list, cx), rows[0..=1], "and up");
        list.update(cx, |list, cx| list.clear_selection(cx));
        assert!(selected(&list, cx).is_empty());
        // With nothing clicked the range starts at the row the keys are on.
        let focused = list.read_with(cx, |list, _| list.focused.clone());
        assert_eq!(focused.as_deref(), Some(rows[0].as_str()));
        list.update(cx, |list, cx| list.select_range(&rows[2], cx));
        assert_eq!(selected(&list, cx), rows[0..=2]);
    }

    #[gpui::test]
    fn a_project_name_and_an_unknown_id_are_never_selected(cx: &mut gpui::TestAppContext) {
        let list = cx.new(|cx| JobsList::mock(&filter_jobs(), cx));
        list.update(cx, |list, cx| {
            list.toggle_selected("Armada", cx);
            list.toggle_selected("no-such-job", cx);
        });
        assert!(selected(&list, cx).is_empty());
    }

    #[gpui::test]
    fn shift_down_and_up_select_the_rows_the_keys_move_over_and_give_one_back_on_the_way_back(cx: &mut gpui::TestAppContext) {
        let list = cx.new(|cx| JobsList::mock(&filter_jobs(), cx));
        let rows = drawn_ids(&list, cx);
        list.update(cx, |list, cx| list.extend_selection(1, cx));
        assert_eq!(selected(&list, cx), rows[0..=1], "the row the keys were on, and the one they landed on");
        list.update(cx, |list, cx| list.extend_selection(1, cx));
        assert_eq!(selected(&list, cx), rows[0..=2]);
        assert_eq!(list.read_with(cx, |list, _| list.focused.clone()).as_deref(), Some(rows[2].as_str()));
        list.update(cx, |list, cx| list.extend_selection(-1, cx));
        assert_eq!(selected(&list, cx), rows[0..=1], "back over a selected row gives up the one left");
        // Forward again takes the next row back in.
        list.update(cx, |list, cx| list.extend_selection(1, cx));
        assert_eq!(selected(&list, cx), rows[0..=2]);
    }

    #[gpui::test]
    fn a_selected_job_that_leaves_the_list_leaves_the_selection(cx: &mut gpui::TestAppContext) {
        let list = cx.new(|cx| JobsList::mock(&filter_jobs(), cx));
        list.update(cx, |list, cx| {
            list.toggle_selected("mock-01", cx);
            list.toggle_selected("mock-02", cx);
            list.gone(&["mock-02".to_string()], cx);
        });
        assert_eq!(selected(&list, cx), ["mock-01"]);
    }

    /// A parent the Jobs list sits in, which counts the Esc that reaches it.
    struct Closes {
        list: Entity<JobsList>,
        reached: usize,
    }

    impl gpui::Render for Closes {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().on_action(cx.listener(|this, act: &Act, _, _| this.reached += usize::from(act.is("close")))).child(self.list.clone())
        }
    }

    #[gpui::test]
    fn escape_clears_a_selection_and_goes_on_up_when_there_is_none(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init_app);
        let list = cx.new(|cx| JobsList::mock(&filter_jobs(), cx));
        let (above, cx) = cx.add_window_view(|_, _| Closes { list: list.clone(), reached: 0 });
        let keys = list.read_with(cx, |list, _| list.focus_handle().clone());
        cx.update(|window, cx| window.focus(&keys, cx));
        list.update(cx, |list, cx| list.extend_selection(1, cx));
        assert_eq!(list.read_with(cx, |list, _| list.selected().len()), 2);
        cx.simulate_keystrokes("escape");
        assert!(list.read_with(cx, |list, _| list.selected().is_empty()), "Esc lets go of the selection");
        assert_eq!(above.read_with(cx, |above, _| above.reached), 0, "and goes no further");
        cx.simulate_keystrokes("escape");
        assert_eq!(above.read_with(cx, |above, _| above.reached), 1, "with none selected it is the Esc it was");
    }

    // -- the view button: how the list is grouped, and what it shows --------------------------------------------

    fn attach(job: &mut SessionRecord, kind: &str, state: &str, detail: &str) {
        let raw = format!(r#"{{"kind":"{kind}","target":"41","state":"{state}","detail":{detail},"since":"2026-11-12T09:30:00Z","changed_at":"2026-11-12T09:30:00Z"}}"#);
        job.attachments.push(ipc::decode::<ipc::Attachment>("Attachment", raw.as_bytes()).expect("an attachment"));
    }

    /// The fixture's Live Jobs, none paused, quiet, snoozed, on another Mac or under another, each standing in one state:
    /// Needs you 01, 07, 10; Working 02, 03, 09, 13; Waiting on ci 04; Ready to try 14; Held 06, 11; Idle 08.
    fn standing_jobs() -> Vec<SessionRecord> {
        let mut all: Vec<SessionRecord> = jobs().into_iter().filter(|job| job.state == SessionState::Live && !["mock-05", "mock-12"].contains(&job.id.as_str())).collect();
        for job in &mut all {
            job.attachments.retain(|one| !["started_by", "pr", "replaced_by"].contains(&one.kind.as_str()));
            job.running.clear();
            job.snoozed_until = None;
            job.mac = None;
        }
        // Waiting on ci: idle, with a pull request whose checks still run.
        let idle = all.iter_mut().find(|job| job.id.as_str() == "mock-04").expect("an idle Job");
        attach(idle, "pr", "standing", r#"{"state":"open","checks":"pending"}"#);
        // Ready to try: its one open item is a build to try.
        let ready = all.iter_mut().find(|job| job.id.as_str() == "mock-14").expect("a done Job");
        ready.mark = Some(JobMark { state: JobState::Waiting, lamp: JobLamp::Needs, group: JobGroup::Needs });
        let item = r#"{"id":"try:j","text":"Ready to try","since":"2026-11-09T14:20:00Z","source":"ready_to_try","artifact":"try:j"}"#;
        ready.waiting_for = vec![ipc::decode::<ipc::WaitingItem>("WaitingItem", item.as_bytes()).expect("an item")];
        all
    }

    fn grouped(all: &[SessionRecord], group: Group, order: &[String]) -> Vec<Line> {
        lines_grouped(all, &[], Scope::Active, "", order, &Names::default(), None, &[], group, false)
    }

    /// Each heading with its count, and each Job by id, in the order they are drawn.
    fn layout(lines: &[Line]) -> Vec<String> {
        lines
            .iter()
            .filter_map(|line| match line {
                Line::Group(head) => Some(format!("{} ({})", head.name, head.count)),
                Line::Job(row) => Some(row.id.to_string()),
                Line::Asleep(_) => None,
            })
            .collect()
    }

    /// The Jobs drawn under a heading, down to the next.
    fn under(lines: &[Line], name: &str) -> Vec<String> {
        let mut at = lines.iter().skip_while(|line| !matches!(line, Line::Group(head) if head.name == name)).skip(1);
        std::iter::from_fn(|| match at.next() {
            Some(Line::Job(row)) => Some(Some(row.id.to_string())),
            _ => None,
        })
        .flatten()
        .collect()
    }

    #[test]
    fn by_state_heads_needs_you_then_ready_to_try_then_working_then_ci_with_their_counts_and_the_rest_after() {
        let lines = grouped(&standing_jobs(), Group::State, &[]);
        assert_eq!(headings(&lines), ["Needs you", "Ready to try", "Working", "Waiting on ci", "Held", "Idle"]);
        assert_eq!(
            layout(&lines),
            [
                "Needs you (3)", "mock-07", "mock-01", "mock-10",
                "Ready to try (1)", "mock-14",
                "Working (4)", "mock-02", "mock-03", "mock-09", "mock-13",
                "Waiting on ci (1)", "mock-04",
                "Held (2)", "mock-06", "mock-11",
                "Idle (1)", "mock-08",
            ]
        );
        // A Job with a real question stays under Needs you, though a Ready to try is among its items.
        let mut asking = standing_jobs();
        let item = r#"{"id":"ask:1","text":"May I touch paging.rs?","since":"2026-11-09T14:20:00Z","source":"ask_card"}"#;
        let ready = asking.iter_mut().find(|job| job.id.as_str() == "mock-14").expect("the Ready to try Job");
        ready.waiting_for.push(ipc::decode::<ipc::WaitingItem>("WaitingItem", item.as_bytes()).expect("an item"));
        assert!(under(&grouped(&asking, Group::State, &[]), "Needs you").contains(&"mock-14".to_string()));
    }

    /// Fleet marks a Job whose owed work is all built `Ready`, by lamp or by group, and By state stands it under Ready to try.
    #[test]
    fn a_job_fleet_marks_ready_stands_under_ready_to_try_by_its_lamp_or_its_group() {
        for (state, lamp, group) in [(JobState::Ready, JobLamp::Ready, JobGroup::Ready), (JobState::Working, JobLamp::Ready, JobGroup::Ready), (JobState::Working, JobLamp::Running, JobGroup::Ready)] {
            let mut jobs = standing_jobs();
            let job = jobs.iter_mut().find(|job| job.id.as_str() == "mock-02").expect("a working Job");
            job.waiting_for = Vec::new();
            job.mark = Some(JobMark { state, lamp, group });
            let lines = grouped(&jobs, Group::State, &[]);
            assert!(under(&lines, "Ready to try").contains(&"mock-02".to_string()), "{lamp:?} {group:?}: {:?}", under(&lines, "Ready to try"));
            assert!(!under(&lines, "Needs you").contains(&"mock-02".to_string()), "{lamp:?} {group:?} is not Needs you");
        }
    }

    /// A Job Fleet marks Done stands under its own Done heading after Idle, apart from the quiet Jobs Fleet files in its Done group.
    fn with_one_done() -> Vec<SessionRecord> {
        let mut jobs = standing_jobs();
        let job = jobs.iter_mut().find(|job| job.id.as_str() == "mock-11").expect("a held Job");
        job.mark = Some(JobMark { state: JobState::Done, lamp: JobLamp::Idle, group: JobGroup::Done });
        jobs
    }

    #[test]
    fn a_job_fleet_marks_done_stands_under_its_own_done_heading_after_idle_and_a_quiet_one_stays_idle() {
        let lines = grouped(&with_one_done(), Group::State, &[]);
        assert_eq!(headings(&lines), ["Needs you", "Ready to try", "Working", "Waiting on ci", "Held", "Idle", "Done"]);
        assert_eq!(under(&lines, "Done"), ["mock-11"]);
        assert_eq!(under(&lines, "Held"), ["mock-06"], "the Done Job left the Held group");
        // Fleet's Done group also holds quiet Jobs, whose state is not Done: they stay where they were.
        let mut quiet = standing_jobs();
        let job = quiet.iter_mut().find(|job| job.id.as_str() == "mock-08").expect("an idle Job");
        job.mark = Some(JobMark { state: JobState::Idle, lamp: JobLamp::Idle, group: JobGroup::Done });
        let lines = grouped(&quiet, Group::State, &[]);
        assert!(under(&lines, "Idle").contains(&"mock-08".to_string()));
        assert!(!headings(&lines).contains(&"Done".to_string()));
        // The group is muted, as Idle is.
        assert_eq!(Stand::Done.hue(), Stand::Idle.hue());
    }

    #[test]
    fn the_done_jobs_switch_off_leaves_the_done_jobs_out_of_every_grouping_and_its_heading_with_them() {
        let all = with_one_done();
        for group in [Group::State, Group::Project, Group::List] {
            let shown = lines_grouped(&all, &[], Scope::Active, "", &[], &Names::default(), None, &[], group, false);
            let hidden = lines_grouped(&all, &[], Scope::Active, "", &[], &Names::default(), None, &[], group, true);
            assert!(layout(&shown).contains(&"mock-11".to_string()), "{group:?} lists the Done Job while the switch is on");
            assert!(!layout(&hidden).contains(&"mock-11".to_string()), "{group:?} leaves it out while the switch is off");
            let jobs_of = |lines: &[Line]| layout(lines).into_iter().filter(|one| one.starts_with("mock-")).collect::<Vec<_>>();
            let kept: Vec<String> = jobs_of(&shown).into_iter().filter(|one| one != "mock-11").collect();
            assert_eq!(jobs_of(&hidden), kept, "{group:?} drops that one Job and no other");
        }
        assert!(!headings(&lines_grouped(&all, &[], Scope::Active, "", &[], &Names::default(), None, &[], Group::State, true)).contains(&"Done".to_string()));
    }

    #[test]
    fn the_done_jobs_switch_is_in_the_show_part_before_root_jobs_only_on_by_default_and_kept_between_runs() {
        let toggles = |hide_done: bool| -> Vec<String> {
            view_entries(Scope::Active, Group::State, false, hide_done)
                .into_iter()
                .filter_map(|entry| match entry {
                    Entry::Toggle { label, on, .. } => Some(format!("{label} {on}")),
                    _ => None,
                })
                .collect()
        };
        // Beside Root Jobs only, ahead of it, in the part after the Show choices.
        assert_eq!(toggles(false), ["Done Jobs true", "Root Jobs only false"]);
        assert_eq!(toggles(true), ["Done Jobs false", "Root Jobs only false"]);
        assert!(!Folds::default().hide_done, "Done Jobs are shown until the person turns them off");
        let off = Folds { hide_done: true, ..Folds::default() };
        assert_eq!(Folds::from_text(&off.text()), off);
        assert_eq!(Folds::from_text(&Folds::default().text()), Folds::default());
    }

    #[test]
    fn by_project_keeps_its_project_headings_each_with_the_count_of_its_jobs() {
        let (all, names) = in_projects();
        let lines = lines_grouped(&all, &[], Scope::Active, "", &[], &names, None, &[], Group::Project, false);
        assert_eq!(headings(&lines), ["Armada", "Storefront", "sample-api"]);
        for line in &lines {
            let Line::Group(head) = line else { continue };
            assert_eq!(head.stand, None, "a project's heading stands for no state");
            assert_eq!(head.count, under(&lines, &head.name).len(), "{} counts the Jobs drawn under it", head.name);
        }
    }

    #[test]
    fn one_list_draws_no_headings_and_sorts_every_job_by_title() {
        let all = standing_jobs();
        let lines = grouped(&all, Group::List, &[]);
        assert!(headings(&lines).is_empty(), "{:?}", headings(&lines));
        let mut want: Vec<String> = all.iter().map(|job| title_of(job).to_string()).collect();
        want.sort_by_key(|title| title.to_lowercase());
        assert_eq!(titles(&lines), want);
        // The order the person placed them in does not move a Job in a list sorted by name.
        let placed: Vec<String> = all.iter().rev().map(|job| job.id.as_str().to_string()).collect();
        assert_eq!(titles(&grouped(&all, Group::List, &placed)), want);
    }

    #[test]
    fn a_job_keeps_its_place_in_a_group_as_it_changes_state_and_lands_at_its_own_place_in_the_next() {
        let order: Vec<String> = ["mock-02", "mock-03", "mock-09", "mock-13", "mock-01", "mock-07", "mock-10"].iter().map(|id| id.to_string()).collect();
        let mut all = standing_jobs();
        let drawn = |all: &[SessionRecord]| {
            let lines = grouped(all, Group::State, &order);
            (under(&lines, "Working"), under(&lines, "Needs you"))
        };
        let (working, needs) = drawn(&all);
        assert_eq!(working, ["mock-02", "mock-03", "mock-09", "mock-13"]);
        assert_eq!(needs, ["mock-01", "mock-07", "mock-10"]);
        // The third working Job asks him something: it leaves Working, and none of the others moves.
        let asks = all.iter_mut().find(|job| job.id.as_str() == "mock-09").expect("a working Job");
        let was = asks.mark;
        asks.mark = Some(JobMark { state: JobState::Waiting, lamp: JobLamp::Needs, group: JobGroup::Needs });
        let (working, needs) = drawn(&all);
        assert_eq!(working, ["mock-02", "mock-03", "mock-13"]);
        assert_eq!(needs, ["mock-09", "mock-01", "mock-07", "mock-10"], "it lands before the Jobs placed after it, not at the foot");
        // It goes back to work: back at its own place among the working, the others where they were.
        all.iter_mut().find(|job| job.id.as_str() == "mock-09").expect("a working Job").mark = was;
        let (working, needs) = drawn(&all);
        assert_eq!(working, ["mock-02", "mock-03", "mock-09", "mock-13"]);
        assert_eq!(needs, ["mock-01", "mock-07", "mock-10"]);
    }

    #[test]
    fn the_view_menu_offers_the_groups_then_what_to_show_then_the_root_switch() {
        let words = |entries: Vec<Entry>| -> Vec<String> {
            entries
                .into_iter()
                .map(|entry| match entry {
                    Entry::Label(label) => format!("# {label}"),
                    Entry::Choice { label, on, .. } => format!("{} {label}", if on { "x" } else { " " }),
                    Entry::Toggle { label, on, .. } => format!("{label} {on}"),
                    Entry::Rule => "---".to_string(),
                    _ => "?".to_string(),
                })
                .collect()
        };
        assert_eq!(
            words(view_entries(Scope::Needs, Group::Project, true, false)),
            ["# Group", "  By state", "x By project", "  One list", "---", "# Show", "  All Jobs", "x Needs you only", "  Paused", "  Snoozed", "  Quiet", "  Ended", "---", "Done Jobs true", "Root Jobs only true"]
        );
        assert_eq!(words(view_entries(Scope::Active, Group::State, false, false))[1..2], ["x By state"]);
    }

    #[gpui::test]
    fn a_pick_in_the_view_menu_regroups_or_narrows_the_list(cx: &mut gpui::TestAppContext) {
        let list = cx.new(|cx| JobsList::mock(&standing_jobs(), cx));
        list.update(cx, |list, cx| {
            assert_eq!((list.scope, list.group), (Scope::Active, Group::State), "a list opens All Jobs, by state");
            assert_eq!(headings(&list.lines).first().map(String::as_str), Some("Needs you"));
            list.pick_view("list", cx);
            assert!(headings(&list.lines).is_empty() && list.group == Group::List);
            list.pick_view("project", cx);
            assert_eq!(list.group, Group::Project);
            assert!(!headings(&list.lines).contains(&"Needs you".to_string()));
            list.pick_view("needs", cx);
            assert_eq!(list.scope, Scope::Needs);
            assert!(titles(&list.lines).len() < 12, "Needs you only holds the Jobs that need him");
            list.pick_view("active", cx);
            assert_eq!(list.scope, Scope::Active);
        });
    }

    /// A window that draws the list as the app does: its root clears the labels before each frame.
    struct Hosted(Entity<JobsList>);

    impl gpui::Render for Hosted {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            crate::labels::label_root(div().size_full().child(self.0.clone()))
        }
    }

    fn named(cx: &mut gpui::VisualTestContext, role: Role, name: &str) -> bool {
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), role, name).is_some()
        })
    }

    #[gpui::test]
    fn the_view_button_names_the_view_and_turns_amber_once_the_list_is_narrowed(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init_app);
        let list = cx.new(|cx| JobsList::mock(&standing_jobs(), cx));
        let (_, cx) = cx.add_window_view(|_, _| Hosted(list.clone()));
        assert!(named(cx, Role::Button, "By state"), "the group alone while every Job is shown");
        assert!(named(cx, Role::Heading, "Waiting on ci"), "and the groups under it");
        list.update(cx, |list, cx| list.pick_view("needs", cx));
        assert!(named(cx, Role::Button, "Needs you · By state"));
        assert!(!named(cx, Role::Button, "By state"), "one button, not a scope beside a group");
        list.update(cx, |list, cx| list.pick_view("project", cx));
        assert!(named(cx, Role::Button, "Needs you · By project"));
        assert_eq!(view_label(Scope::Paused, Group::List), "Paused · One list");
        assert_eq!(view_ink(Scope::Active), token::FG_MUTED);
        for (scope, _, _) in SHOWS.iter().skip(1) {
            assert_eq!(view_ink(*scope), token::STATUS_AWAITING_REVIEW, "{scope:?} is amber");
        }
    }

    #[test]
    fn the_group_is_kept_beside_the_scope_and_a_deck_file_from_before_it_still_reads() {
        let order: Vec<String> = ["mock-02", "mock-01"].iter().map(|id| id.to_string()).collect();
        assert_eq!(parse_deck(&deck_text(Scope::Needs, Group::Project, &order)), (Scope::Needs, Group::Project, order.clone()));
        assert_eq!(parse_deck(&deck_text(Scope::Active, Group::List, &[])), (Scope::Active, Group::List, Vec::new()));
        // A file from before the view button: the scope, then a Job id a line. It opens by state, with its order.
        assert_eq!(parse_deck("needs\nmock-02\nmock-01\n"), (Scope::Needs, Group::State, order));
        // Its All scope is not a choice any more: it reads as All Jobs.
        assert_eq!(parse_deck("all\nmock-02").0, Scope::Active);
        assert_eq!(parse_deck(""), (Scope::Active, Group::State, Vec::new()));
        assert_eq!(parse_deck("active\ngroup=no-such-group").1, Group::State, "a group this build does not know opens by state");
    }

    // -- the Cockpit: a Job left for the Operator's page -------------------------------------------------------

    /// Counts the Jobs the list says were left.
    struct Leaves(usize);

    #[gpui::test]
    fn a_job_can_be_left_in_the_cockpit_and_a_lent_one_is_no_job_the_person_opened(cx: &mut gpui::TestAppContext) {
        let all = jobs();
        let (first, second) = (all[0].id.as_str().to_string(), all[1].id.as_str().to_string());
        let list = cx.new(|cx| JobsList::mock(&all, cx));
        let left = cx.new(|_| Leaves(0));
        let counted = left.clone();
        let _subscription = cx.update(|cx| cx.subscribe(&list, move |_, _: &Left, cx| counted.update(cx, |left, _| left.0 += 1)));
        list.update(cx, |list, cx| {
            assert!(list.open_job(&first, cx));
            assert!(!list.closable() && !list.at_home(), "outside the Cockpit there is no home to leave to");
            list.set_cockpit(true);
            assert!(list.closable() && !list.at_home());
            list.open_quietly(None, cx);
            assert!(list.at_home(), "no Job open is the page's");
            list.lend(second.clone().into(), cx);
            assert!(list.at_home() && !list.closable(), "a Job lent to the page for a card leaves the page in front");
            list.open_job(&second, cx);
            assert!(!list.at_home() && list.closable(), "a pick of the very row lent is a real open");
        });
        assert!(list.update(cx, |list, cx| list.close_job(cx)));
        cx.run_until_parked();
        list.read_with(cx, |list, _| assert!(list.opened.is_none() && list.at_home()));
        assert_eq!(left.read_with(cx, |left, _| left.0), 1, "it is said once");
        assert!(!list.update(cx, |list, cx| list.close_job(cx)), "with none open there is nothing to leave");
    }

    /// The Jobs screen as the Cockpit has it: the Operator's page at the right until a Job is opened.
    fn cockpit(cx: &mut gpui::TestAppContext) -> (Entity<JobsScreen>, &mut gpui::VisualTestContext) {
        cx.update(crate::init_app);
        let (screen, cx) = cx.add_window_view(|window, cx| {
            let list = cx.new(|cx| JobsList::mock(&jobs(), cx));
            let page = cx.new(|cx| crate::operator::page::OperatorPageView::mock(window, cx));
            let mut screen = JobsScreen::new(list, None, window, cx);
            screen.set_home(page, cx);
            screen
        });
        redraw(cx);
        (screen, cx)
    }

    fn redraw(cx: &mut gpui::VisualTestContext) {
        for _ in 0..3 {
            cx.run_until_parked();
            cx.update(|window, cx| window.draw(cx).clear(cx));
        }
    }

    const FIRST: &str = "Fix the merge line freshness chip";

    fn row_at(cx: &mut gpui::VisualTestContext, title: &str) -> gpui::Point<gpui::Pixels> {
        let book = cx.update(|window, cx| cx.try_global::<crate::labels::LabelBook>().and_then(|book| book.find(window.window_handle().window_id(), Role::Row, title)).map(|one| one.bounds));
        book.unwrap_or_else(|| panic!("the row {title} is drawn")).center()
    }

    fn at_home(screen: &Entity<JobsScreen>, cx: &mut gpui::VisualTestContext) -> bool {
        screen.read_with(cx, |screen, cx| screen.list.read(cx).at_home())
    }

    #[gpui::test]
    fn a_row_pressed_opens_its_job_over_the_page_and_pressed_again_brings_the_page_back(cx: &mut gpui::TestAppContext) {
        let (screen, cx) = cockpit(cx);
        assert!(at_home(&screen, cx), "no Job is open: the page is at the right");
        let at = row_at(cx, FIRST);
        cx.simulate_click(at, gpui::Modifiers::default());
        redraw(cx);
        assert!(!at_home(&screen, cx), "a press opens the Job over the whole right side");
        assert_eq!(screen.read_with(cx, |screen, cx| screen.list.read(cx).opened_id()).as_deref(), Some("mock-01"));
        let at = row_at(cx, FIRST);
        cx.simulate_click(at, gpui::Modifiers::default());
        redraw(cx);
        assert!(at_home(&screen, cx), "the same row again puts the Operator back");
        let list = screen.read_with(cx, |screen, _| screen.list.clone());
        assert!(list.read_with(cx, |list, _| list.opened.is_none()));
    }

    #[gpui::test]
    fn escape_leaves_an_open_job_for_the_page_and_the_page_has_the_keys(cx: &mut gpui::TestAppContext) {
        let (screen, cx) = cockpit(cx);
        let at = row_at(cx, FIRST);
        cx.simulate_click(at, gpui::Modifiers::default());
        redraw(cx);
        assert!(!at_home(&screen, cx));
        // The keys are in the Job's box, which gives them to the list first, as it does in any Job.
        let mut presses = 0;
        while !at_home(&screen, cx) && presses < 3 {
            cx.simulate_keystrokes("escape");
            redraw(cx);
            presses += 1;
        }
        assert_eq!(presses, 2, "Esc puts the box's keys on the list and Esc again leaves");
        let keys = screen.read_with(cx, |screen, cx| screen.home.as_ref().expect("a home").read(cx).focus.clone());
        assert!(cx.update(|window, _| keys.is_focused(window)), "the keys are on the page");
    }

    #[gpui::test]
    fn escape_shuts_a_rows_menu_before_it_leaves_the_job(cx: &mut gpui::TestAppContext) {
        let (screen, cx) = cockpit(cx);
        let at = row_at(cx, FIRST);
        cx.simulate_click(at, gpui::Modifiers::default());
        redraw(cx);
        let list = screen.read_with(cx, |screen, _| screen.list.clone());
        let keys = list.read_with(cx, |list, _| list.focus_handle().clone());
        cx.update(|window, cx| window.focus(&keys, cx));
        let at = row_at(cx, FIRST);
        cx.simulate_mouse_down(at, gpui::MouseButton::Right, gpui::Modifiers::default());
        cx.simulate_mouse_up(at, gpui::MouseButton::Right, gpui::Modifiers::default());
        redraw(cx);
        assert!(list.read_with(cx, |list, _| list.menu.is_some()), "a right press opens the row's menu");
        cx.simulate_keystrokes("escape");
        redraw(cx);
        assert!(list.read_with(cx, |list, _| list.menu.is_none()), "Esc shuts the menu");
        assert!(!at_home(&screen, cx), "and the Job stays open");
        cx.simulate_keystrokes("escape");
        redraw(cx);
        assert!(at_home(&screen, cx), "Esc again leaves it");
    }

    #[gpui::test]
    fn escape_in_the_search_box_goes_to_the_rows_and_leaves_the_job_for_the_next_one(cx: &mut gpui::TestAppContext) {
        let (screen, cx) = cockpit(cx);
        let at = row_at(cx, FIRST);
        cx.simulate_click(at, gpui::Modifiers::default());
        redraw(cx);
        let list = screen.read_with(cx, |screen, _| screen.list.clone());
        cx.update(|window, cx| list.update(cx, |list, cx| list.to_search(window, cx)));
        redraw(cx);
        cx.simulate_keystrokes("escape");
        redraw(cx);
        assert!(!at_home(&screen, cx), "the search box took the Esc: the Job stays open");
        let keys = list.read_with(cx, |list, _| list.focus_handle().clone());
        assert!(cx.update(|window, _| keys.is_focused(window)), "the rows have the keys");
        cx.simulate_keystrokes("escape");
        redraw(cx);
        assert!(at_home(&screen, cx));
    }
}
