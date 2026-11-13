//! His other Macs, as the fake answers them: nothing is shared or kept, and one Studio stands for what the share
//! filter holds back, so the socket can be shown to drop what it is told to.

use std::sync::Arc;

use ipc::{BugReports, Event, JobThread, JobThreadRead, MacNamed, OtherMacBugReports, OtherMacEvent, OtherMacProjects, OtherMacSessions, ProjectActFromMac, Projects, SessionList};

use super::FakeDaemon;
use crate::{OtherMacs, Refusal};

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

    async fn forget_other_mac(&self, mac: MacNamed) -> Result<MacNamed, Refusal> {
        Ok(mac)
    }
}
