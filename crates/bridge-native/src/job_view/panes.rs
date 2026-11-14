//! The bodies Electron's Job panes hold beside the thread: Plan, Changes, Files and Checks
//! (`job-panes/bodies.tsx`, `ChecksLayer`), and the file and Check panels their rows open. Each body
//! draws on its own, so the pane system mounts it as it is; until that lands, + opens each as a tab of
//! the right column's top pane.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use gpui::{
    anchored, deferred, div, point, prelude::*, px, svg, Anchor, Animation, AnimationExt, AnyElement, Context, Entity, FocusHandle, FontWeight, KeyDownEvent, Role,
    SharedString, Subscription, Task, Window,
};
use gpui_component::input::{Input, InputEvent, InputState};
use ipc::{CheckOutput, PullRequestFilePatches, SessionCheckState, SessionRow, ThreadItem};

use super::head::{below_left, drawn};
use super::plan::task_look as task_state_look;
use super::rows::{age, clock, glyph, mono, tip, Size};
use super::JobView;
use crate::assets::icon_path;
use crate::diff;
use crate::dropdown::Entry;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look, Token};

/// A running Check's log is read again this often, as Electron's panel does.
const EVERY: Duration = Duration::from_secs(2);

/// Menu ids for the Changes picker's sources, by place: a menu id is `'static`.
const SOURCE_IDS: [&str; 16] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15"];

/// What the panes hold between draws, by Job.
#[derive(Default)]
pub(super) struct Panes {
    /// The top pane's tab strip, so a tab added from + scrolls into view.
    pub(super) top_tabs: gpui::ScrollHandle,
    /// Plan groups unfolded, as `{job}/{group}`.
    plan_open: HashSet<String>,
    /// The source each Job's Changes reads, by its value (`useKept("changes:<id>")`).
    changes_from: HashMap<SharedString, String>,
    /// Each pull request's patches as Fleet answered, by `{manifest}#{number}`: kept while it is read again.
    pub(super) patches: HashMap<String, Result<PullRequestFilePatches, String>>,
    /// The tests that reach each pull request's changed files, by `{manifest}#{number}`: None where the map holds no edge.
    pub(super) reach: HashMap<String, Option<super::pr_files::FilesReach>>,
    /// His notes on each pull request's lines as Fleet keeps them, by `{manifest}#{number}`.
    pub(super) notes: HashMap<String, Vec<ipc::PullRequestNote>>,
    /// What each Job's Files search holds (`useKept("files:<id>")`).
    files_query: HashMap<SharedString, String>,
    /// The paths Fleet found, by `{manifest}\0{query}`.
    found: HashMap<String, Vec<String>>,
    /// Folders pressed open or shut, by `{job}\0{query}\0{path}`; a new search starts from the defaults.
    folders: HashMap<String, bool>,
    search: Option<Search>,
    /// The file or Check panel over the Job, while one is open.
    panel: Option<Panel>,
    /// The Jobs whose Ledger lists every command it ran, not the first few.
    pub(super) commands_all: HashSet<SharedString>,
    /// Closed items pressed to reopen, as `{job}\0{item}`, until Fleet's record no longer holds them.
    pub(super) reopened: HashSet<String>,
    /// A pull request's act out with Fleet, by `{job}#{number}`: its button spins and the others wait.
    pub(super) pull_pending: HashMap<String, super::pulls::PullAct>,
    /// The running Ticket lifecycle's states, by repository; none where it has none or the read was refused.
    pub(super) tickets: HashMap<String, Vec<ipc::LifecycleStateDefinition>>,
    /// The System tab's reads and its poll.
    pub(super) systems: super::system::Systems,
    /// The pool the Bays and System tabs draw a Job's bays from, and the clashes unfolded.
    pub(super) bays: super::bays::Bays,
}

struct Search {
    job: SharedString,
    field: Entity<InputState>,
    _typed: Subscription,
}

struct Panel {
    job: SharedString,
    focus: FocusHandle,
    back: Option<FocusHandle>,
    /// The issue a pull request's panel was opened from, which its Back returns to (`useBack`).
    came_from: Option<super::issues::IssueItem>,
    of: Of,
}

enum Of {
    File { path: String, read: Option<FileRead> },
    Check { name: String, run: u64, tab: CheckTab, output: Option<Result<CheckOutput, String>>, _poll: Option<Task<()>> },
    Pull(Box<super::pr_panel::PullPanel>),
    Issue(Box<super::issues::IssuePanel>),
    Bay(Box<super::bays::BayPanel>),
    ClashFile(Box<super::bays::ClashFilePanel>),
    Watch(Box<super::watching::WatchPanel>),
    /// What a rule did on each instance, by its id in the Job's copy.
    Rule { job: SharedString, rule: String },
    /// A mod, by its folder name, opened from a row of the mods a Job made (`ModPanel`).
    Mod { name: String, title: String, kind: ipc::ModKind },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum CheckTab {
    Runs,
    Log,
}

/// A file as the panel reads it: its text, or why it was not opened.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum FileRead {
    Text(String),
    Refused(Refusal),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Refusal {
    NotAddressable,
    TooBig,
    Binary,
    Unreadable,
}

impl Refusal {
    pub fn from_wire(why: &str) -> Self {
        match why {
            "not_addressable" => Refusal::NotAddressable,
            "too_big" => Refusal::TooBig,
            "binary" => Refusal::Binary,
            _ => Refusal::Unreadable,
        }
    }

    /// What a read was refused for, as a fact (`NOT_OPENED`).
    fn said(self) -> &'static str {
        match self {
            Refusal::NotAddressable => "Not a file this panel opens",
            Refusal::TooBig => "Too big to open",
            Refusal::Binary => "Not a text file",
            Refusal::Unreadable => "Could not be read",
        }
    }
}

/// Where Changes can be read from: each pull request the Job opened, then each of a Job it started.
struct Source {
    value: String,
    label: String,
    manifest: Option<String>,
    number: u64,
}

impl Source {
    fn key(&self) -> String {
        format!("{}#{}", self.manifest.as_deref().unwrap_or(""), self.number)
    }
}

/// A plan group's mark, from its tasks (`groupState`).
fn group_state(tasks: &[ipc::SessionPlanTask]) -> ipc::PlanTaskState {
    use ipc::PlanTaskState::*;
    if tasks.iter().all(|task| task.state == Done) {
        Done
    } else if tasks.iter().any(|task| task.state == Failed) {
        Failed
    } else if tasks.iter().any(|task| task.state == Working) {
        Working
    } else if tasks.iter().all(|task| matches!(task.state, Done | HandedIn)) {
        HandedIn
    } else {
        Open
    }
}

/// One entry of the Files tree: a folder holds children, a file none.
#[derive(Debug, PartialEq, Eq)]
struct Node {
    name: String,
    children: Option<Vec<Node>>,
}

