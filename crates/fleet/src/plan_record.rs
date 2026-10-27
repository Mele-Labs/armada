//! A Session's plan: what a recording changes, what the agent's task list and the edits Fleet saw
//! make of it, and how it reads on the wire. `docs/concepts/session.md`, *The plan*.
//!
//! **Pure.** Every instant and row is passed in, so each rule is read and tested without a Fleet.
//! `plan_keeping` is the store glue and `plan_marks` words what the person marked.
//!
//! **Observed decides.** A task's state comes from the agent's own task list, matched to the plan by
//! text, and from edits Fleet saw under its group's paths. A task the list calls completed with no edit
//! seen is `handed_in`, claimed and not observed. Nothing here reads a word of the agent's about
//! progress. **Fleet marks a task itself where it has proof**, a commit naming it, its group's pull
//! request merged, or edits in its group's files with nothing in progress, and that mark stands until
//! the agent's list changes the task. A task in progress whose work looks to be elsewhere is stale
//! (`stale_of`). `.claude/decisions/2026-10-27-fleet-keeps-the-task-list-honest.md`.

use ipc::{
    Instant, PlanChanges, PlanGroupInput, PlanMoved, PlanRemoved, PlanRenamed, PlanSketch, PlanTaskState,
    ReportedTask, ReportedTaskStatus, SessionPlan, SessionPlanGroup, SessionPlanTask, TaskStale,
};
use store::{KeptPlan, KeptPlanChange, KeptPlanGroup, KeptPlanTask, KeptTouch};

/// How many of the agent's own calls without a touch make a task in progress stale (owner, 27 Oct 2026).
pub(crate) const QUIET_CALLS: u32 = 40;

/// A task's text is cut here, as the build record cuts a task list's item.
const TEXT_AT_MOST: usize = 200;

