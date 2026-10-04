//! A stranded slot rescued: a Scout reads it on a person's press, its Finding
//! is kept against the slot, and a person then scraps it or stashes it.
//! `docs/concepts/fleet.md`, *Rescuing a stranded slot*.
//!
//! **A Scout reads and Fleet acts.** The Scout has no tool that writes; the
//! scrap and the stash are `Vcs::rescue_slot`, run on the press that asks.
//! **Nothing starts on its own**: `rescue_slot` is the only door.

use std::sync::Arc;

use adapter_traits::{
    AgentHarness, Delivery, RescueRefused, SlotRescue, Vcs, WorkProduct,
};
use api::Refusal;
use core_model::{ScoutEnded, ScoutLook, ScoutOutcome};
use ipc::{ManifestId, RescueAct, RescueSlot, WireError};
use store::{KeptRescue, RescueState};
use tokio::sync::mpsc;

use crate::daemon::Fleet;
use crate::leasing::{pool_of, NO_SUCH_SLOT, SLOT_BUSY, SLOT_UNCHANGED};

/// A slot that is not stranded. A 409, as are the rest below.
const NOT_STRANDED: &str = "fleet.slot_not_stranded";
/// A scout is reading it, so it is neither read again nor acted on.
const RESCUE_READING: &str = "fleet.rescue_reading";
/// A stop on a slot no scout is reading.
const RESCUE_NOT_RUNNING: &str = "fleet.rescue_not_running";
/// A stash of a checkout on no branch.
const RESCUE_ON_NO_BRANCH: &str = "fleet.rescue_on_no_branch";
/// A stash of a checkout on the base itself.
const RESCUE_ON_THE_BASE: &str = "fleet.rescue_on_the_base";
/// A stash with nowhere to push it.
const RESCUE_NO_REMOTE: &str = "fleet.rescue_no_remote";

/// Why a Finding left reading across a restart, in its own words.
pub(crate) const LOST: &str = "Fleet stopped while the scout was reading, so nothing read it to its end";

