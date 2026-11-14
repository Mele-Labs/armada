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

impl OtherMacs for FakeDaemon {
    async fn share_sessions(&self) -> Result<SessionList, Refusal> {
        Ok(SessionList { sessions: Vec::new() })
    }

    async fn share_bug_reports(&self) -> Result<BugReports, Refusal> {
        Ok(BugReports { reports: Vec::new() })
    }

    async fn share_projects(&self) -> Result<Projects, Refusal> {
        Ok(Projects { projects: Vec::new(), loose: Vec::new(), left_out: Vec::new() })
    }

    async fn project_act_from_mac(&self, _sent: ProjectActFromMac) -> Result<Projects, Refusal> {
        Ok(Projects { projects: Vec::new(), loose: Vec::new(), left_out: Vec::new() })
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
}

/// Nothing is shared here, so every Job is answered as one that does not exist.
fn no_job(job: &SessionId) -> Refusal {
    Refusal::NotFound(ipc::WireError::raised("fleet.no_such_session", format!("No Job has the id {}.", job.as_str()), super::shapes::run_id()))
}