/// Words compared with case and spacing ignored, so a task the agent retyped is the same task.
pub(crate) fn norm(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

pub(crate) fn cut_norm(text: &str) -> String {
    norm(&text.trim().chars().take(TEXT_AT_MOST).collect::<String>())
}

/// The version after `before`, from a plan that passed `RecordPlan::checked`.
///
/// A group or task is matched to one of the version before **by the id the agent sent back, else by
/// its text**; the rest are new and take the next `g<n>` or `t<n>`. An id the agent made up that the
/// version before never held is not an id: the text decides. What differs is kept on the version.
/// A new version is pending, whatever the one before was.
pub(crate) fn recorded(
    before: Option<&KeptPlan>,
    groups: &[PlanGroupInput],
    sketch: Option<(String, String)>,
    walk: Option<String>,
    summary: String,
    now: &str,
) -> KeptPlan {
    let empty = Vec::new();
    let old_groups = before.map_or(&empty, |plan| &plan.groups);
    let (mut next_group, mut next_task) = before.map_or((1, 1), |plan| (plan.next_group, plan.next_task));
    let calls = before.map_or(0, |plan| plan.calls);

    let mut claimed_groups = vec![false; old_groups.len()];
    let mut group_at: Vec<Option<usize>> = vec![None; groups.len()];
    for pass in [Pass::Id, Pass::Text] {
        for (at, group) in groups.iter().enumerate() {
            if group_at[at].is_some() {
                continue;
            }
            let found = old_groups.iter().enumerate().find(|(held, old)| {
                !claimed_groups[*held]
                    && match pass {
                        Pass::Id => group.id.as_deref() == Some(old.id.as_str()),
                        Pass::Text => norm(&group.title) == norm(&old.title),
                    }
            });
            if let Some((held, _)) = found {
                claimed_groups[held] = true;
                group_at[at] = Some(held);
            }
        }
    }

    // Tasks are matched across the whole plan, so one moved to another group is still that task.
    let old_tasks: Vec<(usize, &KeptPlanTask)> =
        old_groups.iter().enumerate().flat_map(|(at, group)| group.tasks.iter().map(move |task| (at, task))).collect();
    let mut claimed_tasks = vec![false; old_tasks.len()];
    let flat: Vec<(usize, &ipc::PlanTaskInput)> =
        groups.iter().enumerate().flat_map(|(at, group)| group.tasks.iter().map(move |task| (at, task))).collect();
    let mut task_at: Vec<Option<usize>> = vec![None; flat.len()];
    for pass in [Pass::Id, Pass::Text] {
        for (place, (_, task)) in flat.iter().enumerate() {
            if task_at[place].is_some() {
                continue;
            }
            let found = old_tasks.iter().enumerate().find(|(held, (_, old))| {
                !claimed_tasks[*held]
                    && match pass {
                        Pass::Id => task.id.as_deref() == Some(old.id.as_str()),
                        Pass::Text => norm(&task.title) == norm(&old.text),
                    }
            });
            if let Some((held, _)) = found {
                claimed_tasks[held] = true;
                task_at[place] = Some(held);
            }
        }
    }

    let mut changes = Vec::new();
    let mut place = 0;
    let mut kept_groups = Vec::with_capacity(groups.len());
    for (at, group) in groups.iter().enumerate() {
        let old = group_at[at].map(|held| &old_groups[held]);
        let id = match old {
            Some(old) => old.id.clone(),
            None => {
                let id = format!("g{next_group}");
                next_group += 1;
                changes.push(KeptPlanChange { kind: "added".into(), id: id.clone(), other: String::new() });
                id
            }
        };
        if let Some(old) = old.filter(|old| norm(&old.title) != norm(&group.title)) {
            changes.push(KeptPlanChange { kind: "renamed".into(), id: id.clone(), other: old.title.clone() });
        }
        let mut tasks = Vec::with_capacity(group.tasks.len());
        for task in &group.tasks {
            let found = task_at[place].map(|held| old_tasks[held]);
            place += 1;
            let kept = match found {
                Some((from, old)) => {
                    if norm(&task.title) != norm(&old.text) {
                        changes.push(KeptPlanChange { kind: "renamed".into(), id: old.id.clone(), other: old.text.clone() });
                    }
                    if group_at[at] != Some(from) {
                        changes.push(KeptPlanChange {
                            kind: "moved".into(),
                            id: old.id.clone(),
                            other: old_groups[from].id.clone(),
                        });
                    }
                    KeptPlanTask {
                        id: old.id.clone(),
                        text: task.title.clone(),
                        listed: old.listed.clone(),
                        completed_at: old.completed_at.clone(),
                        edited: old.edited,
                        started_at: old.started_at.clone(),
                        touched_at: old.touched_at.clone(),
                        touched_call: old.touched_call,
                        marked: old.marked.clone(),
                        marked_why: old.marked_why.clone(),
                        told: old.told,
                    }
                }
                None => {
                    let id = format!("t{next_task}");
                    next_task += 1;
                    changes.push(KeptPlanChange { kind: "added".into(), id: id.clone(), other: String::new() });
                    KeptPlanTask {
                        id,
                        text: task.title.clone(),
                        listed: None,
                        completed_at: None,
                        edited: false,
                        started_at: None,
                        touched_at: None,
                        touched_call: calls,
                        marked: None,
                        marked_why: None,
                        told: false,
                    }
                }
            };
            tasks.push(kept);
        }
        kept_groups.push(KeptPlanGroup {
            id,
            agent: group.agent.clone(),
            title: group.title.clone(),
            gives: group.gives.clone(),
            paths: group.paths.clone(),
            issue: group.issue.clone(),
            worked_at: old.and_then(|old| old.worked_at.clone()),
            tasks,
        });
    }
    // What went. A task of a group that went with it is covered by the group's line.
    for (held, old) in old_groups.iter().enumerate() {
        if !claimed_groups[held] {
            changes.push(KeptPlanChange { kind: "removed".into(), id: old.id.clone(), other: old.title.clone() });
        }
    }
    for (held, (from, old)) in old_tasks.iter().enumerate() {
        if !claimed_tasks[held] && claimed_groups[*from] {
            changes.push(KeptPlanChange {
                kind: "removed".into(),
                id: old_groups[*from].id.clone(),
                other: old.text.clone(),
            });
        }
    }
    KeptPlan {
        version: before.map_or(1, |plan| plan.version + 1),
        summary,
        recorded_at: now.to_string(),
        approved_at: None,
        set_aside_at: None,
        sketch,
        walk,
        changes: before.map(|_| changes),
        next_group,
        next_task,
        calls,
        groups: kept_groups,
    }
}

#[derive(Clone, Copy)]
enum Pass {
    Id,
    Text,
}

/// Take the agent's task list into the plan: a plan task keeps the status of the item of the list that
/// says it, the same words or reworded close enough (see `said`), one item to one task. A task
/// completed now is `edited` if an edit was seen under its group's paths since the group's last
/// completion (or the version was recorded): those completing together share the window. A task that
/// goes back to not completed loses its instant. A task put in progress is stamped, for the store to
/// say which of several began last. A task no item matches keeps what it had. Whether
/// the plan moved.
pub(crate) fn listed(plan: &mut KeptPlan, list: &[ReportedTask], now: &str) -> bool {
    let recorded = plan.recorded_at.clone();
    let calls = plan.calls;
    let mut moved = false;
    let texts: Vec<String> = plan.groups.iter().flat_map(|group| &group.tasks).map(|task| cut_norm(&task.text)).collect();
    let items: Vec<String> = list.iter().map(|item| cut_norm(&item.text)).collect();
    let mut claims = claimed(&texts, &items).into_iter();
    for group in &mut plan.groups {
        let start = group
            .tasks
            .iter()
            .filter_map(|task| task.completed_at.as_deref())
            .chain(std::iter::once(recorded.as_str()))
            .max()
            .unwrap_or_default()
            .to_string();
        let worked = group.worked_at.as_deref().is_some_and(|at| at > start.as_str());
        for task in &mut group.tasks {
            let Some(item) = claims.next().flatten().map(|at| &list[at]) else { continue };
            let status = match item.status {
                ReportedTaskStatus::Pending => "pending",
                ReportedTaskStatus::InProgress => "in_progress",
                ReportedTaskStatus::Completed => "completed",
            };
            if task.listed.as_deref() == Some(status) {
                continue;
            }
            moved = true;
            // The agent changed the task, so its word stands over a mark Fleet set.
            task.marked = None;
            task.marked_why = None;
            touched(task, now, calls);
            task.listed = Some(status.to_string());
            task.started_at = (status == "in_progress").then(|| now.to_string());
            if status == "completed" {
                task.completed_at = Some(now.to_string());
                task.edited = worked;
            } else {
                task.completed_at = None;
                task.edited = false;
            }
        }
    }
    moved
}

/// The plan as the record carries it, with each task in progress that looks left behind read as stale
/// from `touches`, the Job's recent file touches.
pub(crate) fn wire_read(plan: &KeptPlan, touches: &[KeptTouch]) -> SessionPlan {
    let mut wired = wire(plan);
    for (g, group) in wired.groups.iter_mut().enumerate() {
        for (t, task) in group.tasks.iter_mut().enumerate() {
            task.stale = stale_of(plan, (g, t), touches)
                .map(|why| TaskStale { why, since: Instant::carried(touched_since(plan, &plan.groups[g].tasks[t])) });
        }
    }
    wired
}

/// Words too common to say which task a line is.
const FILLER: [&str; 14] = ["a", "an", "the", "to", "of", "in", "on", "for", "and", "with", "it", "is", "by", "at"];

/// A line's words, lowercase, without filler or punctuation.
fn words(text: &str) -> Vec<&str> {
    text.split(|c: char| !c.is_alphanumeric()).filter(|word| !word.is_empty() && !FILLER.contains(word)).collect()
}

/// How well an item says a task, if it does. Both are `cut_norm`ed. The same text is the best; the
/// task's words in a row inside the item's, or the item's inside the task's, come next, the closer in
/// length the better; failing that, 0.6 of the longer line's words shared. Two words at least, so one
/// word that two tasks share never says either.
pub(crate) fn said(task: &str, item: &str) -> Option<f64> {
    if task == item {
        return Some(3.0);
    }
    let (task, item) = (words(task), words(item));
    let (short, long) = if task.len() <= item.len() { (&task, &item) } else { (&item, &task) };
    if short.len() < 2 {
        return None;
    }
    if long.windows(short.len()).any(|run| run == short.as_slice()) {
        return Some(1.0 + short.len() as f64 / long.len() as f64);
    }
    let shared = short.iter().filter(|word| long.contains(word)).count();
    let share = shared as f64 / long.len() as f64;
    (share >= 0.6).then_some(share)
}

/// For each plan task, the index of the list item that claims it. The pairs are taken best first, so
/// an item says at most one task and a task is said by at most one item; ties go to the earlier task,
/// then the earlier item. An item no task wants is left out.
fn claimed(tasks: &[String], items: &[String]) -> Vec<Option<usize>> {
    let mut pairs: Vec<(f64, usize, usize)> = tasks
        .iter()
        .enumerate()
        .flat_map(|(task, text)| items.iter().enumerate().filter_map(move |(item, said_in)| said(text, said_in).map(|score| (score, task, item))))
        .collect();
    pairs.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    let mut claims = vec![None; tasks.len()];
    let mut taken = vec![false; items.len()];
    for (_, task, item) in pairs {
        if claims[task].is_none() && !taken[item] {
            claims[task] = Some(item);
            taken[item] = true;
        }
    }
    claims
}

/// Whether `path` is one of `paths` or lies under it. A group that names none is scoped to the whole
/// repository.
fn under(paths: &[String], path: &str) -> bool {
    paths.is_empty() || paths.iter().any(|one| path == one || path.strip_prefix(one.as_str()).is_some_and(|rest| rest.starts_with('/')))
}

/// What an edit moved in the plan.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Seen {
    /// Whether the edit lay under any group's paths, so the plan is kept again.
    pub moved: bool,
    /// A row would show it: a group worked for the first time, a task that is now solid, or one Fleet
    /// marked.
    pub shown: bool,
}

