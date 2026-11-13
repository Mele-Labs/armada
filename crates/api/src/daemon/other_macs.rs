//! His other Macs, as this Mac's Phone Gateway reaches Fleet for them: what Fleet hands it to send the other
//! Mac, always through the share filter, and what it takes from it about the other Mac to keep and serve.
//! `docs/concepts/pocket.md`, *Another Mac*.
//!
//! **Its own surface**, for [`Watches`](super::Watches)' reason: whole on its own, and its caller is the
//! Gateway rather than Bridge or a Job.

use std::future::Future;
use std::sync::Arc;

use ipc::{BugReports, Event, JobThread, JobThreadRead, MacNamed, OtherMacBugReports, OtherMacEvent, OtherMacProjects, OtherMacSessions, ProjectActFromMac, Projects, SessionList};

use crate::daemon::Refusal;

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

    /// `forget_other_mac` — every row and report held from Mac `mac` dropped.
    fn forget_other_mac(&self, mac: MacNamed) -> impl Future<Output = Result<MacNamed, Refusal>> + Send;
}
