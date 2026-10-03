//! The three plan tools: `record_plan` for the step whose product is a plan,
//! and `add_task` and `update_task` for a step that follows it. Their own
//! module for [`ask`](mod@super::ask)'s reason.
//!
//! **A call reads as `core_model::PlanChange`**, the change the store appends,
//! so there is no second shape of a change between the tool and the record.
//! Whether this step may make it is the daemon's answer, not this module's.
//!
//! **No field names a Job, a step or who is changing the plan.** The step is
//! the connection's, and the author of a tool call is always a Drone.

use std::fmt;

use core_model::{Approach, NewTask, NotAnUpdate, PlanChange, TaskId, TaskTier, TaskUpdate};
use serde_json::{json, Map, Value};

use super::tools::{closed, filled, text, NotAnArgument};

pub const RECORD_PLAN_TOOL: &str = "record_plan";
pub const ADD_TASK_TOOL: &str = "add_task";
pub const UPDATE_TASK_TOOL: &str = "update_task";

pub const RECORD_PLAN_FIELDS: &[&str] = &["approach", "tasks"];
/// The fields of one entry of `record_plan`'s `tasks`. `group` may be left
/// out, and is the group of the task before it; `tier` may be, and is Armada
/// picking the model; `concurrent_with` may be, and the task runs alone.
pub const TASK_FIELDS: &[&str] = &[
    "title",
    "note",
    "scope",
    "expects",
    "group",
    "tier",
    "concurrent_with",
];
pub const ADD_TASK_FIELDS: &[&str] = &["title", "note", "scope", "expects", "after"];
pub const UPDATE_TASK_FIELDS: &[&str] = &["task", "state", "reason", "shown"];

/// One call of a plan tool, read as the change it asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanCall {
    /// Which of the three was called. Fleet gives each to a different step.
    pub tool: &'static str,
    pub change: PlanChange,
}

/// Why a plan tool's arguments did not read as a change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanArgument {
    NotATaskList,
    NotAPathList,
    NotATaskId {
        field: &'static str,
        named: String,
    },
    NotAnUpdate(NotAnUpdate),
    /// A task's `group` that is not a positive whole number.
    NotAGroup,
    /// A task's `tier` that is not one of the three.
    NotATier,
    /// A task's `concurrent_with` that is not a list of task numbers.
    NotBeside,
}

impl fmt::Display for PlanArgument {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlanArgument::NotATaskList => out.write_str(
                "`tasks` is not a list of tasks. Each entry is an object with `title`, \
                 `note`, `scope` and `expects`",
            ),
            PlanArgument::NotAPathList => out.write_str(
                "`scope` is not a list of repository-relative paths. Send [] where the \
                 task's files are not known yet, rather than naming them in `note`",
            ),
            PlanArgument::NotATaskId { field, named } => write!(
                out,
                "`{field}` is `{named}`, which is not a task id. Task ids are `T1`, \
                 `T2` and so on, as the plan numbered them"
            ),
            PlanArgument::NotAGroup => out.write_str(
                "`group` is not a whole number of 1 or more. Number the groups 1, 2 and so \
                 on in the order they run, or leave `group` out to keep a task in the group \
                 of the task before it",
            ),
            PlanArgument::NotBeside => out.write_str(
                "`concurrent_with` is a list of the numbers of other tasks in the same group, \
                 1 for the first task you list, 2 for the second. Leave it out where the task \
                 runs alone",
            ),
            PlanArgument::NotATier => out.write_str(
                "`tier` is one of `difficult`, `medium` or `easy`. Leave it out where you \
                 cannot say how hard the task is, and Armada picks its model",
            ),
            PlanArgument::NotAnUpdate(NotAnUpdate::NoSuchState { named }) => write!(
                out,
                "`state` is `{named}`. It is one of `open`, `working`, `done` or `dropped`"
            ),
            PlanArgument::NotAnUpdate(NotAnUpdate::FleetMarksIt { state }) => write!(
                out,
                "`state` is `{}`, which Armada marks itself. Send `open`, `working`, \
                 `done` or `dropped`",
                state.as_wire()
            ),
            PlanArgument::NotAnUpdate(NotAnUpdate::DroppedWithoutAReason) => out.write_str(
                "`dropped` needs a `reason`: say in a sentence why the task will not be \
                 done, and call again",
            ),
            PlanArgument::NotAnUpdate(NotAnUpdate::ReasonWithoutADrop { state }) => write!(
                out,
                "`reason` is kept only for `dropped`, and this sets the task to `{}`. \
                 Send \"\" as the reason and call again",
                state.as_wire()
            ),
        }
    }
}

/// A call of one of the three, or `None` where `tool` is not one of them.
pub(super) fn read(
    tool: &'static str,
    arguments: &Map<String, Value>,
) -> Option<Result<PlanCall, NotAnArgument>> {
    let change = match tool {
        RECORD_PLAN_TOOL => recorded(arguments),
        ADD_TASK_TOOL => added(arguments),
        UPDATE_TASK_TOOL => updated(arguments),
        _ => return None,
    };
    Some(change.map(|change| PlanCall { tool, change }))
}