/// An edit of `path` (from the repository's root), seen now. Every group whose paths hold it is
/// worked, and a task of it that is in progress has an edit seen and is touched, where the group names
/// paths of its own. With nothing in progress anywhere, the first open task of the first such group is
/// marked working. Changes `plan` as the store will.
pub(crate) fn edit_seen(plan: &mut KeptPlan, path: &str, now: &str) -> Seen {
    let mut seen = Seen::default();
    let calls = plan.calls;
    let mut idle = !plan.groups.iter().flat_map(|group| &group.tasks).any(|task| status(task) == Some("in_progress"));
    for group in plan.groups.iter_mut().filter(|group| under(&group.paths, path)) {
        seen.moved = true;
        seen.shown |= group.worked_at.is_none();
        group.worked_at = Some(now.to_string());
        let owned = !group.paths.is_empty();
        for task in group.tasks.iter_mut().filter(|task| status(task) == Some("in_progress")) {
            seen.shown |= !task.edited;
            task.edited = true;
            if owned {
                touched(task, now, calls);
            }
        }
        if !(idle && owned) {
            continue;
        }
        if let Some(task) = group.tasks.iter_mut().find(|task| status(task).is_none_or(|said| said == "pending")) {
            task.marked = Some("in_progress".into());
            task.marked_why = Some(format!("edits in {path}"));
            task.started_at = Some(now.to_string());
            task.edited = true;
            touched(task, now, calls);
            seen.shown = true;
            idle = false;
        }
    }
    seen
}

