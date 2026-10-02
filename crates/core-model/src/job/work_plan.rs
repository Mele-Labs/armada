//! A Job's plan: an approach, and the tasks the steps after it keep current.
//!
//! **Not the declared scope.** `DeclaredPaths` and `store::DeclaredPlan` say
//! where a step's work will be; this says what the work is. The types are
//! `WorkPlan` and `PlanTask` so neither reads as the other.
//!
//! **The history is the record and the list is derived.** Every change is a
//! [`PlanEntry`], appended and never edited, and [`WorkPlan::after`] is the one
//! place a change is applied — when it is appended and when it is replayed —
//! so a refused call and a row that will not replay are the same rule.
//!
//! **A task's state is a Drone's claim.** Nothing here gates a submission on
//! it; the Judge weighs it beside the diff.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::fmt::Write as _;
use core::num::NonZeroU32;

use crate::envelope::Timestamp;
use crate::job::attempt::Attempt;
use crate::job::ids::{RepoPath, StepId};
use crate::job::plan_group::GroupId;

/// A task's stable name within one plan: `T1`, `T2`, … in the order minted.
///
/// A number rather than a string, so a spelling that is not one cannot be held.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(NonZeroU32);

impl TaskId {
    pub fn numbered(number: NonZeroU32) -> TaskId {
        TaskId(number)
    }

    /// `T` and a positive number with no leading zero, or `None`. Case matters:
    /// the id a Drone was handed is the one it names.
    pub fn read(spelling: &str) -> Option<TaskId> {
        let digits = spelling.strip_prefix('T')?;
        if digits.starts_with('0') || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        digits.parse::<NonZeroU32>().ok().map(TaskId)
    }

    pub fn number(self) -> u32 {
        self.0.get()
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(out, "T{}", self.0)
    }
}

/// Where one task stands, as the last change to it said.
///
/// **`HandedIn` and `Failed` are Fleet's**: the first at a task Drone's
/// hand-in (answer 1), the second when its group's Checks are still red after
/// the step's retries run out (answer 9). No Drone or person can spell either.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskState {
    Open,
    Working,
    /// Its agent handed the work in, and the step's Checks have not answered.
    HandedIn,
    Done,
    Failed,
    Dropped,
}

impl TaskState {
    pub const ALL: &'static [TaskState] = &[
        TaskState::Open,
        TaskState::Working,
        TaskState::HandedIn,
        TaskState::Done,
        TaskState::Failed,
        TaskState::Dropped,
    ];

    pub fn as_wire(&self) -> &'static str {
        match self {
            TaskState::Open => "open",
            TaskState::Working => "working",
            TaskState::HandedIn => "handed_in",
            TaskState::Done => "done",
            TaskState::Failed => "failed",
            TaskState::Dropped => "dropped",
        }
    }

    pub fn from_wire(value: &str) -> Option<TaskState> {
        TaskState::ALL
            .iter()
            .copied()
            .find(|s| s.as_wire() == value)
    }
}

/// Why a task was dropped. **Never blank**, so a drop with no reason cannot be
/// constructed, let alone recorded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DropReason(String);

