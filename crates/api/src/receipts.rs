//! What an agent is told after it acts on its own Session through the door.
//!
//! The row grows with every attachment, 10 KB at the median and past 60 KB for a long Session, and
//! an agent pastes each answer into its context for good. The receipt is the session id and what
//! the act set, so it weighs the same on the first call and the thousandth. Bridge's route answers
//! the row as it always did: the door alone sets [`crate::door::DoorCalled`].

use ipc::{SessionRecord, WaitingSource};
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct Receipt<'a> {
    session_id: &'a str,
    #[serde(flatten)]
    set: Set<'a>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum Set<'a> {
    WaitingFor {
        waiting_for: Vec<Item<'a>>,
    },
    Window {
        window: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<&'a str>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        takes: Vec<&'a str>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        steps: Vec<&'a str>,
    },
    Sketch {
        sketch: &'a str,
    },
    Plan {
        #[serde(skip_serializing_if = "Option::is_none")]
        plan: Option<PlanReceipt<'a>>,
    },
    Asked {
        #[serde(skip_serializing_if = "Option::is_none")]
        asked: Option<&'a str>,
    },
    Done {
        done: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        done_since: Option<&'a str>,
    },
}

/// What the agent reads when its plan is recorded and not yet put to the person, ahead of each
/// window that holds it.
const PLAN_HELD: &str = "The owner is not asked to approve this plan until he has approved every page you showed. A page you changed after showing it waits until you show it again with show_window.";

/// What settles a window that holds the plan, said after its address.
fn settled_by(why: ipc::PlanHoldWhy) -> &'static str {
    match why {
        ipc::PlanHoldWhy::Unapproved => "not approved since you last showed it. It is in his Needs you list, and his Approve settles it",
        ipc::PlanHoldWhy::Changing => "you are changing it. The end of your turn settles it",
        ipc::PlanHoldWhy::Changed => "changed since you last showed it. Show it again with show_window when it is ready, and his Approve settles it",
        ipc::PlanHoldWhy::Walk => "the walk this plan names. Show it with show_window if you have not, and his Approve settles it",
    }
}

/// [`PLAN_HELD`], then each window that holds the plan and what settles it.
fn plan_held(held_by: &[ipc::PlanHold]) -> String {
    let windows = held_by.iter().map(|one| format!(" {}: {}.", one.url, settled_by(one.why)));
    std::iter::once(String::from(PLAN_HELD)).chain(windows).collect()
}

/// The plan as the agent needs it back: the version, and each group and task with the id Fleet gave
/// it, which the agent sends again to keep it the same one through a rename.
#[derive(Serialize)]
struct PlanReceipt<'a> {
    version: u32,
    /// What the agent does with the tasks below: they are its task list, word for word.
    task_list: &'static str,
    /// Said while the plan is recorded but not yet put to the person, so the agent does not tell him
    /// it waits on his approval.
    #[serde(skip_serializing_if = "Option::is_none")]
    held: Option<String>,
    groups: Vec<GroupReceipt<'a>>,
}

#[derive(Serialize)]
struct GroupReceipt<'a> {
    id: &'a str,
    title: &'a str,
    tasks: Vec<TaskReceipt<'a>>,
}

#[derive(Serialize)]
struct TaskReceipt<'a> {
    id: &'a str,
    title: &'a str,
}

#[derive(Serialize)]
struct Item<'a> {
    id: &'a str,
    text: &'a str,
}

impl<'a> Receipt<'a> {
    /// The agent's own items as Fleet kept them: one of Fleet's that names the same target wins, and
    /// the agent's is then not among them.
    pub(crate) fn waiting_for(record: &'a SessionRecord) -> Self {
        let waiting_for = record
            .waiting_for
            .iter()
            .filter(|item| item.source == WaitingSource::Agent)
            .map(|item| Item {
                id: &item.id,
                text: &item.text,
            })
            .collect();
        Receipt::of(record, Set::WaitingFor { waiting_for })
    }

    pub(crate) fn window(record: &'a SessionRecord, show: &'a ipc::ShowWindow) -> Self {
        let set = Set::Window {
            window: &show.url,
            title: show.title.as_deref(),
            takes: show
                .variations
                .iter()
                .map(|take| take.name.as_str())
                .collect(),
            steps: show.steps.iter().map(|step| step.say.as_str()).collect(),
        };
        Receipt::of(record, set)
    }

    pub(crate) fn sketch(record: &'a SessionRecord, show: &'a ipc::ShowSketch) -> Self {
        Receipt::of(record, Set::Sketch { sketch: show.title.trim() })
    }

    pub(crate) fn plan(record: &'a SessionRecord) -> Self {
        let plan = record.plan.as_ref().map(|plan| PlanReceipt {
            version: plan.version,
            task_list: ipc::TASK_LIST_IS_THE_PLAN,
            held: (!plan.approved
                && !record.waiting_for.iter().any(|item| {
                    item.source == WaitingSource::Plan && item.id == format!("plan:{}", plan.version)
                }))
            .then(|| plan_held(&plan.held_by)),
            groups: plan
                .groups
                .iter()
                .map(|group| GroupReceipt {
                    id: &group.id,
                    title: &group.title,
                    tasks: group.tasks.iter().map(|task| TaskReceipt { id: &task.id, title: &task.text }).collect(),
                })
                .collect(),
        });
        Receipt::of(record, Set::Plan { plan })
    }

    /// The newest open question at the place the agent named.
    pub(crate) fn asked(record: &'a SessionRecord, ask: &ipc::AskInWindow) -> Self {
        let asked = record
            .waiting_for
            .iter()
            .rev()
            .filter(|item| item.source == WaitingSource::WindowAsk)
            .find(|item| {
                item.place.as_ref().is_some_and(|place| {
                    place.url == ask.url.trim() && place.selector == ask.selector.trim()
                })
            })
            .map(|item| item.id.as_str());
        Receipt::of(record, Set::Asked { asked })
    }

    pub(crate) fn done(record: &'a SessionRecord) -> Self {
        let done_since = record.done_since.as_ref().map(|at| at.as_str());
        Receipt::of(
            record,
            Set::Done {
                done: done_since.is_some(),
                done_since,
            },
        )
    }

    fn of(record: &'a SessionRecord, set: Set<'a>) -> Self {
        Receipt {
            session_id: record.id.as_str(),
            set,
        }
    }
}
