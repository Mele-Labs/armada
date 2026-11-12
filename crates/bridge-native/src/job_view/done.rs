//! Done, as Armada Classic's `StoryColumn` draws it alone in a pane (`bare`): a head with the landed
//! lamp and the By step / By kind switch (⌥L), then what the Job made (`madeOf`, `ledger.ts`). By step
//! lists each finished plan group with its closed issue and merged pull requests under it, then what
//! no group claims, oldest first; By kind lists Jobs, Documents, Pull requests, Issues, Artifacts and
//! Fleet, newest first. A list of more than five keeps the latest five behind Show earlier or Show
//! more. With nothing made only the head is drawn.
//!
//! The view is kept on the machine beside the list's scope, as Classic keeps it in its `localStorage`.
//!
//! **Departures:** a document opens its card as a tab beside the thread, where Classic opens the page
//! in the reading layer, which waits on Chrome; a finished group unfolds to its tasks on a press, as
//! Open's groups do, where Classic lists every group's tasks with Whole plan.
//!
//! Still owed: a workflow's finished steps (`StepBeat`), which arrive with the workflow group; the mods
//! a Job made, which Fleet does not serve; the report row with its conclusion; the lanes of a stack of
//! pull requests.

use std::collections::HashSet;
use std::rc::Rc;

use chrono::Local;
use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString, Window};
use ipc::{AttachmentState, FleetAct, FleetActState, JobState, PlanTaskState, SessionPlanGroup, SessionRecord, SessionRow, WaitingSource};

use super::head::act_button;
use super::issues::{issue_of, job_card, IssueItem};
use super::ledger::{artifact_line, commit_of, Line, Opens};
use super::pulls::{pull_line, pull_of, Pull, PullState};
use super::rows::{age, glyph, tip, Size};
use super::side::{stands, Stands};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look};

/// How Done lists what was made (`LedgerView`).
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum LedgerView {
    #[default]
    Step,
    Kind,
}

/// Where the choice is kept between runs, beside the list's own (`armada.bridge.job-ledger`).
fn ledger_file() -> Option<std::path::PathBuf> {
    crate::jobs::deck_file().map(|deck| deck.with_file_name("native-job-ledger"))
}

/// The choice kept, By step where none is.
pub(super) fn read_ledger_view() -> LedgerView {
    match ledger_file().and_then(|path| std::fs::read_to_string(path).ok()).as_deref().map(str::trim) {
        Some("kind") => LedgerView::Kind,
        _ => LedgerView::Step,
    }
}

fn write_ledger_view(view: LedgerView) {
    let Some(path) = ledger_file() else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, if view == LedgerView::Kind { "kind" } else { "step" });
}

/// The lists By kind makes, in their order (`BUCKETS`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Bucket {
    Job,
    Document,
    PullRequest,
    Issue,
    Artifact,
    Fleet,
}

const BUCKETS: [(Bucket, &str); 6] = [
    (Bucket::Job, "Jobs"),
    (Bucket::Document, "Documents"),
    (Bucket::PullRequest, "Pull requests"),
    (Bucket::Issue, "Issues"),
    (Bucket::Artifact, "Artifacts"),
    (Bucket::Fleet, "Fleet"),
];

/// How many of the latest things made a list shows before he asks for the rest (`RECENT`).
const RECENT: usize = 5;

/// What a test run or a look in a browser writes as it goes, never something made for him (`SCRATCH`).
fn scratch(path: &str) -> bool {
    path.starts_with(".playwright-mcp/") || path.contains("/.playwright-mcp/") || path.starts_with("__screenshots__/") || path.contains("/__screenshots__/") || path.starts_with("/tmp/") || path.starts_with("/private/tmp/")
}

/// How a made thing is drawn: a row with a glyph, or as the ledger draws an issue, a pull request, a
/// Job or what Fleet did.
enum Shape {
    Row(Line),
    Issue(IssueItem),
    Pull(Pull),
    Job(String),
    Fleet(FleetAct),
}

