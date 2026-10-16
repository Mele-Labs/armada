//! What the person marked on a Session's plan: checked against the version he read, and worded as the
//! one message the agent gets. `docs/concepts/session.md`, *The plan*. Pure; `plan_record` holds the
//! plan itself.
//!
//! **A mark is a proposal and changes nothing.** The agent weighs each against its group or task,
//! may push back, and records the plan again.

use ipc::{PlanChangeLine, PlanMark, MOST_MARKS};
use store::KeptPlan;

/// The longest text a mark carries: a note is the one that runs long.
const TEXT_AT_MOST: usize = 2_000;

/// One mark, worded twice: for the thread's row, and for the agent, which needs the ids to answer by.
pub(crate) struct Said {
    pub line: PlanChangeLine,
    pub agent: String,
}

/// What the plan holds, and what the marks add to it, by id.
struct Names<'a> {
    plan: &'a KeptPlan,
    groups: Vec<(String, String)>,
    tasks: Vec<(String, String, String)>,
}

impl<'a> Names<'a> {
    fn of(plan: &'a KeptPlan, marks: &[PlanMark]) -> Self {
        let mut names = Names { plan, groups: Vec::new(), tasks: Vec::new() };
        for group in &plan.groups {
            names.groups.push((group.id.clone(), group.title.clone()));
            for task in &group.tasks {
                names.tasks.push((group.id.clone(), task.id.clone(), task.text.clone()));
            }
        }
        for mark in marks {
            match mark {
                PlanMark::AddGroup { id, text } => names.groups.push((id.clone(), text.clone())),
                PlanMark::AddTask { group, id, text } => names.tasks.push((group.clone(), id.clone(), text.clone())),
                _ => {}
            }
        }
        names
    }

    fn group(&self, id: &str) -> Result<&str, String> {
        self.groups
            .iter()
            .find(|(held, _)| held == id)
            .map(|(_, title)| title.as_str())
            .ok_or_else(|| format!("plan v{} holds no group `{id}`", self.plan.version))
    }

    /// A task of the group `group`, or an unplaced `task` id the plan holds under another.
    fn task(&self, group: &str, id: &str) -> Result<&str, String> {
        self.tasks
            .iter()
            .find(|(home, held, _)| held == id && home == group)
            .map(|(_, _, text)| text.as_str())
            .ok_or_else(|| format!("group `{group}` holds no task `{id}` in plan v{}", self.plan.version))
    }

    /// `"Group"` or `"Group › task"`, for the thread; and the same with the ids, for the agent.
    fn place(&self, group: &str, task: Option<&str>) -> Result<(String, String), String> {
        let title = self.group(group)?;
        match task {
            None => Ok((title.to_string(), format!("\"{title}\" ({group})"))),
            Some(task) => {
                let text = self.task(group, task)?;
                Ok((format!("{title} › {text}"), format!("\"{title}\" ({group}) › \"{text}\" ({task})")))
            }
        }
    }
}

fn line(verb: &str, place: String, from: Option<String>, text: Option<String>) -> PlanChangeLine {
    PlanChangeLine { verb: verb.to_string(), place, from, text }
}

fn blank(what: &str, text: &str) -> Result<(), String> {
    match (text.trim().is_empty(), text.chars().count() > TEXT_AT_MOST) {
        (true, _) => Err(format!("a mark has a blank {what}")),
        (_, true) => Err(format!("a {what} is longer than {TEXT_AT_MOST} characters")),
        _ => Ok(()),
    }
}

