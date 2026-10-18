//! Fleet's side of the Session model's step down: what a plan reading and a settings change do to
//! it, and the four things the owner does. `docs/concepts/machine.md`, *Quota*;
//! [`crate::model_downshift`] is the machine.
//!
//! **The Session model moves through settings.json and nowhere else.** Accepting and restoring call
//! `save_settings_now`, the path the Settings page saves by, so the file is checked whole and a
//! refusal leaves the state as it was. While one of them writes, the settings hook is told to stand
//! aside; any other change to `models.session` is the owner's, and settles the state.

use std::sync::atomic::{AtomicBool, Ordering};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use config::settings::{self as keys, Key};
use ipc::{ModelDownshiftState, PlanUsage, SaveSettings, SettingValue, WireError};
use store::settings_file::SettingValue as FileValue;
use tokio::sync::{Mutex, MutexGuard};

use crate::daemon::Fleet;
use crate::model_downshift::{Downshift, Phase, Terms, Weekly};

/// A command that does not fit where the state stands: nothing offered to accept, nothing shifted to
/// restore. A 409.
const DOWNSHIFT_WRONG_PHASE: &str = "fleet.downshift_wrong_phase";

/// The weekly window's kind in [`PlanUsage`].
const WEEKLY: &str = "seven_day";

const SESSION_KEY: &str = "models.session";