/// One thing the Job made, with the plan group whose issue or pull request it is.
struct Made {
    bucket: Bucket,
    group: Option<String>,
    shape: Shape,
}

fn row(out: &mut Vec<Made>, bucket: Bucket, line: Line) {
    out.push(Made { bucket, group: None, shape: Shape::Row(line) });
}

/// The numbers a comma list names, with no `#`.
fn numbers(said: Option<&String>) -> Vec<String> {
    said.map(|said| said.split(',').map(|one| one.trim().trim_start_matches('#').to_string()).filter(|one| !one.is_empty()).collect()).unwrap_or_default()
}

/// A pull request works an issue where it closes it, or the issue names it (`works`).
pub(super) fn works(pull: &ipc::Attachment, issue: &ipc::Attachment) -> bool {
    let number = pull.target.trim_start_matches('#');
    numbers(pull.detail.get("closes")).iter().any(|one| *one == issue.target) || numbers(issue.detail.get("pull_requests")).iter().any(|one| one == number)
}

/// The plan group whose issue this pull request works.
fn group_of_pull(record: &SessionRecord, groups: &[SessionPlanGroup], pull: &ipc::Attachment) -> Option<String> {
    groups
        .iter()
        .find(|group| group.issue.as_ref().is_some_and(|number| record.attachments.iter().any(|issue| issue.kind == "issue" && issue.target == *number && works(pull, issue))))
        .map(|group| group.id.clone())
}

/// A Job it started that has finished (`finishedChild`).
fn finished(state: JobState) -> bool {
    matches!(state, JobState::Done | JobState::Ended | JobState::Quiet)
}

