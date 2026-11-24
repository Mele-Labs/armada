//! His other Macs, as the fake answers them: nothing is shared or kept, and one Studio stands for what the share
//! filter holds back, so the socket can be shown to drop what it is told to.

use std::sync::Arc;

use ipc::{
    BugReports, Event, JobAct, JobMessage, JobThread, JobThreadRead, MacAct, MacNamed, OpenOtherMacPage, OtherMacBugReports, OtherMacEvent, OtherMacPageOpened,
    OtherMacProjects, OtherMacSessions, ProjectActFromMac, Projects, QueuedList, SessionId, SessionList, TakeBack,
};

use super::FakeDaemon;
use crate::{ActAnswered, ActBegun, ActTurn, OtherMacs, Refusal};

/// The Studio whose deletion the fake's share filter holds back.
pub const THE_PRIVATE_STUDIO: &str = "01PRIVATE";

fn no_such_job(job: &SessionId) -> Refusal {
    Refusal::NotFound(ipc::WireError::raised("fleet.no_such_session", format!("No Job Fleet holds is named {}.", job.as_str()), super::shapes::run_id()))
}

impl OtherMacs for FakeDaemon {
    async fn share_sessions(&self) -> Result<SessionList, Refusal> {
        Ok(SessionList { sessions: Vec::new() })
    }

    async fn share_preview_offers(&self) -> Result<ipc::PreviewOffers, Refusal> {
        Ok(ipc::PreviewOffers { offers: Vec::new(), refresh: None, left_out: Vec::new() })
    }

    async fn share_bug_reports(&self) -> Result<BugReports, Refusal> {
        Ok(BugReports { reports: Vec::new() })
    }

    async fn share_projects(&self) -> Result<Projects, Refusal> {
        Ok(Projects { projects: Vec::new(), loose: Vec::new(), left_out: Vec::new(), left_behind: Vec::new() })
    }

    async fn job_for_phone(&self, _job: &str) -> bool {
        true
    }

    async fn run_for_phone(&self, _run_id: &str) -> bool {
        true
    }

    async fn event_for_phone(&self, event: Event) -> Option<Event> {
        Some(event)
    }

    async fn start_from_phone(&self, _start: &ipc::StartSession) -> Result<(), Refusal> {
        Ok(())
    }

    async fn projects_for_phone(&self, projects: Projects) -> Projects {
        projects
    }

    async fn manifests_for_phone(&self, manifests: Vec<ipc::ManifestSummary>) -> Vec<ipc::ManifestSummary> {
        manifests
    }

    async fn project_act_from_mac(&self, _sent: ProjectActFromMac) -> Result<Projects, Refusal> {
        Ok(Projects { projects: Vec::new(), loose: Vec::new(), left_out: Vec::new(), left_behind: Vec::new() })
    }

    async fn begin_act_from_mac(&self, _mac: String, sent: &MacAct) -> Result<ActBegun, Refusal> {
        Err(Refusal::NotFound(ipc::WireError::raised(
            "fleet.no_such_session",
            format!("No Job Fleet holds is named {}.", sent.act.session_id().map_or("", SessionId::as_str)),
            super::shapes::run_id(),
        )))
    }

    async fn finish_act_from_mac(&self, _turn: ActTurn, _status: u16, _body: &[u8]) {}

    async fn mac_holding_job(&self, _job: &str) -> Option<String> {
        None
    }

    async fn mac_holding_pull(&self, _repository: &str, _number: u64) -> Option<(String, SessionId)> {
        None
    }

    fn ready_in_this_build(&self, _asked: &ipc::AnswerReadyToTry) -> Result<(), Refusal> {
        Ok(())
    }

    async fn act_on_other_mac(self: Arc<Self>, mac: String, _act: JobAct) -> Result<ActAnswered, Refusal> {
        Err(Refusal::NotFound(ipc::WireError::raised("fleet.other_mac_not_linked", format!("This Mac is not linked to Mac {mac}."), super::shapes::run_id())))
    }

    async fn list_queued(&self, _mac: Option<String>) -> Result<QueuedList, Refusal> {
        Ok(QueuedList { queued: Vec::new() })
    }

    async fn take_back_queued(&self, take: TakeBack) -> Result<QueuedList, Refusal> {
        Err(Refusal::NotFound(ipc::WireError::raised("fleet.queued_not_found", format!("Nothing waits under {}.", take.id), super::shapes::run_id())))
    }

    async fn share_event(&self, event: Event) -> Option<Event> {
        match &event {
            Event::StudioDeleted(gone) if gone.id.as_str() == THE_PRIVATE_STUDIO => None,
            _ => Some(event),
        }
    }

    async fn share_job_thread(self: Arc<Self>, read: JobThreadRead) -> Result<JobThread, Refusal> {
        Err(Refusal::NotFound(ipc::WireError::raised(
            "fleet.no_such_session",
            format!("No Job Fleet holds is named {}.", read.session_id.as_str()),
            super::shapes::run_id(),
        )))
    }

