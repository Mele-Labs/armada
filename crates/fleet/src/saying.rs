//! What an [`Adrift`] tells whoever reports it: the sentence, the Job it names,
//! and the cause underneath.
//!
//! Split from [`adrift`](mod@crate::adrift) at the 900-line refusal, along the
//! seam that module's own header already names — it declares what could stop a
//! turn and why each variant is shaped as it is, and this answers the three
//! questions anything asks of one afterwards. `settling::noted_adrift` asks two
//! in the same breath, taking [`Adrift::job`] to find the log to write into and
//! the `Display` sentence to write there; `refusing` asks the third on the way
//! to a wire code.
//!
//! **Nothing here decides anything.** No arm maps a failure onto a recovery, a
//! retry or a status — those live where the failure is raised. Reading is all
//! this module does, which is why a new variant touches the enum and these
//! three matches and nothing else.
//!
//! Why the adapter halves are boxed and the domain halves are not is on
//! `adrift`'s own header, beside the variants that make the choice.

use std::error::Error;
use std::fmt;

use core_model::{JobId, JobStatus};

use crate::adrift::Adrift;

impl fmt::Display for Adrift {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Adrift::BootRead(cause) => write!(out, "the boot read failed: {cause}"),
            Adrift::Reading(cause) => write!(out, "a Job could not be read: {cause}"),
            Adrift::Writing(cause) => write!(out, "a write did not land: {cause}"),
            Adrift::IllegalMove(cause) => {
                write!(out, "Fleet asked for a move the machine refuses: {cause}")
            }
            Adrift::IllegalStepMove(cause) => write!(
                out,
                "Fleet asked for a step move the machine refuses: {cause}"
            ),
            Adrift::Unworkable { job, cause } => {
                write!(
                    out,
                    "no worktree can be named for {}: {}",
                    job.as_str(),
                    cause.said()
                )
            }
            Adrift::NotPrepared { job, cause } => write!(
                out,
                "{} has a worktree and no step ran in it: {cause}",
                job.as_str()
            ),
            Adrift::PortsRefused { job, cause } => write!(
                out,
                "{}'s worktree has no port span and nothing was spawned: {cause}",
                job.as_str()
            ),
            Adrift::NotDelivered { job, doing, said } => write!(
                out,
                "{}'s work is committed and {doing} did not happen: {said}. The branch holds \
                 the change and every Check passed",
                job.as_str()
            ),
            Adrift::ReviewUnreadable { job, pull_request } => write!(
                out,
                "nothing on this machine could say what is on {}'s pull request at \
                 {pull_request}",
                job.as_str()
            ),
            Adrift::NoRemarksChosen { job } => write!(
                out,
                "no comment on {}'s pull request was picked, so there is nothing to \
                 hand a Drone",
                job.as_str()
            ),
            Adrift::RemarksGone { job, gone } => write!(
                out,
                "{}'s pull request no longer has {} of the comments that were picked, \
                 so none of them was sent",
                job.as_str(),
                gone.len()
            ),
            Adrift::RemarksAlreadyTakenUp { job, already } => write!(
                out,
                "{} of the comments picked off {}'s pull request have already reached a \
                 Drone, so none of them was sent again",
                already.len(),
                job.as_str()
            ),
            Adrift::NoDismissalReason { job } => write!(
                out,
                "a finding on {}'s review was dismissed with no reason, so it was not \
                 dismissed. Say why it is wrong",
                job.as_str()
            ),
            Adrift::FindingNotInReview { job, finding } => write!(
                out,
                "{}'s latest review did not raise \"{finding}\", so there is nothing to dismiss",
                job.as_str()
            ),
            Adrift::RemarksFileUnwritable { job, cause } => write!(
                out,
                "the comments picked off {}'s pull request could not be written into its \
                 worktree: {cause}",
                job.as_str()
            ),
            Adrift::NothingToMerge { job } => write!(
                out,
                "{} opened no pull request, so approving it is the act",
                job.as_str()
            ),
            Adrift::NothingToRerun { job } => write!(
                out,
                "{} has no open pull request, so there is no failed run to start again",
                job.as_str()
            ),
            Adrift::RerunRefused { job, said } => write!(
                out,
                "the forge did not start {}'s failed runs again: {said}",
                job.as_str()
            ),
            Adrift::NothingToInvestigate { job } => write!(
                out,
                "no check failed on {}'s pull request when Fleet last read it, so there is nothing to investigate",
                job.as_str()
            ),
            Adrift::NoStepToRedo { job } => write!(
                out,
                "{}'s frozen workflow has nothing before the step that delivers, so there \
                 is no step to send a conflict back to",
                job.as_str()
            ),
            Adrift::NotMerged { job, why } => write!(
                out,
                "{}'s pull request was not merged and nothing retries — {}",
                job.as_str(),
                why.said()
            ),
            Adrift::NotForgettable { job, status } => write!(
                out,
                "{} is {} and cannot be forgotten. Forgetting is a real deletion and only \
                 applies to a terminal Job — `kill_job` is how one still in flight is ended",
                job.as_str(),
                status.as_wire()
            ),
            // Dollars, because dollars are what a person set and what the
            // detail draws beside the spend. The micros are the wire's unit and
            // are nobody's sentence.
            // Turns, in the unit they are counted in — the same three
            // sentences as the cost cap's above, saying which ceiling and what
            // a person does about it. The remedy differs and is named: a Job
            // going in circles is redispatched, and a Job that finished and
            // was held out of its last step wants the number.
            Adrift::NotReclaimable {
                job,
                status: JobStatus::Piloted,
            } => write!(
                out,
                "{} is piloted: a person is working in its worktree, so it is not Fleet's to \
                 give back. End the pilot first, by submitting the Job, attesting it complete or \
                 closing it as superseded",
                job.as_str()
            ),
            Adrift::NotReclaimable { job, status } => write!(
                out,
                "{} is {} and its worktree cannot be reclaimed. There is no disk to give back \
                 while a Drone might still write to it — `kill_job` is how one still in flight \
                 is ended",
                job.as_str(),
                status.as_wire()
            ),
            Adrift::SlotKept { job, slot, why } => write!(
                out,
                "{}'s worktree is slot-{slot}, and it stays held: {why}",
                job.as_str()
            ),
            Adrift::WorktreeNotSaved { job, why } => write!(
                out,
                "{}'s worktree was not removed: {why}",
                job.as_str()
            ),
            // The repository, named. A person fixes this by looking at that
            // path, and a message that only said the Job would send them to
            // the Job.
            Adrift::NotReclaimed { job, cause } => write!(
                out,
                "{}'s worktree could not be reclaimed: {} would not open: {}",
                job.as_str(),
                cause.repo,
                cause.why
            ),
            Adrift::BranchNotDeletable { job, why } => {
                write!(out, "{}'s branch was not deleted: {why}", job.as_str())
            }
            Adrift::BranchNotDeleted { job, why } => {
                write!(out, "{}'s branch could not be deleted: {why}", job.as_str())
            }
            Adrift::NotRedispatchable { job, status } => write!(
                out,
                "{} is {} and cannot be redispatched. Redispatch replaces a Job that ran and \
                 stopped, which is `escalated`, `completed_failed` or `killed`",
                job.as_str(),
                status.as_wire()
            ),
            Adrift::NeverRan { job } => write!(
                out,
                "{} was rejected and never ran, so a redispatch would carry nothing forward — no \
                 Facts, no Evidence, no worktree. Asking for the work again is proposing a new \
                 Job, which is what `propose_job` does",
                job.as_str()
            ),
            Adrift::NotReplaceable { job } => write!(
                out,
                "{} was dispatched by a step of another Job, and replacing it is that Job's act \
                 rather than this one's",
                job.as_str()
            ),
            Adrift::WorkflowWithdrawn { job, workflow_id } => write!(
                out,
                "{} froze workflow `{workflow_id}`, which this Fleet no longer holds — the file \
                 was renamed or deleted since. Nothing was replaced",
                job.as_str()
            ),
            Adrift::NoDroneToRedirect { job } => write!(
                out,
                "{} has no Drone to redirect — it is gone, and what is left is a restart onto \
                 the worktree it left behind",
                job.as_str()
            ),
            Adrift::NotTheJobsProcess { job, pid } => write!(
                out,
                "pid {pid} is not in {}'s process tree as Fleet read it just now, so nothing was \
                 signalled — it has exited, or the number now names another process",
                job.as_str()
            ),
            // `questioning::NotAnswered`'s sentence, whole: a second wording
            // here would be a second authority for one refusal.
            Adrift::WorkUnreadable { job, cause } => write!(
                out,
                "{}'s worktree would not be read, so there is no diff to show: {cause}",
                job.as_str()
            ),
            // The sentence alone, with no Job id in front of it: the refusal
            // already names the Job it is about, and this is what the control
            // puts under the button a person just pressed.
            Adrift::ChecksRunningAgain { job } => write!(
                out,
                "{}'s Checks are running again on its worktree. Wait for that run to finish \
                 before acting on the Job",
                job.as_str()
            ),
            Adrift::ProposalAbandoned => write!(
                out,
                "the proposal ended without an answer; the Job it created is moved the next \
                 time Fleet starts"
            ),
            Adrift::PressAbandoned { job } => write!(
                out,
                "the run showing {}'s work ended without an answer. Its log says how far it got; \
                 press again to run it again",
                job.as_str()
            ),
            Adrift::MainNotFixable { why, .. } => write!(out, "{why}"),
            Adrift::Unnameable => out.write_str("a Job needs a title somebody can read"),
            // The Job is named and then the cause speaks, so the sentence reads
            // as one and the field it is about is in it.
            Adrift::NotFileable { job, cause } => {
                write!(out, "nothing was filed about {}: {cause}", job.as_str())
            }
            Adrift::NotProposable(cause) => write!(
                out,
                "the proposer call could not be configured: {}",
                cause.said()
            ),
            // **Never the sentence above.** Saying this again differently
            // changes nothing: the request was read and the workflow was
            // fine. What fixes it is a model this machine runs, or installing
            // the one asked for. `Unresolved::ModelNotHeld` names both the
            // model asked for and the set.
            // The resolver's own words, which quote what the caller said. A
            // sentence written here would be a second wording of the same
            // refusal, and the two would drift.
            Adrift::Unresolvable(why) => write!(out, "{why}"),
            Adrift::NoSuchCheckOutput { named } => write!(
                out,
                "no Check of this Job kept an output named `{named}`. The record holds where \
                 each Check wrote its own, so a name from a row whose `.armada` directory has \
                 since been reclaimed reaches no file"
            ),
            Adrift::NoSuchFrame { named } => write!(
                out,
                "no step of this Job kept a frame named `{named}`. The record holds where each \
                 run's harness wrote its own, so a name from a row whose `.armada/frames` \
                 directory has since been reclaimed reaches no file"
            ),
            Adrift::NoSuchBrief { named } => write!(
                out,
                "this Job kept no brief named `{named}`. A brief is named by the last part of \
                 its `brief_path`, and one whose `.armada/briefs` directory has since been \
                 reclaimed reaches no file"
            ),
            Adrift::NoSuchLandLog {
                root,
                branch,
                check,
            } => write!(
                out,
                "no Check named `{check}` has a log for `{branch}` on the merge line in {root}. \
                 The branch's outcome names its turn and the Checks that turn has started, and \
                 nothing else is opened"
            ),
            Adrift::NoSuchManifest { named, held } => write!(
                out,
                "no Manifest is named `{named}`. This Fleet holds `{held}`, and `list_manifests` \
                 says which repository declares each"
            ),
            Adrift::NotServed { manifest, .. } => write!(
                out,
                "this Job belongs to Manifest `{manifest}`, which no repository this Fleet serves \
                 declares. Add that repository and its Jobs are served again"
            ),
            Adrift::ManifestUnreadable { path, cause } => write!(
                out,
                "{path} could not be read: {cause}. What Fleet is running with is unchanged — \
                 this is the file on disk, not the configuration in force"
            ),
            Adrift::ManifestMovedUnderTheEdit {
                path,
                on_disk: Some(_),
            } => write!(
                out,
                "{path} changed after this edit read it, so the save was refused rather than \
                 taking the incoming change with it. What is on disk now is on `on_disk`"
            ),
            Adrift::ManifestMovedUnderTheEdit { path, on_disk: None } => write!(
                out,
                "{path} is no longer there, so the save was refused rather than putting a file \
                 back that nobody asked for. Whatever removed it is what to look at first"
            ),
            Adrift::ManifestUnwritable { path, cause } => write!(
                out,
                "the corrected Manifest could not be written to {path}: {cause}. Nothing was \
                 changed, so what is on disk is what was there before"
            ),
            Adrift::CommandTimedOut { job: Some(job), waited } => write!(
                out,
                "Fleet gave up waiting on this after {waited:?}, which is its own limit and \
                 not Bridge's — the command was not cancelled and may still be finishing \
                 against {}, whose log will say so if it lands",
                job.as_str()
            ),
            Adrift::CommandTimedOut { job: None, waited } => write!(
                out,
                "Fleet gave up waiting on this after {waited:?}, which is its own limit and \
                 not Bridge's — the command was not cancelled and may still be finishing"
            ),
            Adrift::PlanRefused { job, why } => {
                write!(out, "{}'s plan refuses this change: {why}", job.as_str())
            }
            Adrift::BranchesUnread { why, .. } => {
                write!(out, "git would not list this repository's branches: {why}")
            }
            Adrift::TaskNotFailed { job, named, state } => write!(
                out,
                "{}'s task {named} is {}, and only a failed task, or a done one in a group \
                 the Judge refused, restarts on its own. A task working or handed in is \
                 still in its group's run",
                job.as_str(),
                state.as_wire()
            ),
            Adrift::TaskInFlight { job, named, state } => write!(
                out,
                "{}'s task {named} is {}, so its group is still running and cannot move \
                 until its gate has answered",
                job.as_str(),
                state.as_wire()
            ),
        }
    }
}

