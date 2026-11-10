//! Starting a repair for a Job, and what its end does to the Job.
//!
//! **Fleet reads the end, the agent does not announce it**: the first line of the last message, or the
//! limit, or the Job moving on. A repair that put things right wakes the Job once, in Fleet's own words;
//! one that did not leaves a note and, for a Job that is dead, the row amber as it was.

use std::sync::Arc;

use adapter_traits::{slot_path, AgentHarness, Delivery, SlotLeased, SlotPool, Vcs, WorkProduct};
use api::Refusal;

use super::hosting::Running;
use super::{brief, verdict_of, RepairBreak, Verdict, NOT_REPAIRED, REPAIRED, STARTED};
use crate::daemon::Fleet;

/// A repair already running for the Job, or one is not allowed to start.
const REPAIR_RUNNING: &str = "fleet.repair_running";
/// The Job holds no checkout to repair in, or a slot for one could not be had.
const REPAIR_NO_CHECKOUT: &str = "fleet.repair_no_checkout";
/// Fleet is unhealthy, so a repair would fail the way the Job did.
pub(crate) const REPAIR_UNHEALTHY: &str = "fleet.repair_unhealthy";

/// A slot leased for a repair on a pull request's branch, given back when it ends.
struct Leased {
    pool: SlotPool,
    slot: u32,
}

/// Everything a started repair is run with.
struct Prepared {
    repair: String,
    job: String,
    broke: RepairBreak,
    directory: String,
    told: String,
    leased: Option<Leased>,
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
    /// Start a repair for `job`, hidden: no session row, no entry in the Jobs list, no start card.
    /// **The one call every trigger makes.** It refuses while Fleet is unhealthy and while a repair is
    /// already running for the Job, picks the checkout the break asks for, writes the start note and
    /// starts the host. `saw` is what Fleet saw, each as a label and a value, told to the repair last.
    pub(crate) async fn repair_started(
        self: &Arc<Self>,
        job: &str,
        broke: RepairBreak,
        saw: Vec<(String, String)>,
    ) -> Result<(), Refusal> {
        let Some(running) = self.repairs().listed(job) else {
            return Err(self.hosted_refusal(REPAIR_RUNNING, &format!("a repair is already running for job {job}")));
        };
        match self.repair_prepared(job, broke, &saw).await {
            Ok(prepared) => {
                let fleet = Arc::clone(self);
                tokio::spawn(async move { fleet.repair_run(prepared, running).await });
                Ok(())
            }
            Err(why) => {
                self.repairs().ended(job);
                Err(why)
            }
        }
    }

