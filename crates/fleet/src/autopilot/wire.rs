//! An Autopilot as the store keeps it, gathered with what its Sessions say now, and as the wire
//! carries it. **The figures are read, never kept**: what its Sessions spent and which hold a place
//! change without Autopilot doing anything.

use std::collections::BTreeMap;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::Timestamp;
use store::{KeptAutopilot, KeptPick};

use super::bounds::{room, Cap, Caps, PickState, Spent, Standing, State};
use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// One Autopilot with this run's picks and what their Sessions say now.
pub(crate) struct Gathered {
    pub kept: KeptAutopilot,
    pub picks: Vec<KeptPick>,
    /// By pick id, for each started pick whose Session still loads.
    pub standing: BTreeMap<String, Standing>,
    pub spent: Spent,
}

impl Gathered {
    /// A word the store holds that this build does not know reads as stopped: an Autopilot that
    /// cannot say what it is does not pick.
    pub fn state(&self) -> State {
        State::from_stored(&self.kept.state).unwrap_or(State::Stopped)
    }

    pub fn caps(&self) -> Caps {
        Caps {
            until: self.kept.until.as_deref().map(Timestamp::from_rfc3339),
            spend_cap_micros: self.kept.spend_cap_micros,
            job_cap: self.kept.job_cap,
        }
    }

    pub fn room(&self) -> u32 {
        let standing: Vec<Standing> = self.standing.values().copied().collect();
        room(self.kept.at_once, &standing)
    }

    pub fn wire(&self) -> ipc::Autopilot {
        let kept = &self.kept;
        let standing: Vec<Standing> = self.standing.values().copied().collect();
        let at_work = standing.iter().filter(|one| one.holds_a_place()).count();
        ipc::Autopilot {
            manifest_id: ipc::ManifestId::carried(kept.manifest_id.clone()),
            instruction: kept.instruction.clone(),
            at_once: kept.at_once,
            until: kept.until.clone().map(ipc::Instant::carried),
            spend_cap_micros: kept.spend_cap_micros,
            job_cap: kept.job_cap,
            state: match self.state() {
                State::Running => ipc::AutopilotState::Running,
                State::Paused => ipc::AutopilotState::Paused,
                State::Stopped => ipc::AutopilotState::Stopped,
            },
            started_at: ipc::Instant::carried(kept.started_at.clone()),
            ended_at: kept.ended_at.clone().map(ipc::Instant::carried),
            capped_by: kept
                .capped_by
                .as_deref()
                .and_then(Cap::from_stored)
                .map(|cap| match cap {
                    Cap::Until => ipc::AutopilotCap::Until,
                    Cap::Spend => ipc::AutopilotCap::Spend,
                    Cap::Jobs => ipc::AutopilotCap::Jobs,
                }),
            spent_micros: self.spent.micros,
            jobs_started: self.spent.started,
            at_work: u32::try_from(at_work).unwrap_or(u32::MAX),
            picks: self.picks.iter().filter_map(pick).collect(),
            fault: None,
        }
    }
}

/// A pick on the wire. One whose state this build does not know is left off rather than guessed.
fn pick(kept: &KeptPick) -> Option<ipc::AutopilotPick> {
    let state = match PickState::from_stored(&kept.state)? {
        PickState::Started => ipc::AutopilotPickState::Started,
        PickState::SetAside => ipc::AutopilotPickState::SetAside,
        PickState::Withdrawn => ipc::AutopilotPickState::Withdrawn,
        PickState::Dismissed => ipc::AutopilotPickState::Dismissed,
    };
    Some(ipc::AutopilotPick {
        pick_id: kept.pick_id.clone(),
        issue_reference: kept.issue_reference.clone(),
        issue_title: kept.issue_title.clone(),
        issue_url: kept.issue_url.clone(),
        workflow_id: kept.workflow_id.clone().map(ipc::WorkflowId::carried),
        session_id: kept.started_session.as_deref().map(ipc::SessionId::carried),
        state,
        reason: kept.reason.clone(),
        at: ipc::Instant::carried(kept.at.clone()),
    })
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
    /// `kept`, with this run's picks and what each started Session says now.
    ///
    /// **Spend is each Session's own reported cost**, summed. **Started counts a pick that started
    /// a Session**, and a withdrawn one too: it was started, and the cap is on starts.
    pub(crate) async fn autopilot_gathered(&self, kept: KeptAutopilot) -> Result<Gathered, Adrift> {
        let picks = self
            .store()
            .lock()
            .await
            .autopilot_picks(&kept.run_id)
            .map_err(Adrift::Writing)?;
        let mut standing = BTreeMap::new();
        let mut spent = Spent::default();
        for one in &picks {
            let Some(started) = one.started_session.as_deref() else {
                continue;
            };
            if matches!(
                PickState::from_stored(&one.state),
                Some(PickState::Started | PickState::Withdrawn)
            ) {
                spent.started += 1;
            }
            // A Session forgotten since it was picked is no longer anybody's to count.
            let Some((cost, standing_now)) = self.pick_session_standing(started).await else {
                continue;
            };
            spent.micros += cost;
            standing.insert(one.pick_id.clone(), standing_now);
        }
        Ok(Gathered {
            kept,
            picks,
            standing,
            spent,
        })
    }

    /// What the Session `id` has cost and how it stands, as room and Stop read it. `None` where the
    /// ledger holds no such Session.
    async fn pick_session_standing(&self, id: &str) -> Option<(u64, Standing)> {
        let store = self.store().lock().await;
        let session = store.session(id).ok().flatten()?;
        let ever_ran = store
            .hosting(id)
            .ok()
            .flatten()
            .is_some_and(|hosting| hosting.ran);
        let record = self.ledger_row(&store, &session).ok();
        let waiting = record.as_ref().is_some_and(|record| !record.waiting_for.is_empty());
        let working = record.as_ref().is_some_and(|record| {
            matches!(
                record.standing,
                Some(ipc::SessionStanding::Working | ipc::SessionStanding::Stuck)
            )
        });
        Some((
            session.figures.cost_micros.unwrap_or(0),
            Standing {
                live: session.state == store::SessionState::Live,
                waiting_on_a_person: waiting,
                working,
                ever_ran,
            },
        ))
    }
}