impl Adrift {
    /// Which Job could not be carried forward, where one can be named.
    ///
    /// **What makes a failure reachable from the thing it happened to.** A turn
    /// that fails is reported on Fleet's stdout, which is not where a Job is
    /// read; this is what lets the same failure be written into the Job's own
    /// log, where the person watching it is looking.
    ///
    /// `None` is honest rather than a gap. A boot read, a proposal that named
    /// no workflow this Fleet holds, a request with nothing in it — none of
    /// those has a Job yet, and the three illegal-move variants carry the
    /// machine's own refusal rather than the record it was asked about.
    pub fn job(&self) -> Option<&JobId> {
        match self {
            Adrift::Unworkable { job, .. }
              | Adrift::NotPrepared { job, .. } | Adrift::PortsRefused { job, .. } | Adrift::NotDelivered { job, .. } | Adrift::NothingToMerge { job } | Adrift::NothingToRerun { job } | Adrift::RerunRefused { job, .. } | Adrift::NothingToInvestigate { job } | Adrift::NoStepToRedo { job } | Adrift::NotMerged { job, .. } | Adrift::ReviewUnreadable { job, .. } | Adrift::NoRemarksChosen { job } | Adrift::RemarksGone { job, .. } | Adrift::RemarksAlreadyTakenUp { job, .. } | Adrift::NoDismissalReason { job } | Adrift::FindingNotInReview { job, .. } | Adrift::RemarksFileUnwritable { job, .. } | Adrift::NotForgettable { job, .. } | Adrift::NotReclaimable { job, .. } | Adrift::SlotKept { job, .. } | Adrift::WorktreeNotSaved { job, .. } | Adrift::NotReclaimed { job, .. } | Adrift::BranchNotDeletable { job, .. } | Adrift::BranchNotDeleted { job, .. } | Adrift::NotRedispatchable { job, .. } | Adrift::NeverRan { job } | Adrift::NotReplaceable { job } | Adrift::WorkflowWithdrawn { job, .. } | Adrift::NoDroneToRedirect { job } | Adrift::NotTheJobsProcess { job, .. } | Adrift::WorkUnreadable { job, .. } | Adrift::ChecksRunningAgain { job } | Adrift::PressAbandoned { job } | Adrift::NotFileable { job, .. } | Adrift::PlanRefused { job, .. } | Adrift::TaskNotFailed { job, .. } | Adrift::TaskInFlight { job, .. } | Adrift::NotServed { job, .. } => Some(job),
            Adrift::CommandTimedOut { job, .. } | Adrift::BranchesUnread { job, .. } => {
                job.as_ref()
            }
            Adrift::BootRead(_)
            | Adrift::Reading(_)
            | Adrift::Writing(_)
            | Adrift::IllegalMove(_)
            | Adrift::IllegalStepMove(_)
             | Adrift::Unnameable | Adrift::NoSuchManifest { .. } | Adrift::ManifestUnreadable { .. } | Adrift::ManifestUnwritable { .. } | Adrift::ManifestMovedUnderTheEdit { .. } | Adrift::NoSuchCheckOutput { .. } | Adrift::Unresolvable(_) | Adrift::NoSuchFrame { .. } | Adrift::NoSuchBrief { .. } | Adrift::NoSuchLandLog { .. } | Adrift::NotProposable(_) | Adrift::ProposalAbandoned | Adrift::MainNotFixable { .. } => None,
        }
    }
}

