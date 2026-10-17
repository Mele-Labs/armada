//! Why the loop could not carry a Job forward.
//!
//! # An illegal transition is in here, and that is the point
//!
//! [`Adrift::IllegalMove`] and [`Adrift::IllegalStepMove`] exist so that a move
//! the machine refuses **surfaces as a bug in Fleet** rather than being logged
//! and stepped over. There is no arm anywhere in this crate that turns one into
//! a warning and continues: `Job::transition` returning `Err` means Fleet asked
//! for something the registry says cannot happen, and a fallback for it would
//! be Fleet quietly disagreeing with the edge table.
//!
//! # The adapter failures are boxed and the domain failures are not
//!
//! `docs/contracts/error-contract.md` puts a typed leaf enum beside the code
//! that raised it and a real cause chain in the wrapper. The typed halves below
//! are the ones this crate can name: the store's, the machine's, the spawn
//! configuration's. The adapter halves arrive through a generic seam whose
//! error type is the implementation's — a `V::Error` cannot be a variant here
//! without making this enum generic over three parameters, which would put the
//! adapter's vocabulary into every signature in the crate. They are boxed, and
//! reachable through [`Error::source`].
//!
//! This is not `Box<dyn Error>` collapsing everything: every variant still says
//! *what Fleet was doing*, which is the thing a caller matches on, and the box
//! carries only the leaf underneath it.

use std::error::Error;
use std::io;
use std::time::Duration;

use adapter_traits::{NotMerged, SpawnConfigRefused, WorktreeSpecRefused};
use adapters::RepoUnreadable;
use core_model::{
    IllegalStepTransition, IllegalTransition, JobId,
    JobStatus, PlanRefused, TaskId, TaskState,
};
use store::{LoadAllError, LoadJobError, ResolveJobError, WriteError};

use crate::preparing::NotPrepared;
use crate::reporting::NotFiled;