impl JobView {
    /// What the Job made, in the order Classic's `madeOf` lists it: plan, documents, sketches, pages,
    /// files, commits, walks, issues, pull requests, reported bugs, follow-ups kept, finished Jobs and
    /// what Fleet did; within a kind, oldest first.
    fn made_of(&self, id: &str, cx: &gpui::App) -> Vec<Made> {
        let Some(record) = self.record(id, cx) else { return Vec::new() };
        let plan = record.plan.as_ref().filter(|plan| plan.approved);
        let groups: &[SessionPlanGroup] = plan.map_or(&[], |plan| plan.groups.as_slice());
        let thread = self.thread(id, cx);
        let rows = self.thread_rows(id, cx);
        let mut out = Vec::new();

        if let Some(plan) = plan {
            let card = plan.sketch.as_ref().and_then(|sketch| self.sketch_card(id, &sketch.id, cx));
            let opens = card.map(|card| -> Opens { Rc::new(move |this: &mut JobView, window: &mut Window, cx: &mut Context<JobView>| this.read_sketch(&card, &card, window, cx)) });
            row(&mut out, Bucket::Document, Line { key: "plan".into(), kind: "doc", glyph: "badge-check", said: "Plan", title: "Plan".into(), opens, ..Line::default() });
        }

        let made: Vec<&ipc::Attachment> = record.attachments.iter().filter(|one| one.kind == "artifact").collect();
        let form = |one: &ipc::Attachment| one.detail.get("form").cloned().unwrap_or_default();
        let of_form = |forms: &[&str]| made.iter().copied().filter(|one| forms.contains(&form(one).as_str())).collect::<Vec<_>>();

        // A page whose newest revision waits on his answer is still being made: it stands in Now.
        let waits = |page: &str| record.waiting_for.iter().any(|one| one.source == WaitingSource::Document && one.id.strip_prefix("document:") == Some(page));
        for one in of_form(&["doc", "document"]) {
            let page = one.detail.get("page").cloned().unwrap_or_else(|| one.target.clone());
            if waits(&page) {
                continue;
            }
            let mut line = artifact_line(one, "doc");
            if thread.is_some_and(|held| super::review::document_card(&held.items, &page).is_some()) {
                line.opens = Some(Rc::new(move |this: &mut JobView, window: &mut Window, cx: &mut Context<JobView>| this.open_document_tab(&page, window, cx)));
            }
            row(&mut out, Bucket::Document, line);
        }

        // A sketch opens its drawing where the thread holds its card; the plan's own stands as the plan.
        let plan_sketch = plan.and_then(|plan| plan.sketch.as_ref()).map(|sketch| sketch.id.clone());
        let sketches = record.attachments.iter().filter(|one| one.kind == "sketch").chain(of_form(&["sketch"]));
        for one in sketches {
            let bare = one.target.trim_start_matches("sketch:");
            if plan_sketch.as_deref() == Some(bare) {
                continue;
            }
            let card = self.sketch_card(id, &one.target, cx);
            let opens = card.map(|card| -> Opens { Rc::new(move |this: &mut JobView, window: &mut Window, cx: &mut Context<JobView>| this.read_sketch(&card, &card, window, cx)) });
            let title = one.detail.get("title").filter(|title| !title.is_empty()).cloned().unwrap_or_else(|| bare.to_string());
            row(&mut out, Bucket::Document, Line { key: format!("sketch:{}", one.target), kind: "sketch", glyph: "pencil-ruler", said: "Sketch", title, opens, ..Line::default() });
        }

        for one in of_form(&["page", "window"]) {
            row(&mut out, Bucket::Artifact, artifact_line(one, "page"));
        }
        // Only what the Job wrote as a thing to open: an edit is the commits' and pull requests'.
        for one in of_form(&["file", "image"]) {
            if !scratch(&one.target) {
                row(&mut out, Bucket::Artifact, artifact_line(one, "file"));
            }
        }
        for one in &rows {
            if let SessionRow::Tool { id, at, text, .. } = one {
                if let Some(title) = commit_of(text) {
                    row(&mut out, Bucket::Artifact, Line { key: format!("commit:{id}"), kind: "commit", glyph: "git-commit-horizontal", said: "Commit", title, sub: Some(at.as_str().to_string()), ..Line::default() });
                }
            }
        }
        // A walk ends in his Approve. One shown again stands once, where it was shown last.
        let walks: Vec<(&String, &String)> = rows
            .iter()
            .filter_map(|one| match one {
                SessionRow::Window { id, title, ended_at: Some(_), .. } => Some((id, title)),
                _ => None,
            })
            .collect();
        for (at, (id, title)) in walks.iter().enumerate() {
            if !walks[at + 1..].iter().any(|(_, later)| later == title) {
                row(&mut out, Bucket::Artifact, Line { key: format!("walk:{id}"), kind: "page", glyph: "footprints", said: "Walk approved", title: (*title).clone(), ..Line::default() });
            }
        }

        // An issue is done once it closes and a pull request once it merges; until then they stand in Open.
        for one in record.attachments.iter().filter(|one| one.kind == "issue" && one.state != AttachmentState::GivenBack) {
            let item = issue_of(record, one);
            if item.closed {
                let group = groups.iter().find(|group| group.issue.as_deref() == Some(one.target.as_str())).map(|group| group.id.clone());
                out.push(Made { bucket: Bucket::Issue, group, shape: Shape::Issue(item) });
            }
        }
        for one in &record.attachments {
            if let Some(pull) = pull_of(record, one, None).filter(|pull| pull.state == PullState::Merged) {
                out.push(Made { bucket: Bucket::PullRequest, group: group_of_pull(record, groups, one), shape: Shape::Pull(pull) });
            }
        }
        // The report the Job was started on stands beside its issues.
        for one in record.attachments.iter().filter(|one| one.kind == "report") {
            let title = one.detail.get("title").cloned().unwrap_or_else(|| one.target.clone());
            row(&mut out, Bucket::Issue, Line { key: format!("report:{}", one.target), kind: "issue", glyph: "bug", said: "Reported bug", title, ..Line::default() });
        }
        // A follow-up he marked Deferred is settled; the rest are still to come.
        for later in record.later.iter().filter(|later| later.confirmed) {
            row(&mut out, Bucket::Issue, Line { key: format!("later:{}", later.id), kind: "issue", glyph: "calendar-clock", said: "Deferred", title: later.sentence.clone(), ..Line::default() });
        }
        // A Job it started outside its plan, once finished; one handed a group stands on the group's row.
        for one in record.attachments.iter().filter(|one| one.kind == "started_to" && !one.detail.contains_key("group")) {
            if self.record(&one.target, cx).and_then(|child| child.mark).is_some_and(|mark| finished(mark.state)) {
                out.push(Made { bucket: Bucket::Job, group: None, shape: Shape::Job(one.target.clone()) });
            }
        }
        // What Fleet did for it, oldest first.
        let mut acts: Vec<FleetAct> = thread.map(|held| held.head.fleet_acts.iter().filter(|act| act.state == FleetActState::Over).cloned().collect()).unwrap_or_default();
        acts.sort_by(|a, b| a.at.as_str().cmp(b.at.as_str()));
        out.extend(acts.into_iter().map(|act| Made { bucket: Bucket::Fleet, group: None, shape: Shape::Fleet(act) }));
        out
    }