impl Error for Adrift {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Adrift::BootRead(cause) => Some(cause),
            Adrift::Reading(cause) => Some(cause),
            Adrift::Unresolvable(cause) => Some(cause),
            Adrift::Writing(cause) => Some(cause),
            Adrift::IllegalMove(cause) => Some(cause),
            Adrift::IllegalStepMove(cause) => Some(cause),
            Adrift::NotPrepared { cause, .. } => Some(cause),
            // The call that would not be made, as the typed leaf the runner
            // raised rather than as a sentence this rewrote.
            Adrift::PortsRefused { cause, .. } => Some(cause),
            Adrift::RemarksFileUnwritable { cause, .. } | Adrift::ManifestUnreadable { cause, .. } | Adrift::ManifestUnwritable { cause, .. } => Some(cause),
            // The rest say what the record or the request holds rather than wrapping anything that
            // failed underneath it.
            Adrift::Unworkable { .. }
            | Adrift::NotDelivered { .. }
            | Adrift::ReviewUnreadable { .. }
            | Adrift::NoRemarksChosen { .. }
            | Adrift::RemarksGone { .. }
            | Adrift::RemarksAlreadyTakenUp { .. }
            | Adrift::NoDismissalReason { .. }
            | Adrift::FindingNotInReview { .. }
            | Adrift::NothingToMerge { .. }
            | Adrift::NothingToRerun { .. }
            | Adrift::RerunRefused { .. }
            | Adrift::NothingToInvestigate { .. }
            | Adrift::NoStepToRedo { .. }
            | Adrift::NotMerged { .. }
            | Adrift::NotForgettable { .. }
            | Adrift::NotReclaimable { .. }
            | Adrift::SlotKept { .. }
            | Adrift::WorktreeNotSaved { .. }
            | Adrift::NotReclaimed { .. }
            | Adrift::BranchNotDeletable { .. }
            | Adrift::BranchNotDeleted { .. }
            | Adrift::NotRedispatchable { .. }
            | Adrift::NeverRan { .. }
            | Adrift::NotReplaceable { .. }
            | Adrift::WorkflowWithdrawn { .. }
            | Adrift::NoDroneToRedirect { .. }
            | Adrift::NotTheJobsProcess { .. }
            | Adrift::ChecksRunningAgain { .. }
            | Adrift::ProposalAbandoned
             | Adrift::PressAbandoned { .. } | Adrift::MainNotFixable { .. } | Adrift::Unnameable | Adrift::NotFileable { .. } | Adrift::NoSuchManifest { .. } | Adrift::NotServed { .. } | Adrift::ManifestMovedUnderTheEdit { .. } | Adrift::NoSuchCheckOutput { .. } | Adrift::NoSuchFrame { .. } | Adrift::NoSuchBrief { .. } | Adrift::NoSuchLandLog { .. } | Adrift::NotProposable (_) | Adrift::CommandTimedOut { .. } | Adrift::PlanRefused { .. } | Adrift::BranchesUnread { .. } | Adrift::TaskNotFailed { .. } | Adrift::TaskInFlight { .. } => None,
            Adrift::WorkUnreadable { cause, .. } => Some(cause.as_ref()),
        }
    }
}