/// The marks against `plan`, each worded, or the first reason one cannot be: it names a group or task
/// the version does not hold, or says nothing.
pub(crate) fn checked(plan: &KeptPlan, marks: &[PlanMark], approve: bool) -> Result<Vec<Said>, String> {
    if marks.is_empty() && !approve {
        return Err(String::from("there is nothing to send: no marks and no approval"));
    }
    if marks.len() > MOST_MARKS {
        return Err(format!("{} marks, and one message carries at most {MOST_MARKS}", marks.len()));
    }
    let names = Names::of(plan, marks);
    marks
        .iter()
        .map(|mark| {
            Ok(match mark {
                PlanMark::Rename { group, task, from, to } => {
                    blank("new name", to)?;
                    let (place, agent) = names.place(group, task.as_deref())?;
                    Said {
                        agent: format!("Renamed {agent}: \"{from}\" to \"{to}\""),
                        line: line("Renamed", place, Some(from.clone()), Some(to.clone())),
                    }
                }
                PlanMark::Remove { group, task } => {
                    let (place, agent) = names.place(group, task.as_deref())?;
                    Said { agent: format!("Removed {agent}"), line: line("Removed", place, None, None) }
                }
                PlanMark::AddTask { group, text, .. } => {
                    blank("task", text)?;
                    let (place, agent) = names.place(group, None)?;
                    Said {
                        agent: format!("Added to {agent}: \"{text}\""),
                        line: line("Added", place, None, Some(text.clone())),
                    }
                }
                PlanMark::AddGroup { text, .. } => {
                    blank("group", text)?;
                    Said {
                        agent: format!("Added a group: \"{text}\""),
                        line: line("Added", String::from("Plan"), None, Some(text.clone())),
                    }
                }
                PlanMark::MoveTask { task, from, to } => {
                    let text = names.task(from, task)?;
                    let (from_title, to_title) = (names.group(from)?, names.group(to)?);
                    Said {
                        agent: format!(
                            "Moved \"{text}\" ({task}) from \"{from_title}\" ({from}) to \"{to_title}\" ({to})"
                        ),
                        line: line("Moved", text.to_string(), Some(from_title.to_string()), Some(to_title.to_string())),
                    }
                }
                PlanMark::Subagent { group, from, to } => {
                    blank("subagent type", to)?;
                    let (place, agent) = names.place(group, None)?;
                    Said {
                        agent: format!("Subagent for {agent}: {from} to {to}"),
                        line: line("Subagent", place, Some(from.clone()), Some(to.clone())),
                    }
                }
                PlanMark::Note { group, task, text, .. } => {
                    blank("note", text)?;
                    let (place, agent) = names.place(group, task.as_deref())?;
                    Said { agent: format!("Note on {agent}: {text}"), line: line("Note", place, None, Some(text.clone())) }
                }
            })
        })
        .collect()
}

/// The one message the agent gets for the marks. `approve` says the plan is approved with them.
pub(crate) fn message(plan: &KeptPlan, approve: bool, said: &[Said]) -> String {
    let version = plan.version;
    let head = match approve {
        true => format!("The owner approved plan v{version} with changes marked on it."),
        false => format!("The owner marked changes on plan v{version}."),
    };
    let how = match (approve, said.is_empty()) {
        (_, true) => String::new(),
        (true, false) => "\nThey are proposals. Weigh each and say where you disagree; build the plan as recorded and record it again if you take any.".to_string(),
        (false, false) => "\nThey are proposals. Weigh each, say where you disagree, and record the plan again if you take any. The plan stays as recorded until you do.".to_string(),
    };
    let lines: String = said.iter().map(|one| format!("\n- {}", one.agent)).collect();
    // Approved with marks, the plan as recorded is what is built: its tasks are the agent's list.
    let build = if approve { format!("\n\n{}", the_tasks(plan)) } else { String::new() };
    format!("{head}{how}{lines}{build}")
}

/// The plan's tasks, verbatim and in order, with what the agent does with them.
fn the_tasks(plan: &KeptPlan) -> String {
    let tasks: String = plan.groups.iter().flat_map(|group| &group.tasks).map(|task| format!("\n- {}", task.text)).collect();
    format!("{} The tasks:{tasks}", ipc::TASK_LIST_IS_THE_PLAN)
}

/// The one message for an approval with no marks.
pub(crate) fn approved_message(plan: &KeptPlan) -> String {
    format!(
        "The owner approved plan v{}. Build it as recorded, and record it again if the work shows it is wrong.\n\n{}",
        plan.version,
        the_tasks(plan)
    )
}
