//! The proposer at a Session's dispatch: what it is asked, how its answer reads, and the words
//! both it and the Job proposer share. `docs/concepts/session.md`, *A Session's proposal*.
//!
//! **Nothing here names a Job.** A Session is one conversation and is never split, so its
//! answer is [`SessionProposed`] and not a plan of Jobs, and a reading that cannot be used is
//! [`NamesNoWorkflow`] and not a refusal of a request. The Job proposer in `crate::proposing`
//! lays its question with [`question_for`] and reads its settings lines with [`setting`], so
//! the one table that fills the placeholders serves both.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use adapter_traits::{Environment, Model, ModelClient};
use config::ResolvedWorkflow;
use core_model::{Ulid, WorkflowId};
use verification::field;

use crate::judging::JudgeBudget;
use crate::prompts::fill;

/// The block a Session's answer owes. **Three lines, once**: a Session is one conversation and
/// is never split.
///
/// `none` for the workflow is a custom Session and a legitimate answer, not a failure: a
/// Session started on a near miss would be held to steps that are not its work, so the format
/// asks for the nearest fit to be refused exactly as the Job's does. A model is asked for by
/// how hard the work reads, from the list the question carries.
const SESSION_ANSWER_FORMAT: &str = "\
Answer with nothing but the block below.

    workflow: <the id, spelled exactly as it appears above, or `none`>
    title: <what to call this Job, in a few of the words the request used>
    settings: model=<one of the models listed above, spelled exactly>, \
effort=<low, medium or high>

Write the lines in the order above.

Answer `workflow: none` where no workflow above fits the request. A Job with \
no workflow works without steps laid out for it, and a near miss would hold it to \
steps that are not its work.

Pick the model and the effort by how hard the work reads. A small change to a \
file the request names wants the lightest model and the lowest effort; work \
across many files, a design or a hard diagnosis wants the strongest model and a \
higher effort. Leave a pair out where you cannot tell.";

/// The Session proposer's prompt as it ships. **Not an editable setting**: `prompts.proposer`
/// is the Job's, and this reads the same three placeholders, so the one table that fills them
/// serves both.
pub(crate) fn session_shipped() -> String {
    format!(
        "You are deciding what kind of work a person just asked an agent to do, so \
         that the agent starts on the right footing. Answer only the question at \
         the end.\n\n\
         The request, as the person wrote it:\n\n{{request}}\n\n\
         The workflows this repository holds, and the steps each runs:\n\n{{workflows}}\n\n\
         The models this machine can run the agent on:\n\n{{models}}\n\n{SESSION_ANSWER_FORMAT}"
    )
}

/// What a Session's proposer picked. **Each field is absent where it picked nothing a Session
/// can use**: a workflow nothing holds, a model this machine does not run and an effort the
/// CLI does not take are all absent rather than refused, and an absent field is the machine's
/// own default. No arm of this means "the usual workflow".
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SessionProposed {
    pub title: Option<String>,
    pub workflow: Option<WorkflowId>,
    pub model: Option<String>,
    pub effort: Option<String>,
}

/// The longest title a proposer may give a Session, in characters.
const MOST_TITLE: usize = 80;

/// How many of his renames the question carries.
pub(crate) const RENAMES_SHOWN: usize = 12;

/// How much of a renamed Job's request the question carries, in characters.
const MOST_RENAMED_REQUEST: usize = 200;

/// The person's renames as examples, ahead of the answer format. Empty where he has renamed none,
/// so the question is as it was before he did.
fn names_he_gave(renames: &[store::KeptRename]) -> String {
    if renames.is_empty() {
        return String::new();
    }
    let mut said = String::from(
        "The person renamed these Jobs after Armada named them. Name this Job the way he names his: \
         his words, his order, his length. Where his names differ from the title line's advice \
         below, follow his names.\n\n",
    );
    for one in renames {
        let request: String = one
            .request
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(MOST_RENAMED_REQUEST)
            .collect();
        said.push_str(&format!(
            "  in {}, asked: {request}\n    Armada named it: {}\n    he renamed it: {}\n",
            one.repository, one.was, one.title
        ));
    }
    said.push('\n');
    said
}