/// What a task's state is: Fleet's mark where it set one, else what the agent's list last said.
pub(crate) fn status(task: &KeptPlanTask) -> Option<&str> {
    task.marked.as_deref().or(task.listed.as_deref())
}

/// The task was touched now: its mark moved, or work was seen under its group's paths.
fn touched(task: &mut KeptPlanTask, now: &str, calls: u32) {
    task.touched_at = Some(now.to_string());
    task.touched_call = calls;
}

/// Mark the task at `at` completed on Fleet's proof `why`. It is `edited` if an edit was seen under its
/// group's paths since the group's last completion, as a completion the agent's list says is.
fn completed_by_fleet(plan: &mut KeptPlan, at: (usize, usize), why: String, now: &str) {
    let calls = plan.calls;
    let group = &mut plan.groups[at.0];
    let start = group
        .tasks
        .iter()
        .filter_map(|task| task.completed_at.as_deref())
        .chain(std::iter::once(plan.recorded_at.as_str()))
        .max()
        .unwrap_or_default()
        .to_string();
    let worked = group.worked_at.as_deref().is_some_and(|at| at > start.as_str());
    let task = &mut group.tasks[at.1];
    task.marked = Some("completed".into());
    task.marked_why = Some(why);
    task.completed_at = Some(now.to_string());
    task.edited = worked;
    touched(task, now, calls);
}

