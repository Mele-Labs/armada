//! Autopilot, as the fake answers it: none on any repository, so each act on one is refused, and
//! a sentence read as itself. What Fleet keeps and picks is `fleet::autopilot`'s.

use ipc::{
    Autopilot, AutopilotList, AutopilotProposal, AutopilotSettings, ManifestId, ProposeAutopilot,
    WireError,
};

use super::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{Autopilots, Refusal};

fn none_here() -> Refusal {
    Refusal::IllegalMove(WireError::raised(
        "fake.no_autopilot",
        "the fake runs no Autopilot",
        run_id(),
    ))
}

impl Autopilots for FakeDaemon {
    async fn propose_autopilot(
        &self,
        _manifest_id: ManifestId,
        propose: ProposeAutopilot,
    ) -> Result<AutopilotProposal, Refusal> {
        Ok(AutopilotProposal {
            settings: AutopilotSettings {
                instruction: propose.sentence,
                at_once: 1,
                until: None,
                spend_cap_micros: None,
                job_cap: None,
            },
            quoted: Default::default(),
        })
    }

    async fn start_autopilot(
        &self,
        _manifest_id: ManifestId,
        _settings: AutopilotSettings,
    ) -> Result<Autopilot, Refusal> {
        Err(none_here())
    }

    async fn pause_autopilot(&self, _manifest_id: ManifestId) -> Result<Autopilot, Refusal> {
        Err(none_here())
    }

    async fn resume_autopilot(&self, _manifest_id: ManifestId) -> Result<Autopilot, Refusal> {
        Err(none_here())
    }

    async fn update_autopilot(
        &self,
        _manifest_id: ManifestId,
        _settings: AutopilotSettings,
    ) -> Result<Autopilot, Refusal> {
        Err(none_here())
    }

    async fn stop_autopilot(&self, _manifest_id: ManifestId) -> Result<Autopilot, Refusal> {
        Err(none_here())
    }

    async fn list_autopilots(&self) -> Result<AutopilotList, Refusal> {
        Ok(AutopilotList::default())
    }

    async fn get_autopilot(&self, _manifest_id: ManifestId) -> Result<Autopilot, Refusal> {
        Err(none_here())
    }

    async fn dispatch_set_aside(
        self: std::sync::Arc<Self>,
        _manifest_id: ManifestId,
        _pick_id: String,
    ) -> Result<Autopilot, Refusal> {
        Err(none_here())
    }

    async fn dismiss_set_aside(
        &self,
        _manifest_id: ManifestId,
        _pick_id: String,
    ) -> Result<Autopilot, Refusal> {
        Err(none_here())
    }
}
