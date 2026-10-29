//! A Session's plan: what the agent records after planning with the person, what he approves and
//! marks up, and how far Fleet has seen it carried out. `docs/concepts/session.md`, *The plan*.
//!
//! **The plan is the agent's statement and progress is Fleet's observation.** `record_plan` carries
//! groups, the subagent type that takes each, the paths it works in and its tasks. A task's state
//! is read off the agent's own task list and the edits Fleet saw under the group's paths, never
//! from a word of the agent's about it.
//!
//! Wire additions of the plan live here and in `SessionRecord::{plan, touches}`, so a change to one
//! does not touch the rest of the record.

use serde::{Deserialize, Serialize};

use crate::ids::Instant;
use crate::issue_reference::IssueReference;
use crate::sessions::SessionId;
use crate::sketch::{RawScene, ShowSketch};

/// Most groups, tasks in one group and tasks in the plan. A plan is read in one sitting.
pub const MOST_GROUPS: usize = 24;
pub const MOST_TASKS_IN_A_GROUP: usize = 40;
pub const MOST_TASKS: usize = 200;
/// Most paths one group names.
pub const MOST_PATHS: usize = 24;
/// Characters in a title, a task, a path and a subagent type.
pub const MOST_TITLE_CHARS: usize = 160;
pub const MOST_PATH_CHARS: usize = 200;
pub const MOST_AGENT_CHARS: usize = 60;
pub const MOST_ID_CHARS: usize = 40;
/// Characters in the plan's `summary` and in a group's `gives`: what the person reads first.
pub const MOST_SUMMARY_CHARS: usize = 600;
pub const MOST_GIVES_CHARS: usize = 160;
/// Characters in a group's `lands_in` label, such as `PR 1`.
pub const MOST_LANDS_IN_CHARS: usize = 24;
/// The pull request a group lands in where it names none.
pub const FIRST_PULL_REQUEST: &str = "PR 1";
/// What the agent is told, wherever it is given a plan, so its own task list is the plan's tasks and
/// Fleet can read how far each got. The Planning prompt, the `record_plan` receipt and the message
/// for an approved plan all say these words.
pub const TASK_LIST_IS_THE_PLAN: &str =
    "Your task list is this plan: TaskCreate each task word for word, and TaskUpdate it as you work.";

/// Most marks one `send_plan_changes` carries.
pub const MOST_MARKS: usize = 200;
/// Problems listed in one refusal.
const MOST_PROBLEMS_SAID: usize = 12;

// ---- What the agent records -----------------------------------------------------------------------

/// One task of a group, as the agent records it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanTaskInput {
    /// The id this task had in the plan before, which keeps it the same task through a rename.
    /// Left out for a task that is new.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub title: String,
}

/// One group: the tasks a single subagent takes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanGroupInput {
    /// As on [`PlanTaskInput::id`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub title: String,
    /// The type of subagent that takes it, free text such as `rust-engineer`.
    pub agent: String,
    /// The paths it works in, from the repository's root. Fleet marks the group worked when it sees
    /// an edit under one.
    #[serde(default)]
    pub paths: Vec<String>,
    /// The issue the group works: the forge's number, or a tracker's key such as `ABC-12`. Fleet
    /// holds an issue row for it on the Session. A JSON number still reads.
    #[serde(
        default,
        deserialize_with = "crate::issue_reference::number_or_text",
        skip_serializing_if = "Option::is_none"
    )]
    pub issue: Option<String>,
    /// One line, in the person's own words, of what this group gives him.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gives: Option<String>,
    /// A short label for the pull request the group lands in, such as `PR 1`. Groups that ship in one
    /// pull request share a label. Absent is `PR 1`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lands_in: Option<String>,
    pub tasks: Vec<PlanTaskInput>,
}

/// A sketch drawn with the plan, in the format `show_sketch` takes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanSketchInput {
    /// What the ledger calls it, as `show_sketch`'s `title`: a second plan with the same title
    /// replaces the sketch where it stands.
    pub title: String,
    pub scene: RawScene,
}

