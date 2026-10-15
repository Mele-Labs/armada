//! An Autopilot as the store keeps it, gathered with what its Jobs say now, and as the wire
//! carries it. **The figures are read, never kept**: what its Jobs spent and which hold a place
//! change without Autopilot doing anything.

use std::collections::BTreeMap;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{JobId, Timestamp, Ulid};
use store::{KeptAutopilot, KeptPick};

use super::bounds::{room, Cap, Caps, PickState, Spent, Standing, State};
use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// One Autopilot with this run's picks and what their Jobs say now.
pub(crate) struct Gathered {
    pub kept: KeptAutopilot,
    pub picks: Vec<KeptPick>,
    /// By pick id, for each started pick whose Job still loads.
    pub standing: BTreeMap<String, Standing>,
    pub spent: Spent,
}

pub(crate) fn job_id(stored: &str) -> JobId {
    JobId::carried(Ulid::carried(stored))
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
        job_id: kept
            .started_job
            .as_deref()
            .map(|id| ipc::JobId::from(&job_id(id))),
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
    /// `kept`, with this run's picks and what each started Job says now.
    ///
    /// **Spend is each Job's own**, `crate::allowance`'s figure, summed. **Started counts a pick
    /// that started a Job**, and a withdrawn one too: it was started, and the cap is on starts.
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
            let Some(started) = one.started_job.as_deref() else {
                continue;
            };
            if matches!(
                PickState::from_stored(&one.state),
                Some(PickState::Started | PickState::Withdrawn)
            ) {
                spent.started += 1;
            }
            let id = job_id(started);
            spent.micros += self
                .spend_of(&id)
                .await
                .map(|spend| spend.cost_micros)
                .unwrap_or(0);
            // A Job forgotten since it was picked is no longer anybody's to count.
            let Ok(job) = self.load(&id).await else {
                continue;
            };
            let ever_ran = self.job_started_at(&id).await.ok().flatten().is_some();
            standing.insert(one.pick_id.clone(), Standing::of(&job, ever_ran));
        }
        Ok(Gathered {
            kept,
            picks,
            standing,
            spent,
        })
    }
}