    async fn share_job_session(self: Arc<Self>, job: SessionId, _window: Option<ipc::SessionWindow>) -> Result<ipc::SessionThread, Refusal> {
        Err(no_such_job(&job))
    }

    async fn share_job_artifacts(&self, job: SessionId) -> Result<Vec<ipc::JobArtifact>, Refusal> {
        Err(no_such_job(&job))
    }

    async fn share_job_artifact_page(&self, job: SessionId, _artifact: String, _revision: Option<u32>) -> Result<ipc::ArtifactPageShared, Refusal> {
        Err(no_such_job(&job))
    }

    async fn share_job_page(&self, job: SessionId, _port: u16) -> Result<(), Refusal> {
        Err(Refusal::NotFound(ipc::WireError::raised(
            "fleet.no_such_session",
            format!("No Job Fleet holds is named {}.", job.as_str()),
            super::shapes::run_id(),
        )))
    }

    async fn open_other_mac_page(&self, open: OpenOtherMacPage) -> Result<OtherMacPageOpened, Refusal> {
        Ok(OtherMacPageOpened { url: open.url })
    }

    async fn hold_other_mac_sessions(&self, held: OtherMacSessions) -> Result<MacNamed, Refusal> {
        Ok(MacNamed { mac: held.mac })
    }

    async fn hold_other_mac_projects(self: Arc<Self>, held: OtherMacProjects) -> Result<MacNamed, Refusal> {
        Ok(MacNamed { mac: held.mac })
    }

    async fn hold_other_mac_bug_reports(&self, held: OtherMacBugReports) -> Result<MacNamed, Refusal> {
        Ok(MacNamed { mac: held.mac })
    }

    async fn hold_other_mac_preview_offers(self: Arc<Self>, held: ipc::OtherMacPreviewOffers) -> Result<MacNamed, Refusal> {
        Ok(MacNamed { mac: held.mac })
    }

    async fn hold_other_mac_event(self: Arc<Self>, event: OtherMacEvent) -> Result<MacNamed, Refusal> {
        Ok(MacNamed { mac: event.mac })
    }

    async fn message_from_mac(self: Arc<Self>, _mac: String, message: JobMessage) -> Result<(), Refusal> {
        Err(Refusal::NotFound(ipc::WireError::raised(
            "fleet.no_such_session",
            format!("No Job has the id {}.", message.to.as_str()),
            super::shapes::run_id(),
        )))
    }

    async fn hold_refresh_claim(&self, _mac: String, _sent: ipc::RefreshClaimSent) -> Result<(), Refusal> {
        Ok(())
    }

    async fn share_kit(&self, _mac: Option<String>) -> Result<ipc::KitOffer, Refusal> {
        Ok(ipc::KitOffer::default())
    }

    async fn hold_other_mac_kit(self: Arc<Self>, held: ipc::OtherMacKit) -> Result<MacNamed, Refusal> {
        Ok(MacNamed { mac: held.mac })
    }

    async fn kit_settle_from_mac(&self, _mac: String, _settled: ipc::KitSettled) -> Result<(), Refusal> {
        Ok(())
    }

    async fn forget_other_mac(&self, mac: MacNamed) -> Result<MacNamed, Refusal> {
        Ok(mac)
    }

    async fn hold_mac_claims(&self, claims: ipc::MacClaimsWaiting) -> Result<ipc::MacClaimsWaiting, Refusal> {
        Ok(claims)
    }

    async fn share_job_terminal(self: Arc<Self>, asked: ipc::TerminalFromMac) -> Result<crate::TerminalShared, Refusal> {
        Err(no_job(asked.session_id()))
    }

    async fn share_job_terminal_output(&self, which: ipc::TerminalOfJob) -> Result<crate::ObservedSessionRun, Refusal> {
        Err(no_job(&which.session_id))
    }

    async fn move_job_to_mac(self: Arc<Self>, asked: ipc::MoveJobToMac) -> Result<ipc::SessionRecord, Refusal> {
        Err(no_job(&asked.session_id))
    }

    async fn move_from_mac(self: Arc<Self>, _mac: String, job: SessionId) -> Result<ipc::SessionRecord, Refusal> {
        Err(no_job(&job))
    }

    async fn arrive_from_mac(self: Arc<Self>, _mac: String, job: ipc::ArrivingJob) -> Result<ipc::SessionRecord, Refusal> {
        Err(no_job(&job.record.id))
    }

    async fn move_settled_from_mac(self: Arc<Self>, _mac: String, settled: ipc::MoveSettled) -> Result<(), Refusal> {
        Err(no_job(&settled.session_id))
    }

    async fn share_job_conversation(&self, _mac: String, job: SessionId, _from: u64) -> Result<Vec<u8>, Refusal> {
        Err(no_job(&job))
    }
}

/// Nothing is shared here, so every Job is answered as one that does not exist.
fn no_job(job: &SessionId) -> Refusal {
    Refusal::NotFound(ipc::WireError::raised("fleet.no_such_session", format!("No Job has the id {}.", job.as_str()), super::shapes::run_id()))
}