/// A Session's question, assembled. **Held rather than passed as a bare string**, for the
/// reason `verification::Brief` is: what the proposer is told is built in one place and asserted
/// on without a model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionBrief {
    question: String,
}

/// An answer with no `workflow:` line in it, which did not follow the format. The caller falls
/// back from it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NamesNoWorkflow;

impl fmt::Display for NamesNoWorkflow {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str("the answer names no workflow at all, so nothing was proposed either way")
    }
}

impl std::error::Error for NamesNoWorkflow {}

impl SessionBrief {
    /// The question for a Session: the request, the workflows held, the models run and the Jobs
    /// the person renamed. **His renames go in after the placeholders are filled**, so a brace in
    /// his words is never read as one.
    pub fn about(
        request: &str,
        workflows: &BTreeMap<WorkflowId, ResolvedWorkflow>,
        models: &[String],
        renames: &[store::KeptRename],
    ) -> SessionBrief {
        let mut question = question_for(&session_shipped(), request, workflows, models);
        if let Some(asked) = question.strip_suffix(SESSION_ANSWER_FORMAT) {
            question = format!("{asked}{}{SESSION_ANSWER_FORMAT}", names_he_gave(renames));
        }
        SessionBrief { question }
    }

    /// The text that goes to the model, exactly as it goes.
    pub fn question(&self) -> &str {
        &self.question
    }

    /// Read a Session's answer back. **Total except where it names no workflow line at all**,
    /// which is an answer that did not follow the format and is `Err` for the caller to fall
    /// back from. Everything else degrades one field at a time: `none`, a workflow this
    /// repository does not hold, a model this machine does not run and an effort the CLI does
    /// not take each leave that field absent.
    pub fn read(
        &self,
        answer: &str,
        held: &BTreeMap<WorkflowId, ResolvedWorkflow>,
        models: &[String],
    ) -> Result<SessionProposed, NamesNoWorkflow> {
        let Some(named) = field(answer, "workflow") else {
            return Err(NamesNoWorkflow);
        };
        let workflow = Some(WorkflowId::carried(Ulid::carried(named)))
            .filter(|id| held.contains_key(id));
        Ok(SessionProposed {
            title: field(answer, "title")
                .map(|title| title.chars().take(MOST_TITLE).collect::<String>())
                .filter(|title| !title.trim().is_empty()),
            workflow,
            model: named_model(answer).filter(|named| models.iter().any(|held| held == named)),
            effort: setting(answer, "effort")
                .map(|said| said.to_ascii_lowercase())
                .filter(|said| core_model::Effort::from_wire(said).is_some()),
        })
    }
}