/// `record_plan`: a Session records its plan. Recording again replaces the plan as a new version.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordPlan {
    /// A few plain lines in the person's own words: what he gets. No file names or types. A plan
    /// without one is refused, which a missing field reaches as blank.
    #[serde(default)]
    pub summary: String,
    pub groups: Vec<PlanGroupInput>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sketch: Option<PlanSketchInput>,
    /// The page the plan depends on, as `show_window` was given it. The plan is put to the person
    /// only after he approves that walk. Recording again without it clears it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub walk: Option<String>,
    /// Which session records it. **Read only where the connection places no hosted session**, as
    /// `ShowSketch::session_id` is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// A plan that passed every rule: trimmed, with the sketch checked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckedPlan {
    pub summary: String,
    pub groups: Vec<PlanGroupInput>,
    pub sketch: Option<crate::Sketch>,
    /// The walk the plan depends on, trimmed. Absent where blank.
    pub walk: Option<String>,
}

fn slug(word: &str) -> bool {
    !word.is_empty() && word.chars().all(|c| c.is_ascii_alphanumeric() || "-_".contains(c))
}

/// A path from the repository's root: relative, inside it, in the form git names files.
fn rooted(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.split('/').any(|part| part == "..")
}

impl RecordPlan {
    /// The plan, or every reason it is refused in one message an agent can act on.
    pub fn checked(&self) -> Result<CheckedPlan, String> {
        let mut problems: Vec<String> = Vec::new();
        let summary = self.summary.trim().to_string();
        if summary.is_empty() {
            problems.push(String::from("`summary` is blank. Write a few plain lines saying what the person gets"));
        } else if summary.chars().count() > MOST_SUMMARY_CHARS {
            problems.push(format!("`summary` is longer than {MOST_SUMMARY_CHARS} characters"));
        }
        let mut groups = Vec::with_capacity(self.groups.len());
        if self.groups.is_empty() {
            problems.push(String::from("`groups` is empty. A plan has at least one group of tasks"));
        }
        if self.groups.len() > MOST_GROUPS {
            problems.push(format!("{} groups, and a plan holds at most {MOST_GROUPS}", self.groups.len()));
        }
        let mut seen: Vec<String> = Vec::new();
        let mut taken = |at: String, id: &Option<String>, problems: &mut Vec<String>| -> Option<String> {
            let id = id.as_deref()?.trim();
            if !slug(id) || id.chars().count() > MOST_ID_CHARS {
                problems.push(format!("{at} has the id `{id}`, which is not up to {MOST_ID_CHARS} letters, digits, `-` or `_`"));
                return None;
            }
            if seen.iter().any(|one| one == id) {
                problems.push(format!("{at} repeats the id `{id}`, which another group or task holds"));
                return None;
            }
            seen.push(id.to_string());
            Some(id.to_string())
        };
        let mut total = 0;
        for (at, group) in self.groups.iter().enumerate() {
            let place = format!("groups[{at}]");
            let title = group.title.trim().to_string();
            if title.is_empty() {
                problems.push(format!("{place} has a blank `title`"));
            } else if title.chars().count() > MOST_TITLE_CHARS {
                problems.push(format!("{place} `title` is longer than {MOST_TITLE_CHARS} characters"));
            }
            let agent = group.agent.trim().to_string();
            if agent.is_empty() || agent.chars().count() > MOST_AGENT_CHARS || agent.chars().any(char::is_whitespace) {
                problems.push(format!(
                    "{place} `agent` is `{agent}`. Name the subagent type that takes the group, one word such as `rust-engineer`"
                ));
            }
            let mut paths = Vec::new();
            if group.paths.len() > MOST_PATHS {
                problems.push(format!("{place} names {} paths and a group names at most {MOST_PATHS}", group.paths.len()));
            }
            for path in &group.paths {
                let path = path.trim().trim_end_matches('/').trim_start_matches("./").to_string();
                if !rooted(&path) || path.chars().count() > MOST_PATH_CHARS {
                    problems.push(format!(
                        "{place} path `{path}` is not a path inside the repository. Write it from the root, with no leading `/` or `..`"
                    ));
                } else if !paths.contains(&path) {
                    paths.push(path);
                }
            }
            if group.tasks.is_empty() {
                problems.push(format!("{place} `{title}` has no tasks. A group holds the tasks its subagent takes"));
            }
            if group.tasks.len() > MOST_TASKS_IN_A_GROUP {
                problems.push(format!("{place} has {} tasks and a group holds at most {MOST_TASKS_IN_A_GROUP}", group.tasks.len()));
            }
            total += group.tasks.len();
            let id = taken(place.clone(), &group.id, &mut problems);
            let mut tasks = Vec::with_capacity(group.tasks.len());
            for (nth, task) in group.tasks.iter().enumerate() {
                let at = format!("{place}.tasks[{nth}]");
                let text = task.title.trim().to_string();
                if text.is_empty() {
                    problems.push(format!("{at} has a blank `title`"));
                } else if text.chars().count() > MOST_TITLE_CHARS {
                    problems.push(format!("{at} `title` is longer than {MOST_TITLE_CHARS} characters"));
                }
                tasks.push(PlanTaskInput { id: taken(at, &task.id, &mut problems), title: text });
            }
            let issue = group.issue.as_deref().and_then(|written| {
                let read = IssueReference::read(written);
                if read.is_none() {
                    problems.push(format!(
                        "{place} `issue` is `{written}`, which is neither an issue number such as `7` nor a tracker's key such as `ABC-12`"
                    ));
                }
                read.map(|one| one.as_str().to_string())
            });
            let gives = group.gives.as_deref().map(str::trim).filter(|one| !one.is_empty()).map(str::to_string);
            if gives.as_deref().is_some_and(|one| one.chars().count() > MOST_GIVES_CHARS) {
                problems.push(format!("{place} `gives` is longer than {MOST_GIVES_CHARS} characters"));
            }
            let lands_in = group.lands_in.as_deref().map(str::trim).filter(|one| !one.is_empty()).map(str::to_string);
            if lands_in.as_deref().is_some_and(|one| one.chars().count() > MOST_LANDS_IN_CHARS) {
                problems.push(format!("{place} `lands_in` is longer than {MOST_LANDS_IN_CHARS} characters"));
            }
            groups.push(PlanGroupInput { id, title, agent, paths, issue, gives, lands_in, tasks });
        }
        if total > MOST_TASKS {
            problems.push(format!("{total} tasks, and a plan holds at most {MOST_TASKS}"));
        }
        let sketch = match &self.sketch {
            None => None,
            Some(drawn) => {
                let shown = ShowSketch {
                    title: drawn.title.clone(),
                    scene: drawn.scene.clone(),
                    session_id: None,
                    against: None,
                };
                match shown.checked_for("record_plan") {
                    Ok(sketch) => Some(sketch),
                    Err(why) => {
                        problems.push(format!("`sketch`: {why}"));
                        None
                    }
                }
            }
        };
        let walk = self.walk.as_deref().map(str::trim).filter(|walk| !walk.is_empty()).map(str::to_string);
        if walk.as_deref().is_some_and(|walk| walk.contains(char::is_whitespace)) {
            problems.push(String::from("`walk` is not a page address. Give the url you passed to show_window"));
        }
        if !problems.is_empty() {
            let said: Vec<&str> = problems.iter().take(MOST_PROBLEMS_SAID).map(String::as_str).collect();
            let more = match problems.len().saturating_sub(MOST_PROBLEMS_SAID) {
                0 => String::new(),
                left => format!("; and {left} more"),
            };
            return Err(format!("the plan was not recorded: {}{more}. Fix these and call record_plan again", said.join("; ")));
        }
        Ok(CheckedPlan { summary, groups, sketch, walk })
    }
}