    /// Done holds only merged pull requests, so it is landed once it holds one and none is still open.
    fn landed(&self, id: &str, made: &[Made], cx: &gpui::App) -> bool {
        let Some(record) = self.record(id, cx) else { return false };
        made.iter().any(|one| matches!(one.shape, Shape::Pull(_))) && !record.attachments.iter().filter_map(|one| pull_of(record, one, None)).any(|pull| pull.state != PullState::Merged)
    }

    /// ⌥L: By step and By kind, in turn.
    pub(super) fn turn_ledger_view(&mut self, cx: &mut Context<Self>) {
        let next = match self.ledger_view {
            LedgerView::Step => LedgerView::Kind,
            LedgerView::Kind => LedgerView::Step,
        };
        self.set_ledger_view(next, cx);
    }

    pub(super) fn set_ledger_view(&mut self, view: LedgerView, cx: &mut Context<Self>) {
        self.ledger_view = view;
        // A mock run keeps nothing, as the list's own store does.
        if self.fleet.is_some() {
            write_ledger_view(view);
        }
        cx.notify();
    }
}

impl JobView {
    /// The Done tab: its head, then By step or By kind.
    pub(super) fn side_done(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> AnyElement {
        let (landed, toggle, body) = self.done_parts(id, look, cx);
        let lamp = landed.then(|| {
            labelled(
                div().id("done-lamp").flex_none().size(look.length(token::SPACE_2)).rounded_full().bg(look.color(token::STEP_ADVANCED)).tooltip(tip("Done")),
                Role::Image,
                "Done",
            )
        });
        let head = div().flex().items_center().gap(look.length(token::SPACE_2)).min_h(look.length(token::H_ROW)).children(lamp).child(div().flex_1()).child(toggle);
        div().flex().flex_col().gap(look.length(token::SPACE_2)).child(head).children(body).into_any_element()
    }

    /// Done's pieces, for its own pane and for the Spine: whether all it made has landed, the By step /
    /// By kind switch, and the list, absent with nothing in it.
    pub(super) fn done_parts(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> (bool, AnyElement, Option<AnyElement>) {
        let made = self.made_of(id, cx);
        let landed = self.landed(id, &made, cx);
        let view = self.ledger_view;
        let choice = |key: &'static str, icon: &'static str, said: &'static str, this: LedgerView| {
            labelled(
                act_button(look, key, icon, "ledger", said.into(), view == this, false).on_click(cx.listener(move |me, _, _, cx| me.set_ledger_view(this, cx))),
                Role::RadioButton,
                said,
            )
        };
        let toggle = labelled(
            div().id("done-view").flex().items_center().gap(look.length(token::SPACE_1)).child(choice("done-by-step", "milestone", "By step", LedgerView::Step)).child(choice("done-by-kind", "tags", "By kind", LedgerView::Kind)),
            Role::RadioGroup,
            "View",
        )
        .into_any_element();
        let body = match view {
            LedgerView::Step => self.done_by_step(id, made, look, cx),
            LedgerView::Kind => self.done_by_kind(made, look, cx),
        };
        (landed, toggle, body)
    }

    /// Each finished plan group with what it produced, then what no group claims, oldest first.
    fn done_by_step(&self, id: &SharedString, made: Vec<Made>, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let record = self.record(id, cx)?;
        let finished: Vec<&SessionPlanGroup> = record.plan.iter().filter(|plan| plan.approved).flat_map(|plan| plan.groups.iter()).filter(|group| stands(group) == Stands::Done).collect();
        let ids: HashSet<&str> = finished.iter().map(|group| group.id.as_str()).collect();
        let (under, loose): (Vec<Made>, Vec<Made>) = made.into_iter().partition(|one| one.group.as_deref().is_some_and(|group| ids.contains(group)));
        if finished.is_empty() && loose.is_empty() {
            return None;
        }
        let mut under = under;
        let groups: Vec<AnyElement> = finished
            .into_iter()
            .map(|group| {
                let (mine, rest): (Vec<Made>, Vec<Made>) = under.drain(..).partition(|one| one.group.as_deref() == Some(group.id.as_str()));
                under = rest;
                self.done_group(record, group, mine, look, cx)
            })
            .collect();
        let loose = (!loose.is_empty()).then(|| self.made_list(loose, Some(true), "done:step".into(), look, cx));
        Some(labelled(div().id("done-by-step").flex().flex_col().gap(look.length(token::SPACE_1)).children(groups).children(loose), Role::List, "By step").into_any_element())
    }

    /// One list for each kind with anything in it, newest first.
    fn done_by_kind(&self, made: Vec<Made>, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        if made.is_empty() {
            return None;
        }
        let mut made = made;
        let sections: Vec<AnyElement> = BUCKETS
            .into_iter()
            .filter_map(|(bucket, label)| {
                let (mut mine, rest): (Vec<Made>, Vec<Made>) = made.drain(..).partition(|one| one.bucket == bucket);
                made = rest;
                if mine.is_empty() {
                    return None;
                }
                mine.reverse();
                Some(labelled(
                    div()
                        .id(SharedString::from(format!("done-kind:{label}")))
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_1))
                        .child(div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(look.color(token::FG_SUBTLE)).child(label))
                        .child(self.made_list(mine, Some(false), format!("done:{label}"), look, cx)),
                    Role::Group,
                    label,
                )
                .into_any_element())
            })
            .collect();
        Some(div().flex().flex_col().gap(look.length(token::SPACE_3)).children(sections).into_any_element())
    }

    /// A finished group (`DoneGroup`): its mark, its title and its tasks as one track, its tasks under
    /// it once unfolded, then the issue it worked and its merged pull requests. A group handed to a Job
    /// is that Job's card instead.
    fn done_group(&self, record: &SessionRecord, group: &SessionPlanGroup, mine: Vec<Made>, look: &Look, cx: &Context<Self>) -> AnyElement {
        let child = record.attachments.iter().find(|one| one.kind == "started_to" && one.detail.get("group") == Some(&group.id)).and_then(|one| self.record(&one.target, cx));
        let open = self.side_unfolded(&group.id);
        let head = match child {
            Some(child) => job_card(child, look, cx),
            None => {
                let key = group.id.clone();
                let track = (!open && !group.tasks.is_empty()).then(|| task_track(&group.id, &group.tasks, look));
                labelled(
                    div()
                        .id(SharedString::from(format!("done-group:{}", group.id)))
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .min_h(look.length(token::H_ROW))
                        .px(look.length(token::SPACE_1))
                        .rounded(look.length(token::RADIUS_MD))
                        .cursor_pointer()
                        .text_size(look.length(token::TEXT_XS))
                        .text_color(look.color(token::FG_DEFAULT))
                        .hover(|style| style.bg(look.color(token::BG_HOVER)))
                        .child(glyph(look, if open { "chevron-down" } else { "chevron-right" }, Size::Small, look.color(token::FG_SUBTLE)))
                        .child(div().id(SharedString::from(format!("done-group-mark:{}", group.id))).flex_none().tooltip(tip("Done")).child(glyph(look, "check", Size::Small, look.color(token::STEP_ADVANCED))))
                        .child(div().flex_1().min_w_0().truncate().child(SharedString::from(group.title.clone())))
                        .children(track)
                        .tab_index(0)
                        .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.side_fold(&key);
                            cx.notify();
                        })),
                    Role::Button,
                    format!("Plan group: {}", group.title),
                )
                .into_any_element()
            }
        };
        let tasks = (open && child.is_none()).then(|| {
            labelled(
                div().id(SharedString::from(format!("done-tasks:{}", group.id))).flex().flex_col().pl(look.length(token::SPACE_6)).children(group.tasks.iter().map(|task| {
                    let (icon, ink, _) = super::plan::task_look(task.state);
                    div().flex().items_start().gap(look.length(token::SPACE_2)).min_h(look.length(token::H_ROW)).py(look.length(token::SPACE_1)).text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(div().flex_none().pt(look.length(token::SPACE_1) / 2.).child(glyph(look, icon, Size::Small, look.color(ink)))).child(div().flex_1().min_w_0().child(SharedString::from(task.text.clone())))
                })),
                Role::List,
                "Tasks",
            )
        });
        let made = (!mine.is_empty()).then(|| div().pl(look.length(token::SPACE_6)).child(self.made_list(mine, Some(true), format!("done:group:{}", group.id), look, cx)));
        div().flex().flex_col().child(head).children(tasks).children(made).into_any_element()
    }

    /// What was made, in the order given. Every pull request stands in one stack, where the first of
    /// them stands. Given which end is newest, more than five keep the latest five behind Show earlier,
    /// above a list that runs oldest first, or Show more, under one that runs newest first.
    fn made_list(&self, list: Vec<Made>, newest: Option<bool>, key: String, look: &Look, cx: &Context<Self>) -> AnyElement {
        let total = list.len();
        let newest_last = newest.unwrap_or(true);
        let cut = newest.is_some() && total > RECENT && !self.side_unfolded(&key);
        let shown: Vec<Made> = match (cut, newest_last) {
            (false, _) => list,
            (true, true) => list.into_iter().skip(total - RECENT).collect(),
            (true, false) => list.into_iter().take(RECENT).collect(),
        };
        let more = cut.then(|| {
            let said = if newest_last { "Show earlier" } else { "Show more" };
            let key = key.clone();
            labelled(
                div()
                    .id(SharedString::from(format!("made-more:{key}")))
                    .self_start()
                    .flex()
                    .items_center()
                    .h(look.length(token::H_CONTROL_SM))
                    .px(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_MD))
                    .text_size(look.length(token::TEXT_XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                    .text_color(look.color(token::FG_MUTED))
                    .cursor_pointer()
                    .hover(|style| style.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)))
                    .child(said)
                    .tab_index(0)
                    .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.side_fold(&key);
                        cx.notify();
                    })),
                Role::Button,
                said,
            )
        });
        let mut pulls: Vec<Pull> = shown.iter().filter_map(|one| if let Shape::Pull(pull) = &one.shape { Some(pull.clone()) } else { None }).collect();
        pulls.sort_by_key(|pull| pull.number);
        let mut stack = (!pulls.is_empty()).then(|| {
            let rows = pulls.into_iter().map(|pull| self.pull_row(pull_line(pull.clone()), pull, "pull_request", look, cx));
            labelled(div().id(SharedString::from(format!("made-pulls:{key}"))).flex().flex_col().children(rows), Role::List, "Pull requests").into_any_element()
        });
        let mut rows: Vec<AnyElement> = Vec::new();
        for one in shown {
            match one.shape {
                Shape::Pull(_) => rows.extend(stack.take()),
                Shape::Row(line) => rows.push(self.made_line(line, look, cx)),
                Shape::Issue(item) => rows.push(self.issue_row(item, look, cx)),
                Shape::Job(child) => rows.extend(self.record(&child, cx).map(|job| job_card(job, look, cx))),
                Shape::Fleet(act) => rows.push(fleet_made(act, look)),
            }
        }
        let (before, after) = if newest_last { (more, None) } else { (None, more) };
        div().flex().flex_col().children(before).children(rows).children(after).into_any_element()
    }

    /// What has not landed (`unlandedOf`): an issue still open and a pull request not merged, unless a
    /// group still to finish works it, where it stands under that group instead.
    pub(super) fn not_landed(&self, job: &SessionRecord, held: &[&SessionPlanGroup], look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let holds = |number: &str| held.iter().any(|group| group.issue.as_deref() == Some(number));
        let held_pull = |pull: &ipc::Attachment| held.iter().any(|group| group.issue.as_ref().is_some_and(|number| job.attachments.iter().any(|issue| issue.kind == "issue" && issue.target == *number && works(pull, issue))));
        let mut list = Vec::new();
        for one in job.attachments.iter().filter(|one| one.kind == "issue" && one.state != AttachmentState::GivenBack) {
            let item = issue_of(job, one);
            if !item.closed && !holds(&one.target) {
                list.push(Made { bucket: Bucket::Issue, group: None, shape: Shape::Issue(item) });
            }
        }
        for one in &job.attachments {
            if let Some(pull) = pull_of(job, one, None).filter(|pull| pull.state != PullState::Merged && !held_pull(one)) {
                list.push(Made { bucket: Bucket::PullRequest, group: None, shape: Shape::Pull(pull) });
            }
        }
        (!list.is_empty()).then(|| labelled(div().id("open-not-landed").child(self.made_list(list, None, "open:not-landed".into(), look, cx)), Role::List, "Not landed").into_any_element())
    }

    /// A made thing that is neither a pull request nor an issue (`MadeLine`): its glyph, its words, and
    /// a press where it opens. A commit says how long ago.
    fn made_line(&self, line: Line, look: &Look, cx: &Context<Self>) -> AnyElement {
        let opens = line.opens.clone();
        let ago = line.key.starts_with("commit:").then(|| line.sub.as_deref().and_then(|at| age(at, Local::now()))).flatten();
        let row = div()
            .id(SharedString::from(format!("made:{}", line.key)))
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .w_full()
            .min_h(look.length(token::H_ROW))
            .px(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .text_color(look.color(if opens.is_some() { token::FG_DEFAULT } else { token::FG_MUTED }))
            .child(labelled(div().id(SharedString::from(format!("made-mark:{}", line.key))).flex_none().w(look.length(token::SPACE_4)).flex().justify_center().tooltip(tip(line.said)).child(glyph(look, line.glyph, Size::Small, look.color(token::FG_MUTED))), Role::Image, line.said))
            .child(div().flex_1().min_w_0().truncate().child(SharedString::from(line.title.clone())))
            .children(ago.map(|ago| div().flex_none().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(ago))))
            .when_some(opens, |row, opens| row.cursor_pointer().tab_index(0).focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT))).hover(|style| style.bg(look.color(token::BG_HOVER))).on_click(cx.listener(move |this, _, window, cx| opens(this, window, cx))));
        labelled(row, Role::Button, line.title).into_any_element()
    }
}