/// A Job that could not be carried forward, and what stopped it.
#[derive(Debug)]
pub enum Adrift {
    /// The boot read failed outright. Not the partial-failure case — that one
    /// is carried alongside the Jobs that did load, because a caller must never
    /// be handed a short list with nothing saying so.
    BootRead(LoadAllError),
    /// One Job would not load.
    Reading(LoadJobError),
    /// A write did not land. The Job is where the log says it is, which is
    /// wherever it was before this.
    Writing(WriteError),
    /// **A bug in Fleet.** The machine refused a move Fleet asked for.
    IllegalMove(IllegalTransition),
    /// **A bug in Fleet**, for the inner machine.
    IllegalStepMove(IllegalStepTransition),
    /// The Job's id or the configured repository root could not name a
    /// worktree.
    Unworkable {
        job: JobId,
        cause: WorktreeSpecRefused,
    },
    /// The worktree was cut and what the Manifest requires in one would not
    /// run. **Nothing was spawned, and no step was entered** — the Job is
    /// escalated before the first step moves, so this can never be read as a
    /// step that failed.
    ///
    /// **Boxed, and for the size rather than for the seam.** The leaf is this
    /// crate's own and could be inlined the way its neighbours are; what stops
    /// it is that it carries a command's captured output, which is 136 bytes
    /// against this enum's 104 and would grow every `Result<_, Adrift>` in the
    /// crate — which is to say every signature in it — to pay for the rarest
    /// variant in it.
    NotPrepared { job: JobId, cause: Box<NotPrepared> },
    /// The worktree was cut and its Manifest's `ports:` could not be claimed —
    /// two ports collide on one `env`, or the range has no free span left.
    /// **Nothing was spawned**, the same reading [`Adrift::NotPrepared`] gives.
    PortsRefused {
        job: JobId,
        cause: crate::ports::PortsRefused,
    },
    /// The work is committed and getting it to the branch it merges into did
    /// not finish — the base would not read, the rebase would not run, the push
    /// or the pull request was refused.
    ///
    /// **Nothing is lost and no verdict changed.** A Job that reaches this at
    /// its last step is still `completed_success`, because that is a fact about
    /// its Checks; a Job that reaches it mid-workflow is still running and its
    /// Drone was still told the step advanced.
    ///
    /// Not boxed, unlike its neighbours: the seam answers one shape rather than
    /// an implementation's own enum, so there is nothing here to downcast to.
    NotDelivered {
        job: JobId,
        doing: &'static str,
        said: String,
    },
    /// The forge would not say what is on a pull request.
    ///
    /// **A refusal and never an empty list.** A pull request nobody has
    /// commented on and a forge nobody could reach are opposite answers —
    /// `adapter_traits::UnderReview` keeps them apart and this is where the
    /// second one stops. A person shown it as the first would conclude their
    /// review had vanished.
    ///
    /// **Not the sweep's answer to the same silence.** `crate::under_review`
    /// reads this on a rotation and says nothing, because there is another
    /// sweep coming; here somebody is standing in front of it waiting.
    ReviewUnreadable { job: JobId, pull_request: String },
    /// A press arrived naming no comments at all.
    ///
    /// **Refused rather than taken as a Drone with nothing to do**, for
    /// `redirect_drone`'s reason about a blank note: an act that appears to
    /// work and changes nothing is worse than one that says what was missing.
    NoRemarksChosen { job: JobId },
    /// A press named comments the pull request no longer has.
    ///
    /// **The whole press, not the part that survived.** A person picked a set,
    /// and acting on what was left of it without saying so is the silent
    /// divergence choosing exists to prevent. What comes back is which handles
    /// are gone, so the answer is a re-read and another choice.
    RemarksGone { job: JobId, gone: Vec<String> },
    /// A press named comments a Drone on this Job has already been handed.
    ///
    /// **`RedirectAlreadyWaiting`'s rule one scope wider.** That one refuses a
    /// second note over an undelivered first; this refuses a second delivery of
    /// the same words. A Drone that ran and did not fully satisfy a comment
    /// must not meet it again as if it were new — the comment reads the same
    /// on the forge forever, and only Armada's own record can tell them apart.
    RemarksAlreadyTakenUp { job: JobId, already: Vec<String> },
    /// A person dismissed a finding with no reason. A 422: the reason is what the next
    /// review pass is told, and a dismissal that says nothing would tell it nothing. #907.
    NoDismissalReason { job: JobId },
    /// A person dismissed a finding the Job's latest review did not raise.
    FindingNotInReview { job: JobId, finding: String },
    /// The comments a person picked would not write to a file in the Drone's
    /// worktree.
    ///
    /// **Refused, not sent half-written.** `fleet::remarks::write_comments_file`
    /// writes before `request_changes` moves the Job, on the same ground
    /// [`Adrift::AttachmentUnreadable`] stands on: a pointer the note gives to
    /// a file that is not there, or holds only some of what was picked, is
    /// worse than telling a person the press did not go through.
    RemarksFileUnwritable { job: JobId, cause: io::Error },
    /// A merge was asked for on a Job whose record holds no pull request.
    ///
    /// **Not [`Adrift::NotMerged`]**, and the difference is whose fault it is:
    /// nothing was ever opened, so there is nothing for a forge to refuse. A
    /// workflow that declares no delivering step is the ordinary way to get
    /// here, and the act a person wants on one of those is an approval.
    NothingToMerge { job: JobId },
    /// Re-run was pressed on a Job with no open pull request. #905.
    NothingToRerun { job: JobId },
    /// The forge would not start the failed runs again, in its own words. #905.
    RerunRefused { job: JobId, said: String },
    /// Investigate was pressed on a Job whose pull request has no failed check. #905.
    NothingToInvestigate { job: JobId },
    /// A conflict resolution was asked for on a workflow whose delivering step
    /// has nothing before it — a single step that both writes the work and
    /// sends it out, which this cannot redo without redelivering onto its own
    /// conflict. `#663`.
    NoStepToRedo { job: JobId },
    /// The forge would not merge the pull request, and which kind of would-not
    /// it was.
    ///
    /// **One variant carrying the typed kind, and one wire code per kind.**
    /// The kinds send a person to different places — an administrator, the
    /// branch, a failing check — so nothing about them is collapsed; what is
    /// not duplicated is this enum, because no arm in this crate does anything
    /// different about them. `crate::refusing` is where the kind becomes a code
    /// a client dispatches on, which is where every other refusal's kind
    /// becomes one.
    ///
    /// **Nothing retries.** A merge Fleet retried on its own would be a machine
    /// deciding the thing `auto_merge: never` reserves for a person, and the
    /// Job is left exactly where the press found it.
    NotMerged { job: JobId, why: NotMerged },
    /// A forget was asked for on a Job that has not reached a terminal
    /// status.
    ///
    /// **Not an illegal transition.** `forget_job` does not move the machine
    /// at all — it deletes the row — so there is no edge for a refusal to
    /// point at, only the status the Job is still standing in.
    NotForgettable { job: JobId, status: JobStatus },
    /// A reclaim was asked for on a Job that has not reached a terminal status.
    ///
    /// **Its own variant beside `NotForgettable`, not a reuse of it.** The two
    /// acts refuse on the same predicate and name different acts as the answer
    /// — forgetting takes the record and this takes the disk — and a message
    /// naming the wrong one sends a person to the wrong button.
    NotReclaimable { job: JobId, status: JobStatus },
    /// A reclaim on a finished Job whose pool slot the pool would not take
    /// back: the tree holds something uncommitted, or commits on neither the
    /// remote nor the base. The slot stays the Job's.
    SlotKept { job: JobId, slot: u32, why: String },
    /// A reclaim on a Job's own worktree whose uncommitted files could not be
    /// committed to its branch. **Nothing was removed.**
    WorktreeNotSaved { job: JobId, why: String },
    /// The repository a Job's worktree is in would not open, so neither half
    /// of the reclaim was attempted.
    ///
    /// **Never a silent success.** A root that is not a repository at all — a
    /// bare directory where `armada.yml` says a checkout is — comes back here
    /// naming the repository and what git said about it, because the
    /// alternative reading is "there was nothing to reclaim" and that is the
    /// answer a person would act on by never looking again.
    NotReclaimed { job: JobId, cause: RepoUnreadable },
    /// A person's `delete_branch` refused before anything was touched. A 409,
    /// and [`Undeletable`](crate::reclaiming::Undeletable) says what to do.
    BranchNotDeletable {
        job: JobId,
        why: crate::reclaiming::Undeletable,
    },
    /// The delete was allowed and did not happen: the repository would not
    /// open, or git refused. A 500, in git's own words.
    BranchNotDeleted { job: JobId, why: String },
    /// A redispatch was asked for on a Job that has not stopped.
    ///
    /// **Not an illegal transition**, which is why it is its own variant: the
    /// machine was never asked. A redispatch mints a replacement, so it is a
    /// refusal to *create* rather than to move, and there is no edge whose
    /// absence could have said so.
    NotRedispatchable { job: JobId, status: JobStatus },
    /// A redispatch was asked for on a Job that was rejected before it ran.
    ///
    /// **Its own variant because the reason differs in kind.** Every other
    /// refusal here is about where the Job is; this one is that nothing was
    /// ever produced to carry into a replacement.
    NeverRan { job: JobId },
    /// A redispatch was asked for on a Job that is a step of another Job.
    /// Replacing one is the parent's act, and nothing dispatches a sub-Job yet.
    NotReplaceable { job: JobId },
    /// A redispatch was asked for on a Job whose own `workflow_id` this Fleet
    /// no longer holds. The file it named was renamed or deleted after the
    /// Job was created — not the case `NoSuchWorkflow` names, which is a
    /// proposal, at creation, naming an id that never existed.
    WorkflowWithdrawn { job: JobId, workflow_id: String },
    /// A redirect was asked for on a Job whose Drone is gone.
    ///
    /// **The other act applies**, and this says so rather than respawning: a
    /// redirect that spawned would be a restart that lost nothing it could
    /// have kept, and the record would say a person injected context into a
    /// session that did not exist.
    NoDroneToRedirect { job: JobId },
    /// A kill named a pid that is not in the Job's process tree as Fleet read
    /// it at the act: it exited since the screen was drawn, its number went to
    /// something else, or it was never the Job's. `#1647`.
    NotTheJobsProcess { job: JobId, pid: u32 },
    /// An act was asked of a Job whose Checks are running again — a person's
    /// re-run, or a `merge_by: push` merge gating a moved base. Each would
    /// change the worktree the Checks are reading.
    ChecksRunningAgain { job: JobId },
    /// The task a dispatched request's proposal ran on ended without
    /// answering, which only a panic in it does. The Job it created, if it got
    /// that far, is moved at the next boot.
    ProposalAbandoned,
    /// The task a press ran on ended without answering, which only a panic in
    /// it does. **A fault and not a refusal**: nothing about the Job refused,
    /// and the Job is pressable again because the hold went with the task.
    PressAbandoned { job: JobId },
    /// Main's red could not be handed to a Job. `crate::main_fix`.
    MainNotFixable {
        root: String,
        why: crate::main_fix::MainNotFixable,
    },
    /// A proposal carried a title nothing could be picked out of a list by.
    Unnameable,
    /// A report could not be made from what was sent, in [`NotFiled`]'s words.
    ///
    /// **Not [`Adrift::Unreasoned`]**, which it shared until a filing with a
    /// long reason and an orphan `step_id` was told its reason was missing: one
    /// refusal for two acts could describe only one of them.
    NotFileable { job: JobId, cause: NotFiled },
    /// A proposal named a Manifest this Fleet does not hold. The same fault as
    /// [`Adrift::NoSuchWorkflow`], for the other id a proposal carries.
    NoSuchManifest { named: String, held: String },
    /// A Job whose Manifest no repository this Fleet serves declares. One store
    /// serves the machine, so the Job is real and is left where it stands.
    NotServed { job: JobId, manifest: String },
    /// `armada.yml` would not open for the file view that edits it.
    ///
    /// **Not a refused parse.** That one is `config::LoadError` and reaches a
    /// client as `ManifestReading`, with the previous values still in force.
    /// This is the file itself unreadable, which is Fleet's own ground failing.
    ManifestUnreadable { path: String, cause: io::Error },
    /// A corrected `armada.yml` would not go to disk. **Nothing about the text
    /// can raise this** — a save is never refused for what it says.
    ManifestUnwritable { path: String, cause: io::Error },
    /// `armada.yml` changed after the edit that was about to be saved read it.
    ///
    /// **Not a failure, and that is why it is not the variant above.** Nothing
    /// broke; what a person does next is reconcile rather than retry. What is
    /// on disk now travels with it so a surface can show both, and is absent
    /// where the file is no longer there at all.
    ManifestMovedUnderTheEdit {
        path: String,
        on_disk: Option<String>,
    },
    /// A request named a kept Check output no row of this Job carries.
    ///
    /// **The Job is there and the file is not**, which is why it is this rather
    /// than a missing Job: an id read off a row whose `.armada`
    /// directory has since been reclaimed, or a name that was never one. It is
    /// also what a caller naming anything else gets, because the Job's own rows
    /// are the only thing that resolves this id to a file.
    NoSuchCheckOutput { named: String },
    /// A request named a kept frame no row of this Job carries.
    ///
    /// **The Job is there and the image is not** — [`NoSuchCheckOutput`]'s
    /// distinction one record over, and it arrives for the same two reasons: a
    /// name read off a row whose `.armada/frames` directory has since been
    /// reclaimed, or a name that was never one.
    ///
    /// It is also what a caller naming anything at all gets where the Job kept
    /// no frames, because the Job's own rows are the only thing that resolves
    /// this name to a file. Nothing else decides what may be opened.
    ///
    /// [`NoSuchCheckOutput`]: Adrift::NoSuchCheckOutput
    NoSuchFrame { named: String },
    /// A request named a brief this Job's briefs directory does not hold.
    ///
    /// **The Job is there and the brief is not** — [`NoSuchCheckOutput`]'s
    /// distinction for the file a Judge or a gaming check was asked. It is
    /// also the answer to a name that would leave the directory, because the
    /// directory is the only thing that resolves this name to a file.
    ///
    /// [`NoSuchCheckOutput`]: Adrift::NoSuchCheckOutput
    NoSuchBrief { named: String },
    /// A merge line Check's log was asked for that Fleet will not open.
    ///
    /// **One answer for every reason**: a root this Fleet does not serve, a
    /// branch with no outcome, a Check its turn has not started, or a name
    /// that would leave the turn's directory. The outcome file is the only
    /// thing that resolves the three names to a file.
    NoSuchLandLog {
        root: String,
        branch: String,
        check: String,
    },
    /// The proposer call could not be configured — the same environment a Drone
    /// and a Judge are built from would not build.
    NotProposable(SpawnConfigRefused),
    /// What a request named is no Job of this Fleet's.
    ///
    /// **Not [`Adrift::Reading`]'s missing Job**, which is a ULID the store
    /// does not hold. This is one step earlier: the text a caller gave —
    /// a ULID, a handle or a number — resolved to nothing, and the refusal
    /// quotes what they said rather than a ULID they never typed.
    Unresolvable(ResolveJobError),
    /// A plain command — one that only reads and writes the store, or does a
    /// small amount of local work beside it — took longer than Fleet's own
    /// bound on it. **The command is not cancelled.** `crate::commanding`
    /// spawns it before racing the bound, so a write already in flight (or a
    /// second write a first one committed before, `redispatch_job`'s shape)
    /// runs to whatever end it was going to reach; only the answer to whoever
    /// was waiting stops here. `#712`.
    CommandTimedOut {
        job: Option<JobId>,
        waited: Duration,
    },
    /// A person's add or drop refused by the plan itself: no plan is recorded
    /// for the Job, or a task or a place to add after that the plan does not
    /// hold. `core_model::PlanRefused` is also what a Drone's own change is
    /// refused by, in `fleet::work_plan::NotPlanned` — this is the same set,
    /// read from a person's act instead. `#897`.
    PlanRefused { job: JobId, why: PlanRefused },
    /// git would not list the repository's branches. Carries its words.
    BranchesUnread { job: Option<JobId>, why: String },
    /// Restart this task named a task that has not failed: one still working
    /// or handed in is its group's own round to finish (answer 9), and one
    /// open, dropped, or done outside a group the Judge refused has nothing
    /// to restart. `#1656`.
    TaskNotFailed {
        job: JobId,
        named: TaskId,
        state: TaskState,
    },
    /// A person's move named a task, or a group holding one, that is working
    /// or handed in: its group's run is still going. `#1685`.
    TaskInFlight {
        job: JobId,
        named: TaskId,
        state: TaskState,
    },
    /// A Job's work product could not be read out of its worktree.
    ///
    /// **Never an empty diff.** A repository that will not open and an agent
    /// that changed nothing are opposite answers, and a reviewer handed the
    /// second when the first happened would take work that was never read.
    WorkUnreadable {
        job: JobId,
        cause: Box<dyn Error + Send + Sync>,
    },
}
impl Adrift {
}