fn not_a_list() -> NotAnArgument {
    NotAnArgument::Planning(PlanArgument::NotATaskList)
}

fn recorded(arguments: &Map<String, Value>) -> Result<PlanChange, NotAnArgument> {
    closed(arguments, RECORD_PLAN_TOOL, RECORD_PLAN_FIELDS)?;
    let approach = Approach::new(&filled(arguments, "approach")?)
        .ok_or(NotAnArgument::Blank { field: "approach" })?;
    let listed = arguments
        .get("tasks")
        .ok_or(NotAnArgument::Missing { field: "tasks" })?
        .as_array()
        .ok_or_else(not_a_list)?;
    let mut tasks = Vec::with_capacity(listed.len());
    for entry in listed {
        let entry = entry.as_object().ok_or_else(not_a_list)?;
        closed(entry, RECORD_PLAN_TOOL, TASK_FIELDS)?;
        let group = match entry.get("group") {
            None => 0,
            Some(number) => number
                .as_u64()
                .and_then(|n| u32::try_from(n).ok())
                .filter(|n| *n > 0)
                .ok_or(NotAnArgument::Planning(PlanArgument::NotAGroup))?,
        };
        let tier = match entry.get("tier") {
            None => None,
            Some(named) => Some(
                named
                    .as_str()
                    .and_then(TaskTier::from_wire)
                    .ok_or(NotAnArgument::Planning(PlanArgument::NotATier))?,
            ),
        };
        let beside = match entry.get("concurrent_with") {
            None => Vec::new(),
            Some(listed) => listed
                .as_array()
                .and_then(|listed| {
                    listed
                        .iter()
                        .map(|n| {
                            n.as_u64()
                                .and_then(|n| u32::try_from(n).ok())
                                .filter(|n| *n > 0)
                        })
                        .collect::<Option<Vec<u32>>>()
                })
                .ok_or(NotAnArgument::Planning(PlanArgument::NotBeside))?,
        };
        tasks.push(task(entry)?.in_group(group).at_tier(tier).beside(&beside));
    }
    Ok(PlanChange::Recorded { approach, tasks })
}

fn task(arguments: &Map<String, Value>) -> Result<NewTask, NotAnArgument> {
    let scope = paths(arguments)?;
    let scope: Vec<&str> = scope.iter().map(String::as_str).collect();
    NewTask::new(
        &filled(arguments, "title")?,
        &text(arguments, "note")?,
        &scope,
        &text(arguments, "expects")?,
    )
    .ok_or(NotAnArgument::Blank { field: "title" })
}

/// `scope`, as the list of strings it has to be. **An entry that is not a
/// string refuses the call** rather than being dropped: a path silently lost
/// here is a file the next step is never told about.
fn paths(arguments: &Map<String, Value>) -> Result<Vec<String>, NotAnArgument> {
    let listed = arguments
        .get("scope")
        .ok_or(NotAnArgument::Missing { field: "scope" })?
        .as_array()
        .ok_or(NotAnArgument::Planning(PlanArgument::NotAPathList))?;
    listed
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .map(String::from)
                .ok_or(NotAnArgument::Planning(PlanArgument::NotAPathList))
        })
        .collect()
}

/// `after` is required and may be empty, which is the end of the list.
fn added(arguments: &Map<String, Value>) -> Result<PlanChange, NotAnArgument> {
    closed(arguments, ADD_TASK_TOOL, ADD_TASK_FIELDS)?;
    let task = task(arguments)?;
    let after = match text(arguments, "after")?.trim() {
        "" => None,
        named => Some(task_id("after", named)?),
    };
    Ok(PlanChange::Added { task, after })
}

fn updated(arguments: &Map<String, Value>) -> Result<PlanChange, NotAnArgument> {
    closed(arguments, UPDATE_TASK_TOOL, UPDATE_TASK_FIELDS)?;
    let task = task_id("task", &text(arguments, "task")?)?;
    let to = TaskUpdate::read(&text(arguments, "state")?, &text(arguments, "reason")?)
        .map_err(|why| NotAnArgument::Planning(PlanArgument::NotAnUpdate(why)))?;
    let shown = core_model::Shown::new(&text(arguments, "shown")?);
    Ok(PlanChange::Updated { task, to, shown })
}

fn task_id(field: &'static str, named: &str) -> Result<TaskId, NotAnArgument> {
    TaskId::read(named.trim()).ok_or_else(|| {
        NotAnArgument::Planning(PlanArgument::NotATaskId {
            field,
            named: named.to_string(),
        })
    })
}