/// What Fleet holds of the step down. Read from the store when first asked, and never again.
#[derive(Default)]
pub(crate) struct Holding {
    held: Mutex<Option<Downshift>>,
    /// One command at a time, so two cannot each find the state they expect and both write the file.
    acting: Mutex<()>,
    /// A command is writing `models.session`, which the settings hook must not read as a hand edit.
    writing: AtomicBool,
}

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
    /// The state, loaded. A store that will not answer is a refusal and loads nothing, so a state
    /// the store holds is never overwritten by a default that was only assumed.
    async fn downshift_held(&self) -> Result<MutexGuard<'_, Option<Downshift>>, Refusal> {
        let mut held = self.downshift().held.lock().await;
        if held.is_none() {
            let kept = self.store().lock().await.model_downshift().map_err(|why| self.ledger_fault(why))?;
            *held = Some(Downshift::from_kept(kept));
        }
        Ok(held)
    }

    /// Keep the state, then say so. The state in memory stands whether or not the store took it: it is
    /// what `models.session` already is.
    async fn downshift_moved(&self, now: &Downshift) -> Result<ModelDownshiftState, Refusal> {
        let kept = self.store().lock().await.keep_model_downshift(&now.kept());
        let wire = now.wire();
        self.publish(ipc::Event::ModelDownshiftChanged(wire.clone()));
        kept.map_err(|why| self.ledger_fault(why))?;
        Ok(wire)
    }

    fn wrong_phase(&self, command: &str, wanted: &str) -> Refusal {
        Refusal::IllegalMove(WireError::raised(
            DOWNSHIFT_WRONG_PHASE,
            format!("{command} needs the Job model to be {wanted}"),
            self.run_id(),
        ))
    }

    /// `get_model_downshift`.
    pub(crate) async fn model_downshift_state(&self) -> Result<ModelDownshiftState, Refusal> {
        Ok(self.downshift_held().await?.as_ref().map(Downshift::wire).unwrap_or_default())
    }

    /// A session reported the plan's windows. The weekly one is judged against the line.
    pub(crate) async fn downshift_read(&self, usage: &PlanUsage) {
        let Some(weekly) = usage.windows.iter().find(|one| one.kind == WEEKLY) else { return };
        let Some(resets_at) = weekly.resets_at.as_ref() else { return };
        let settings = self.machine_settings().now();
        let current: String = settings.get(keys::MODELS_SESSION);
        let to: String = settings.get(keys::PLAN_DOWNSHIFT_TO);
        let line = u32::try_from(settings.get(keys::PLAN_DOWNSHIFT_AT)).unwrap_or(u32::MAX);
        let pinned = keys::entry(keys::MODELS_SESSION.name())
            .is_some_and(|entry| settings.overridden_by_env(entry).is_some());
        let now = {
            let Ok(mut held) = self.downshift_held().await else { return };
            let machine = held.as_mut().expect("loaded");
            let weekly = Weekly { percent: weekly.percent_used, resets_at: resets_at.as_str() };
            let terms = Terms { line, to: &to, current: &current, pinned };
            if !machine.read(&weekly, &terms) {
                return;
            }
            machine.clone()
        };
        let _ = self.downshift_moved(&now).await;
    }

    /// settings.json changed, by a save or by hand. Anything but a command's own write that leaves
    /// `models.session` somewhere the state does not expect ends the state.
    pub(crate) async fn downshift_settings_moved(&self, settings: &config::settings::Resolved) {
        if self.downshift().writing.load(Ordering::SeqCst) {
            return;
        }
        let now = {
            let Ok(mut held) = self.downshift_held().await else { return };
            let machine = held.as_mut().expect("loaded");
            if !machine.edited_to(&settings.get(keys::MODELS_SESSION)) {
                return;
            }
            machine.clone()
        };
        let _ = self.downshift_moved(&now).await;
    }

    /// `models.session` written as `value`, or removed, the way `save_settings` writes it.
    async fn downshift_writes(&self, value: Option<String>) -> Result<(), Refusal> {
        self.downshift().writing.store(true, Ordering::SeqCst);
        let saved = self
            .save_settings_now(SaveSettings {
                changes: [(SESSION_KEY.to_string(), value.map(SettingValue::Text))].into(),
            })
            .await;
        self.downshift().writing.store(false, Ordering::SeqCst);
        saved.map(|_| ())
    }

    /// `accept_downshift`.
    pub(crate) async fn downshift_accepted(&self) -> Result<ModelDownshiftState, Refusal> {
        let _one = self.downshift().acting.lock().await;
        let (was, to, window) = {
            let held = self.downshift_held().await?;
            let machine = held.as_ref().expect("loaded");
            match (&machine.phase, &machine.offered_for) {
                (Some(Phase::Offered { was, to, .. }), Some(window)) => (was.clone(), to.clone(), window.clone()),
                _ => return Err(self.wrong_phase("accepting", "offered")),
            }
        };
        let restore_to = match self.machine_settings().now().saved().get(SESSION_KEY) {
            Some(FileValue::Text(saved)) => Some(saved.clone()),
            _ => None,
        };
        self.downshift_writes(Some(to.clone())).await?;
        let now = {
            let mut held = self.downshift_held().await?;
            let machine = held.as_mut().expect("loaded");
            machine.phase = Some(Phase::Shifted { was, to, until: window, restore_to });
            machine.clone()
        };
        self.downshift_moved(&now).await
    }

    /// `restore_downshift`.
    pub(crate) async fn downshift_restored(&self) -> Result<ModelDownshiftState, Refusal> {
        let _one = self.downshift().acting.lock().await;
        let restore_to = {
            let held = self.downshift_held().await?;
            match &held.as_ref().expect("loaded").phase {
                Some(Phase::Shifted { restore_to, .. } | Phase::Reset { restore_to, .. }) => restore_to.clone(),
                _ => return Err(self.wrong_phase("restoring", "shifted or reset")),
            }
        };
        self.downshift_writes(restore_to).await?;
        self.downshift_ended().await
    }

    /// `keep_downshift`.
    pub(crate) async fn downshift_kept(&self) -> Result<ModelDownshiftState, Refusal> {
        let _one = self.downshift().acting.lock().await;
        let reset = matches!(self.downshift_held().await?.as_ref().expect("loaded").phase, Some(Phase::Reset { .. }));
        if !reset {
            return Err(self.wrong_phase("keeping", "reset"));
        }
        self.downshift_ended().await
    }

    /// `settle_downshift`.
    pub(crate) async fn downshift_settled(&self) -> Result<ModelDownshiftState, Refusal> {
        let _one = self.downshift().acting.lock().await;
        let offered = matches!(self.downshift_held().await?.as_ref().expect("loaded").phase, Some(Phase::Offered { .. }));
        if !offered {
            return Err(self.wrong_phase("settling", "offered"));
        }
        self.downshift_ended().await
    }

    /// The state cleared. The window an offer was last made for stays, so none is made again in it.
    async fn downshift_ended(&self) -> Result<ModelDownshiftState, Refusal> {
        let now = {
            let mut held = self.downshift_held().await?;
            let machine = held.as_mut().expect("loaded");
            machine.phase = None;
            machine.clone()
        };
        self.downshift_moved(&now).await
    }
}