// ---- What the record carries ----------------------------------------------------------------------

/// Where one task stands. `open` until it is worked; `handed_in` is done by the agent's word with no
/// edit seen under the group's paths; `done` is done with one seen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanTaskState {
    Open,
    Working,
    HandedIn,
    Done,
    Failed,
}

/// One task of the plan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionPlanTask {
    pub id: String,
    pub text: String,
    pub state: PlanTaskState,
    /// An edit was seen under its group's paths. Absent when none was.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub seen: bool,
    /// Fleet set the state itself, from what it saw: the proof, as a person reads it ("commit 1a2b3c4
    /// names it"). Absent where the agent's own list set it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marked: Option<String>,
    /// In progress, and the work looks to be elsewhere. Absent while it is not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stale: Option<TaskStale>,
    /// When it was put in progress, or completed, by the agent's list or Fleet's mark. Absent while it
    /// is open.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changed_at: Option<Instant>,
}

/// Why a task in progress looks left behind, and since when it was last touched.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskStale {
    /// One line, as a person reads it: "edits went to Bridge", "40 calls with no update".
    pub why: String,
    /// When its mark last moved, or Fleet last saw work under its group's paths.
    pub since: Instant,
}

/// One group of the plan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionPlanGroup {
    pub id: String,
    pub agent: String,
    pub title: String,
    /// One line of what the group gives the person. Absent where the agent wrote none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gives: Option<String>,
    pub paths: Vec<String>,
    pub tasks: Vec<SessionPlanTask>,
    /// The issue the group works, as the ledger's `issue` row names it: digits or a tracker's key.
    /// Absent where it names none.
    #[serde(
        default,
        deserialize_with = "crate::issue_reference::number_or_text",
        skip_serializing_if = "Option::is_none"
    )]
    pub issue: Option<String>,
    /// When Fleet last saw an edit under one of the group's paths. Absent while it has seen none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worked_at: Option<Instant>,
    /// The pull request the group lands in: its label, and the pull request itself once one is bound
    /// to that label. Always present; the label is `PR 1` where the agent named none.
    #[serde(default)]
    pub lands_in: PlanLandsIn,
}