    async fn repair_prepared(&self, job: &str, broke: RepairBreak, saw: &[(String, String)]) -> Result<Prepared, Refusal> {
        let healthy = self.health().await.is_ok_and(|health| !health.probes.iter().any(|one| one.outcome == "fail"));
        if !healthy {
            return Err(self.hosted_refusal(REPAIR_UNHEALTHY, "Fleet is unhealthy, so a repair would fail the same way"));
        }
        let (session, hosting) = self.session_and_hosting(job).await?;
        let served = self.served_named(Some(&ipc::ManifestId::carried(&hosting.manifest_id)))?;
        let repair = self.mint().ulid().as_str().to_lowercase();
        let no_checkout = |why: String| self.hosted_refusal(REPAIR_NO_CHECKOUT, &why);
        let (directory, leased) = match broke {
            RepairBreak::Stranded { slot } => (slot_path(served.root(), slot), None),
            // An ended Job gave its slot back: a slot is leased onto the pull request's own branch.
            RepairBreak::RedPull { number } if session.state == store::SessionState::Ended || hosting.lease_slot.is_none() => {
                let branch = self.pull_request_branch(job, number).await.or(hosting.lease_branch.clone());
                let branch = branch.ok_or_else(|| no_checkout(format!("pull request #{number} names no branch to repair on")))?;
                let pool = crate::leasing::pool_of(&served);
                let (vcs, held, on, by) = (Arc::clone(self.vcs()), pool.clone(), branch, repair.clone());
                let leased = tokio::task::spawn_blocking(move || vcs.lease_existing_slot(&held, &on, &by))
                    .await
                    .map_err(|_| no_checkout("the lease of a slot panicked".to_string()))?;
                match leased {
                    Ok(SlotLeased::Took { slot, worktree, .. }) => (worktree.path().to_string(), Some(Leased { pool, slot })),
                    Ok(SlotLeased::Full) => return Err(no_checkout("every slot is held, so there is none to repair in".to_string())),
                    Err(why) => return Err(no_checkout(format!("a slot could not be leased on the branch: {why}"))),
                }
            }
            _ => match hosting.lease_slot {
                Some(slot) => (slot_path(served.root(), slot), None),
                None => return Err(no_checkout(format!("job {job} holds no checkout"))),
            },
        };
        let title = session.title.clone().unwrap_or_else(|| job.to_string());
        let told = brief::told(
            &self.prompts(),
            &broke,
            &brief::Material { job, title: &title, branch: hosting.lease_branch.as_deref(), checkout: &directory, saw },
        );
        let at = self.now().as_str().to_string();
        let kept = self.store().lock().await.keep_repair_started(job, &repair, broke.key(), &at);
        if let Err(why) = kept {
            self.repair_slot_given_back(&leased, &repair).await;
            return Err(self.ledger_fault(why));
        }
        self.fleet_note(job, format!("{STARTED}{}", broke.words())).await;
        let _ = self.published_hosted(job).await;
        Ok(Prepared { repair, job: job.to_string(), broke, directory, told, leased })
    }

    /// The branch the Job's pull request `number` is on, as its attachment recorded it.
    async fn pull_request_branch(&self, job: &str, number: u64) -> Option<String> {
        let held = self.store().lock().await.attachments_of(&store::Holder::session(job)).ok()?;
        held.iter()
            .find(|one| one.kind == crate::pull_requesting::PR && one.target == number.to_string())
            .and_then(|one| one.detail.get("branch").cloned())
    }

    async fn repair_slot_given_back(&self, leased: &Option<Leased>, repair: &str) {
        if let Some(Leased { pool, slot }) = leased {
            let (vcs, pool, slot, repair) = (Arc::clone(self.vcs()), pool.clone(), *slot, repair.to_string());
            let _ = tokio::task::spawn_blocking(move || vcs.release_slot(&pool, slot, &repair)).await;
        }
    }

    /// Run the repair to its end, then say how it went and, where it put things right, wake the Job.
    async fn repair_run(self: Arc<Self>, prepared: Prepared, running: Arc<Running>) {
        let Prepared { repair, job, broke, directory, told, leased } = prepared;
        let ended = self.repairs().host().run(&directory, &told, &running).await;
        // Out of the list before the Job is woken: its next turn stops a repair still listed.
        self.repairs().ended(&job);
        self.repair_slot_given_back(&leased, &repair).await;
        let verdict = verdict_of(&ended);
        self.repair_ended(&job, &repair, broke, &verdict).await;
    }

    async fn repair_ended(self: &Arc<Self>, job: &str, repair: &str, broke: RepairBreak, verdict: &Verdict) {
        let at = self.now().as_str().to_string();
        let _ = self.store().lock().await.keep_repair_ended(repair, &at, verdict.outcome, &verdict.said);
        let note = match (verdict.repaired, verdict.said.is_empty()) {
            (true, false) => format!("{REPAIRED}{}", verdict.said),
            (true, true) => REPAIRED.trim_end().to_string(),
            (false, _) => format!("{NOT_REPAIRED}{}", verdict.said),
        };
        self.fleet_note(job, note).await;
        if let (true, Some(did)) = (verdict.repaired, broke.woke_with()) {
            let _ = self.told_of_a_repair(job, &format!("Fleet: a repair {did}. Carry on.")).await;
        }
        let _ = self.published_hosted(job).await;
    }
}
