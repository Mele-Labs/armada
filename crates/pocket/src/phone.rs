//! What the phone may see of a Job.
//!
//! **An allowlist, built field by field.** [`PhoneJob`] is constructed from
//! named fields of Fleet's Job, never by serialising Fleet's type and removing
//! what is unwanted, so a field Fleet adds tomorrow does not reach the phone
//! until somebody writes it here. There is no field for a diff, a file, a log,
//! a transcript, a brief or an environment value, and so none to fill.

use ipc::{JobDetail, JobSummary};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct PhoneStep {
    /// Counted from one.
    pub at: usize,
    pub of: usize,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct PhoneCheck {
    pub name: String,
    pub passed: bool,
}

#[derive(Debug, Serialize)]
pub struct PhoneJob {
    pub id: String,
    pub title: String,
    /// The repository's directory name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    /// The status as the wire spells it.
    pub status: String,
    /// Why it stopped: the escalation trigger's name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Asking is a Drone waiting on an answer, which `status` does not say.
    pub asking: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<PhoneStep>,
    /// Instants, so the phone counts the ages itself.
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<String>,
    /// Since when it has been waiting on the person, where Fleet says.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waiting_since: Option<String>,
    /// `pass` when the Judge answered every criterion with no objection, `veto`
    /// when it refused one. Absent when it has not answered.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verdict: Option<&'static str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub checks: Vec<PhoneCheck>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pull_request: Option<String>,
}

impl PhoneJob {
    /// What a list row or an event carries: no step, Checks or verdict.
    pub fn of(job: &JobSummary, repository: Option<&str>) -> PhoneJob {
        PhoneJob {
            id: job.id.as_str().to_string(),
            title: job.title.clone(),
            repository: repository.map(str::to_string),
            status: job.status.as_wire().to_string(),
            reason: job.reason.as_ref().and_then(|reason| reason.named.clone()),
            asking: job.asking,
            step: None,
            created_at: job.created_at.as_str().to_string(),
            started_at: job.started_at.as_ref().map(|at| at.as_str().to_string()),
            ended_at: job.ended_at.as_ref().map(|at| at.as_str().to_string()),
            waiting_since: None,
            verdict: None,
            checks: Vec::new(),
            pull_request: None,
        }
    }

    /// The row with what `get_job` adds: the step it is at, that step's Checks
    /// and Judge answer, and the pull request.
    pub fn of_detail(detail: &JobDetail, repository: Option<&str>) -> PhoneJob {
        let mut phone = PhoneJob::of(&detail.job, repository);
        let current = detail
            .steps
            .iter()
            .position(|step| Some(&step.step_id) == detail.job.current_step_id.as_ref());
        if let Some(index) = current {
            phone.step = Some(PhoneStep {
                at: index + 1,
                of: detail.steps.len(),
                name: detail.steps[index].label.clone(),
            });
        }
        // The current step's results, or the last step that ran any.
        let shown = current
            .map(|index| &detail.steps[index])
            .filter(|step| !step.check_runs.is_empty() || !step.judged.is_empty())
            .or_else(|| {
                detail
                    .steps
                    .iter()
                    .rev()
                    .find(|step| !step.check_runs.is_empty() || !step.judged.is_empty())
            });
        if let Some(step) = shown {
            let latest = step.check_runs.iter().map(|run| run.attempt).max();
            for run in step.check_runs.iter().filter(|run| Some(run.attempt) == latest) {
                let passed = run.outcome.domain() == core_model::CheckOutcome::Passed;
                match phone.checks.iter_mut().find(|check| check.name == run.name) {
                    Some(seen) => seen.passed = passed,
                    None => phone.checks.push(PhoneCheck {
                        name: run.name.clone(),
                        passed,
                    }),
                }
            }
            if let Some(latest) = step.judged.iter().map(|row| row.attempt).max() {
                let refused = step
                    .judged
                    .iter()
                    .filter(|row| row.attempt == latest)
                    .any(|row| row.verdict.domain() == core_model::JudgeVerdict::NotMet);
                phone.verdict = Some(if refused { "veto" } else { "pass" });
            }
        }
        phone.pull_request = detail
            .delivery
            .as_ref()
            .and_then(|delivery| delivery.pull_request.clone());
        phone
    }
}

/// Bridge's Needs you tab, `packages/screens/src/needs-you.ts` `tabOf`, in
/// Rust: not cleared, not over, and a Drone waiting on an answer or a status
/// whose `who_is_acting` is `Person` in `job-statuses.toml`.
///
/// **Every status is named, with no `_` arm**, so a status added to the
/// registry fails to compile here until somebody places it.
pub fn needs_you(job: &JobSummary) -> bool {
    use core_model::JobStatus::*;
    if job.reclaimed_at.is_some() {
        return false;
    }
    match job.status.domain() {
        CompletedFailed | CompletedSuccess | Killed | Rejected | Superseded => false,
        _ if job.asking => true,
        Running | Piloted | Proposing | Queued => false,
        AwaitingApproval | AwaitingAttestation | AwaitingRepair | AwaitingReview | Escalated => {
            true
        }
    }
}

/// The Running list: the registry's `mode = "Working"`, which Bridge reads
/// before the actor, and not a Job that needs a person.
pub fn running(job: &JobSummary) -> bool {
    use core_model::JobStatus::*;
    job.reclaimed_at.is_none()
        && !job.asking
        && matches!(job.status.domain(), Running | Piloted | Proposing)
}

/// The Done list: over, and not cleared.
pub fn done(job: &JobSummary) -> bool {
    job.reclaimed_at.is_none() && job.status.domain().is_terminal()
}
