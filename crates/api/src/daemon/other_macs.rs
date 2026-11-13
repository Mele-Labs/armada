//! His other Macs, as this Mac's Phone Gateway reaches Fleet for them: what Fleet hands it to send the other
//! Mac, always through the share filter, and what it takes from it about the other Mac to keep and serve.
//! `docs/concepts/pocket.md`, *Another Mac*.
//!
//! **Its own surface**, for [`Watches`](super::Watches)' reason: whole on its own, and its caller is the
//! Gateway rather than Bridge or a Job.

use std::future::Future;
use std::sync::Arc;

use ipc::{
    BugReports, Event, JobAct, JobMessage, JobThread, JobThreadRead, MacAct, MacClaimsWaiting, MacNamed, OtherMacBugReports, OtherMacEvent, OtherMacProjects, OtherMacSessions,
    ProjectActFromMac, Projects, QueuedList, SessionId, SessionList, TakeBack,
};
use ipc::{JobTerminals, SessionRunUnderway, TerminalFromMac, TerminalOfJob};

use crate::daemon::Refusal;

/// What a Mac answered an act with, to be handed back as it was said: the status and the body's bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActAnswered {
    pub status: u16,
    pub body: Vec<u8>,
}

/// The right to run one act another Mac sent, held until its answer is kept. **One act from a Mac runs at a time**,
/// so a send and its retry cannot both run.
pub struct ActTurn {
    pub mac: String,
    pub id: String,
    pub what: &'static str,
    /// What holds the others off, dropped with the turn.
    pub held: Box<dyn std::any::Any + Send>,
}

/// What `begin_act_from_mac` decided.
pub enum ActBegun {
    /// The act was run before: its answer, as it was.
    Answered(ActAnswered),
    /// It has not been: run it, then hand the answer to `finish_act_from_mac`.
    Run(ActTurn),
}

