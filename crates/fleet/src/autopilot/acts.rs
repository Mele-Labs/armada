//! The acts a person takes on an Autopilot, as `api::Autopilots` serves them.
//!
//! **Pause and Stop never stop a Job that is spending**: that is the Job's own Pause. Pause stops
//! the picks; Stop also withdraws a pick whose Job has never had a Drone, and ends it.
//! The owner's call of 11 Oct 2026, `2026-10-11-pause-drains-stop-withdraws.md`.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Autopilots, Redirector, Refusal};
use core_model::TopLevelOrigin;
use ipc::{Autopilot, AutopilotList, AutopilotSettings, ManifestId, ProposeAutopilot, WireError};
use store::KeptAutopilot;

use super::bounds::{Caps, NotNow, PickState, Settings, State};
use super::sentence::{settings_from, utc};
use super::wire::job_id;
use crate::daemon::Fleet;

const NO_AUTOPILOT: &str = "fleet.no_autopilot";
const RUNNING: &str = "fleet.autopilot_running";
const NOT_RUNNING: &str = "fleet.autopilot_not_running";
const NOT_PAUSED: &str = "fleet.autopilot_not_paused";
const STOPPED: &str = "fleet.autopilot_stopped";
const UNSETTABLE: &str = "fleet.autopilot_unsettable";
const NOT_READ: &str = "fleet.autopilot_not_read";
const NOT_SET_ASIDE: &str = "fleet.pick_not_set_aside";

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
    fn refused_as(&self, code: &str, said: impl Into<String>) -> WireError {
        WireError::raised(code, said.into(), self.run_id())
    }

    fn faulted(&self, why: impl std::fmt::Debug) -> Refusal {
        Refusal::Fault(self.refused_as("fleet.autopilot_unreadable", format!("{why:?}")))
    }

    async fn kept_autopilot(&self, manifest_id: &ManifestId) -> Result<KeptAutopilot, Refusal> {
        self.served_named(Some(manifest_id))?;
        let kept = self
            .store()
            .lock()
            .await
            .autopilot(manifest_id.as_str())
            .map_err(|why| self.faulted(why))?;
        kept.ok_or_else(|| {
            Refusal::IllegalMove(self.refused_as(NO_AUTOPILOT, "this repository has no Autopilot"))
        })
    }

    async fn answered(&self, kept: KeptAutopilot) -> Result<Autopilot, Refusal> {
        let gathered = self
            .autopilot_gathered(kept)
            .await
            .map_err(|why| self.refusal(why))?;
        let now = self.autopilot_wire(&gathered);
        // Told here as well as answered, so a second Bridge sees the act.
        if self.tracking().autopilot.moved(&now) {
            self.publish(ipc::Event::AutopilotChanged(now.clone()));
        }
        Ok(now)
    }

    fn not_now(&self, refused: NotNow) -> Refusal {
        let (code, said) = match refused.from {
            State::Running => (RUNNING, "Autopilot is running"),
            State::Paused => (NOT_RUNNING, "Autopilot is paused"),
            State::Stopped => (STOPPED, "Autopilot is stopped"),
        };
        Refusal::IllegalMove(self.refused_as(code, said))
    }

    async fn moved_to(&self, kept: &KeptAutopilot, to: State) -> Result<(), Refusal> {
        // What the last pass said no longer describes an Autopilot a person just moved.
        self.tracking().autopilot.faulted(&kept.manifest_id, None);
        let ended = (to == State::Stopped).then(|| self.now());
        self.store()
            .lock()
            .await
            .move_autopilot(
                &kept.run_id,
                to.as_stored(),
                ended.as_ref().map(|at| at.as_str()),
                None,
            )
            .map(|_| ())
            .map_err(|why| self.faulted(why))
    }

    /// Withdraw every started pick whose Job has never had a Drone: the Job is killed, as the
    /// person's act, and the pick reads `withdrawn`.
    async fn withdrawn(&self, kept: KeptAutopilot) -> Result<(), Refusal> {
        let gathered = self
            .autopilot_gathered(kept)
            .await
            .map_err(|why| self.refusal(why))?;
        for pick in &gathered.picks {
            let (Some(job), Some(standing)) =
                (&pick.started_job, gathered.standing.get(&pick.pick_id))
            else {
                continue;
            };
            if PickState::from_stored(&pick.state) != Some(PickState::Started)
                || !standing.withdrawn_at_stop()
            {
                continue;
            }
            self.kill_job(&job_id(job))
                .await
                .map_err(|why| self.refusal(why))?;
            self.store()
                .lock()
                .await
                .move_autopilot_pick(
                    &pick.pick_id,
                    "started",
                    PickState::Withdrawn.as_stored(),
                    None,
                )
                .map_err(|why| self.faulted(why))?;
        }
        Ok(())
    }

    /// The pick `pick_id` of this run, where it is set aside.
    async fn set_aside_pick(
        &self,
        kept: &KeptAutopilot,
        pick_id: &str,
    ) -> Result<store::KeptPick, Refusal> {
        let picks = self
            .store()
            .lock()
            .await
            .autopilot_picks(&kept.run_id)
            .map_err(|why| self.faulted(why))?;
        picks
            .into_iter()
            .find(|one| one.pick_id == pick_id && one.state == PickState::SetAside.as_stored())
            .ok_or_else(|| {
                Refusal::IllegalMove(self.refused_as(
                    NOT_SET_ASIDE,
                    "no pick of this Autopilot is set aside under that id",
                ))
            })
    }
}