/// A commit whose first line says a task not yet completed, as an item of the list says one
/// (`said`): that task is completed on its proof. The best said wins, and the earlier task a tie.
/// Whether a task moved.
pub(crate) fn commit_named(plan: &mut KeptPlan, commit: &str, subject: &str, now: &str) -> bool {
    let subject = cut_norm(subject);
    let mut best: Option<(f64, (usize, usize))> = None;
    for (g, group) in plan.groups.iter().enumerate() {
        for (t, task) in group.tasks.iter().enumerate() {
            if status(task) == Some("completed") {
                continue;
            }
            let Some(score) = said(&cut_norm(&task.text), &subject) else { continue };
            if best.is_none_or(|(most, _)| score > most) {
                best = Some((score, (g, t)));
            }
        }
    }
    let Some((_, at)) = best else { return false };
    let why = if commit.is_empty() { "a commit names it".to_string() } else { format!("commit {commit} names it") };
    completed_by_fleet(plan, at, why, now);
    true
}

/// Pull request `number`, opened while task `task` was in progress, merged: every task of that task's
/// group not yet completed is completed on it. The texts of the tasks it moved.
pub(crate) fn merge_marked(plan: &mut KeptPlan, task: &str, number: u64, now: &str) -> Vec<String> {
    let Some(g) = plan.groups.iter().position(|group| group.tasks.iter().any(|one| one.id == task)) else { return Vec::new() };
    let open: Vec<usize> =
        (0..plan.groups[g].tasks.len()).filter(|&t| status(&plan.groups[g].tasks[t]) != Some("completed")).collect();
    for &t in &open {
        completed_by_fleet(plan, (g, t), format!("#{number} merged"), now);
    }
    open.into_iter().map(|t| plan.groups[g].tasks[t].text.clone()).collect()
}

/// Why the task at `at` looks left behind, if it is in progress and does. **A newer task in progress
/// leads**, unless a subagent edited this one's group since it started, as two at work side by side do;
/// then **edits in another group's files** that has nothing in progress, since this was last touched or
/// its own group last edited, where its group names paths of its own; then **`QUIET_CALLS` of the
/// agent's own calls** since it was last touched.
pub(crate) fn stale_of(plan: &KeptPlan, at: (usize, usize), touches: &[KeptTouch]) -> Option<String> {
    let own = &plan.groups[at.0];
    let task = &own.tasks[at.1];
    let working = |one: &KeptPlanTask| status(one) == Some("in_progress");
    if !working(task) {
        return None;
    }
    let owned = !own.paths.is_empty();
    let started = task.started_at.as_deref().unwrap_or_default();
    let newer = plan
        .groups
        .iter()
        .flat_map(|group| &group.tasks)
        .filter(|one| working(one) && one.started_at.as_deref().unwrap_or_default() > started)
        .max_by(|a, b| a.started_at.cmp(&b.started_at));
    if let Some(newer) = newer {
        let alongside = owned && touches.iter().any(|one| one.by.is_some() && one.at.as_str() >= started && under(&own.paths, &one.path));
        if !alongside {
            return Some(format!("\"{}\" started after it", newer.text));
        }
    }
    if owned {
        let since = touched_since(plan, task).max(own.worked_at.as_deref().unwrap_or_default());
        let elsewhere = plan
            .groups
            .iter()
            .enumerate()
            .filter(|(g, group)| *g != at.0 && !group.paths.is_empty() && !group.tasks.iter().any(working))
            .filter_map(|(_, group)| group.worked_at.as_deref().filter(|worked| *worked > since).map(|worked| (worked, group)))
            .max_by(|a, b| a.0.cmp(b.0));
        if let Some((_, group)) = elsewhere {
            return Some(format!("edits went to {}", group.title));
        }
    }
    (plan.calls.saturating_sub(task.touched_call) >= QUIET_CALLS).then(|| format!("{QUIET_CALLS} calls with no update"))
}

/// When a task was last touched, else started, else when the plan was recorded.
fn touched_since<'a>(plan: &'a KeptPlan, task: &'a KeptPlanTask) -> &'a str {
    task.touched_at.as_deref().or(task.started_at.as_deref()).unwrap_or(plan.recorded_at.as_str())
}

