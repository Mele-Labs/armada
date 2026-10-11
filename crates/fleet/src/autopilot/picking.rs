//! The picker: one call that reads the instruction, the issues nothing holds and the workflows,
//! and names one issue, a workflow, and whether it is sure. **Pure but for [`picked`]**.
//!
//! **Sure or unsure, and nothing between.** A pick it is unsure of is set aside with its reason
//! and the queue moves on; a pick it is sure of starts with no approval. A reading that names an
//! issue it was not offered, or a workflow the repository does not hold, is neither: it is
//! refused, and starts nothing.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use adapter_traits::{Ask, OpenIssue};
use config::ResolvedWorkflow;
use core_model::WorkflowId;

use super::sentence::line;
use super::Asking;
use crate::judging::{said, CallFailed};

/// The question, with `{instruction}`, `{issues}` and `{workflows}` to fill.
const QUESTION: &str = "\
You choose the next issue a repository works on by itself, while the person who owns it is \
away. Answer only the block at the end.

What the person asked for, in their words:

    {instruction}

The open issues nothing is working on yet, newest first:

{issues}
The workflows this repository holds, and the steps each runs:

{workflows}
Choose the one issue the instruction most wants worked next, and the workflow it fits. Then say \
whether you are sure. Be sure only where the instruction plainly covers the issue and the \
workflow plainly fits it. Where the issue may need a decision the person has not made, or you \
cannot tell whether the instruction covers it, you are not sure: it is set aside for them with \
your reason, and you are asked about the next issue.

Answer with nothing but these four lines:

    issue: <its number, or none where no issue above is one the instruction covers>
    workflow: <the id, spelled exactly as it appears above>
    sure: <yes or no>
    because: <why, in one line the person will read in the morning>";

/// One issue the picker may choose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub excerpt: String,
    pub labels: Vec<String>,
}

/// The open issues nothing holds, in the tracker's order: **not one a pull request already
/// closes**, and not one whose address is in `taken` — a Job's, or a pick's of this run.
pub fn candidates(open: &[OpenIssue], taken: &BTreeSet<String>) -> Vec<Candidate> {
    open.iter()
        .filter(|issue| !issue.linked_pull_request)
        .filter(|issue| !taken.contains(issue.url.as_written()))
        .map(|issue| Candidate {
            number: issue.number,
            title: issue.title.as_written().to_string(),
            url: issue.url.as_written().to_string(),
            excerpt: issue.excerpt.as_written().to_string(),
            labels: issue
                .labels
                .iter()
                .map(|one| one.as_written().to_string())
                .collect(),
        })
        .collect()
}

/// A pick the picker was sure of. **No public constructor**: [`read_pick`] is the only thing
/// that makes one, so a Job that skips the dispatch gate is always one the picker was sure of.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurePick {
    issue: Candidate,
    workflow: WorkflowId,
    reason: String,
}

impl SurePick {
    pub fn issue(&self) -> &Candidate {
        &self.issue
    }

    pub fn workflow(&self) -> &WorkflowId {
        &self.workflow
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

/// A pick set aside for a person, with the picker's reason. `workflow` is what it chose, where
/// it chose one this repository holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetAside {
    pub issue: Candidate,
    pub workflow: Option<WorkflowId>,
    pub reason: String,
}

/// What one call picked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pick {
    Sure(SurePick),
    Unsure(SetAside),
    /// No issue offered is one the instruction covers.
    Nothing {
        reason: String,
    },
}

/// Why nothing was picked. **Never a pick**: nothing starts and nothing is set aside.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotPicked {
    Call(CallFailed),
    Unreadable { why: String, answer: String },
}

impl fmt::Display for NotPicked {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotPicked::Call(failed) => write!(out, "the picker did not answer: {failed}"),
            NotPicked::Unreadable { why, .. } => {
                write!(out, "the picker's answer was refused: {why}")
            }
        }
    }
}