/// The question both proposers put: the request, the workflows this repository holds with the
/// steps each runs, and the models this machine can run, filled into `prompt`'s placeholders.
pub(crate) fn question_for(
    prompt: &str,
    request: &str,
    workflows: &BTreeMap<WorkflowId, ResolvedWorkflow>,
    models: &[String],
) -> String {
    let mut held = String::new();
    if workflows.is_empty() {
        held.push_str("  (this repository holds none)\n");
    }
    for workflow in workflows.values() {
        held.push_str(&format!(
            "  {} — {}\n",
            workflow.id().as_str(),
            workflow.name()
        ));
        // **What it is for, where the definition says, and in the words a
        // requester would use.** The steps below tell one workflow from
        // its neighbours; they do not say whether a request is this kind
        // of work at all. A milestone asked for by link was declined on
        // 3 Sep 2026 while `epic` sat on this list, because nothing in
        // `Plan the wave` is a word the request used. #424.
        //
        // Absent writes no line, so a definition that declares nothing is
        // offered exactly as it was before the key existed.
        if let Some(asked) = workflow.for_requests() {
            held.push_str(&format!("    for: {asked}\n"));
        }
        // The steps are how a workflow is told apart from its five
        // neighbours. A name alone separates `bug` from `revert` and does
        // not separate `feature` from `refactor`.
        let steps: Vec<&str> = workflow.steps().iter().map(|step| step.label()).collect();
        held.push_str(&format!("    {}\n", steps.join(" -> ")));
    }
    // **The models this machine actually holds, so the settings line can
    // name one rather than invent one.** The owner chose on 30 Sep 2026 to
    // let this call pick the model, over the argument that a model choosing
    // which model runs the work is a dial deciding every later cost — and
    // the guard he took with it is that it picks from the list. A name that
    // is not here is refused exactly as a workflow id that is not held is.
    let mut runs = String::new();
    if models.is_empty() {
        runs.push_str("  (this machine has named none, so leave `model` out)\n");
    }
    for model in models {
        runs.push_str(&format!("  {model}\n"));
    }
    fill(
        prompt,
        &[
            ("request", request),
            ("workflows", held.trim_end_matches('\n')),
            ("models", runs.trim_end_matches('\n')),
        ],
    )
}

/// One `key=value` pair off the `settings` line.
///
/// `key=value`, comma separated, so a second setting is a second pair rather
/// than a second line — which is what keeps the four fields four.
pub(crate) fn setting(block: &str, key: &str) -> Option<String> {
    let said = field(block, "settings")?;
    said.split(',')
        .filter_map(|pair| pair.split_once('='))
        .find(|(named, _)| named.trim().eq_ignore_ascii_case(key))
        .map(|(_, value)| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// What the `settings` line named as the model, as it spelled it.
///
/// **Unchecked here.** Whether this machine holds it is the caller's, because
/// the two callers answer differently: the Job's `Brief::read` refuses the plan and
/// [`Settled`] simply does not settle the field — which is what the workflow id
/// one line up already does in each of them.
pub(crate) fn named_model(block: &str) -> Option<String> {
    setting(block, "model")
}

/// Everything one call needs in order to ask.
///
/// One client, one budget, Fleet's own environment, and a model of its own: the model is a
/// separate dial.
#[derive(Clone)]
pub struct Proposing {
    pub client: Arc<dyn ModelClient + Send + Sync>,
    pub budget: JudgeBudget,
    /// What a request is read by. Resolved by the composition root — which
    /// model is cheap is a vendor's fact and nothing below Fleet may spell one.
    pub model: Model,
    /// Fleet's own, because the call authenticates as Fleet.
    pub environment: Environment,
    /// Every model a Session on this machine may be started as —
    /// `ipc::ModelChoices::models`, as `list_models` serves it.
    ///
    /// **Not the dial above.** `model` is what reads the request; this is what the request's
    /// Session will be worked by. A list rather than a free field, so the proposer cannot name
    /// a model nothing can run.
    pub choices: Vec<String>,
}

// ---- A Session that waits on another ----------------------------------------------------------------------

/// A group or task of the plan of the Session waited on, offered to the reader to name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanPart {
    pub id: String,
    pub title: String,
    /// A group, else a task.
    pub group: bool,
}

/// What a brief that tags a Session owes. **Five lines**, in order.
const WAIT_ANSWER_FORMAT: &str = "\
Answer with nothing but the block below.

    plain: <yes or no>
    condition: <ends, group_done, task_done, pr_pushed or pr_lands>
    part: <the id of the group or task, spelled exactly as listed above, or `none`>
    base: <main or branch>
    needs: <what the new Job needs from that Job, in a few words, or `none`>

`plain` is `yes` where the request only refers to that Job and says nothing about \
waiting for it or for anything it does, and `no` where the new Job is to start after \
something it does. A plain request has `condition: ends`. \
`ends` is the default: the new Job starts when that Job ends. `group_done` and \
`task_done` mean a group or a task of its plan is done, and name it in `part`; \
`pr_pushed` means its pull request is pushed; `pr_lands` means its pull request merges. \
Say `pr_pushed` or `pr_lands` only where the request says pull request, PR, push or \
land. `base` is `branch` where the request asks to start from that Job's branch, and \
`main` otherwise. `needs` is only what the request asks of that Job.";

/// The question a brief that tags a Session puts, and the reading of its answer. **Total**: whatever
/// the answer says, the reading is one the wire holds, and what cannot be used is the default.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaitBrief {
    question: String,
    parts: Vec<PlanPart>,
}