/// The line the agent is told for a task Fleet marked or saw go stale, if it has not been told of it.
fn told_line(plan: &KeptPlan, at: (usize, usize), touches: &[KeptTouch]) -> Option<String> {
    let task = &plan.groups[at.0].tasks[at.1];
    if task.told {
        return None;
    }
    if let (Some(state), Some(why)) = (task.marked.as_deref(), task.marked_why.as_deref()) {
        let state = if state == "completed" { "completed" } else { "in progress" };
        return Some(format!("- \"{}\" is {state}: {why}.", task.text));
    }
    stale_of(plan, at, touches).map(|why| format!("- \"{}\" is in progress, but {why}.", task.text))
}

/// What the agent is told of the tasks Fleet marked or saw go stale that it has not been told of, each
/// marked told: **once a task**, however long it stays so. Absent where there is none.
pub(crate) fn untold(plan: &mut KeptPlan, touches: &[KeptTouch]) -> Option<String> {
    let mut lines = Vec::new();
    for g in 0..plan.groups.len() {
        for t in 0..plan.groups[g].tasks.len() {
            if let Some(line) = told_line(plan, (g, t), touches) {
                lines.push(line);
                plan.groups[g].tasks[t].told = true;
            }
        }
    }
    (!lines.is_empty()).then(|| {
        format!(
            "Fleet set these tasks of your plan from what it saw. Update your task list to match with TaskUpdate, \
             or change the task there if Fleet has it wrong:\n{}",
            lines.join("\n")
        )
    })
}

/// A task's state and whether an edit was seen for it.
pub(crate) fn state_of(task: &KeptPlanTask) -> (PlanTaskState, bool) {
    match status(task) {
        Some("completed") if task.edited => (PlanTaskState::Done, true),
        Some("completed") => (PlanTaskState::HandedIn, false),
        Some("in_progress") => (PlanTaskState::Working, task.edited),
        _ => (PlanTaskState::Open, false),
    }
}

/// The plan as the record carries it.
pub(crate) fn wire(plan: &KeptPlan) -> SessionPlan {
    let mut scope: Vec<String> = Vec::new();
    for path in plan.groups.iter().flat_map(|group| group.paths.iter()) {
        if !scope.contains(path) {
            scope.push(path.clone());
        }
    }
    let held: &[KeptPlanChange] = plan.changes.as_deref().unwrap_or_default();
    let renamed = |kind: &'static str| held.iter().filter(move |one| one.kind == kind);
    SessionPlan {
        version: plan.version,
        summary: plan.summary.clone(),
        recorded_at: Instant::carried(&plan.recorded_at),
        approved: plan.approved_at.is_some(),
        approved_at: plan.approved_at.as_deref().map(Instant::carried),
        set_aside_at: plan.set_aside_at.as_deref().map(Instant::carried),
        scope,
        groups: plan
            .groups
            .iter()
            .map(|group| SessionPlanGroup {
                id: group.id.clone(),
                agent: group.agent.clone(),
                title: group.title.clone(),
                gives: group.gives.clone(),
                paths: group.paths.clone(),
                issue: group.issue.clone(),
                tasks: group
                    .tasks
                    .iter()
                    .map(|task| {
                        let (state, seen) = state_of(task);
                        SessionPlanTask {
                            id: task.id.clone(),
                            text: task.text.clone(),
                            state,
                            seen,
                            marked: task.marked_why.clone(),
                            stale: None,
                            changed_at: match state {
                                PlanTaskState::Working => task.started_at.as_deref().map(Instant::carried),
                                PlanTaskState::Open => None,
                                _ => task.completed_at.as_deref().map(Instant::carried),
                            },
                        }
                    })
                    .collect(),
                worked_at: group.worked_at.as_deref().map(Instant::carried),
            })
            .collect(),
        changes: plan.changes.as_ref().map(|_| PlanChanges {
            added: renamed("added").map(|one| one.id.clone()).collect(),
            renamed: renamed("renamed").map(|one| PlanRenamed { id: one.id.clone(), from: one.other.clone() }).collect(),
            moved: renamed("moved").map(|one| PlanMoved { id: one.id.clone(), from: one.other.clone() }).collect(),
            removed: renamed("removed").map(|one| PlanRemoved { group: one.id.clone(), text: one.other.clone() }).collect(),
        }),
        sketch: plan.sketch.as_ref().map(|(id, title)| PlanSketch { id: id.clone(), title: title.clone() }),
        held_by: Vec::new(),
        adversarial: None,
    }
}