/// **The description says it is not the report**, because a Drone that read a
/// recorded plan as a finished step would stop before submitting.
pub(super) fn record_plan_tool() -> Value {
    json!({
        "name": RECORD_PLAN_TOOL,
        "description":
            "Record the plan for this Job: the approach in a paragraph, and the \
             tasks it breaks into in the order they will be done, in groups. Each \
             group's tasks are done one after another, save those you mark as safe \
             to do at the same time, and its Checks run once they all are, so a \
             group is tasks that only make sense checked together. \
             Fleet keeps it, and the parts after this one work from it and keep \
             each task's state current. Tasks are named T1, T2 and so on in the \
             order you give them. Calling it again replaces the whole plan, so \
             correct a plan by recording it again. It does not finish this part — \
             submit_evidence still does.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "approach": {
                    "type": "string",
                    "description": "How the work will be done, in a paragraph a person can read.",
                },
                "tasks": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "title": {
                                "type": "string",
                                "description": "One line saying what the task is.",
                            },
                            "note": {
                                "type": "string",
                                "description": "One line for what the other fields cannot \
                                    hold -- the exact new wording, a gotcha you found. \
                                    Not the files, which are `scope`. \"\" if there is none.",
                            },
                            "scope": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "The repository-relative paths this task \
                                    touches. The part that does this task is given them and \
                                    starts from them instead of searching for them again. \
                                    [] where you genuinely do not know yet.",
                            },
                            "expects": {
                                "type": "string",
                                "description": "What should prove this task is done: a \
                                    named test, a rendered string, a command and its exit \
                                    code. Whoever does the task records what actually \
                                    proved it, and the two are read side by side. \"\" if \
                                    you cannot say yet.",
                            },
                            "group": {
                                "type": "integer",
                                "minimum": 1,
                                "description": "Which group the task is in: 1, 2 and so on, \
                                    in the order the groups run, never going back. Leave it \
                                    out to keep the task in the group of the task before \
                                    it; a plan naming no group is one group.",
                            },
                            "tier": {
                                "type": "string",
                                "enum": ["difficult", "medium", "easy"],
                                "description": "How hard the task is, which picks the model \
                                    the part doing it runs on: difficult for a task that needs \
                                    the most careful reasoning, easy for a mechanical one. \
                                    Leave it out where you cannot say, and Armada picks.",
                            },
                            "concurrent_with": {
                                "type": "array",
                                "items": { "type": "integer", "minimum": 1 },
                                "description": "Other tasks in the same group that may be \
                                    done at the same time as this one, by their place in this \
                                    list: 1 for the first task. Tasks done at once share one \
                                    copy of the repository, so name only tasks that change \
                                    different files and do not need each other's work. Two \
                                    that edit one file anyway are done again one after the \
                                    other. Leave it out to do this task on its own.",
                            },
                        },
                        "required": ["title", "note", "scope", "expects"],
                        "additionalProperties": false,
                    },
                    "description": "The tasks, in the order they will be done.",
                },
            },
            "required": ["approach", "tasks"],
            "additionalProperties": false,
        },
    })
}

pub(super) fn add_task_tool() -> Value {
    json!({
        "name": ADD_TASK_TOOL,
        "description":
            "Add a task to the plan you are working from, when the work turns out \
             to need one the plan does not name. It is added open, after the task \
             you name or at the end, and the call answers with its id.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "title": { "type": "string", "description": "One line saying what the task is." },
                "note": {
                    "type": "string",
                    "description": "One line for what the other fields cannot hold. \"\" if none.",
                },
                "scope": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "The repository-relative paths this task touches. [] if \
                        you do not know yet.",
                },
                "expects": {
                    "type": "string",
                    "description": "What should prove this task is done. \"\" if you cannot \
                        say yet.",
                },
                "after": {
                    "type": "string",
                    "description": "The id of the task it comes after, such as T2. \"\" for the end.",
                },
            },
            "required": ["title", "note", "scope", "expects", "after"],
            "additionalProperties": false,
        },
    })
}

/// **The description says a state decides nothing**: the diff is still what is
/// checked, and a Drone that believed otherwise would mark its way to a pass.
pub(super) fn update_task_tool() -> Value {
    json!({
        "name": UPDATE_TASK_TOOL,
        "description":
            "Say where one task of the plan stands: working when you start it, done \
             when it is done, open to put it back, or dropped with a reason when it \
             will not be done. A task's state is your account of the work. It \
             passes nothing and advances nothing; the work itself is what is \
             checked.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "task": { "type": "string", "description": "The task's id, such as T3." },
                "state": {
                    "type": "string",
                    "enum": ["open", "working", "done", "dropped"],
                },
                "reason": {
                    "type": "string",
                    "description": "Why the task is dropped. Required for dropped, \"\" otherwise.",
                },
                "shown": {
                    "type": "string",
                    "description": "What actually proved this task, when you mark it done \
                        -- the test that covers it, the string it now renders. It is \
                        read beside what the plan expected, and saying something different \
                        from the plan is the useful answer, not a wrong one. \"\" otherwise.",
                },
            },
            "required": ["task", "state", "reason", "shown"],
            "additionalProperties": false,
        },
    })
}