/// The paths as a tree, folders before files, each in name order (`treeOf`).
fn tree_of(paths: &[String]) -> Vec<Node> {
    let mut root: Vec<Node> = Vec::new();
    for path in paths {
        let parts: Vec<&str> = path.split('/').collect();
        let mut at = &mut root;
        for (ix, name) in parts.iter().enumerate() {
            let last = ix == parts.len() - 1;
            let found = at.iter().position(|one| one.name == *name && one.children.is_none() == last);
            let place = match found {
                Some(place) => place,
                None => {
                    at.push(Node { name: name.to_string(), children: (!last).then(Vec::new) });
                    at.len() - 1
                }
            };
            match at[place].children.as_mut() {
                Some(children) => at = children,
                None => break,
            }
        }
    }
    fn order(nodes: &mut [Node]) {
        nodes.sort_by(|x, y| y.children.is_some().cmp(&x.children.is_some()).then_with(|| x.name.to_lowercase().cmp(&y.name.to_lowercase())));
        for node in nodes.iter_mut() {
            if let Some(children) = node.children.as_mut() {
                order(children);
            }
        }
    }
    order(&mut root);
    root
}

/// A Check's state as Electron's Checks rows draw it: the glyph before the name, the mark after it,
/// that mark's hue and its word.
fn check_row_look(state: SessionCheckState) -> (&'static str, &'static str, Token, &'static str) {
    match state {
        SessionCheckState::Running => ("loader-circle", "circle-dot", token::STEP_RUNNING, "Running"),
        SessionCheckState::Passed => ("shield-check", "shield-check", token::STEP_ADVANCED, "Passed"),
        SessionCheckState::Failed => ("circle-x", "shield-x", token::STEP_FAILED, "Failed"),
        SessionCheckState::TimedOut => ("circle-x", "shield-x", token::STEP_FAILED, "Timed out"),
    }
}

/// The Check panel's badge for a run: its glyph, its word and the status hue it takes (`CHECK`).
fn check_badge(state: SessionCheckState) -> (&'static str, &'static str, Token, Token) {
    match state {
        SessionCheckState::Running => ("loader-circle", "Running", token::STATUS_RUNNING, token::STATUS_RUNNING_BG),
        SessionCheckState::Passed => ("shield-check", "Passed", token::STATUS_COMPLETED_SUCCESS, token::STATUS_COMPLETED_SUCCESS_BG),
        SessionCheckState::Failed | SessionCheckState::TimedOut => ("shield-x", "Failed", token::STATUS_COMPLETED_FAILED, token::STATUS_COMPLETED_FAILED_BG),
    }
}

/// A 16 px glyph, as the panes' rows draw theirs.
fn glyph16(look: &Look, name: &'static str, ink: gpui::Hsla) -> gpui::Svg {
    svg().path(icon_path(name)).flex_none().size(look.length(token::SPACE_4)).text_color(ink)
}

/// The strip over a body that picks what it shows (`.job-pane__pick`).
fn pick_strip(look: &Look) -> gpui::Div {
    div().flex_none().px(look.length(token::SPACE_3)).py(look.length(token::SPACE_2)).border_b(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE))
}

/// The body under it, which scrolls (`.job-pane__scroll`).
fn scroll(look: &Look, id: impl Into<gpui::ElementId>) -> gpui::Stateful<gpui::Div> {
    div().id(id).flex_1().min_h_0().min_w_0().overflow_y_scroll().p(look.length(token::SPACE_3))
}

impl JobView {
    /// The check rows of the thread held for a Job, newest first.
    pub(super) fn check_rows(&self, id: &str, cx: &gpui::App) -> Vec<SessionRow> {
        let Some(held) = self.thread(id, cx) else { return Vec::new() };
        let mut rows: Vec<SessionRow> = Vec::new();
        for item in &held.items {
            match item {
                ThreadItem::Row { row: row @ SessionRow::Check { .. } } => rows.push(row.clone()),
                ThreadItem::Checks { rows: runs, .. } => rows.extend(runs.iter().filter(|row| matches!(row, SessionRow::Check { .. })).cloned()),
                _ => {}
            }
        }
        rows.reverse();
        rows
    }

    fn sources(&self, id: &str, cx: &gpui::App) -> Vec<Source> {
        let Some(job) = self.record(id, cx) else { return Vec::new() };
        let pulls = |record: &ipc::SessionRecord, from: Option<String>| -> Vec<Source> {
            record
                .attachments
                .iter()
                .filter(|one| matches!(one.kind.as_str(), "pr" | "pull_request"))
                .filter_map(|one| {
                    let number: u64 = one.target.trim_start_matches('#').parse().ok()?;
                    let title = one.detail.get("title").cloned().unwrap_or_default();
                    let (value, label) = match &from {
                        None => (format!("pr:{number}"), format!("#{number} {title}").trim_end().to_string()),
                        Some(job) => (format!("job:{}:{number}", record.id.as_str()), format!("{job} #{number}")),
                    };
                    Some(Source { value, label, manifest: record.manifest_id.as_ref().map(|id| id.as_str().to_string()), number })
                })
                .collect()
        };
        let mut out = pulls(job, None);
        for one in job.attachments.iter().filter(|one| one.kind == "started_to") {
            if let Some(child) = self.record(&one.target, cx) {
                let name = child.title.clone().unwrap_or_else(|| child.id.as_str().to_string());
                out.extend(pulls(child, Some(name)));
            }
        }
        out
    }

    fn source(&self, id: &SharedString, cx: &gpui::App) -> Option<Source> {
        let chosen = self.panes.changes_from.get(id).cloned().unwrap_or_default();
        let mut sources = self.sources(id, cx);
        let at = sources.iter().position(|one| one.value == chosen).unwrap_or(0);
        (at < sources.len()).then(|| sources.swap_remove(at))
    }

    fn files_query(&self, id: &SharedString) -> String {
        self.panes.files_query.get(id).cloned().unwrap_or_default()
    }

    /// Ask for what the top pane's body reads, where it has not been read: a pull request's patches
    /// for Changes, the matching paths for Files. Called where the tab or the Job changes, never while drawing.
    pub(super) fn want_pane_reads(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else {
            self.want_system(window, cx);
            return;
        };
        match self.zone_shown() {
            Some(super::side::Zone::Changes) => self.want_patches(&id, window, cx),
            Some(super::side::Zone::Files) => {
                self.keep_search(&id, window, cx);
                self.want_found(&id, window, cx);
            }
            Some(super::side::Zone::Judge) => self.keep_judge(window, cx),
            Some(super::side::Zone::Ledger) => {
                self.forget_reopened(&id, cx);
                self.judge_read(&id, cx);
            }
            Some(super::side::Zone::Subagents | super::side::Zone::Pulls) => self.judge_read(&id, cx),
            Some(super::side::Zone::Issues) => self.want_tickets(&id, window, cx),
            Some(super::side::Zone::Bays | super::side::Zone::System) => self.read_pool(window, cx),
            _ => {}
        }
        // The System tab's 5 s read runs only while that tab is in front; this stops it otherwise.
        self.want_system(window, cx);
    }

