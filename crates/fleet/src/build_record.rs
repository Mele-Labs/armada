//! A Session's build record: what it has built, read off what Fleet observed.
//! `docs/concepts/session.md`, *The build record*.
//!
//! **Pure.** Every instant and every row is passed in, so each rule is read and tested without a Fleet.
//! `build_noting` keeps the facts and `build_reading` takes them back out of the store; this is the
//! rules between the two.
//!
//! **Observed decides, and the agent's task list only adds detail.** An edit Fleet saw is what makes
//! a completed task solid; a completed task with no edit seen is hollow, claimed and not observed.
//! With no task list there are no pieces: progress is never invented.

use ipc::{
    BuildPhase, BuildPiece, BuildPieceKind, BuildStop, BuildStopId, Instant, ReportedTask, ReportedTaskStatus,
    SessionBuild,
};
use store::{AttachmentState, KeptAttachment, KeptBuild, KeptTask, RoadTimes, WorkTimes};

/// How many of a repository's recent landed pull requests its forecast reads.
pub(crate) const FORECAST_WINDOW: u32 = 10;

/// A task's text is cut here, as the hosted gate cuts a `TodoWrite` item.
pub(crate) const TASK_TEXT_AT_MOST: usize = 200;

/// A tool call this long after the last one is worth a published row, so `quiet_since` stays within a
/// minute of the truth without a row going out for every call.
pub(crate) const QUIET_REFRESH_MS: i64 = 60_000;

/// Later than any instant Fleet writes, for a bound that should match nothing.
pub(crate) const NEVER: &str = "9999-12-31T23:59:59.999Z";

/// The stops in the order the road usually runs, which settles two reached in the same instant.
const ORDER: [BuildStopId; 5] =
    [BuildStopId::Walk, BuildStopId::Pushed, BuildStopId::Pr, BuildStopId::Checks, BuildStopId::AutoMerge];

/// What Fleet heard that bears on a build.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Observed {
    /// A file written.
    Edited,
    /// A file read or a command run: the Session is working, and no more is known.
    Worked,
    /// A commit or a push. Code changed, even where a script made the edits through a shell, which is
    /// never heard as an edit, so this sets the first edit where none was heard.
    Committed,
    /// The agent's whole task list as it now stands.
    Tasks(Vec<ReportedTask>),
    /// The agent left plan mode.
    PlanLeft,
}

impl Observed {
    /// Whether this is work that starts a new build once the last one landed. A read or a command after
    /// the merge is the Session tidying up.
    pub(crate) fn begins_work(&self) -> bool {
        !matches!(self, Observed::Worked | Observed::Committed)
    }
}

/// The rows after taking in one thing heard.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Taken {
    pub build: KeptBuild,
    pub tasks: Vec<KeptTask>,
    /// What the row shows moved, so it is worth publishing: a first edit, scaffold going up, a task
    /// list that changed, or a first call after a minute's quiet.
    pub visible: bool,
}

fn millis(at: &str) -> Option<i64> {
    core_model::Timestamp::from_rfc3339(at).epoch_millis()
}

/// The instant a task completing now is measured from: the latest completion already kept, else the
/// list's first write.
fn window_start<'a>(build: &'a KeptBuild, tasks: &'a [KeptTask]) -> Option<&'a str> {
    tasks
        .iter()
        .filter_map(|task| task.completed_at.as_deref())
        .max()
        .or(build.tasks_at.as_deref())
}

/// Edits have landed since the last completion (or the list's first write): scaffold is up.
fn scaffolded(build: &KeptBuild, tasks: &[KeptTask]) -> bool {
    match (build.last_edit_at.as_deref(), window_start(build, tasks)) {
        (Some(edit), Some(start)) => edit > start,
        _ => false,
    }
}

fn status_text(status: ReportedTaskStatus) -> &'static str {
    match status {
        ReportedTaskStatus::Pending => "pending",
        ReportedTaskStatus::InProgress => "in_progress",
        ReportedTaskStatus::Completed => "completed",
    }
}

fn cut(text: &str) -> String {
    text.trim().chars().take(TASK_TEXT_AT_MOST).collect()
}

