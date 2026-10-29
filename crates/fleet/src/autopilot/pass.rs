//! The pass: on the noticing interval, each running Autopilot with room is asked for a pick, and
//! a sure pick becomes a Session that starts unasked, briefed on the issue.
//!
//! **Its own loop, beside the noticing one and on the same interval.** A pick is a model call, and
//! a landing told late because it waited behind one is the cost `crate::notice_loop` exists to
//! avoid.
//!
//! **Asked again before each pick is kept.** A call takes seconds, and a Pause pressed while it
//! was out is honoured: the pick is dropped, never started.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use store::{KeptAutopilot, KeptPick};
use tokio::task::JoinHandle;

use super::bounds::{PickState, State};
use super::picking::{candidates, picked, Pick, SetAside, SurePick};
use super::starting::Issue;
use super::wire::Gathered;
use super::Asking;
use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// The most calls one Autopilot makes in one pass. **A bound on money, not on picks**: a backlog
/// it is unsure of throughout sets this many aside a pass and carries on at the next.
const CALLS_PER_PASS: usize = 4;

/// Pick for every running Autopilot, every `tick`. `adrift` hears only a store that would not list
/// them; what stopped one repository's picks is on its record, for the person who reads it.
pub fn keep_autopiloting<H, V, W>(
    fleet: Arc<Fleet<H, V, W>>,
    tick: Duration,
    adrift: Arc<impl Fn(Adrift) + Send + Sync + 'static>,
) -> JoinHandle<()>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(tick);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            if let Err(why) = fleet.autopilot_pass().await {
                adrift(why);
            }
        }
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
    /// One pass over every Autopilot. Answers how many Sessions it started.
    ///
    /// **One repository's failure is kept on its record and the rest still pass**: a tracker that
    /// will not answer for one is not a reason for the others to stop, and the person who comes
    /// back in the morning reads why it picked nothing on the Autopilot itself.
    pub async fn autopilot_pass(self: &Arc<Self>) -> Result<usize, Adrift> {
        let every = self
            .store()
            .lock()
            .await
            .autopilots()
            .map_err(Adrift::Writing)?;
        let mut started = 0;
        for kept in every {
            if State::from_stored(&kept.state) != Some(State::Running) {
                continue;
            }
            let manifest_id = ipc::ManifestId::carried(kept.manifest_id.clone());
            let passed = self.passed_over(kept).await;
            self.tracking()
                .autopilot
                .faulted(manifest_id.as_str(), passed.as_ref().err().cloned());
            started += passed.unwrap_or(0);
            self.autopilot_told(&manifest_id).await;
        }
        Ok(started)
    }

    async fn passed_over(self: &Arc<Self>, kept: KeptAutopilot) -> Result<usize, String> {
        let said = |why: Adrift| why.to_string();
        let manifest_id = ipc::ManifestId::carried(kept.manifest_id.clone());
        let mut gathered = self.autopilot_gathered(kept).await.map_err(said)?;
        // **Paused, not stopped**: the owner's 12 Oct 2026. Raising the cap lets Resume through.
        if let Some(cap) = gathered.caps().first_reached(&self.now(), &gathered.spent) {
            let run = gathered.kept.run_id.clone();
            self.store()
                .lock()
                .await
                .move_autopilot(&run, State::Paused.as_stored(), None, Some(cap.as_stored()))
                .map_err(|why| format!("{why:?}"))?;
            self.autopilot_told(&manifest_id).await;
            return Ok(0);
        }
        let mut room = gathered.room();
        if room == 0 {
            self.told_if_moved(&gathered);
            return Ok(0);
        }
        let served = self
            .served_named(Some(&manifest_id))
            .map_err(|why| format!("{why:?}"))?;
        let open = self
            .open_issues(&manifest_id)
            .await
            .map_err(|why| format!("{why:?}"))?;
        let mut taken: BTreeSet<String> = {
            let store = self.store().lock().await;
            let mut taken: BTreeSet<String> = BTreeSet::new();
            // **Every run's** live Sessions: a Stop and a Start must not pick an issue twice.
            for one in store
                .autopilot_picks_that_started_a_session()
                .map_err(|why| format!("{why:?}"))?
            {
                let live = one
                    .started_session
                    .as_deref()
                    .and_then(|id| store.session(id).ok().flatten())
                    .is_some_and(|session| session.state == store::SessionState::Live);
                if live {
                    taken.insert(one.issue_url);
                }
            }
            taken
        };
        taken.extend(gathered.picks.iter().map(|one| one.issue_url.clone()));
        let mut offered = candidates(&open, &taken);
        let urls: Vec<String> = offered.iter().map(|one| one.url.clone()).collect();
        if offered.is_empty()
            || self
                .tracking()
                .autopilot
                .declined(&gathered.kept.manifest_id, &urls)
        {
            self.told_if_moved(&gathered);
            return Ok(0);
        }
        let memory = &self.tracking().autopilot;
        let now_ms = || self.now().epoch_millis().unwrap_or_default();
        // Resting after a failed call: the fault it left stays on the record.
        if memory.picker_resting(&gathered.kept.manifest_id, now_ms()) {
            return memory.fault(&gathered.kept.manifest_id).map_or(Ok(0), Err);
        }
        let asking = self.asking().map_err(|why| format!("{why:?}"))?;
        let workflows = self.workflows_offered(&gathered.kept.manifest_id, &served.workflows());
        let mut made = 0;
        for _ in 0..CALLS_PER_PASS {
            if room == 0 || offered.is_empty() {
                break;
            }
            let pick = match picked(&gathered.kept.instruction, &offered, &workflows, &asking).await
            {
                Ok(pick) => {
                    memory.picker_answered(&gathered.kept.manifest_id);
                    pick
                }
                Err(why) => {
                    memory.picker_failed(&gathered.kept.manifest_id, now_ms());
                    // Kept as the pass's fault, so the person reads why it picked nothing.
                    if made == 0 {
                        return Err(why.to_string());
                    }
                    break;
                }
            };
            // A Pause or a Stop pressed while the call was out wins over its answer.
            if !self.still_picking(&gathered.kept).await.map_err(said)? {
                break;
            }
            match pick {
                Pick::Sure(sure) => {
                    let url = sure.issue().url.clone();
                    self.sure_pick_started(&gathered.kept, &served, sure).await?;
                    offered.retain(|one| one.url != url);
                    room -= 1;
                    made += 1;
                    let session_cap = gathered.kept.session_cap;
                    gathered = self.autopilot_gathered(gathered.kept).await.map_err(said)?;
                    if session_cap.is_some_and(|cap| gathered.spent.started >= cap) {
                        break;
                    }
                }
                Pick::Unsure(aside) => {
                    let url = aside.issue.url.clone();
                    self.set_aside(&gathered.kept, aside).await.map_err(said)?;
                    offered.retain(|one| one.url != url);
                }
                Pick::Nothing { .. } => {
                    self.tracking()
                        .autopilot
                        .decline(&gathered.kept.manifest_id, urls);
                    break;
                }
            }
        }
        self.autopilot_told(&manifest_id).await;
        Ok(made)
    }

    /// The proposer's client, budget, model and environment.
    pub(crate) fn asking(&self) -> Result<Asking, adapter_traits::SpawnConfigRefused> {
        let proposing = self.proposing()?;
        Ok(Asking {
            client: proposing.client,
            budget: proposing.budget,
            model: proposing.model,
            environment: proposing.environment,
            prompts: self.prompts(),
        })
    }

    async fn still_picking(&self, kept: &KeptAutopilot) -> Result<bool, Adrift> {
        let now = self
            .store()
            .lock()
            .await
            .autopilot(&kept.manifest_id)
            .map_err(Adrift::Writing)?;
        Ok(now.is_some_and(|now| {
            now.run_id == kept.run_id && State::from_stored(&now.state) == Some(State::Running)
        }))
    }

    async fn set_aside(&self, kept: &KeptAutopilot, aside: SetAside) -> Result<(), Adrift> {
        let pick = KeptPick {
            pick_id: self.mint().ulid().as_str().to_string(),
            run_id: kept.run_id.clone(),
            manifest_id: kept.manifest_id.clone(),
            issue_reference: aside.issue.reference,
            issue_title: aside.issue.title,
            issue_url: aside.issue.url,
            workflow_id: aside.workflow.map(|id| id.as_str().to_string()),
            started_session: None,
            state: PickState::SetAside.as_stored().to_string(),
            reason: aside.reason,
            at: self.now().as_str().to_string(),
        };
        self.store()
            .lock()
            .await
            .add_autopilot_pick(&pick)
            .map_err(Adrift::Writing)
    }

    /// A sure pick: a Session started on its issue, with no one asked. **Takes the [`SurePick`]
    /// whole**, so only reading the picker's answer starts one unasked.
    async fn sure_pick_started(
        self: &Arc<Self>,
        kept: &KeptAutopilot,
        served: &crate::repositories::Served,
        sure: SurePick,
    ) -> Result<(), String> {
        let issue = sure.issue();
        let session = self
            .session_on_issue(
                served,
                &Issue { url: &issue.url, title: &issue.title, excerpt: &issue.excerpt },
                Some(sure.workflow().as_str()),
                Some(sure.reason()),
            )
            .await
            .map_err(|refused| refused.error().message.clone())?;
        let pick = KeptPick {
            pick_id: self.mint().ulid().as_str().to_string(),
            run_id: kept.run_id.clone(),
            manifest_id: kept.manifest_id.clone(),
            issue_reference: issue.reference.clone(),
            issue_title: issue.title.clone(),
            issue_url: issue.url.clone(),
            workflow_id: Some(sure.workflow().as_str().to_string()),
            started_session: Some(session),
            state: PickState::Started.as_stored().to_string(),
            reason: sure.reason().to_string(),
            at: self.now().as_str().to_string(),
        };
        self.store()
            .lock()
            .await
            .add_autopilot_pick(&pick)
            .map_err(|why| Adrift::Writing(why).to_string())
    }

    /// Publish this repository's Autopilot as it stands now, if it moved since last told.
    pub(crate) async fn autopilot_told(&self, manifest_id: &ipc::ManifestId) {
        let kept = self.store().lock().await.autopilot(manifest_id.as_str());
        if let Ok(Some(kept)) = kept {
            if let Ok(gathered) = self.autopilot_gathered(kept).await {
                self.told_if_moved(&gathered);
            }
        }
    }

    /// The wire's Autopilot, with why the last pass picked nothing where one did.
    pub(crate) fn autopilot_wire(&self, gathered: &Gathered) -> ipc::Autopilot {
        let mut now = gathered.wire();
        now.fault = self.tracking().autopilot.fault(&gathered.kept.manifest_id);
        now
    }

    fn told_if_moved(&self, gathered: &Gathered) {
        let now = self.autopilot_wire(gathered);
        if self.tracking().autopilot.moved(&now) {
            self.publish(ipc::Event::AutopilotChanged(now));
        }
    }
}