/// The longest need Fleet keeps, in characters.
const MOST_NEED: usize = 160;

impl WaitBrief {
    pub fn about(request: &str, on: &str, parts: &[PlanPart]) -> WaitBrief {
        let mut listed = String::new();
        if parts.is_empty() {
            listed.push_str("  (it has recorded no plan, so `part` is `none`)\n");
        }
        for part in parts {
            listed.push_str(&format!("  {} {} - {}\n", part.id, if part.group { "group" } else { "task" }, part.title));
        }
        let question = format!(
            "A person is dispatching a new agent Job and tagged another Job, \"{on}\", with @ in \
             the request. The request may only refer to it, or it may ask the new Job to wait for it. Read from \
             the request whether it waits, when the new Job starts and what it needs of \"{on}\". Answer only the question at the end.\n\n\
             The request, as the person wrote it:\n\n{request}\n\n\
             The plan of \"{on}\", its groups and tasks with their ids:\n\n{}\n\n{WAIT_ANSWER_FORMAT}",
            listed.trim_end_matches('\n')
        );
        WaitBrief { question, parts: parts.to_vec() }
    }

    /// The text that goes to the model, exactly as it goes.
    pub fn question(&self) -> &str {
        &self.question
    }

    /// Read the answer back. A condition that names a part the plan does not hold is `ends`.
    pub fn read(&self, answer: &str) -> ipc::WaitReading {
        let said = |key: &str| field(answer, key).map(|one| one.trim().to_string()).filter(|one| !one.eq_ignore_ascii_case("none") && !one.is_empty());
        let part = said("part").and_then(|id| self.parts.iter().find(|one| one.id == id));
        let condition = match said("condition").map(|one| one.to_ascii_lowercase()).as_deref() {
            Some("group_done") => part.filter(|one| one.group).map(|one| ipc::StartCondition::GroupDone { group: one.id.clone() }),
            Some("task_done") => part.filter(|one| !one.group).map(|one| ipc::StartCondition::TaskDone { task: one.id.clone() }),
            Some("pr_pushed") => Some(ipc::StartCondition::PrPushed),
            Some("pr_lands") => Some(ipc::StartCondition::PrLands),
            _ => None,
        }
        .unwrap_or_default();
        let base = match said("base").map(|one| one.to_ascii_lowercase()).as_deref() {
            Some("branch") => ipc::StartBase::Branch,
            Some("main") => ipc::StartBase::Main,
            _ if condition == ipc::StartCondition::PrPushed => ipc::StartBase::Branch,
            _ => ipc::StartBase::Main,
        };
        let needs = said("needs").map(|one| one.chars().take(MOST_NEED).collect::<String>()).unwrap_or_default();
        // Plain only where the answer says so and the words named nothing: a condition is a wait.
        let plain = condition == ipc::StartCondition::Ends && said("plain").is_some_and(|one| one.eq_ignore_ascii_case("yes"));
        ipc::WaitReading { condition, base, needs, plain }
    }
}

/// What a reading that could not be had says: it ends, from main, asking nothing.
pub fn unread_wait() -> ipc::WaitReading {
    ipc::WaitReading { condition: ipc::StartCondition::Ends, base: ipc::StartBase::Main, needs: String::new(), plain: false }
}
