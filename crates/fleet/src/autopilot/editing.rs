//! Changing a running or paused Autopilot's settings, and the reading of settings a person
//! confirmed that a start and a change share. #2105.
//!
//! **A change never resumes one.** A cap that paused it stays named on it until a person's Resume,
//! and raising that cap is what lets the Resume through: the owner's *fix the gaps*, 12 Oct 2026.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::Timestamp;
use ipc::{Autopilot, AutopilotSettings, ManifestId};
use store::KeptAutopilot;

use super::acts::{STOPPED, UNSETTABLE};
use super::bounds::{Caps, Settings, State};
use super::sentence::utc;
use crate::daemon::Fleet;

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// `settings` as a person confirmed them, or the 422 that says what nothing could run under.
    pub(super) fn confirmed(
        &self,
        settings: &AutopilotSettings,
        now: &Timestamp,
    ) -> Result<Settings, Refusal> {
        let unsettable = |said: String| Refusal::Unacceptable(self.refused_as(UNSETTABLE, said));
        let until = match settings.until.as_ref() {
            None => None,
            Some(said) => Some(
                utc(said.as_str())
                    .ok_or_else(|| unsettable(format!("`{}` is not a time", said.as_str())))?,
            ),
        };
        let caps = Caps {
            until,
            spend_cap_micros: settings.spend_cap_micros,
            session_cap: settings.session_cap,
        };
        Settings::confirmed(&settings.instruction, settings.at_once, caps, now)
            .map_err(|why| unsettable(why.to_string()))
    }

    /// `update_autopilot`: the new settings on the run, its state, `capped_by` and picks kept.
    pub(super) async fn autopilot_updated(
        &self,
        manifest_id: &ManifestId,
        settings: &AutopilotSettings,
    ) -> Result<Autopilot, Refusal> {
        let kept = self.kept_autopilot(manifest_id).await?;
        if State::from_stored(&kept.state).unwrap_or(State::Stopped) == State::Stopped {
            return Err(Refusal::IllegalMove(self.refused_as(
                STOPPED,
                "Autopilot is stopped. Start it again with these settings instead",
            )));
        }
        let confirmed = self.confirmed(settings, &self.now())?;
        let changed = KeptAutopilot {
            instruction: confirmed.instruction().to_string(),
            at_once: confirmed.at_once(),
            until: confirmed
                .caps()
                .until
                .as_ref()
                .map(|at| at.as_str().to_string()),
            spend_cap_micros: confirmed.caps().spend_cap_micros,
            session_cap: confirmed.caps().session_cap,
            ..kept
        };
        self.store()
            .lock()
            .await
            .update_autopilot(&changed)
            .map_err(|why| self.faulted(why))?;
        // A new instruction may cover what the picker turned down under the old one.
        self.tracking().autopilot.forget(manifest_id.as_str());
        self.answered(self.kept_autopilot(manifest_id).await?).await
    }
}