    /// Items pressed to reopen that the record no longer lists are done with: one closed again is its own row.
    fn forget_reopened(&mut self, id: &SharedString, cx: &gpui::App) {
        let Some(record) = self.record(id, cx) else { return };
        let held: HashSet<String> = record.closed.iter().map(|one| format!("{id}\0{}", one.item.id)).collect();
        let prefix = format!("{id}\0");
        self.panes.reopened.retain(|mark| !mark.starts_with(&prefix) || held.contains(mark));
    }

    fn want_patches(&mut self, id: &SharedString, window: &mut Window, cx: &mut Context<Self>) {
        let Some(source) = self.source(id, cx) else { return };
        let key = source.key();
        if self.fleet.is_none() {
            if let Some(patches) = crate::mock::patches(&key) {
                self.panes.patches.insert(key, Ok(patches));
            }
            return;
        }
        let Some(manifest) = source.manifest.clone() else {
            self.panes.patches.insert(key, Err("This Job names no repository to read its changes from.".into()));
            return;
        };
        let path = format!("/pull_request_patches/{}/{}", super::wire::encoded(&manifest), source.number);
        self.get(
            path,
            window,
            cx,
            Box::new(move |this, answer, _, cx| {
                let read = answer.and_then(|bytes| ipc::decode::<PullRequestFilePatches>("PullRequestFilePatches", &bytes).map_err(|why| why.to_string()));
                this.panes.patches.insert(key, read);
                cx.notify();
            }),
        );
    }

    fn want_found(&mut self, id: &SharedString, window: &mut Window, cx: &mut Context<Self>) {
        let query = self.files_query(id);
        let manifest = self.record(id, cx).and_then(|job| job.manifest_id.as_ref().map(|id| id.as_str().to_string())).unwrap_or_default();
        let key = format!("{manifest}\0{query}");
        if self.fleet.is_none() {
            // Mock data: the repository's files are the fixture's, narrowed as Fleet narrows them.
            let lowered = query.to_lowercase();
            let paths = self.composer.catalog.mock_files.iter().filter(|path| path.to_lowercase().contains(&lowered)).take(50).cloned().collect();
            self.panes.found.insert(key, paths);
            return;
        }
        let path = format!("/manifest/files?q={}&manifest_id={}", super::wire::encoded(&query), super::wire::encoded(&manifest));
        self.get(
            path,
            window,
            cx,
            Box::new(move |this, answer, _, cx| {
                if let Ok(found) = answer.and_then(|bytes| ipc::decode::<ipc::FilesFound>("FilesFound", &bytes).map_err(|why| why.to_string())) {
                    this.panes.found.insert(key, found.paths);
                    cx.notify();
                }
            }),
        );
    }

    /// The Files search for the Job on screen, holding what was last typed in it.
    fn keep_search(&mut self, id: &SharedString, window: &mut Window, cx: &mut Context<Self>) {
        if self.panes.search.as_ref().is_some_and(|search| search.job == *id) {
            return;
        }
        let field = cx.new(|cx| InputState::new(window, cx).placeholder("Search files"));
        let held = self.files_query(id);
        if !held.is_empty() {
            field.update(cx, |field, cx| field.set_value(held, window, cx));
        }
        let job = id.clone();
        let typed = cx.subscribe_in(&field, window, move |this, field, event: &InputEvent, window, cx| {
            if matches!(event, InputEvent::Change) {
                let query = field.read(cx).value().to_string();
                this.panes.files_query.insert(job.clone(), query);
                this.want_found(&job.clone(), window, cx);
                cx.notify();
            }
        });
        self.panes.search = Some(Search { job: id.clone(), field, _typed: typed });
    }

    fn pick_source(&mut self, at: Option<gpui::Point<gpui::Pixels>>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let chosen = self.source(&id, cx).map(|one| one.value);
        let sources = self.sources(&id, cx);
        let entries = sources
            .iter()
            .zip(SOURCE_IDS)
            .map(|(one, menu_id)| Entry::Choice { id: menu_id, label: one.label.clone().into(), on: Some(&one.value) == chosen.as_ref() })
            .collect();
        let at = at.unwrap_or_else(|| window.mouse_position());
        let back = window.focused(cx);
        self.open_button_menu("Changes from", entries, at, back, move |this, picked, window, cx| {
            let Some(id) = this.shown.clone() else { return };
            let Some(ix) = SOURCE_IDS.iter().position(|one| *one == picked) else { return };
            if let Some(one) = this.sources(&id, cx).into_iter().nth(ix) {
                this.panes.changes_from.insert(id, one.value);
                this.want_pane_reads(window, cx);
                cx.notify();
            }
        }, window, cx);
    }

    // ---- bodies ------------------------------------------------------------------------------------