impl DropReason {
    pub fn new(text: &str) -> Option<DropReason> {
        let text = text.trim();
        (!text.is_empty()).then(|| DropReason(String::from(text)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Why a task failed: which group's Checks, on which run, said what. **Never
/// blank**, for [`DropReason`]'s reason, and written by Fleet alone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FailReason(String);

impl FailReason {
    pub fn new(text: &str) -> Option<FailReason> {
        let text = text.trim();
        (!text.is_empty()).then(|| FailReason(String::from(text)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The state a change moves a task to. `Dropped` and `Failed` carry their
/// reasons, which is what makes "a drop says why" a type rather than a check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskUpdate {
    Open,
    Working,
    /// Fleet's, at a task Drone's hand-in. [`TaskUpdate::read`] refuses it, so
    /// neither `update_task` nor a person's act can write it.
    HandedIn,
    Done,
    /// Fleet's, once its group's retries are spent. Refused by `read` too.
    Failed(FailReason),
    Dropped(DropReason),
}

/// Why a state and a reason did not read as an update.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotAnUpdate {
    NoSuchState {
        named: String,
    },
    DroppedWithoutAReason,
    /// A reason given with a state that keeps none. Refused rather than
    /// dropped, because a reason nothing keeps is one the caller believes was.
    ReasonWithoutADrop {
        state: TaskState,
    },
    /// A state only Fleet marks, from a hand-in or a group's Checks, and never
    /// from a Drone's `update_task` or a person's act.
    FleetMarksIt {
        state: TaskState,
    },
}

impl TaskUpdate {
    /// A state as spelled, and the reason beside it; empty is no reason.
    pub fn read(state: &str, reason: &str) -> Result<TaskUpdate, NotAnUpdate> {
        let named = TaskState::from_wire(state).ok_or_else(|| NotAnUpdate::NoSuchState {
            named: String::from(state),
        })?;
        let given = DropReason::new(reason);
        match (named, given) {
            (state @ (TaskState::HandedIn | TaskState::Failed), _) => {
                Err(NotAnUpdate::FleetMarksIt { state })
            }
            (TaskState::Dropped, Some(reason)) => Ok(TaskUpdate::Dropped(reason)),
            (TaskState::Dropped, None) => Err(NotAnUpdate::DroppedWithoutAReason),
            (state, Some(_)) => Err(NotAnUpdate::ReasonWithoutADrop { state }),
            (TaskState::Open, None) => Ok(TaskUpdate::Open),
            (TaskState::Working, None) => Ok(TaskUpdate::Working),
            (TaskState::Done, None) => Ok(TaskUpdate::Done),
        }
    }

    /// A row read back off the store. **Admits `handed_in` and `failed`**,
    /// which [`read`](Self::read) refuses: the store keeps Fleet's marks beside
    /// a Drone's and a person's, and a mark Fleet kept has to replay.
    pub fn stored(state: &str, reason: &str) -> Result<TaskUpdate, NotAnUpdate> {
        match TaskState::from_wire(state) {
            Some(TaskState::HandedIn) if DropReason::new(reason).is_none() => {
                Ok(TaskUpdate::HandedIn)
            }
            Some(TaskState::Failed) => FailReason::new(reason)
                .map(TaskUpdate::Failed)
                .ok_or(NotAnUpdate::DroppedWithoutAReason),
            _ => TaskUpdate::read(state, reason),
        }
    }

    pub fn state(&self) -> TaskState {
        match self {
            TaskUpdate::Open => TaskState::Open,
            TaskUpdate::Working => TaskState::Working,
            TaskUpdate::HandedIn => TaskState::HandedIn,
            TaskUpdate::Done => TaskState::Done,
            TaskUpdate::Failed(_) => TaskState::Failed,
            TaskUpdate::Dropped(_) => TaskState::Dropped,
        }
    }

    /// The reason a drop or a failure carries, which is the one the store's
    /// `reason` column keeps.
    pub fn reason(&self) -> Option<&str> {
        match self {
            TaskUpdate::Dropped(reason) => Some(reason.as_str()),
            TaskUpdate::Failed(reason) => Some(reason.as_str()),
            _ => None,
        }
    }
}

/// A task as it is asked for.
///
/// **Four fields, and the note is the only free text.** `scope` and `expects`
/// hold what a planning step used to bury in prose, where the step after it
/// could not act on either: the paths and the artifact that should prove the
/// task. The note is what neither of those can hold — the exact new wording, a
/// gotcha found while planning — and it is one line, deliberately.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewTask {
    title: String,
    note: String,
    scope: Vec<RepoPath>,
    expects: String,
    /// The group the planner put it in, by its own number, on a recording.
    /// `None` is the group of the task before it, and the first group for the
    /// first task, so a plan recorded without groups is one group.
    group: Option<NonZeroU32>,
}

impl NewTask {
    /// `None` where the title is blank. Every other field may be empty: a task
    /// whose paths are not yet known is a task, and refusing one would push
    /// the planner back into writing them into the note.
    pub fn new(title: &str, note: &str, scope: &[&str], expects: &str) -> Option<NewTask> {
        let title = title.trim();
        (!title.is_empty()).then(|| NewTask {
            title: String::from(title),
            note: String::from(note.trim()),
            scope: scope
                .iter()
                .map(|path| path.trim())
                .filter(|path| !path.is_empty())
                .map(RepoPath::new)
                .collect(),
            expects: String::from(expects.trim()),
            group: None,
        })
    }

    /// The same task, in the planner's group `number`. Zero names no group.
    pub fn in_group(self, number: u32) -> NewTask {
        NewTask {
            group: NonZeroU32::new(number),
            ..self
        }
    }

    /// The planner's own number for its group, where it named one.
    pub fn group(&self) -> Option<u32> {
        self.group.map(NonZeroU32::get)
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    /// What the other three fields cannot hold. May be empty.
    pub fn note(&self) -> &str {
        &self.note
    }

    /// The repository-relative paths this task touches, in the order given.
    pub fn scope(&self) -> &[RepoPath] {
        &self.scope
    }

    /// What the planner says should prove the task. May be empty.
    pub fn expects(&self) -> &str {
        &self.expects
    }
}

/// What the work itself says proved a task. **Never blank**, for
/// [`DropReason`]'s reason: an empty one is a caller believing it said
/// something.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shown(String);

impl Shown {
    pub fn new(text: &str) -> Option<Shown> {
        let text = text.trim();
        (!text.is_empty()).then(|| Shown(String::from(text)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The paragraph a plan opens with. **Never blank**, for [`DropReason`]'s reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Approach(String);

impl Approach {
    pub fn new(text: &str) -> Option<Approach> {
        let text = text.trim();
        (!text.is_empty()).then(|| Approach(String::from(text)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Who made a change: a run of a step, or a person.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanAuthor {
    Step { step_id: StepId, attempt: Attempt },
    Person,
}

/// One change to a Job's plan, as it is appended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanChange {
    /// A whole plan. **Replaces whatever was there**, which is how a retry of
    /// the recording step starts again rather than appending to its last try.
    Recorded {
        approach: Approach,
        tasks: Vec<NewTask>,
    },
    /// One task, after the one named and into that task's group, or at the
    /// end and into the last group.
    Added {
        task: NewTask,
        after: Option<TaskId>,
    },
    /// A person moves a task into `group`, after `after`, or first in it where
    /// `after` is absent. **By `after` and never by index**: an index counted
    /// at the drag is stale the moment a Drone adds or drops a task.
    MovedTask {
        task: TaskId,
        group: GroupId,
        after: Option<TaskId>,
    },
    /// A person moves a group after `after`, or first where it is absent.
    MovedGroup {
        group: GroupId,
        after: Option<GroupId>,
    },
    /// **`shown` is the other end of `expects`.** The planner wrote what ought
    /// to prove the task; this is what did, written by whoever worked it.
    /// `None` leaves whatever an earlier update recorded, so marking a task
    /// `open` and `done` again does not erase it.
    Updated {
        task: TaskId,
        to: TaskUpdate,
        shown: Option<Shown>,
    },
}

/// One row of a plan's history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanEntry {
    pub change: PlanChange,
    pub by: PlanAuthor,
    pub at: Timestamp,
}

/// A change the plan as it stands cannot take.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanRefused {
    /// A task added or updated on a Job no plan was recorded for.
    NoPlan,
    NoSuchTask {
        named: TaskId,
    },
    /// `after` names a task the plan does not hold.
    NoSuchPlace {
        named: TaskId,
    },
    /// A move out of `dropped`. A dropped task stays dropped; the work comes
    /// back only as a new task, which is a person's add.
    StaysDropped {
        named: TaskId,
    },
    /// A move names a group the plan does not hold.
    NoSuchGroup {
        named: GroupId,
    },
    /// A task move's `after` is a task outside the group it moves into.
    NotInGroup {
        named: TaskId,
        group: GroupId,
    },
    /// A recording numbers a task's group below the task's before it: groups
    /// are listed in the order they run.
    GroupsOutOfOrder {
        named: u32,
    },
}

impl fmt::Display for PlanRefused {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlanRefused::NoPlan => out.write_str("no plan has been recorded for this Job"),
            PlanRefused::NoSuchTask { named } => write!(out, "the plan holds no task {named}"),
            PlanRefused::NoSuchPlace { named } => {
                write!(out, "the plan holds no task {named} to add after")
            }
            PlanRefused::StaysDropped { named } => {
                write!(
                    out,
                    "task {named} was dropped, and a dropped task stays dropped"
                )
            }
            PlanRefused::NoSuchGroup { named } => write!(out, "the plan holds no group {named}"),
            PlanRefused::NotInGroup { named, group } => {
                write!(out, "task {named} is not in group {group} to move after")
            }
            PlanRefused::GroupsOutOfOrder { named } => write!(
                out,
                "group {named} comes after a higher group: list the tasks group by group"
            ),
        }
    }
}

/// A stretch a task was marked `working`, read off the history and never guessed
/// from the work: a Drone that never marks a task leaves none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkingWindow {
    pub entered: Timestamp,
    /// `None` while the task is still marked working.
    pub left: Option<Timestamp>,
}

/// One line of the plan, as the history leaves it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanTask {
    id: TaskId,
    group: GroupId,
    task: NewTask,
    state: TaskUpdate,
    shown: Option<Shown>,
    windows: Vec<WorkingWindow>,
}

impl PlanTask {
    fn new(id: TaskId, group: GroupId, task: &NewTask) -> PlanTask {
        PlanTask {
            id,
            group,
            task: task.clone(),
            state: TaskUpdate::Open,
            shown: None,
            windows: Vec::new(),
        }
    }

    pub fn id(&self) -> TaskId {
        self.id
    }

    /// The group it runs in.
    pub fn group(&self) -> GroupId {
        self.group
    }

    pub fn title(&self) -> &str {
        self.task.title()
    }

    /// What the other fields cannot hold. Empty where the task has none.
    pub fn note(&self) -> &str {
        self.task.note()
    }

    /// The paths the planner said this task touches.
    pub fn scope(&self) -> &[RepoPath] {
        self.task.scope()
    }

    /// What the planner said should prove it. Empty where none was named.
    pub fn expects(&self) -> &str {
        self.task.expects()
    }

    /// What the work said proved it, where an update carried one.
    pub fn shown(&self) -> Option<&Shown> {
        self.shown.as_ref()
    }

    pub fn state(&self) -> TaskState {
        self.state.state()
    }

    /// Present on a dropped task and on nothing else.
    pub fn reason(&self) -> Option<&str> {
        match &self.state {
            TaskUpdate::Dropped(reason) => Some(reason.as_str()),
            _ => None,
        }
    }

    /// Present on a failed task and on nothing else: which group's Checks,
    /// on which run, were still red.
    pub fn failed_reason(&self) -> Option<&str> {
        match &self.state {
            TaskUpdate::Failed(reason) => Some(reason.as_str()),
            _ => None,
        }
    }

    /// Every stretch it was marked working, oldest first.
    pub fn working_windows(&self) -> &[WorkingWindow] {
        &self.windows
    }
}

/// How many tasks stand where. `done` over [`not_dropped`](Self::not_dropped)
/// is the figure a person reads: a handed-in task and a failed one are owed
/// work, so both join the total and neither joins `done`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TaskCounts {
    pub done: u32,
    pub working: u32,
    pub open: u32,
    pub dropped: u32,
    pub handed_in: u32,
    pub failed: u32,
}

impl TaskCounts {
    pub fn not_dropped(&self) -> u32 {
        self.done + self.working + self.open + self.handed_in + self.failed
    }
}

/// A Job's plan as its history leaves it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkPlan {
    approach: Approach,
    recorded_by: PlanAuthor,
    recorded_at: Timestamp,
    /// In the order they run. A group a move emptied stays, and runs nothing.
    groups: Vec<GroupId>,
    /// In plan order: group by group, and in each group as placed.
    tasks: Vec<PlanTask>,
}

impl WorkPlan {
    /// The plan a whole history leaves, or `None` where nothing was recorded.
    /// A history holding a change its plan could not take is refused, because
    /// replaying past it would draw a list nobody wrote.
    pub fn fold(history: &[PlanEntry]) -> Result<Option<WorkPlan>, PlanRefused> {
        let mut plan = None;
        for entry in history {
            plan = Some(WorkPlan::after(plan.as_ref(), entry)?);
        }
        Ok(plan)
    }

    /// The plan once one more change is applied to `current`.
    pub fn after(current: Option<&WorkPlan>, entry: &PlanEntry) -> Result<WorkPlan, PlanRefused> {
        if let PlanChange::Recorded { approach, tasks } = &entry.change {
            return WorkPlan::recorded(approach, tasks, entry);
        }
        let mut plan = current.cloned().ok_or(PlanRefused::NoPlan)?;
        match &entry.change {
            PlanChange::Recorded { .. } => unreachable!("answered above"),
            PlanChange::Added { task, after } => {
                let (at, group) = match after {
                    None => {
                        if plan.groups.is_empty() {
                            plan.groups.push(GroupId::FIRST);
                        }
                        let last = plan.groups[plan.groups.len() - 1];
                        (plan.tasks.len(), last)
                    }
                    Some(named) => plan
                        .position(*named)
                        .map(|at| (at + 1, plan.tasks[at].group))
                        .ok_or(PlanRefused::NoSuchPlace { named: *named })?,
                };
                let next = plan.tasks.iter().map(|t| t.id.number()).max().unwrap_or(0);
                let id = TaskId(NonZeroU32::MIN.saturating_add(next));
                plan.tasks.insert(at, PlanTask::new(id, group, task));
                plan.in_group_order();
            }
            PlanChange::MovedTask { task, group, after } => {
                if !plan.groups.contains(group) {
                    return Err(PlanRefused::NoSuchGroup { named: *group });
                }
                let from = plan
                    .position(*task)
                    .ok_or(PlanRefused::NoSuchTask { named: *task })?;
                let mut moved = plan.tasks.remove(from);
                moved.group = *group;
                let at = match after {
                    Some(named) => {
                        let at = plan
                            .position(*named)
                            .ok_or(PlanRefused::NoSuchPlace { named: *named })?;
                        if plan.tasks[at].group != *group {
                            return Err(PlanRefused::NotInGroup {
                                named: *named,
                                group: *group,
                            });
                        }
                        at + 1
                    }
                    None => plan
                        .tasks
                        .iter()
                        .position(|t| t.group == *group)
                        .unwrap_or(plan.tasks.len()),
                };
                plan.tasks.insert(at, moved);
                plan.in_group_order();
            }
            PlanChange::MovedGroup { group, after } => {
                let from = plan
                    .groups
                    .iter()
                    .position(|g| g == group)
                    .ok_or(PlanRefused::NoSuchGroup { named: *group })?;
                plan.groups.remove(from);
                let at = match after {
                    Some(named) => {
                        plan.groups
                            .iter()
                            .position(|g| g == named)
                            .ok_or(PlanRefused::NoSuchGroup { named: *named })?
                            + 1
                    }
                    None => 0,
                };
                plan.groups.insert(at, *group);
                plan.in_group_order();
            }
            PlanChange::Updated { task, to, shown } => {
                let at = plan
                    .position(*task)
                    .ok_or(PlanRefused::NoSuchTask { named: *task })?;
                // A new reason on a dropped task is not a move out of it.
                if plan.tasks[at].state() == TaskState::Dropped && to.state() != TaskState::Dropped
                {
                    return Err(PlanRefused::StaysDropped { named: *task });
                }
                let task = &mut plan.tasks[at];
                let was = task.state();
                // Opened by the move into `working`, closed by the move out.
                match (was == TaskState::Working, to.state() == TaskState::Working) {
                    (false, true) => task.windows.push(WorkingWindow {
                        entered: entry.at.clone(),
                        left: None,
                    }),
                    (true, false) => {
                        if let Some(open) = task.windows.last_mut() {
                            open.left = Some(entry.at.clone());
                        }
                    }
                    _ => {}
                }
                task.state = to.clone();
                // Kept rather than replaced, so reopening a task and finishing
                // it again does not lose what the first pass showed.
                if shown.is_some() {
                    task.shown = shown.clone();
                }
            }
        }
        Ok(plan)
    }

    /// A whole recording: tasks numbered in order, and a group minted for each
    /// number the planner gave, in the order they first appear.
    fn recorded(
        approach: &Approach,
        tasks: &[NewTask],
        entry: &PlanEntry,
    ) -> Result<WorkPlan, PlanRefused> {
        let mut groups: Vec<GroupId> = Vec::new();
        let mut numbered: Option<u32> = None;
        let mut planned = Vec::with_capacity(tasks.len());
        for (task, n) in tasks.iter().zip(1u32..) {
            let named = task.group().or(numbered).unwrap_or(1);
            match numbered {
                Some(before) if named < before => {
                    return Err(PlanRefused::GroupsOutOfOrder { named })
                }
                Some(before) if named == before => {}
                _ => groups.push(GroupId::after(groups.len() as u32)),
            }
            numbered = Some(named);
            let id = TaskId(NonZeroU32::MIN.saturating_add(n - 1));
            planned.push(PlanTask::new(id, groups[groups.len() - 1], task));
        }
        Ok(WorkPlan {
            approach: approach.clone(),
            recorded_by: entry.by.clone(),
            recorded_at: entry.at.clone(),
            groups,
            tasks: planned,
        })
    }

    /// Tasks group by group, each group's in the order placed: a stable sort,
    /// so a move within a group is the only thing that reorders one.
    fn in_group_order(&mut self) {
        let groups = &self.groups;
        self.tasks.sort_by_key(|task| {
            groups
                .iter()
                .position(|g| *g == task.group)
                .unwrap_or(usize::MAX)
        });
    }

    fn position(&self, id: TaskId) -> Option<usize> {
        self.tasks.iter().position(|task| task.id == id)
    }

    pub fn approach(&self) -> &str {
        self.approach.as_str()
    }

    /// Who recorded the plan as it stands — the last whole recording.
    pub fn recorded_by(&self) -> &PlanAuthor {
        &self.recorded_by
    }

    pub fn recorded_at(&self) -> &Timestamp {
        &self.recorded_at
    }

    /// In plan order: recorded order, with each added task where it was put.
    pub fn tasks(&self) -> &[PlanTask] {
        &self.tasks
    }

    pub fn task(&self, id: TaskId) -> Option<&PlanTask> {
        self.tasks.iter().find(|task| task.id == id)
    }

    /// Every group, in the order they run.
    pub fn groups(&self) -> &[GroupId] {
        &self.groups
    }

    /// One group's tasks, in plan order.
    pub fn tasks_in(&self, group: GroupId) -> impl Iterator<Item = &PlanTask> + Clone {
        self.tasks.iter().filter(move |task| task.group == group)
    }

    pub fn counts(&self) -> TaskCounts {
        let mut counts = TaskCounts::default();
        for task in &self.tasks {
            match task.state() {
                TaskState::Open => counts.open += 1,
                TaskState::Working => counts.working += 1,
                TaskState::HandedIn => counts.handed_in += 1,
                TaskState::Done => counts.done += 1,
                TaskState::Failed => counts.failed += 1,
                TaskState::Dropped => counts.dropped += 1,
            }
        }
        counts
    }

    /// The plan as it stands, in words: the approach, then each task with its
    /// id, title, state, detail and — for a dropped one — the reason.
    ///
    /// **Fleet's own reading of its own record, and never the Drone's.**
    /// `record_plan`, `add_task` and `update_task` are the only three calls a
    /// plan can be built from, so what this renders is what Fleet kept, not an
    /// account of a turn. `#895` is where a step's Judge is handed this — its
    /// own, where its product is a plan, and a later step's through
    /// `reference_docs`.
    ///
    /// **Every field a task carries is rendered.** The step after a planning
    /// one reads this and nothing else of the plan, so a field left out of it
    /// is a field the next Drone re-derives — which is what a title-only
    /// rendering cost, before `scope` and `expects` were fields at all.
    pub fn rendered(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "Approach: {}", self.approach.as_str());
        let _ = writeln!(out);
        let _ = write!(out, "Tasks:");
        // Headed only where there is more than one group, so a plan of one
        // reads as it did before groups.
        let grouped = self.groups.len() > 1;
        let mut heading = None;
        for task in &self.tasks {
            if grouped && heading != Some(task.group) {
                heading = Some(task.group);
                let _ = write!(out, "\n  Group {}:", task.group);
            }
            let _ = write!(
                out,
                "\n  {} [{}] {}",
                task.id(),
                task.state().as_wire(),
                task.title()
            );
            if let Some(reason) = task.reason() {
                let _ = write!(out, " — dropped: {reason}");
            }
            if let Some(reason) = task.failed_reason() {
                let _ = write!(out, " — failed: {reason}");
            }
            // Hung under the title and labelled, so a task carrying three of
            // them cannot read as three tasks.
            if !task.note().is_empty() {
                let _ = write!(out, "\n      note: {}", task.note());
            }
            if !task.scope().is_empty() {
                let _ = write!(out, "\n      files:");
                for path in task.scope() {
                    let _ = write!(out, " {}", path.as_str());
                }
            }
            if !task.expects().is_empty() {
                let _ = write!(out, "\n      expects: {}", task.expects());
            }
            if let Some(shown) = task.shown() {
                let _ = write!(out, "\n      shown: {}", shown.as_str());
            }
        }
        out
    }
}