/// The key a scout reading a slot is listed under.
fn listed_as(manifest: &str, slot: u32) -> String {
    format!("rescue:{manifest}:{slot}")
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
    /// One act on a stranded slot, from Cleanup's bay grid.
    pub(crate) async fn rescue_slot(
        self: Arc<Self>,
        asked: RescueSlot,
        manifest_id: Option<&ManifestId>,
    ) -> Result<ipc::SlotRescued, Refusal> {
        let served = self.served_named(manifest_id)?;
        let manifest = served.manifest().id().as_str().to_string();
        let pool = pool_of(&served);
        let slot = asked.slot;
        let answered = |branch, branch_kept, committed| ipc::SlotRescued {
            manifest_id: ManifestId::from(served.manifest().id()),
            slot,
            branch,
            branch_kept,
            committed,
        };
        match asked.act {
            RescueAct::Start => {
                self.refuse_while_reading(&manifest, slot).await?;
                let work = self
                    .vcs()
                    .stranded_work(&pool, slot)
                    .map_err(|why| self.rescue_refusal(why))?;
                let diff = self
                    .vcs()
                    .stranded_diff(&pool, slot)
                    .map_err(|why| self.rescue_refusal(why))?;
                let root = adapter_traits::slot_path(served.root(), slot);
                let commits: Vec<(String, String)> = work
                    .commits
                    .iter()
                    .map(|commit| (commit.sha.clone(), commit.subject.clone()))
                    .collect();
                let (told, cut) = crate::scout::told_a_rescue(&crate::scout::Stranded {
                    root: &root,
                    branch: work.branch.as_deref(),
                    commit: &work.commit,
                    base: pool.base(),
                    uncommitted: &work.uncommitted,
                    commits: &commits,
                    diff: &diff,
                });
                let reading = KeptRescue {
                    manifest_id: manifest,
                    slot,
                    state: RescueState::Reading,
                    commit: work.commit,
                    uncommitted: !work.uncommitted.is_empty(),
                    cut,
                    read: Vec::new(),
                    searched: Vec::new(),
                    summary: None,
                    why: None,
                    cost_micros: None,
                };
                self.kept_rescue(&reading).await?;
                self.rescue_scouting(reading, root, told).await;
                Ok(answered(work.branch, false, None))
            }
            RescueAct::Stop => {
                if !self.scouts().stop(&listed_as(&manifest, slot)) {
                    return Err(Refusal::IllegalMove(WireError::raised(
                        RESCUE_NOT_RUNNING,
                        format!("no scout is reading slot-{slot}, so there is nothing to stop"),
                        self.run_id(),
                    )));
                }
                Ok(answered(None, false, None))
            }
            RescueAct::Scrap | RescueAct::Stash => {
                self.refuse_while_reading(&manifest, slot).await?;
                let rescue = match asked.act {
                    RescueAct::Scrap => SlotRescue::Scrap,
                    _ => SlotRescue::Stash {
                        message: format!("Work left in slot-{slot}, kept"),
                    },
                };
                let done = self
                    .vcs()
                    .rescue_slot(&pool, slot, rescue)
                    .map_err(|why| self.rescue_refusal(why))?;
                // The slot is free, so what was read of it is of nothing now.
                let _ = self.store().lock().await.forget_rescue(&manifest, slot);
                Ok(answered(done.branch, done.branch_kept, done.committed))
            }
        }
    }

    /// Refused where a scout is reading the slot, or the store would not say.
    async fn refuse_while_reading(&self, manifest: &str, slot: u32) -> Result<(), Refusal> {
        let reading = self
            .store()
            .lock()
            .await
            .rescues()
            .map_err(|why| self.refusal(crate::adrift::Adrift::Writing(why)))?
            .iter()
            .any(|one| {
                one.manifest_id == manifest && one.slot == slot && one.state == RescueState::Reading
            });
        if reading {
            return Err(Refusal::IllegalMove(WireError::raised(
                RESCUE_READING,
                format!("a scout is reading slot-{slot}"),
                self.run_id(),
            )));
        }
        Ok(())
    }

    async fn kept_rescue(&self, rescue: &KeptRescue) -> Result<(), Refusal> {
        self.store()
            .lock()
            .await
            .keep_rescue(rescue)
            .map_err(|why| self.refusal(crate::adrift::Adrift::Writing(why)))
    }

    /// Every Finding a restart left reading, ended as failed and keeping what
    /// it read. Read once, at start: no scout outlives the Fleet reading it.
    pub(crate) async fn rescues_left_reading(&self) {
        let _ = self.store().lock().await.rescues_left_reading(LOST);
    }

    /// Start the scout and hand its reading to a task of its own. **Listed
    /// before it starts**, so a stop pressed at once reaches it. What it reads
    /// is written as each read is answered, so a Bridge reload mid-read finds
    /// the Finding as far as it got.
    async fn rescue_scouting(self: Arc<Self>, rescue: KeptRescue, root: String, told: String) {
        let key = listed_as(&rescue.manifest_id, rescue.slot);
        let host = self.scouts().host();
        let running = self.scouts().listed(&key);
        let started = match host.start(&root, &told).await {
            Ok(started) => started,
            Err(why) => {
                self.scouts().ended(&key);
                self.rescue_ended(rescue, None, crate::scouting::failed(&why))
                    .await;
                return;
            }
        };
        tokio::spawn(async move {
            let (sent, mut heard) = mpsc::unbounded_channel();
            let reading = host.read(started, &running, sent);
            tokio::pin!(reading);
            let mut rescue = rescue;
            let (learned, ended) = loop {
                tokio::select! {
                    ended = &mut reading => break ended,
                    Some(look) = heard.recv() => {
                        if looked(&mut rescue, look) {
                            let _ = self.kept_rescue(&rescue).await;
                        }
                    }
                }
            };
            while let Ok(look) = heard.try_recv() {
                looked(&mut rescue, look);
            }
            self.scouts().ended(&key);
            self.rescue_ended(rescue, learned, ended).await;
        });
    }

    async fn rescue_ended(
        &self,
        mut rescue: KeptRescue,
        learned: Option<String>,
        ended: ScoutEnded,
    ) {
        rescue.summary = learned;
        rescue.cost_micros = ended.cost_micros;
        match ended.outcome {
            ScoutOutcome::Answered => rescue.state = RescueState::Answered,
            ScoutOutcome::Stopped => rescue.state = RescueState::Stopped,
            ScoutOutcome::Failed { why } => {
                rescue.state = RescueState::Failed;
                rescue.why = Some(why);
            }
        }
        let _ = self.kept_rescue(&rescue).await;
    }

    fn rescue_refusal(&self, why: RescueRefused) -> Refusal {
        let raised = |code: &str, said: String| WireError::raised(code, said, self.run_id());
        match why {
            RescueRefused::NoSuchSlot(n) => {
                Refusal::Unacceptable(raised(NO_SUCH_SLOT, format!("no slot-{n}")))
            }
            RescueRefused::NotStranded(what) => {
                Refusal::IllegalMove(raised(NOT_STRANDED, format!("not stranded, {what}")))
            }
            RescueRefused::Busy => {
                Refusal::IllegalMove(raised(SLOT_BUSY, String::from("a lease is under way")))
            }
            RescueRefused::OnNoBranch => Refusal::IllegalMove(raised(
                RESCUE_ON_NO_BRANCH,
                String::from("on no branch to keep it on"),
            )),
            RescueRefused::OnTheBase(base) => Refusal::IllegalMove(raised(
                RESCUE_ON_THE_BASE,
                format!("on {base}, which a stash does not push to"),
            )),
            RescueRefused::NoRemote => Refusal::IllegalMove(raised(
                RESCUE_NO_REMOTE,
                String::from("no origin to push to"),
            )),
            RescueRefused::Vcs(said) => Refusal::Fault(raised(SLOT_UNCHANGED, said)),
        }
    }
}

/// Record what the scout looked at. `true` where that changed the Finding.
fn looked(rescue: &mut KeptRescue, look: ScoutLook) -> bool {
    let (list, item) = match look {
        ScoutLook::File(path) => (&mut rescue.read, path),
        ScoutLook::Search(search) => (&mut rescue.searched, search),
    };
    if list.contains(&item) {
        return false;
    }
    list.push(item);
    true
}