/// Where a group lands. Every group that shares a label reads the same `pull_request`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanLandsIn {
    pub label: String,
    /// The pull request bound to the label. Absent until one is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pull_request: Option<PlanLandsInPr>,
}

impl Default for PlanLandsIn {
    fn default() -> Self {
        Self { label: FIRST_PULL_REQUEST.to_string(), pull_request: None }
    }
}

/// A pull request a plan lands in, as the ledger's `pr` row last read it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanLandsInPr {
    pub number: u64,
    pub title: String,
    pub state: PlanLandsInState,
}

/// A pull request's standing, as the forge last said it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanLandsInState {
    Open,
    Draft,
    Merged,
    Closed,
}

/// A group or task the agent renamed when it recorded the plan again, and what it was called.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanRenamed {
    pub id: String,
    pub from: String,
}

/// A task the agent moved to another group, and the group it came from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanMoved {
    pub id: String,
    pub from: String,
}

/// A group or task the agent took out. `group` is the group it was in, or its own id for a group.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanRemoved {
    pub group: String,
    pub text: String,
}

/// What differs from the version before, matched by id and then by text.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanChanges {
    /// The ids of groups and tasks that are new.
    pub added: Vec<String>,
    pub renamed: Vec<PlanRenamed>,
    pub moved: Vec<PlanMoved>,
    pub removed: Vec<PlanRemoved>,
}

impl PlanChanges {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.renamed.is_empty() && self.moved.is_empty() && self.removed.is_empty()
    }
}

/// The sketch drawn with the plan. Its scene is the Session's ledger artifact `id`, so the record
/// does not carry it twice.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanSketch {
    pub id: String,
    pub title: String,
}