/// The row with what the timeline kept of the Session's work before Fleet heard any of it filled in,
/// for a Session whose row is new or has no first edit: Fleet restarted, or the work was heard by an
/// earlier Fleet. The first edit is the earliest edit, commit or push; the last edit is the last edit
/// entry. A row that already has a first edit is not touched, and none with nothing kept is made.
pub(crate) fn seeded(build: Option<KeptBuild>, work: &WorkTimes) -> Option<KeptBuild> {
    let first = [work.edited.as_ref().map(|(first, _)| first.as_str()), work.first_commit.as_deref(), work.first_push.as_deref()]
        .into_iter()
        .flatten()
        .min();
    let Some(first) = first else { return build };
    let mut build = build.unwrap_or_else(|| KeptBuild {
        begun_at: first.to_string(),
        first_edit_at: None,
        last_edit_at: None,
        last_tool_at: None,
        plan_left_at: None,
        tasks_at: None,
    });
    if build.first_edit_at.is_some() {
        return Some(build);
    }
    build.first_edit_at = Some(first.to_string());
    build.last_edit_at = work.edited.as_ref().map(|(_, last)| last.clone());
    if first < build.begun_at.as_str() {
        build.begun_at = first.to_string();
    }
    Some(build)
}

/// Take one thing heard into the rows. `build` is `None` for a Session that has done nothing yet.
pub(crate) fn taken(build: Option<KeptBuild>, tasks: Vec<KeptTask>, heard: &Observed, now: &str) -> Taken {
    let mut build = build.unwrap_or_else(|| KeptBuild {
        begun_at: now.to_string(),
        first_edit_at: None,
        last_edit_at: None,
        last_tool_at: None,
        plan_left_at: None,
        tasks_at: None,
    });
    let before = (build.first_edit_at.is_some(), scaffolded(&build, &tasks));
    let was_quiet = build
        .last_tool_at
        .as_deref()
        .and_then(millis)
        .zip(millis(now))
        .is_none_or(|(last, now)| now - last >= QUIET_REFRESH_MS);
    let mut tasks = tasks;
    let mut visible = false;
    match heard {
        Observed::Edited => {
            build.first_edit_at.get_or_insert_with(|| now.to_string());
            build.last_edit_at = Some(now.to_string());
        }
        Observed::Worked => {}
        Observed::Committed => {
            build.first_edit_at.get_or_insert_with(|| now.to_string());
        }
        Observed::PlanLeft => {
            build.plan_left_at = Some(now.to_string());
            visible = true;
        }
        Observed::Tasks(list) => {
            let (merged, changed) = merged_tasks(&build, &tasks, list, now);
            build.tasks_at = match (merged.is_empty(), build.tasks_at.take()) {
                (true, _) => None,
                (false, Some(first)) => Some(first),
                (false, None) => Some(now.to_string()),
            };
            tasks = merged;
            visible = changed;
        }
    }
    build.last_tool_at = Some(now.to_string());
    let after = (build.first_edit_at.is_some(), scaffolded(&build, &tasks));
    Taken { build, tasks, visible: visible || before != after || was_quiet }
}

