//! His other Macs, as this Mac's Phone Gateway reaches Fleet for them: what Fleet hands it to send the other
//! Mac, always through the share filter, and what it takes from it about the other Mac to keep and serve.
//! `docs/concepts/pocket.md`, *Another Mac*.
//!
//! **Its own surface**, for [`Watches`](super::Watches)' reason: whole on its own, and its caller is the
//! Gateway rather than Bridge or a Job.

use std::future::Future;
use std::sync::Arc;

use ipc::{
    ArtifactPageShared, BugReports, Event, JobAct, JobArtifact, JobMessage, JobThread, JobThreadRead, MacAct, MacClaimsWaiting, MacNamed, OpenOtherMacPage, OtherMacBugReports, OtherMacEvent,
    OtherMacPageOpened, OtherMacPreviewOffers, OtherMacProjects, OtherMacSessions, ProjectActFromMac, Projects, QueuedList, SessionId, SessionList, SessionThread, SessionWindow, TakeBack,
};
use ipc::{ArrivingJob, JobTerminals, ManifestSummary, MoveJobToMac, SessionRunUnderway, StartSession, TerminalFromMac, TerminalOfJob};

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

    /// `share_preview_offers` — this Mac's own ready offers that are on origin, for the other Mac's preview.
    fn share_preview_offers(&self) -> impl Future<Output = Result<ipc::PreviewOffers, Refusal>> + Send;

    /// `share_projects` — this Mac's own shared projects with their tasks. Never a copy held from another Mac.
    fn share_projects(&self) -> impl Future<Output = Result<Projects, Refusal>> + Send;

    /// Whether the phone sees Job `job`: one of this Mac's in a shared project, or one held from another Mac.
    fn job_for_phone(&self, job: &str) -> impl Future<Output = bool> + Send;

    /// Whether the phone sees terminal run `run_id`: one of a Job it sees.
    fn run_for_phone(&self, run_id: &str) -> impl Future<Output = bool> + Send;

    /// `event`, where the phone is sent it.
    fn event_for_phone(&self, event: Event) -> impl Future<Output = Option<Event>> + Send;

    /// A Job the phone starts here: into a shared project, and a fork only of a Job it sees. [`Refusal::NotFound`]
    /// otherwise, worded as for a project or Job that does not exist.
    fn start_from_phone(&self, start: &StartSession) -> impl Future<Output = Result<(), Refusal>> + Send;

    /// The projects the phone is named: this Mac's shared ones, and the copies held from another Mac.
    fn projects_for_phone(&self, projects: Projects) -> impl Future<Output = Projects> + Send;

    /// The repositories the phone is named: those in a project it sees.
    fn manifests_for_phone(&self, manifests: Vec<ManifestSummary>) -> impl Future<Output = Vec<ManifestSummary>> + Send;

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

    /// Whether his answer on Ready to try card `asked`, of a Job held from another Mac, may go there: **the build he
    /// runs here holds the card's branch at its tip**, so he answers what he tried. [`Refusal::IllegalMove`] where it does not.
    fn ready_in_this_build(&self, asked: &ipc::AnswerReadyToTry) -> Result<(), Refusal>;

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

    /// `share_job_session` — a shared Job's row and thread, as `get_session` answers, cut to `window` where there is one.
    /// [`Refusal::NotFound`] for a Job not shared, worded as for one that does not exist.
    fn share_job_session(self: Arc<Self>, job: SessionId, window: Option<SessionWindow>) -> impl Future<Output = Result<SessionThread, Refusal>> + Send;

    /// `share_job_artifacts` — a shared Job's sketches, drafts and pages, as `get_job_artifacts` lists them.
    /// [`Refusal::NotFound`] for a Job not shared, worded as for one that does not exist.
    fn share_job_artifacts(&self, job: SessionId) -> impl Future<Output = Result<Vec<JobArtifact>, Refusal>> + Send;

    /// `share_job_artifact_page` — one kept revision of a shared Job's page, sealed as `get_artifact_page` seals it.
    /// Refused as `share_job_artifacts` is, and as `get_artifact_page` refuses what the Job does not hold.
    fn share_job_artifact_page(
        &self,
        job: SessionId,
        artifact: String,
        revision: Option<u32>,
    ) -> impl Future<Output = Result<ArtifactPageShared, Refusal>> + Send;

    /// `share_job_page` — whether another Mac may be shown what this Mac serves on loopback port `port` for Job
    /// `job`: a page the Job showed, or a server it started that is serving. [`Refusal::NotFound`] for a Job not
    /// shared, worded as for a Job that does not exist.
    fn share_job_page(&self, job: SessionId, port: u16) -> impl Future<Output = Result<(), Refusal>> + Send;

    /// `open_other_mac_page` — a page a Job held from another Mac showed or serves, opened on this Mac through the
    /// link. A page of a Job of this Mac, and one not on the other Mac's loopback, is answered as it is.
    fn open_other_mac_page(&self, open: OpenOtherMacPage) -> impl Future<Output = Result<OtherMacPageOpened, Refusal>> + Send;

    /// `hold_other_mac_sessions` — every Job row Mac `mac` shares, replacing all held from it.
    fn hold_other_mac_sessions(&self, held: OtherMacSessions) -> impl Future<Output = Result<MacNamed, Refusal>> + Send;

    /// `hold_other_mac_projects` — every project Mac `mac` shares, replacing all held from it. Takes the
    /// `Arc` because edits queued for that Mac while it slept are sent on a task of their own.
    fn hold_other_mac_projects(self: Arc<Self>, held: OtherMacProjects) -> impl Future<Output = Result<MacNamed, Refusal>> + Send;

    /// `hold_other_mac_bug_reports` — every report Mac `mac` shares, replacing all held from it.
    fn hold_other_mac_bug_reports(&self, held: OtherMacBugReports) -> impl Future<Output = Result<MacNamed, Refusal>> + Send;

    /// `hold_other_mac_preview_offers` — every ready offer Mac `mac` shares, replacing all held from it. Takes
    /// the `Arc` because holding them asks for a check on a task of its own.
    fn hold_other_mac_preview_offers(self: Arc<Self>, held: OtherMacPreviewOffers) -> impl Future<Output = Result<MacNamed, Refusal>> + Send;

    /// `hold_other_mac_event` — one event that crossed from Mac `mac`. [`Refusal::Unacceptable`] for a kind
    /// that does not cross, [`Refusal::NotFound`] for one about a Job not held from that Mac.
    fn hold_other_mac_event(self: Arc<Self>, event: OtherMacEvent) -> impl Future<Output = Result<MacNamed, Refusal>> + Send;

    /// `act_from_mac`'s `job_message` — Job `message.from` on Mac `mac` wrote to Job `message.to` of this Mac, which
    /// the share filter already let through. Written into `to`'s thread in `from`'s voice with `mac` named, and `to`
    /// woken as for a message from a Job of this Mac. [`Refusal::NotFound`] for a Job not here, and
    /// [`Refusal::IllegalMove`] for one that has ended.
    fn message_from_mac(self: Arc<Self>, mac: String, message: JobMessage) -> impl Future<Output = Result<(), Refusal>> + Send;

    /// `hold_refresh_claim` — the Refresh Mac `mac` has under way, or `None` where it ended, held in memory and shown on
    /// `get_fleet_build`. [`Refusal::NotFound`] for a Mac this one is not paired with.
    fn hold_refresh_claim(&self, mac: String, sent: ipc::RefreshClaimSent) -> impl Future<Output = Result<(), Refusal>> + Send;

    /// `share_kit` — this Mac's Kit as it crosses: the kinds and items switched on, never a piece with a credential.
    /// `mac`, the Mac asking where the Gateway names it, has each piece carry this Mac's marker for it with that Mac.
    fn share_kit(&self, mac: Option<String>) -> impl Future<Output = Result<ipc::KitOffer, Refusal>> + Send;

    /// `hold_other_mac_kit` — what Mac `mac`'s Kit offers, kept, and taken here where it changed there only. Takes
    /// the `Arc` because what is queued for that Mac is sent on a task of its own.
    fn hold_other_mac_kit(self: Arc<Self>, held: ipc::OtherMacKit) -> impl Future<Output = Result<MacNamed, Refusal>> + Send;

    /// `share_home_layout` — the Cockpit home's layout this Mac keeps, with the stamp of the save that made it.
    fn share_home_layout(&self) -> impl Future<Output = Result<ipc::HomeLayout, Refusal>> + Send;

    /// `hold_other_mac_home_layout` — what Mac `mac` keeps of the layout, taken here where its save is the later one.
    fn hold_other_mac_home_layout(&self, held: ipc::OtherMacHomeLayout) -> impl Future<Output = Result<MacNamed, Refusal>> + Send;

    /// `kit_settle_from_mac` — a clash Mac `mac` settled, recorded here so the kept side crosses.
    /// [`Refusal::NotFound`] for a Mac this one is not paired with.
    fn kit_settle_from_mac(&self, mac: String, settled: ipc::KitSettled) -> impl Future<Output = Result<(), Refusal>> + Send;

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

    /// `move_job_to_mac` — the person moves a Job to another of his Macs. A Job this Mac holds waits for its turn
    /// to end, then is paused, pushed and handed to `asked.mac`; one held from another Mac is asked of that Mac,
    /// to come here. Answers the row as it stands, with `hosted.moving_to_mac` set while the move runs.
    fn move_job_to_mac(self: Arc<Self>, asked: MoveJobToMac) -> impl Future<Output = Result<ipc::SessionRecord, Refusal>> + Send;

    /// `act_from_mac`'s `move` — Mac `mac` asks for Job `job` of this Mac, as `move_job_to_mac` to it.
    fn move_from_mac(self: Arc<Self>, mac: String, job: SessionId) -> impl Future<Output = Result<ipc::SessionRecord, Refusal>> + Send;

    /// `act_from_mac`'s `arrive` — Mac `mac` hands this Mac a Job it moved. Answered once it is checked, with the row
    /// as sent; then its conversation is read from `mac` in pieces and checked, the Job is kept under its own id and
    /// resumed on its pushed branch in a fresh slot, and `mac` is sent a `move_settled` saying how it went.
    fn arrive_from_mac(self: Arc<Self>, mac: String, job: ArrivingJob) -> impl Future<Output = Result<ipc::SessionRecord, Refusal>> + Send;

    /// `act_from_mac`'s `move_settled` — the Mac a Job of this Mac moved to took it, and it is ended here, or refused
    /// it, and it is resumed here. [`Refusal::NotFound`] for a Job not moving to `mac`.
    fn move_settled_from_mac(self: Arc<Self>, mac: String, settled: ipc::MoveSettled) -> impl Future<Output = Result<(), Refusal>> + Send;

    /// `share_job_conversation` — up to [`CONVERSATION_PIECE`] bytes of Job `job`'s conversation from byte `from`,
    /// for Mac `mac`, which the Job is moving to. [`Refusal::NotFound`] for any other Job or Mac.
    fn share_job_conversation(&self, mac: String, job: SessionId, from: u64) -> impl Future<Output = Result<Vec<u8>, Refusal>> + Send;
}

/// The most of a conversation one read across the link carries: under the signed link's 1 MiB for a body.
pub const CONVERSATION_PIECE: u64 = 512 * 1024;

/// What `share_job_terminal` answers, as the same act answers at home.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerminalShared {
    Run(SessionRunUnderway),
    Terminals(JobTerminals),
}