/// A Session's plan as it stands. Absent until the agent records one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionPlan {
    /// One for the first recording, and one more for each that replaces it.
    pub version: u32,
    /// A few plain lines in the agent's words of what the person gets. Empty for a plan recorded
    /// before there was one.
    #[serde(default)]
    pub summary: String,
    pub recorded_at: Instant,
    /// The person approved this version. A version recorded again is pending until he approves it.
    pub approved: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_at: Option<Instant>,
    /// When the person put this version away to keep talking. It holds nothing and calls for nothing
    /// until the agent records a new version, which starts without it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set_aside_at: Option<Instant>,
    /// Every path a group names, once each.
    pub scope: Vec<String>,
    pub groups: Vec<SessionPlanGroup>,
    /// What this version changed against the one before. Absent for the first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changes: Option<PlanChanges>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sketch: Option<PlanSketch>,
    /// The windows that keep a pending plan from being put to the person, each with what settles it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub held_by: Vec<PlanHold>,
    /// The latest adversarial review of this Job's plan, whatever version it read. A reader marks it out of
    /// date when its subject's `version` is not this plan's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adversarial: Option<crate::AdversarialReview>,
}

/// A window that holds a pending plan back.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanHold {
    pub url: String,
    pub why: PlanHoldWhy,
}

/// Why a window holds a pending plan back, and so what settles it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanHoldWhy {
    /// He has not approved it since it was last shown. His Approve settles it.
    Unapproved,
    /// The newest window, and the Job is changing it. The end of the Job's turn settles it.
    Changing,
    /// Changed since it was last shown. Showing it again settles it, then his Approve.
    Changed,
    /// The walk the plan names, not approved since it was last shown, or never shown.
    Walk,
}

// ---- What the person does to it -------------------------------------------------------------------

/// One change the person marks on the plan. Nothing changes the plan until the agent records it
/// again. `group` and `task` are ids from the version the marks are against.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlanMark {
    /// A group or, with `task`, a task of it, renamed.
    Rename {
        group: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        task: Option<String>,
        from: String,
        to: String,
    },
    Remove {
        group: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        task: Option<String>,
    },
    /// A new task in `group`. `id` is the person's own, for marks that name it again.
    AddTask { group: String, id: String, text: String },
    AddGroup { id: String, text: String },
    /// A task moved from one group to another.
    MoveTask { task: String, from: String, to: String },
    /// A group handed to another type of subagent.
    Subagent { group: String, from: String, to: String },
    /// Words pinned to a group or, with `task`, a task of it.
    Note {
        id: String,
        group: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        task: Option<String>,
        text: String,
    },
}

/// `approve_plan`: the person approves the plan. `version` is the one he read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovePlan {
    pub session_id: SessionId,
    pub version: u32,
}

/// `send_plan_changes`: the person's marks go to the agent as one message. `approve` approves the
/// plan with them, as Approve pressed with marks on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendPlanChanges {
    pub session_id: SessionId,
    pub version: u32,
    pub marks: Vec<PlanMark>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub approve: bool,
}

/// One line of the changes the person sent, against the group or task it is about.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanChangeLine {
    /// `Renamed`, `Removed`, `Added`, `Moved`, `Subagent` or `Note`.
    pub verb: String,
    /// The group, or `Group › task`, as the plan named it when he marked it.
    pub place: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

// ---- Who touched which file -----------------------------------------------------------------------

/// What was done to a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TouchAct {
    /// A file written whole: made, or replaced.
    Write,
    Edit,
    Remove,
}

/// The subagent that did it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TouchBy {
    /// Its type, as the call that started it named it.
    pub agent: String,
    /// The call that started it, which tells two subagents of one type apart.
    pub id: String,
}

/// One file touch Fleet saw, for the Field.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileTouch {
    /// The subagent that touched it. Absent where the Session's own agent did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<TouchBy>,
    /// From the repository's root where it lies under the Session's directory, else as the call
    /// named it.
    pub path: String,
    pub act: TouchAct,
    pub at: Instant,
}