/// The kept list after the agent wrote `list`, and whether it differs from what was kept.
///
/// An item is matched to a kept one by its id and text, else by its text alone, so a list reordered or
/// added to keeps what Fleet saw of each. A match already completed and still completed keeps its
/// instant. One newly completed takes `now`, and is edited if an edit was seen after the last
/// completion that stands (or the list's first write): those completing together share the window.
fn merged_tasks(build: &KeptBuild, kept: &[KeptTask], list: &[ReportedTask], now: &str) -> (Vec<KeptTask>, bool) {
    let mut taken = vec![false; kept.len()];
    let mut matches: Vec<Option<usize>> = vec![None; list.len()];
    for (place, item) in list.iter().enumerate() {
        let text = cut(&item.text);
        let found = kept.iter().enumerate().find(|(at, one)| !taken[*at] && one.id == item.id && one.text == text);
        if let Some((at, _)) = found {
            taken[at] = true;
            matches[place] = Some(at);
        }
    }
    for (place, item) in list.iter().enumerate() {
        if matches[place].is_some() {
            continue;
        }
        let text = cut(&item.text);
        if let Some((at, _)) = kept.iter().enumerate().find(|(at, one)| !taken[*at] && one.text == text) {
            taken[at] = true;
            matches[place] = Some(at);
        }
    }
    let completed = |item: &ReportedTask| item.status == ReportedTaskStatus::Completed;
    // Completions that stand, which the newly completed are measured from.
    let carried: Vec<&str> = list
        .iter()
        .zip(&matches)
        .filter_map(|(item, at)| {
            let was = kept.get((*at)?)?;
            (completed(item) && was.completed_at.is_some()).then(|| was.completed_at.as_deref()).flatten()
        })
        .collect();
    let start = carried.into_iter().max().or(build.tasks_at.as_deref()).unwrap_or(now);
    let edited = build.last_edit_at.as_deref().is_some_and(|edit| edit > start);

    let merged: Vec<KeptTask> = list
        .iter()
        .zip(&matches)
        .map(|(item, at)| {
            let was = at.and_then(|at| kept.get(at));
            let (completed_at, edit) = match (completed(item), was) {
                (true, Some(was)) if was.completed_at.is_some() => (was.completed_at.clone(), was.edited),
                (true, _) => (Some(now.to_string()), edited),
                (false, _) => (None, false),
            };
            KeptTask {
                id: item.id.clone(),
                text: cut(&item.text),
                status: status_text(item.status).to_string(),
                first_seen_at: was.map_or_else(|| now.to_string(), |was| was.first_seen_at.clone()),
                completed_at,
                edited: edit,
                made_by: item.by.clone(),
            }
        })
        .collect();
    let changed = merged != kept;
    (merged, changed)
}

/// What a derived record is read from.
pub(crate) struct Read<'a> {
    pub build: &'a KeptBuild,
    pub tasks: &'a [KeptTask],
    /// The stops of the Session's current pull request and its own walks and pushes.
    pub road: &'a RoadTimes,
    /// When the Session's current pull request was attached, which bounds when it was opened.
    pub pull_since: Option<&'a str>,
    /// When it merged, where the forge's entry is missing but the ledger says so.
    pub merged_fallback: Option<&'a str>,
    pub forecast: &'a [BuildStopId],
    pub workers: Vec<String>,
    pub stuck: bool,
}

/// The stops reached, with when, in the order they were. A pull request exists before anything
/// Fleet read of it, so its opening is never later than its first check, its queueing or its merge.
pub(crate) fn reached(road: &RoadTimes, pull_since: Option<&str>) -> Vec<(BuildStopId, String)> {
    let pull = [road.opened.as_deref(), pull_since, road.checked.as_deref(), road.queued.as_deref(), road.merged.as_deref()]
        .into_iter()
        .flatten()
        .min();
    let mut found: Vec<(BuildStopId, String)> = [
        (BuildStopId::Walk, road.walked.as_deref()),
        (BuildStopId::Pushed, road.pushed.as_deref()),
        (BuildStopId::Pr, pull),
        (BuildStopId::Checks, road.checked.as_deref()),
        (BuildStopId::AutoMerge, road.queued.as_deref()),
    ]
    .into_iter()
    .filter_map(|(id, at)| Some((id, at?.to_string())))
    .collect();
    found.sort_by_key(|(id, at)| (at.clone(), ORDER.iter().position(|one| one == id)));
    found
}

/// The pieces of the ring: one per task, in list order.
fn pieces(build: &KeptBuild, tasks: &[KeptTask]) -> Vec<BuildPiece> {
    let scaffold = scaffolded(build, tasks);
    let first_open = tasks.iter().position(|task| task.completed_at.is_none());
    tasks
        .iter()
        .enumerate()
        .map(|(at, task)| BuildPiece {
            id: task.id.clone(),
            text: task.text.clone(),
            by: task.made_by.clone(),
            working: task.status == status_text(ReportedTaskStatus::InProgress),
            piece: match (&task.completed_at, task.edited) {
                (Some(_), true) => BuildPieceKind::Solid,
                (Some(_), false) => BuildPieceKind::Hollow,
                (None, _) if scaffold && Some(at) == first_open => BuildPieceKind::Scaffold,
                (None, _) => BuildPieceKind::Ghost,
            },
        })
        .collect()
}

