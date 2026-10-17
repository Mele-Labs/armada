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
    title: <what to call this Session, in a few of the words the request used>
    settings: model=<one of the models listed above, spelled exactly>, \
effort=<low, medium or high>

Write the lines in the order above.

Answer `workflow: none` where no workflow above fits the request. A Session with \
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
    /// The question for a Session: the request, the workflows held and the models run.
    pub fn about(
        request: &str,
        workflows: &BTreeMap<WorkflowId, ResolvedWorkflow>,
        models: &[String],
    ) -> SessionBrief {
        SessionBrief {
            question: question_for(&session_shipped(), request, workflows, models),
        }
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
/// The same four fields as [`Judging`](crate::Judging), and three of them the
/// same values: one client, one budget, Fleet's own environment. Only the model
/// differs, because only the model is a separate dial.
#[derive(Clone)]
pub struct Proposing {
    pub client: Arc<dyn ModelClient + Send + Sync>,
    pub budget: JudgeBudget,
    /// What a request is read by. Resolved by the composition root — which
    /// model is cheap is a vendor's fact and nothing below Fleet may spell one.
    pub model: Model,
    /// Fleet's own, because the call authenticates as Fleet.
    pub environment: Environment,
    /// Every model a Drone on this machine may be spawned as —
    /// `ipc::ModelChoices::models`, as `list_models` serves it.
    ///
    /// **Not the dial above.** `model` is what reads the request; this is what
    /// the request's Job will be worked by, and the owner's decision of 30 Sep
    /// 2026 is that this call picks it from here. A list rather than a free
    /// field, so the proposer cannot name a model nothing can run.
    pub choices: Vec<String>,
    /// `prompts.proposer` as it stood when the call was put together.
    pub prompt: String,
}