impl<H, V, W> Autopilots for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    async fn propose_autopilot(
        &self,
        manifest_id: ManifestId,
        propose: ProposeAutopilot,
    ) -> Result<AutopilotSettings, Refusal> {
        self.served_named(Some(&manifest_id))?;
        if propose.sentence.trim().is_empty() {
            return Err(Refusal::Unacceptable(
                self.refused_as(UNSETTABLE, "there is no sentence to read"),
            ));
        }
        let asking = self.asking().map_err(|why| self.faulted(why))?;
        let read = settings_from(
            &propose.sentence,
            &self.now(),
            propose.local_now.as_deref(),
            &asking,
        )
        .await
        .map_err(|why| Refusal::Fault(self.refused_as(NOT_READ, why.to_string())))?;
        Ok(AutopilotSettings {
            instruction: read.instruction,
            at_once: read.at_once,
            until: read.until.as_ref().map(ipc::Instant::from),
            spend_cap_micros: read.spend_cap_micros,
            job_cap: read.job_cap,
        })
    }

    async fn start_autopilot(
        &self,
        manifest_id: ManifestId,
        settings: AutopilotSettings,
    ) -> Result<Autopilot, Refusal> {
        self.served_named(Some(&manifest_id))?;
        let held = self
            .store()
            .lock()
            .await
            .autopilot(manifest_id.as_str())
            .map_err(|why| self.faulted(why))?;
        if let Some(held) =
            held.filter(|held| State::from_stored(&held.state) != Some(State::Stopped))
        {
            let from = State::from_stored(&held.state).unwrap_or(State::Running);
            return Err(self.not_now(NotNow { from }));
        }
        let until = match settings.until.as_ref() {
            None => None,
            Some(said) => Some(utc(said.as_str()).ok_or_else(|| {
                Refusal::Unacceptable(
                    self.refused_as(UNSETTABLE, format!("`{}` is not a time", said.as_str())),
                )
            })?),
        };
        let caps = Caps {
            until,
            spend_cap_micros: settings.spend_cap_micros,
            job_cap: settings.job_cap,
        };
        let now = self.now();
        let confirmed = Settings::confirmed(&settings.instruction, settings.at_once, caps, &now)
            .map_err(|why| Refusal::Unacceptable(self.refused_as(UNSETTABLE, why.to_string())))?;
        let kept = KeptAutopilot {
            manifest_id: manifest_id.as_str().to_string(),
            run_id: self.mint().ulid().as_str().to_string(),
            instruction: confirmed.instruction().to_string(),
            at_once: confirmed.at_once(),
            until: confirmed
                .caps()
                .until
                .as_ref()
                .map(|at| at.as_str().to_string()),
            spend_cap_micros: confirmed.caps().spend_cap_micros,
            job_cap: confirmed.caps().job_cap,
            state: State::Running.as_stored().to_string(),
            started_at: now.as_str().to_string(),
            ended_at: None,
            capped_by: None,
        };
        self.store()
            .lock()
            .await
            .start_autopilot(&kept)
            .map_err(|why| self.faulted(why))?;
        // A new instruction may cover what the last one's picker turned down.
        self.tracking().autopilot.forget(manifest_id.as_str());
        self.tracking()
            .autopilot
            .faulted(manifest_id.as_str(), None);
        self.answered(kept).await
    }

    async fn pause_autopilot(&self, manifest_id: ManifestId) -> Result<Autopilot, Refusal> {
        let kept = self.kept_autopilot(&manifest_id).await?;
        let from = State::from_stored(&kept.state).unwrap_or(State::Stopped);
        let to = from.paused().map_err(|refused| match refused.from {
            State::Paused => {
                Refusal::IllegalMove(self.refused_as(NOT_RUNNING, "Autopilot is already paused"))
            }
            _ => self.not_now(refused),
        })?;
        self.moved_to(&kept, to).await?;
        self.answered(self.kept_autopilot(&manifest_id).await?)
            .await
    }

    async fn resume_autopilot(&self, manifest_id: ManifestId) -> Result<Autopilot, Refusal> {
        let kept = self.kept_autopilot(&manifest_id).await?;
        let from = State::from_stored(&kept.state).unwrap_or(State::Stopped);
        let to = from.resumed().map_err(|refused| match refused.from {
            State::Running => {
                Refusal::IllegalMove(self.refused_as(NOT_PAUSED, "Autopilot is not paused"))
            }
            _ => self.not_now(refused),
        })?;
        self.moved_to(&kept, to).await?;
        self.answered(self.kept_autopilot(&manifest_id).await?)
            .await
    }

    async fn stop_autopilot(&self, manifest_id: ManifestId) -> Result<Autopilot, Refusal> {
        let kept = self.kept_autopilot(&manifest_id).await?;
        let from = State::from_stored(&kept.state).unwrap_or(State::Stopped);
        let to = from.stopped().map_err(|refused| self.not_now(refused))?;
        // Stopped first, so a pass already out does not keep a pick after the withdrawal.
        self.moved_to(&kept, to).await?;
        self.withdrawn(kept).await?;
        self.answered(self.kept_autopilot(&manifest_id).await?)
            .await
    }

    async fn list_autopilots(&self) -> Result<AutopilotList, Refusal> {
        let every = self
            .store()
            .lock()
            .await
            .autopilots()
            .map_err(|why| self.faulted(why))?;
        let mut autopilots = Vec::with_capacity(every.len());
        for kept in every {
            let gathered = self
                .autopilot_gathered(kept)
                .await
                .map_err(|why| self.refusal(why))?;
            autopilots.push(self.autopilot_wire(&gathered));
        }
        Ok(AutopilotList { autopilots })
    }

    async fn get_autopilot(&self, manifest_id: ManifestId) -> Result<Autopilot, Refusal> {
        let kept = self.kept_autopilot(&manifest_id).await?;
        let gathered = self
            .autopilot_gathered(kept)
            .await
            .map_err(|why| self.refusal(why))?;
        Ok(self.autopilot_wire(&gathered))
    }

    /// Through the proposer and to the gate, as `propose_from_request` would take the issue's
    /// address: a person is choosing now. **Claimed before the call**, so two presses make one Job.
    async fn dispatch_set_aside(
        &self,
        manifest_id: ManifestId,
        pick_id: String,
    ) -> Result<Autopilot, Refusal> {
        let kept = self.kept_autopilot(&manifest_id).await?;
        let pick = self.set_aside_pick(&kept, &pick_id).await?;
        let served = self.served_named(Some(&manifest_id))?;
        let claimed = self
            .store()
            .lock()
            .await
            .move_autopilot_pick(&pick_id, "set_aside", "started", None)
            .map_err(|why| self.faulted(why))?;
        if !claimed {
            return Err(Refusal::IllegalMove(
                self.refused_as(NOT_SET_ASIDE, "that pick was dispatched already"),
            ));
        }
        let made = self
            .proposing_from(
                &pick.issue_url,
                None,
                Vec::new(),
                &served,
                Redirector::Person,
                TopLevelOrigin::Manual,
                None,
            )
            .await;
        let mut store = self.store().lock().await;
        let kept_job = match &made {
            Ok(jobs) => jobs.first().map(|job| job.id().as_str().to_string()),
            Err(_) => None,
        };
        match kept_job {
            Some(job) => store.move_autopilot_pick(&pick_id, "started", "started", Some(&job)),
            None => store.move_autopilot_pick(&pick_id, "started", "set_aside", None),
        }
        .map_err(|why| self.faulted(why))?;
        drop(store);
        if let Err(why) = made {
            return Err(self.refusal(why));
        }
        self.answered(self.kept_autopilot(&manifest_id).await?)
            .await
    }

    async fn dismiss_set_aside(
        &self,
        manifest_id: ManifestId,
        pick_id: String,
    ) -> Result<Autopilot, Refusal> {
        let kept = self.kept_autopilot(&manifest_id).await?;
        self.set_aside_pick(&kept, &pick_id).await?;
        let moved = self
            .store()
            .lock()
            .await
            .move_autopilot_pick(
                &pick_id,
                "set_aside",
                PickState::Dismissed.as_stored(),
                None,
            )
            .map_err(|why| self.faulted(why))?;
        if !moved {
            return Err(Refusal::IllegalMove(
                self.refused_as(NOT_SET_ASIDE, "that pick is no longer set aside"),
            ));
        }
        self.answered(kept).await
    }
}