/// The record: its phase, pieces, road and forecast.
pub(crate) fn derived(read: &Read<'_>) -> SessionBuild {
    let Read { build, tasks, road, .. } = read;
    let landed = road.merged.as_deref().or(read.merged_fallback);
    let stops = reached(road, read.pull_since);
    // The road begins at the first edit: a stop before a known one is another piece of work's. With no
    // edit ever observed, a stop is the evidence of work there is: a pull request, a push or a walk.
    let on_the_road = match build.first_edit_at.as_deref() {
        Some(first) => stops.iter().any(|(_, at)| at.as_str() >= first),
        None => !stops.is_empty(),
    };
    let marked = build.tasks_at.as_deref().into_iter().chain(build.plan_left_at.as_deref()).max();
    // The latest work seen: an edit, or the commit that stood for the first one.
    let worked = build.last_edit_at.as_deref().or(build.first_edit_at.as_deref());
    let planned = marked.is_some_and(|mark| worked.is_none_or(|worked| worked <= mark));
    let phase = if landed.is_some() {
        BuildPhase::Landed
    } else if on_the_road {
        BuildPhase::Road
    } else if planned {
        BuildPhase::Plan
    } else if build.first_edit_at.is_some() {
        BuildPhase::Implement
    } else {
        BuildPhase::Scope
    };
    // The road as it is drawn: what was reached, then what is usual for the repository and has not been.
    let usual = read.forecast.iter().filter(|id| !stops.iter().any(|(held, _)| held == *id));
    let road_drawn: Vec<BuildStop> = stops
        .iter()
        .map(|(id, at)| BuildStop { id: *id, at: Some(Instant::carried(at)) })
        .chain(usual.map(|id| BuildStop { id: *id, at: None }))
        .collect();
    SessionBuild {
        phase,
        pieces: pieces(build, tasks),
        stops: road_drawn,
        forecast: read.forecast.to_vec(),
        landed_at: landed.map(Instant::carried),
        quiet_since: build.last_tool_at.as_deref().map(Instant::carried),
        stalled: read.stuck,
        workers: read.workers.clone(),
    }
}

/// The road a repository's recent landed Sessions usually took: the **most common whole sequence** of
/// stops among `history`, the newest first. A tie goes to the one reached most recently. A Session
/// whose road Fleet observed nothing of takes no part, and a repository with none has no forecast.
pub(crate) fn usual_road(history: &[Vec<BuildStopId>]) -> Vec<BuildStopId> {
    let mut best: Option<(&Vec<BuildStopId>, usize)> = None;
    for sequence in history.iter().filter(|one| !one.is_empty()) {
        let votes = history.iter().filter(|other| *other == sequence).count();
        if best.is_none_or(|(_, most)| votes > most) {
            best = Some((sequence, votes));
        }
    }
    best.map(|(sequence, _)| sequence.clone()).unwrap_or_default()
}

/// Whether the ledger says this pull request merged. A row spent by a `gh pr merge` the forge has not
/// confirmed still reads open, and is not one.
pub(crate) fn merged(pull: &KeptAttachment) -> bool {
    crate::pull_requesting::pull_merged(&pull.detail, pull.state == AttachmentState::Spent)
}

/// The pull request this build is on the road with: the newest one held that is not closed and did not
/// merge before the build began.
pub(crate) fn current_pull<'a>(held: &'a [KeptAttachment], begun_at: &str) -> Option<&'a KeptAttachment> {
    held.iter()
        .filter(|one| one.kind == "pr" && one.state != AttachmentState::GivenBack)
        .filter(|one| !(merged(one) && one.changed_at.as_str() <= begun_at))
        .max_by(|a, b| a.since.cmp(&b.since))
}

/// Whether a pull request merged after this build began, so the next work starts another.
pub(crate) fn landed_since(held: &[KeptAttachment], begun_at: &str) -> bool {
    held.iter().any(|one| one.kind == "pr" && merged(one) && one.changed_at.as_str() > begun_at)
}