/// What Fleet did for the Job (`FleetMade`): the server glyph, Fleet's words with why as their
/// tooltip, and how long ago.
fn fleet_made(act: FleetAct, look: &Look) -> AnyElement {
    let ago = age(act.at.as_str(), Local::now());
    let words = div().id(SharedString::from(format!("fleet-said:{}", act.id))).flex_1().min_w_0().truncate().child(SharedString::from(act.said.clone()));
    let words = match act.why.clone() {
        Some(why) => words.tooltip(tip(why)),
        None => words,
    };
    labelled(
        div()
            .id(SharedString::from(format!("fleet-made:{}", act.id)))
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .w_full()
            .min_h(look.length(token::H_ROW))
            .px(look.length(token::SPACE_2))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .text_color(look.color(token::FG_DEFAULT))
            .child(labelled(div().id(SharedString::from(format!("fleet-mark:{}", act.id))).flex_none().w(look.length(token::SPACE_4)).flex().justify_center().tooltip(tip("Fleet")).child(glyph(look, "server", Size::Small, look.color(token::FG_MUTED))), Role::Image, "Fleet"))
            .child(words)
            .children(ago.map(|ago| div().flex_none().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(ago)))),
        Role::Group,
        act.said,
    )
    .into_any_element()
}

/// The group's tasks as one track of segments, done and to do (`TaskTrack`).
pub(super) fn task_track(group: &str, tasks: &[ipc::SessionPlanTask], look: &Look) -> AnyElement {
    let many = tasks.len() > 12;
    let said = "Tasks in this group, done and to do";
    labelled(
        div()
            .id(SharedString::from(format!("done-track:{group}")))
            .flex_none()
            .flex()
            .items_center()
            .when(!many, |track| track.gap(look.length(token::BORDER_WIDTH)))
            .w(look.length(token::SPACE_8) + look.length(token::SPACE_2))
            .h(look.length(token::BORDER_WIDTH) * 3.)
            .tooltip(tip(said))
            .children(tasks.iter().map(|task| {
                let ink = match task.state {
                    PlanTaskState::Done | PlanTaskState::HandedIn => token::STEP_ADVANCED,
                    PlanTaskState::Working => token::STEP_RUNNING,
                    PlanTaskState::Failed => token::STEP_FAILED,
                    PlanTaskState::Open => token::BORDER_DEFAULT,
                };
                div().flex_1().min_w_0().h_full().bg(look.color(ink))
            })),
        Role::Image,
        said,
    )
    .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attachment(kind: &str, target: &str, detail: &[(&str, &str)]) -> ipc::Attachment {
        let at = ipc::Instant::carried("2026-11-09T13:00:00Z");
        ipc::Attachment {
            kind: kind.into(),
            manifest_id: None,
            target: target.into(),
            state: AttachmentState::Standing,
            detail: detail.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect(),
            since: at.clone(),
            changed_at: at,
        }
    }

    #[test]
    fn what_a_test_run_or_a_browser_writes_is_scratch() {
        assert!(scratch("/work/slot/.playwright-mcp/shot.png"));
        assert!(scratch("/tmp/chip.png"));
        assert!(scratch("/private/tmp/chip.png"));
        assert!(scratch("src/__screenshots__/chip.png"));
        assert!(!scratch("/work/slot/docs/chip.md"));
    }

    #[test]
    fn a_pull_request_works_an_issue_it_closes_or_the_issue_names() {
        let issue = attachment("issue", "3290", &[]);
        assert!(works(&attachment("pr", "3388", &[("closes", "3290, 3291")]), &issue));
        assert!(works(&attachment("pr", "#3388", &[]), &attachment("issue", "3290", &[("pull_requests", "#3388")])));
        assert!(!works(&attachment("pr", "3391", &[("closes", "3200")]), &issue));
    }
}