    /// Plan: each group a line that unfolds to its tasks (`PlanBody` over `PlanRevisionView`, with no
    /// earlier version, so every group is quiet). No plan, no body.
    pub(super) fn pane_plan(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let plan = self.record(id, cx)?.plan.as_ref()?;
        if plan.groups.is_empty() {
            return None;
        }
        let groups = plan.groups.iter().map(|group| {
            let key = format!("{id}/{}", group.id);
            let open = self.panes.plan_open.contains(&key);
            let (mark, ink, said) = task_state_look(group_state(&group.tasks));
            let row = labelled(
                div()
                    .id(SharedString::from(format!("pane-plan:{}", group.id)))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .px(look.length(token::SPACE_2))
                    .py(look.length(token::SPACE_1))
                    .rounded(look.length(token::RADIUS_MD))
                    .cursor_pointer()
                    .text_color(look.color(token::FG_MUTED))
                    .hover(|style| style.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)))
                    .tooltip(tip(if open { "Hide tasks" } else { "Show tasks" }))
                    .child(glyph(look, if open { "chevron-down" } else { "chevron-right" }, Size::Small, look.color(token::FG_MUTED)))
                    .child(div().id(SharedString::from(format!("pane-plan-mark:{}", group.id))).flex_none().child(glyph(look, mark, Size::Small, look.color(ink))).tooltip(tip(said)))
                    .child(div().flex_1().min_w_0().child(SharedString::from(group.title.clone())))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !this.panes.plan_open.remove(&key) {
                            this.panes.plan_open.insert(key.clone());
                        }
                        cx.notify();
                    })),
                Role::Button,
                group.title.clone(),
            );
            let tasks = open.then(|| {
                div()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .pl(look.length(token::SPACE_6) + look.length(token::SPACE_2))
                    .line_height(look.length(token::LEADING_XS))
                    .children(group.tasks.iter().map(|task| {
                        let (icon, ink, said) = task_state_look(task.state);
                        div()
                            .flex()
                            .items_start()
                            .gap(look.length(token::SPACE_2))
                            .text_color(look.color(token::FG_MUTED))
                            .child(div().id(SharedString::from(format!("pane-task:{}", task.id))).flex_none().pt(look.length(token::SPACE_1) / 2.).child(glyph(look, icon, Size::Small, look.color(ink))).tooltip(tip(said)))
                            .child(div().flex_1().min_w_0().child(SharedString::from(task.text.clone())))
                    }))
            });
            div().flex().flex_col().gap(look.length(token::SPACE_1)).child(row).children(tasks)
        });
        Some(labelled(div().id("pane-plan").flex().flex_col().gap(look.length(token::SPACE_2)).text_size(look.length(token::TEXT_XS)).children(groups), Role::List, "Plan").into_any_element())
    }

    /// Checks: the runs of Checks the agent ran, newest first, each opening its Check (`ChecksLayer`).
    pub(super) fn pane_checks(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> AnyElement {
        let rows = self.check_rows(id, cx);
        let runs = rows.iter().filter_map(|row| {
            let SessionRow::Check { id: row_id, at, name, run, state, .. } = row else { return None };
            let (kind, mark, ink, said) = check_row_look(*state);
            let spinning = *state == SessionCheckState::Running;
            let glyph_drawn: AnyElement = if spinning {
                glyph16(look, kind, look.color(token::FG_MUTED))
                    .with_animation(SharedString::from(format!("pane-check-turn:{row_id}")), Animation::new(look.duration(token::DURATION_SPIN)).repeat(), |mark, delta| {
                        mark.with_transformation(gpui::Transformation::rotate(gpui::percentage(delta)))
                    })
                    .into_any_element()
            } else {
                glyph16(look, kind, look.color(token::FG_MUTED)).into_any_element()
            };
            let (name, run) = (name.clone(), *run);
            let label = format!("{name}, {}", said.to_lowercase());
            Some(labelled(
                div()
                    .id(SharedString::from(format!("pane-check:{row_id}")))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .min_h(look.length(token::H_ROW))
                    .px(look.length(token::SPACE_2))
                    .py(look.length(token::SPACE_1))
                    .rounded(look.length(token::RADIUS_MD))
                    .text_size(look.length(token::TEXT_SM))
                    .cursor_pointer()
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .child(div().flex_none().w(look.length(token::SPACE_6)).flex().justify_center().child(glyph_drawn))
                    .child(div().flex_1().min_w_0().truncate().text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(name.clone())))
                    .child(
                        div()
                            .flex_shrink_0()
                            .min_w_0()
                            .max_w(look.length(token::SPACE_12) * 6.)
                            .truncate()
                            .font_family(mono(look))
                            .text_size(look.length(token::TEXT_2XS))
                            .text_color(look.color(token::FG_SUBTLE))
                            .child(SharedString::from(at.as_str().to_string())),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("pane-check-mark:{row_id}")))
                            .flex_none()
                            .min_w(look.length(token::SPACE_4))
                            .flex()
                            .justify_end()
                            .tooltip(tip(said))
                            .child(glyph(look, mark, Size::Small, look.color(ink))),
                    )
                    .on_click(cx.listener(move |this, _, window, cx| this.open_check(name.clone(), run, window, cx))),
                Role::Button,
                label,
            ))
        });
        div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_4))
            .child(
                div()
                    .px(look.length(token::SPACE_2))
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .text_color(look.color(token::FG_MUTED))
                    .child("The agent that ran each Check gets its result when the run ends."),
            )
            .child(labelled(div().id("pane-checks").flex().flex_col().children(runs), Role::List, "Checks"))
            .into_any_element()
    }

    /// Changes: the picker of pull requests over every file's patch as the forge has it (`ChangesBody`).
    /// No pull request, no body.
    pub(super) fn pane_changes(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let source = self.source(id, cx)?;
        let picker = labelled(
            div()
                .id("changes-from")
                .w_full()
                .h(look.length(token::H_CONTROL))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_SUNKEN))
                .text_size(look.length(token::TEXT_SM))
                .text_color(look.color(token::FG_DEFAULT))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .child(div().flex_1().min_w_0().truncate().child(SharedString::from(source.label.clone())))
                .child(glyph(look, "chevron-down", Size::Small, look.color(token::FG_MUTED)))
                .on_click(cx.listener(|this, _, window, cx| {
                    let at = drawn(window, cx, Role::ComboBox, "Changes from").map(below_left);
                    this.pick_source(at, window, cx);
                })),
            Role::ComboBox,
            "Changes from",
        );
        let body: Option<AnyElement> = match self.panes.patches.get(&source.key()) {
            None => None,
            Some(Err(said)) => Some(div().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(said.clone())).into_any_element()),
            Some(Ok(patches)) => Some(
                div()
                    .flex()
                    .flex_col()
                    .children(patches.files.iter().enumerate().map(|(ix, file)| diff::unified(format!("changes:{}:{ix}", source.key()), &diff::file_diff_of(&file.path, file.patch.as_deref(), file.said.as_deref()), look)))
                    .into_any_element(),
            ),
        };
        Some(div().size_full().flex().flex_col().child(pick_strip(look).child(picker)).child(scroll(look, "pane-changes").children(body)).into_any_element())
    }

    /// Files: the repository's files as a tree, narrowed by what is typed (`FilesBody`). A file opens
    /// read-only in its panel.
    pub(super) fn pane_files(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> AnyElement {
        let query = self.files_query(id);
        let manifest = self.record(id, cx).and_then(|job| job.manifest_id.as_ref().map(|id| id.as_str().to_string())).unwrap_or_default();
        let paths = self.panes.found.get(&format!("{manifest}\0{query}")).cloned().unwrap_or_default();
        let field = self.panes.search.as_ref().filter(|search| search.job == *id).map(|search| {
            labelled(
                div().id("files-search").key_context(crate::keymap::TEXT_BOX).w_full().child(
                    Input::new(&search.field)
                        .h(look.length(token::H_CONTROL))
                        .rounded(look.length(token::RADIUS_MD))
                        .prefix(svg().path(icon_path("search")).size(look.length(token::SPACE_4)).text_color(look.color(token::FG_SUBTLE))),
                ),
                Role::SearchInput,
                "Search files",
            )
        });
        let mut rows: Vec<AnyElement> = Vec::new();
        self.tree_rows(id, &query, &tree_of(&paths), "", 0, look, cx, &mut rows);
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(pick_strip(look).children(field))
            .child(scroll(look, "pane-files").child(labelled(div().id("files-tree").flex().flex_col().py(look.length(token::SPACE_2)).children(rows), Role::Tree, "Files")))
            .into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn tree_rows(&self, id: &SharedString, query: &str, nodes: &[Node], path: &str, depth: usize, look: &Look, cx: &Context<Self>, out: &mut Vec<AnyElement>) {
        for node in nodes {
            let at = format!("{path}{}", node.name);
            let row = div()
                .id(SharedString::from(format!("tree:{at}")))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .min_h(look.length(token::H_CONTROL_SM))
                .pl(look.length(token::SPACE_3) * (depth + 1) as f32)
                .pr(look.length(token::SPACE_2))
                .font_family(mono(look))
                .text_size(look.length(token::TEXT_XS))
                .text_color(look.color(token::FG_MUTED))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)));
            match &node.children {
                None => {
                    let file = at.clone();
                    out.push(
                        labelled(
                            row.child(glyph16(look, "file", look.color(token::FG_MUTED))).child(div().min_w_0().truncate().child(SharedString::from(node.name.clone()))).on_click(cx.listener(move |this, _, window, cx| this.open_file(file.clone(), window, cx))),
                            Role::TreeItem,
                            format!("Open {at}"),
                        )
                        .into_any_element(),
                    );
                }
                Some(children) => {
                    let key = format!("{id}\0{query}\0{at}");
                    let open = self.panes.folders.get(&key).copied().unwrap_or(!query.is_empty() || depth < 2);
                    out.push(
                        labelled(
                            row.child(glyph16(look, if open { "folder-open" } else { "folder" }, look.color(token::FG_MUTED))).child(div().min_w_0().truncate().child(SharedString::from(node.name.clone()))).on_click(cx.listener(move |this, _, _, cx| {
                                this.panes.folders.insert(key.clone(), !open);
                                cx.notify();
                            })),
                            Role::TreeItem,
                            node.name.clone(),
                        )
                        .into_any_element(),
                    );
                    if open {
                        self.tree_rows(id, query, children, &format!("{at}/"), depth + 1, look, cx, out);
                    }
                }
            }
        }
    }

    // ---- panels ------------------------------------------------------------------------------------

    fn open_panel(&mut self, of: Of, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let focus = cx.focus_handle();
        // A panel opened from another, as a pull request from an issue, gives the keys back where the first was opened from.
        let open = self.panes.panel.take();
        // Opened from a panel of another kind, the one it was on is the way back; from one of its own kind, that one's way back stands (`openInPanel`).
        let came_from = match (open.as_ref().map(|open| &open.of), &of) {
            (Some(Of::Issue(issue)), Of::Pull(_)) => Some(issue.item.clone()),
            (Some(Of::Pull(_)), Of::Pull(_)) => open.as_ref().and_then(|open| open.came_from.clone()),
            _ => None,
        };
        let back = match open {
            Some(open) => open.back,
            None => window.focused(cx),
        };
        window.focus(&focus, cx);
        self.panes.panel = Some(Panel { job, focus, back, came_from, of });
        cx.notify();
    }

    /// A file of the Job's repository, read-only, as Fleet reads it (`GET /sessions/repository_file`).
    /// A path a call named may be absolute, `~/` led or the repository's own; Fleet places each.
    pub(super) fn open_file(&mut self, path: String, window: &mut Window, cx: &mut Context<Self>) {
        if self.fleet.is_none() {
            let read = crate::mock::repository_file(&path)
                .map(|read| read.map_or_else(|why| FileRead::Refused(Refusal::from_wire(&why)), FileRead::Text));
            self.open_panel(Of::File { path, read }, window, cx);
            return;
        }
        let Some(job) = self.shown.clone() else { return };
        self.open_panel(Of::File { path: path.clone(), read: None }, window, cx);
        let ask = format!("/sessions/repository_file?session_id={}&path={}", super::wire::encoded(&job), super::wire::encoded(&path));
        let done: super::wire::Done = Box::new(move |this, answer, _, cx| {
            let read = match answer {
                Ok(body) => match ipc::decode::<ipc::RepositoryFile>("RepositoryFile", &body) {
                    Ok(ipc::RepositoryFile::Text { text, .. }) => FileRead::Text(text),
                    Ok(ipc::RepositoryFile::Picture { .. }) => FileRead::Refused(Refusal::Binary),
                    Err(_) => FileRead::Refused(Refusal::Unreadable),
                },
                Err(_) => {
                    let code = this.refused_code.clone().unwrap_or_default();
                    FileRead::Refused(Refusal::from_wire(code.strip_prefix("fleet.repository_file_").unwrap_or(&code)))
                }
            };
            if let Some(Panel { of: Of::File { path: open, read: shown }, .. }) = this.panes.panel.as_mut() {
                if *open == path {
                    *shown = Some(read);
                    cx.notify();
                }
            }
        });
        self.get(ask, window, cx, done);
    }

    /// A file the Job showed, read-only in the side panel, as Fleet keeps it (`GET /sessions/shown`): the path the
    /// Job's ledger holds, which may lie outside its tree.
    pub(super) fn open_shown_file(&mut self, path: String, window: &mut Window, cx: &mut Context<Self>) {
        if self.fleet.is_none() {
            let read = crate::mock::shown_text(&path).map_or(FileRead::Refused(Refusal::Unreadable), FileRead::Text);
            self.open_panel(Of::File { path, read: Some(read) }, window, cx);
            return;
        }
        let Some(job) = self.shown.clone() else { return };
        self.open_panel(Of::File { path: path.clone(), read: None }, window, cx);
        let ask = super::pictures::shown_path(&job, &path);
        let done: super::wire::Done = Box::new(move |this, answer, _, cx| {
            let read = match answer {
                Ok(body) => match ipc::decode::<ipc::ShownText>("ShownText", &body) {
                    Ok(shown) => FileRead::Text(shown.text),
                    Err(_) => FileRead::Refused(Refusal::Unreadable),
                },
                Err(_) => FileRead::Refused(Refusal::Unreadable),
            };
            if let Some(Panel { of: Of::File { path: open, read: shown }, .. }) = this.panes.panel.as_mut() {
                if *open == path {
                    *shown = Some(read);
                    cx.notify();
                }
            }
        });
        self.get(ask, window, cx, done);
    }

    /// A Check the agent ran: its runs in this Job, and the pressed run's log, read again while it runs.
    pub(super) fn open_check(&mut self, name: String, run: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let running = self.check_rows(&job, cx).iter().any(|row| matches!(row, SessionRow::Check { run: one, state: SessionCheckState::Running, .. } if *one == run));
        let (output, poll) = match self.port(cx) {
            None => (crate::mock::check_output(run).map(Ok), None),
            Some(port) => {
                let path = format!("/sessions/checks/{run}/output");
                let poll = cx.spawn(async move |this, cx| loop {
                    let path = path.clone();
                    let answer = cx
                        .background_executor()
                        .spawn(async move {
                            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                            let (status, bytes) = runtime.block_on(crate::transport::get(port, &path))?;
                            if !(200..300).contains(&status) {
                                return Err(super::wire::reason(status, &bytes));
                            }
                            ipc::decode::<CheckOutput>("CheckOutput", &bytes).map_err(|why| why.to_string())
                        })
                        .await;
                    let still = this.update(cx, |this, cx| {
                        let Some(Panel { of: Of::Check { run: open, output, .. }, .. }) = this.panes.panel.as_mut() else { return false };
                        if *open != run {
                            return false;
                        }
                        *output = Some(answer);
                        cx.notify();
                        true
                    });
                    if !still.unwrap_or(false) || !running {
                        return;
                    }
                    cx.background_executor().timer(EVERY).await;
                });
                (None, Some(poll))
            }
        };
        self.open_panel(Of::Check { name, run, tab: CheckTab::Runs, output, _poll: poll }, window, cx);
    }

    /// A pull request's panel, opened from its row.
    pub(super) fn open_pull_panel(&mut self, panel: super::pr_panel::PullPanel, window: &mut Window, cx: &mut Context<Self>) {
        self.open_panel(Of::Pull(Box::new(panel)), window, cx);
    }

    /// An issue's panel, opened from its row.
    pub(super) fn open_issue_panel(&mut self, panel: super::issues::IssuePanel, window: &mut Window, cx: &mut Context<Self>) {
        self.open_panel(Of::Issue(Box::new(panel)), window, cx);
    }

    /// A bay's panel, opened from its Berth.
    pub(super) fn open_bay_panel(&mut self, panel: super::bays::BayPanel, window: &mut Window, cx: &mut Context<Self>) {
        self.open_panel(Of::Bay(Box::new(panel)), window, cx);
    }

    pub(super) fn bay_panel_mut(&mut self) -> Option<&mut super::bays::BayPanel> {
        match self.panes.panel.as_mut() {
            Some(Panel { of: Of::Bay(panel), .. }) => Some(panel),
            _ => None,
        }
    }

    /// A clash's file, opened from its chip.
    pub(super) fn open_clash_panel(&mut self, panel: super::bays::ClashFilePanel, window: &mut Window, cx: &mut Context<Self>) {
        self.open_panel(Of::ClashFile(Box::new(panel)), window, cx);
    }

    pub(super) fn clash_panel_mut(&mut self) -> Option<&mut super::bays::ClashFilePanel> {
        match self.panes.panel.as_mut() {
            Some(Panel { of: Of::ClashFile(panel), .. }) => Some(panel),
            _ => None,
        }
    }

    /// A watch's panel, opened from its Watching row.
    pub(super) fn open_watch(&mut self, panel: super::watching::WatchPanel, window: &mut Window, cx: &mut Context<Self>) {
        self.open_panel(Of::Watch(Box::new(panel)), window, cx);
    }

    /// A rule's acts, opened as a rule with acts Fleet kept is picked (`ActsPanel`).
    pub(super) fn open_rule_panel(&mut self, job: SharedString, rule: String, window: &mut Window, cx: &mut Context<Self>) {
        self.open_panel(Of::Rule { job, rule }, window, cx);
    }

    /// A mod's panel, opened from its row (`openMod`).
    pub(super) fn open_mod(&mut self, name: String, title: String, kind: ipc::ModKind, window: &mut Window, cx: &mut Context<Self>) {
        self.open_panel(Of::Mod { name, title, kind }, window, cx);
    }

    pub(super) fn watch_panel(&self) -> Option<&super::watching::WatchPanel> {
        match self.panes.panel.as_ref() {
            Some(Panel { of: Of::Watch(panel), .. }) => Some(panel),
            _ => None,
        }
    }

    /// The keys go back to the panel itself, as when a pressed control in it goes away.
    pub(super) fn keys_to_panel(&self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(panel) = self.panes.panel.as_ref() {
            window.focus(&panel.focus, cx);
        }
    }

    pub(super) fn watch_panel_mut(&mut self) -> Option<&mut super::watching::WatchPanel> {
        match self.panes.panel.as_mut() {
            Some(Panel { of: Of::Watch(panel), .. }) => Some(panel),
            _ => None,
        }
    }

    /// A log of the Job's, in the file panel. Only the mock reads one: Fleet serves no Job log.
    pub(super) fn open_log(&mut self, path: String, window: &mut Window, cx: &mut Context<Self>) {
        self.open_file(path, window, cx);
    }

    pub(super) fn issue_panel(&self) -> Option<&super::issues::IssuePanel> {
        match self.panes.panel.as_ref() {
            Some(Panel { of: Of::Issue(panel), .. }) => Some(panel),
            _ => None,
        }
    }

    pub(super) fn issue_panel_mut(&mut self) -> Option<&mut super::issues::IssuePanel> {
        match self.panes.panel.as_mut() {
            Some(Panel { of: Of::Issue(panel), .. }) => Some(panel),
            _ => None,
        }
    }

    /// The issue the open panel was opened from, where it was.
    pub(super) fn panel_came_from(&self) -> Option<&super::issues::IssueItem> {
        self.panes.panel.as_ref()?.came_from.as_ref()
    }

    pub(super) fn pull_panel(&self) -> Option<&super::pr_panel::PullPanel> {
        match self.panes.panel.as_ref() {
            Some(Panel { of: Of::Pull(panel), .. }) => Some(panel),
            _ => None,
        }
    }

    pub(super) fn pull_panel_mut(&mut self) -> Option<&mut super::pr_panel::PullPanel> {
        match self.panes.panel.as_mut() {
            Some(Panel { of: Of::Pull(panel), .. }) => Some(panel),
            _ => None,
        }
    }

    /// The open pull request panel with the focus its sheet keeps.
    pub(super) fn pull_panel_focus(&self) -> Option<(FocusHandle, &super::pr_panel::PullPanel)> {
        match self.panes.panel.as_ref() {
            Some(Panel { of: Of::Pull(panel), focus, .. }) => Some((focus.clone(), panel)),
            _ => None,
        }
    }

    pub(super) fn close_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(back) = self.panes.panel.take().and_then(|panel| panel.back) {
            window.focus(&back, cx);
        }
        cx.notify();
    }

    pub(super) fn panel_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        if key == "escape" {
            self.close_panel(window, cx);
            cx.stop_propagation();
        } else if self.pull_panel().is_some() {
            if self.pull_panel_key(event, window, cx) {
                cx.stop_propagation();
            }
        } else if matches!(key, "[" | "]") && !event.keystroke.modifiers.modified() && self.issue_panel().is_some() {
            self.issue_panel_key(cx);
            cx.stop_propagation();
        } else if matches!(key, "[" | "]") && !event.keystroke.modifiers.modified() && self.watch_panel().is_some() {
            self.watch_panel_key(key == "[", window, cx);
            cx.stop_propagation();
        } else if matches!(key, "[" | "]") && !event.keystroke.modifiers.modified() {
            if let Some(Panel { of: Of::Check { tab, .. }, .. }) = self.panes.panel.as_mut() {
                *tab = if *tab == CheckTab::Runs { CheckTab::Log } else { CheckTab::Runs };
                cx.stop_propagation();
                cx.notify();
            }
        }
    }

    /// The panel a pane's row opened: a sheet at the trailing edge, under the title bar, as Electron's
    /// `SidePanel` floats one at `--w-dialog-wide`.
    pub(super) fn render_pane_panel(&self, window: &mut Window, cx: &Context<Self>) -> Option<AnyElement> {
        let panel = self.panes.panel.as_ref()?;
        if matches!(panel.of, Of::Pull(_)) {
            return self.render_pull_panel(window, cx);
        }
        let look = cx.look();
        let look = &look;
        let room = crate::room::of(window, cx);
        let viewport = room.size;
        let (title, subtitle, body): (SharedString, Option<SharedString>, AnyElement) = match &panel.of {
            Of::Pull(_) => return None,
            Of::File { path, read } => {
                let name = path.rsplit('/').next().unwrap_or(path).to_string();
                (name.into(), Some(path.clone().into()), Self::file_body(read.as_ref(), look))
            }
            Of::Check { name, run, tab, output, .. } => (format!("Check {name}").into(), None, Self::check_body(name, *run, *tab, output.as_ref(), &Self::runs_of(self.check_rows(&panel.job, cx), name), look, |which| {
                cx.listener(move |this, _, _, cx| {
                    if let Some(Panel { of: Of::Check { tab, .. }, .. }) = this.panes.panel.as_mut() {
                        *tab = which;
                        cx.notify();
                    }
                })
            })),
            Of::Issue(issue) => (format!("Issue {}", issue.item.label()).into(), issue.item.manifest.as_ref().map(|id| id.as_str().to_string().into()), self.issue_body(issue, look, cx)),
            Of::Bay(bay) => (format!("Bay slot-{}", bay.slot).into(), Some(bay.manifest.as_str().to_string().into()), self.bay_body(bay, look, cx)),
            Of::ClashFile(clash) => {
                let name = clash.path.rsplit('/').next().unwrap_or(&clash.path).to_string();
                (name.into(), Some(clash.path.clone().into()), self.clash_file_body(clash, look, cx))
            }
            Of::Watch(watch) => (format!("Watch {}", watch.name).into(), None, self.watch_body(watch, look, cx)),
            Of::Rule { job, rule } => (format!("Rule: {}", self.rule_label(job, rule, cx)).into(), None, self.rule_body(job, rule, look, cx)),
            Of::Mod { name, title, kind } => (title.clone().into(), None, self.mod_body(name, title, *kind, look, cx)),
        };
        let close = labelled(
            div()
                .id("pane-panel-close")
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .size(look.length(token::H_CONTROL_SM))
                .rounded(look.length(token::RADIUS_MD))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .tooltip(tip("Close \u{b7} Esc"))
                .child(glyph(look, "x", Size::Small, look.color(token::FG_MUTED)))
                .on_click(cx.listener(|this, _, window, cx| this.close_panel(window, cx))),
            Role::Button,
            "Close",
        );
        let head = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_6))
            .py(look.length(token::SPACE_4))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(div().truncate().text_size(look.length(token::TEXT_LG)).line_height(look.length(token::LEADING_LG)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(title.clone()))
                    .children(subtitle.map(|path| div().truncate().font_family(mono(look)).text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(path))),
            )
            .child(close);
        let margin = look.length(token::SPACE_4);
        let wide = look.length(token::W_DIALOG_WIDE).min(viewport.width - margin * 2.);
        let sheet = labelled(
            div()
                .id("pane-panel")
                .track_focus(&panel.focus)
                .capture_key_down(cx.listener(Self::panel_key_down))
                .occlude()
                .w(wide)
                .h(viewport.height - look.length(token::H_TITLE_BAR) - margin * 2.)
                .mt(margin)
                .mr(margin)
                .flex()
                .flex_col()
                .overflow_hidden()
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_OVERLAY))
                .shadow(look.shadows(token::SHADOW_OVERLAY))
                .text_size(look.length(token::TEXT_SM))
                .line_height(look.length(token::LEADING_SM))
                .text_color(look.color(token::FG_DEFAULT))
                .child(head)
                .child(body),
            Role::Dialog,
            title,
        );
        Some(
            deferred(
                anchored().position(room.origin + point(px(0.), look.length(token::H_TITLE_BAR))).anchor(Anchor::TopLeft).child(
                    div()
                        .id("pane-panel-scrim")
                        .occlude()
                        .w(viewport.width)
                        .h(viewport.height - look.length(token::H_TITLE_BAR))
                        .flex()
                        .justify_end()
                        .bg(look.color(token::SCRIM))
                        .on_click(cx.listener(|this, _, window, cx| this.close_panel(window, cx)))
                        .child(sheet),
                ),
            )
            .with_priority(0)
            .into_any_element(),
        )
    }

    /// The file: its lines under their numbers, or why it was not opened (`FileBody`, read-only).
    pub(super) fn file_body(read: Option<&FileRead>, look: &Look) -> AnyElement {
        let body = div().id("file-body").flex_1().min_h_0().overflow_y_scroll();
        match read {
            None => body.into_any_element(),
            Some(FileRead::Refused(why)) => body
                .p(look.length(token::PAD_CARD))
                .child(labelled(
                    div().id("pane-file-refused")
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_1))
                        .p(look.length(token::SPACE_3))
                        .rounded(look.length(token::RADIUS_LG))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::STATUS_ESCALATED))
                        .bg(look.color(token::STATUS_ESCALATED_BG))
                        .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::STATUS_ESCALATED)).child("Not opened"))
                        .child(div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(why.said())),
                    Role::Alert,
                    "Not opened",
                ))
                .into_any_element(),
            Some(FileRead::Text(text)) => {
                let lines: Vec<&str> = text.strip_suffix('\n').unwrap_or(text).split('\n').collect();
                let numbers = lines.iter().enumerate().map(|(ix, _)| div().child(SharedString::from((ix + 1).to_string())));
                let words = lines.iter().map(|line| div().px(look.length(token::SPACE_3)).whitespace_nowrap().child(SharedString::from(line.to_string())));
                body.child(
                    div()
                        .flex()
                        .min_h_full()
                        .font_family(mono(look))
                        .text_size(look.length(token::TEXT_XS))
                        .line_height(look.length(token::LEADING_XS))
                        .child(
                            div()
                                .flex_none()
                                .flex()
                                .flex_col()
                                .items_end()
                                .min_w(look.length(token::SPACE_8))
                                .py(look.length(token::SPACE_2))
                                .pl(look.length(token::SPACE_3))
                                .pr(look.length(token::SPACE_2))
                                .bg(look.color(token::BG_SUNKEN))
                                .text_color(look.color(token::FG_SUBTLE))
                                .children(numbers),
                        )
                        .child(div().id("file-lines").flex_1().min_w_0().overflow_x_scroll().py(look.length(token::SPACE_2)).child(div().flex().flex_col().children(words))),
                )
                .into_any_element()
            }
        }
    }

    /// The runs of the Check named `name`, from a Job's Check rows: run, state and when it started.
    pub(super) fn runs_of(rows: Vec<SessionRow>, name: &str) -> Vec<(u64, SessionCheckState, String)> {
        rows.into_iter()
            .filter_map(|row| match row {
                SessionRow::Check { name: one, run, state, at, .. } if one == name => Some((run, state, at.as_str().to_string())),
                _ => None,
            })
            .collect()
    }

    /// The Check: its name and state, then Runs and Log (`CheckPanel`). `pick` makes the press that shows a tab.
    pub(super) fn check_body<F>(name: &str, run: u64, tab: CheckTab, output: Option<&Result<CheckOutput, String>>, runs: &[(u64, SessionCheckState, String)], look: &Look, pick: impl Fn(CheckTab) -> F) -> AnyElement
    where
        F: Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
    {
        let state = runs.iter().find(|(one, ..)| *one == run).map_or(SessionCheckState::Running, |(_, state, _)| *state);
        let (icon, said, hue, ground) = check_badge(state);
        let badge = labelled(
            div().id("pane-check-badge")
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .h(look.length(token::H_BADGE))
                .px(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_SM))
                .bg(look.color(ground))
                .text_color(look.color(hue))
                .text_size(look.length(token::TEXT_2XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .child(glyph(look, icon, Size::Small, look.color(hue)))
                .child(said),
            Role::Status,
            said,
        );
        let tab_button = |which: CheckTab, word: &'static str| {
            let front = which == tab;
            labelled(
                div()
                    .id(SharedString::from(format!("check-tab:{word}")))
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .text_size(look.length(token::TEXT_SM))
                    .font_weight(FontWeight(look.number(if front { token::WEIGHT_HEADING } else { token::WEIGHT_MEDIUM })))
                    .text_color(look.color(if front { token::FG_DEFAULT } else { token::FG_MUTED }))
                    .border_b(look.length(token::BORDER_WIDTH) * 2.)
                    .border_color(if front { look.color(token::ACCENT) } else { gpui::transparent_black() })
                    .cursor_pointer()
                    .hover(|style| style.text_color(look.color(token::FG_DEFAULT)))
                    .on_click(pick(which))
                    .child(word),
                Role::Tab,
                word,
            )
        };
        let tabs = div().id("pane-check-tabs").flex().border_b(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)).child(tab_button(CheckTab::Runs, "Runs")).child(tab_button(CheckTab::Log, "Log"));
        let now = chrono::Local::now();
        let shown: AnyElement = match tab {
            CheckTab::Runs => labelled(
                div().id("check-runs").flex().flex_col().children(runs.iter().enumerate().map(|(ix, (one, state, at))| {
                    let (_, mark, ink, said) = check_row_look(*state);
                    div()
                        .id(SharedString::from(format!("check-run:{one}")))
                        .flex()
                        .items_start()
                        .gap(look.length(token::SPACE_3))
                        .py(look.length(token::SPACE_2))
                        .when(ix + 1 < runs.len(), |row| row.border_b(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)))
                        .text_size(look.length(token::TEXT_XS))
                        .line_height(look.length(token::LEADING_XS))
                        .text_color(look.color(token::FG_MUTED))
                        .child(div().id(SharedString::from(format!("check-run-mark:{one}"))).flex_none().h(look.length(token::LEADING_XS)).flex().items_center().tooltip(tip(said)).child(glyph(look, mark, Size::Small, look.color(ink))))
                        // The time is what the row is for: it is never cut, and the age drops under it where the row is narrow.
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_wrap()
                                .justify_between()
                                .gap_x(look.length(token::SPACE_3))
                                .child(div().flex_none().whitespace_nowrap().font_family(mono(look)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(clock(at, now).unwrap_or_else(|| at.clone()))))
                                .child(div().flex_none().whitespace_nowrap().child(SharedString::from(age(at, now).unwrap_or_default()))),
                        )
                })),
                Role::List,
                "Runs",
            )
            .into_any_element(),
            CheckTab::Log => {
                let lines = match output {
                    None => String::new(),
                    Some(Ok(output)) => output.lines.join("\n"),
                    Some(Err(said)) => said.clone(),
                };
                labelled(
                    div()
                        .id("check-log")
                        .overflow_x_scroll()
                        .p(look.length(token::SPACE_3))
                        .rounded(look.length(token::RADIUS_SM))
                        .bg(look.color(token::BG_SUNKEN))
                        .font_family(mono(look))
                        .text_size(look.length(token::TEXT_XS))
                        .line_height(look.length(token::LEADING_XS))
                        .text_color(look.color(token::FG_DEFAULT))
                        .child(div().flex().flex_col().children(lines.split('\n').map(|line| div().whitespace_nowrap().child(SharedString::from(line.to_string()))).collect::<Vec<_>>())),
                    Role::Region,
                    format!("Log of run {run}"),
                )
                .into_any_element()
            }
        };
        div()
            .id("check-body")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .p(look.length(token::PAD_CARD))
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .child(div().text_size(look.length(token::TEXT_LG)).line_height(look.length(token::LEADING_LG)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(name.to_string())))
            .child(div().flex().items_center().gap(look.length(token::SPACE_2)).child(badge))
            .child(div().flex().flex_col().gap(look.length(token::SPACE_3)).child(labelled(tabs, Role::TabList, "Check")).child(shown))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(nodes: &[Node]) -> Vec<String> {
        nodes.iter().map(|one| format!("{}{}", one.name, if one.children.is_some() { "/" } else { "" })).collect()
    }

    #[test]
    fn the_tree_puts_folders_first_each_in_name_order() {
        let paths = ["src/b.rs", "README.md", "src/a/x.rs", "Cargo.toml", "src/a.rs"].map(String::from);
        let tree = tree_of(&paths);
        assert_eq!(names(&tree), vec!["src/", "Cargo.toml", "README.md"]);
        assert_eq!(names(tree[0].children.as_ref().unwrap()), vec!["a/", "a.rs", "b.rs"]);
    }

    #[test]
    fn a_group_mark_reads_its_tasks() {
        use ipc::PlanTaskState::*;
        let task = |state| ipc::SessionPlanTask { id: "t".into(), text: "x".into(), state, seen: false, marked: None, stale: None, changed_at: None };
        assert_eq!(group_state(&[task(Done), task(Done)]), Done);
        assert_eq!(group_state(&[task(Done), task(Failed), task(Working)]), Failed);
        assert_eq!(group_state(&[task(Open), task(Working)]), Working);
        assert_eq!(group_state(&[task(Done), task(HandedIn)]), HandedIn);
        assert_eq!(group_state(&[task(Open), task(Done)]), Open);
    }

    #[test]
    fn refusals_read_from_the_wire_and_say_electrons_words() {
        assert_eq!(Refusal::from_wire("too_big").said(), "Too big to open");
        assert_eq!(Refusal::from_wire("binary").said(), "Not a text file");
        assert_eq!(Refusal::from_wire("not_addressable").said(), "Not a file this panel opens");
        assert_eq!(Refusal::from_wire("anything").said(), "Could not be read");
    }
}