/// The question one pick is put as.
pub fn pick_question(
    instruction: &str,
    offered: &[Candidate],
    workflows: &BTreeMap<WorkflowId, ResolvedWorkflow>,
) -> String {
    let mut issues = String::new();
    for issue in offered {
        issues.push_str(&format!("  #{} {}\n", issue.number, issue.title));
        if !issue.labels.is_empty() {
            issues.push_str(&format!("    labels: {}\n", issue.labels.join(", ")));
        }
        if !issue.excerpt.is_empty() {
            issues.push_str(&format!("    {}\n", issue.excerpt));
        }
    }
    let mut held = String::new();
    for workflow in workflows.values() {
        held.push_str(&format!(
            "  {} — {}\n",
            workflow.id().as_str(),
            workflow.name()
        ));
        if let Some(asked) = workflow.for_requests() {
            held.push_str(&format!("    for: {asked}\n"));
        }
        let steps: Vec<&str> = workflow.steps().iter().map(|step| step.label()).collect();
        held.push_str(&format!("    {}\n", steps.join(" -> ")));
    }
    QUESTION
        .replace("{instruction}", instruction.trim())
        .replace("{issues}", &issues)
        .replace("{workflows}", &held)
}

/// Ask once, and read the answer against what was offered.
pub async fn picked(
    instruction: &str,
    offered: &[Candidate],
    workflows: &BTreeMap<WorkflowId, ResolvedWorkflow>,
    asking: &Asking,
) -> Result<Pick, NotPicked> {
    let question = pick_question(instruction, offered, workflows);
    let ask = Ask::put(asking.model.clone(), &question, asking.environment.clone())
        .map_err(|_| NotPicked::Call(CallFailed::NothingToAsk))?;
    let answer = said(asking.client.as_ref(), &ask, asking.budget)
        .await
        .map_err(NotPicked::Call)?;
    read_pick(&answer, offered, workflows)
}

/// Read an answer. Sure needs an issue offered and a workflow held; unsure needs only the issue.
pub fn read_pick(
    answer: &str,
    offered: &[Candidate],
    workflows: &BTreeMap<WorkflowId, ResolvedWorkflow>,
) -> Result<Pick, NotPicked> {
    let unreadable = |why: String| NotPicked::Unreadable {
        why,
        answer: answer.to_string(),
    };
    let reason = line(answer, "because").filter(|said| !said.is_empty());
    let named = line(answer, "issue").ok_or_else(|| unreadable("it named no issue".to_string()))?;
    if named.eq_ignore_ascii_case("none") {
        return Ok(Pick::Nothing {
            reason: reason.unwrap_or_else(|| "it gave no reason".to_string()),
        });
    }
    let number: u64 = named
        .trim_start_matches('#')
        .parse()
        .map_err(|_| unreadable(format!("`{named}` is not an issue number")))?;
    let issue = offered
        .iter()
        .find(|one| one.number == number)
        .cloned()
        .ok_or_else(|| unreadable(format!("issue {number} was not one it was offered")))?;
    let workflow_named = line(answer, "workflow").unwrap_or_default();
    let workflow = workflows
        .keys()
        .find(|id| id.as_str() == workflow_named)
        .cloned();
    let sure = match line(answer, "sure")
        .map(|said| said.to_ascii_lowercase())
        .as_deref()
    {
        Some("yes") => true,
        Some("no") => false,
        _ => return Err(unreadable("it did not say whether it was sure".to_string())),
    };
    match (sure, workflow) {
        (true, Some(workflow)) => Ok(Pick::Sure(SurePick {
            issue,
            workflow,
            reason: reason.unwrap_or_else(|| "it gave no reason".to_string()),
        })),
        (true, None) => Err(unreadable(format!(
            "workflow `{workflow_named}` is not one this repository holds"
        ))),
        (false, workflow) => Ok(Pick::Unsure(SetAside {
            issue,
            workflow,
            reason: reason
                .ok_or_else(|| unreadable("it was unsure and gave no reason".to_string()))?,
        })),
    }
}