pub trait OtherMacs: Send + Sync + 'static {
    /// `share_sessions` — this Mac's own Jobs in a project shared with his other Macs, whole. Never a row
    /// held from another Mac.
    fn share_sessions(&self) -> impl Future<Output = Result<SessionList, Refusal>> + Send;

    /// `share_bug_reports` — this Mac's own reports whose Job is in a shared project.
    fn share_bug_reports(&self) -> impl Future<Output = Result<BugReports, Refusal>> + Send;

    /// `share_projects` — this Mac's own shared projects with their tasks. Never a copy held from another Mac.
    fn share_projects(&self) -> impl Future<Output = Result<Projects, Refusal>> + Send;

    /// `project_act_from_mac` — a task act another Mac made on its copy of a shared project of this Mac, run
    /// here. [`Refusal::NotFound`] for a row or project that is not shared, worded as for one that does not
    /// exist. Answers this Mac's shared projects.
    fn project_act_from_mac(&self, sent: ProjectActFromMac) -> impl Future<Output = Result<Projects, Refusal>> + Send;

    /// `act_from_mac` — the part before the act runs: the share filter, and whether the act ran already. **An act on
    /// a Job that is not shared is [`Refusal::NotFound`], worded as for one that does not exist, with nothing written.**
    fn begin_act_from_mac(&self, mac: String, sent: &MacAct) -> impl Future<Output = Result<ActBegun, Refusal>> + Send;

    /// `act_from_mac` — the part after: `status` and `body`, what the act answered, kept under the Mac and id so a
    /// repeat is answered with them. A fault (5xx) is not kept, so a retry runs.
    fn finish_act_from_mac(&self, turn: ActTurn, status: u16, body: &[u8]) -> impl Future<Output = ()> + Send;

    /// The Mac Job `job` is held from. **None where this Mac holds a Job by that id, and for a call that came from
    /// another Mac**, which is never sent on to a third.
    fn mac_holding_job(&self, job: &str) -> impl Future<Output = Option<String>> + Send;

    /// The Mac and Job that own pull request `number` of `repository`, where no Job of this Mac does and a Job held
    /// from another Mac does.
    fn mac_holding_pull(&self, repository: &str, number: u64) -> impl Future<Output = Option<(String, SessionId)>> + Send;

    /// `act` sent to Mac `mac`, which holds the Job, and its answer passed back as it was said. **202 with the
    /// `Queued` where that Mac cannot be reached or has acts waiting**; a Mac this one is not linked to is refused by name.
    fn act_on_other_mac(self: Arc<Self>, mac: String, act: JobAct) -> impl Future<Output = Result<ActAnswered, Refusal>> + Send;

    /// `list_queued` — the acts waiting for another Mac, oldest first.
    fn list_queued(&self, mac: Option<String>) -> impl Future<Output = Result<QueuedList, Refusal>> + Send;

    /// `take_back_queued` — an act that waits, removed. [`Refusal::IllegalMove`] while its Mac's queue is being sent.
    fn take_back_queued(&self, take: TakeBack) -> impl Future<Output = Result<QueuedList, Refusal>> + Send;

    /// `share_events` — what of `event` may go to another Mac, whole or cut. `None` sends nothing.
    fn share_event(&self, event: Event) -> impl Future<Output = Option<Event>> + Send;

    /// `share_job_thread` — a shared Job's thread. [`Refusal::NotFound`] for any other, worded as for a Job
    /// that does not exist.
    fn share_job_thread(self: Arc<Self>, read: JobThreadRead) -> impl Future<Output = Result<JobThread, Refusal>> + Send;

    /// `hold_other_mac_sessions` — every Job row Mac `mac` shares, replacing all held from it.
    fn hold_other_mac_sessions(&self, held: OtherMacSessions) -> impl Future<Output = Result<MacNamed, Refusal>> + Send;

    /// `hold_other_mac_projects` — every project Mac `mac` shares, replacing all held from it. Takes the
    /// `Arc` because edits queued for that Mac while it slept are sent on a task of their own.
    fn hold_other_mac_projects(self: Arc<Self>, held: OtherMacProjects) -> impl Future<Output = Result<MacNamed, Refusal>> + Send;

    /// `hold_other_mac_bug_reports` — every report Mac `mac` shares, replacing all held from it.
    fn hold_other_mac_bug_reports(&self, held: OtherMacBugReports) -> impl Future<Output = Result<MacNamed, Refusal>> + Send;

    /// `hold_other_mac_event` — one event that crossed from Mac `mac`. [`Refusal::Unacceptable`] for a kind
    /// that does not cross, [`Refusal::NotFound`] for one about a Job not held from that Mac.
    fn hold_other_mac_event(self: Arc<Self>, event: OtherMacEvent) -> impl Future<Output = Result<MacNamed, Refusal>> + Send;

    /// `act_from_mac`'s `job_message` — Job `message.from` on Mac `mac` wrote to Job `message.to` of this Mac, which
    /// the share filter already let through. Written into `to`'s thread in `from`'s voice with `mac` named, and `to`
    /// woken as for a message from a Job of this Mac. [`Refusal::NotFound`] for a Job not here, and
    /// [`Refusal::IllegalMove`] for one that has ended.
    fn message_from_mac(self: Arc<Self>, mac: String, message: JobMessage) -> impl Future<Output = Result<(), Refusal>> + Send;

    /// `forget_other_mac` — every row and report held from Mac `mac` dropped.
    fn forget_other_mac(&self, mac: MacNamed) -> impl Future<Output = Result<MacNamed, Refusal>> + Send;

    /// `hold_mac_claims` — every other Mac waiting for his Confirm here, replacing all held, each a call on him.
    fn hold_mac_claims(&self, claims: MacClaimsWaiting) -> impl Future<Output = Result<MacClaimsWaiting, Refusal>> + Send;

    /// `share_job_terminal` — another Mac opens, lists, types into, resizes or stops a terminal of a shared Job, as
    /// the person does at home. [`Refusal::NotFound`] for a Job not shared, worded as for a Job that does not exist,
    /// and for a run that is not one of the Job's open terminals. Takes the `Arc` as `open_terminal` does.
    fn share_job_terminal(self: Arc<Self>, asked: TerminalFromMac) -> impl Future<Output = Result<TerminalShared, Refusal>> + Send;

    /// `share_job_terminal_output` — what one of a shared Job's open terminals printed and prints next, refused as
    /// `share_job_terminal` is.
    fn share_job_terminal_output(&self, which: TerminalOfJob) -> impl Future<Output = Result<crate::ObservedSessionRun, Refusal>> + Send;
}

/// What `share_job_terminal` answers, as the same act answers at home.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerminalShared {
    Run(SessionRunUnderway),
    Terminals(JobTerminals),
}
