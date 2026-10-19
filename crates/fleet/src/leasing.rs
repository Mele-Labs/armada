//! A repository's worktree pool as Fleet reads it: every slot and who holds it,
//! and a person's change to the pool. `docs/concepts/fleet.md`, *Worktree
//! slots*.
//!
//! A hosted Session leases a slot by its own id and `crate::session_host` gives
//! it back; this reads the pool and never takes or returns a lease.

use std::sync::Arc;

use adapter_traits::{
    AgentHarness, Delivery, SlotChange, SlotHeld, SlotPool, SlotReading, SlotRefused, Vcs,
    WorkProduct,
};
use api::Refusal;
use ipc::{ChangeSlotPool, ManifestId, SlotAct, SlotPoolChanged, WireError};

use crate::daemon::Fleet;
use crate::repositories::Served;
use crate::session_host::following::{pid_in, process_listing, SlotSession};

/// The base a Manifest that names none is leased from, as `armada worktree`
/// leases it.
const BASE_UNSTATED: &str = "main";

/// A slot the pool does not have, or none named. A 422.
pub(crate) const NO_SUCH_SLOT: &str = "fleet.no_such_slot";
/// A slot removed while something holds it. A 409, as are the five after.
const SLOT_HELD: &str = "fleet.slot_held";
/// A slot removed whose holder is gone and left work in it.
const SLOT_STRANDED: &str = "fleet.slot_stranded";
/// A slot removed while a lease or a release is under way on it.
pub(crate) const SLOT_BUSY: &str = "fleet.slot_busy";
/// A slot removed that is a directory and not a checkout: a person's to clear.
const SLOT_NOT_A_CHECKOUT: &str = "fleet.slot_not_a_checkout";
/// A slot removed whose checkout holds uncommitted files.
const SLOT_DIRTY: &str = "fleet.slot_dirty";
/// A release asked for on behalf of a holder the slot no longer has. A 409.
pub(crate) const SLOT_HOLDER_CHANGED: &str = "fleet.slot_holder_changed";
/// A release asked for with no holder named. A 422.
pub(crate) const SLOT_HOLDER_UNNAMED: &str = "fleet.slot_holder_unnamed";
/// A release whose uncommitted files git could not commit to a branch. A 409.
pub(crate) const SLOT_NOT_PARKABLE: &str = "fleet.slot_not_parkable";
/// A release of a hosted session's slot while its turn runs: the park ends every process in
/// the slot, its agent's too. A 409.
pub(crate) const SLOT_SESSION_WORKING: &str = "fleet.slot_session_working";
/// The pool's one slot, removed.
const SLOT_LAST: &str = "fleet.slot_last";
/// git refused, and this is what it said. A 500.
pub(crate) const SLOT_UNCHANGED: &str = "fleet.slot_unchanged";

/// Every slot number the pool has on this machine. **A person may add slots past
/// `setup.worktrees`**, and a lease takes any of them, so a count from the Manifest
/// missed a session's slot-9 on a machine of fifteen (15 Oct 2026).
pub(crate) fn bays_of(served: &Served) -> Vec<u32> {
    let pool = pool_of(served);
    adapters::leasing::Pool::at(
        std::path::Path::new(pool.repo_root()),
        pool.slots() as usize,
        pool.base(),
        pool.keep().to_vec(),
    )
    .bays()
    .into_iter()
    .filter_map(|bay| u32::try_from(bay).ok())
    .collect()
}

/// The served repository's pool, as its root Manifest sizes it.
pub(crate) fn pool_of(served: &Served) -> SlotPool {
    let manifest = served.manifest();
    let keep = manifest
        .seed()
        .map(|seed| seed.paths().to_vec())
        .unwrap_or_default();
    SlotPool::of(
        served.root(),
        manifest.worktrees().get(),
        manifest.base().unwrap_or(BASE_UNSTATED),
        keep,
    )
}

/// One slot of a served repository's pool.
pub(crate) struct PoolSlot {
    pub(crate) manifest: String,
    pub(crate) base: String,
    pub(crate) reading: SlotReading,
    /// The hosted Session that holds it, itself or through a process it
    /// started, where one does.
    pub(crate) session: Option<SlotSession>,
    /// What it holds, where it is stranded.
    pub(crate) stranded: Option<adapter_traits::StrandedWork>,
    /// What a rescue Scout read of it, where one has.
    pub(crate) rescue: Option<store::KeptRescue>,
    /// Who held it last, where it shows work and the record or the lease names one.
    pub(crate) last_holder: Option<ipc::SlotLastHolder>,
    /// When Fleet told that holder to commit, push and release, as an instant.
    pub(crate) told: Option<String>,
}

/// How the pool is read for the list.
#[derive(Clone, Copy)]
pub(crate) enum Reading {
    /// Every slot, from git, now. A test's read, which keeps nothing.
    #[cfg(test)]
    Live,
    /// What the adapter keeps, with `whole` reading every slot again.
    Kept { whole: bool },
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
    /// Every served repository's pool, slot by slot: what Bridge's Cleanup
    /// draws, read the way `armada worktree --status` reads it.
    pub(crate) async fn pool_slots(&self, how: Reading) -> Vec<PoolSlot> {
        let (mut kept, strandings) = {
            let store = self.store().lock().await;
            (
                store.rescues().unwrap_or_default(),
                store.strandings().unwrap_or_default(),
            )
        };
        let mut listing: Option<String> = None;
        let mut slots = Vec::new();
        for served in self.repositories().hosted() {
            let manifest = served.manifest().id().as_str().to_string();
            let pool = pool_of(&served);
            for mut reading in self.pool_read(pool.clone(), how).await {
                let stranded = reading.work.take();
                let session = match &reading.held {
                    // A hosted Session's lease is written under its own id.
                    SlotHeld::Hosted(id) => self.session_in_ledger(id).await,
                    SlotHeld::Session(said) => match pid_in(said) {
                        Some(pid) => {
                            if listing.is_none() {
                                listing = Some(
                                    tokio::task::spawn_blocking(process_listing)
                                        .await
                                        .unwrap_or_default(),
                                );
                            }
                            match self.session_holding_pid(pid, listing.as_deref().unwrap_or("")) {
                                Some(id) => {
                                    // Once its process is gone the holder cannot say whose it was.
                                    let _ = adapters::leasing::note_session(
                                        std::path::Path::new(&reading.path),
                                        &id,
                                    );
                                    Some(
                                        self.session_in_ledger(&id)
                                            .await
                                            .unwrap_or(SlotSession { id, title: None }),
                                    )
                                }
                                None => None,
                            }
                        }
                        None => None,
                    },
                    _ => None,
                };
                let last_holder = match (&stranded, &reading.held) {
                    (Some(_), SlotHeld::Hosted(_)) => match &session {
                        Some(held) => Some(self.session_holder(&held.id).await),
                        None => None,
                    },
                    (Some(_), SlotHeld::Stranded(_)) => match &reading.last_session {
                        Some(id) => Some(self.session_holder(id).await),
                        None => None,
                    },
                    _ => None,
                };
                let (branch, since) = crate::stranding::key_of(&reading);
                let told = strandings
                    .iter()
                    .find(|one| {
                        one.manifest_id == manifest
                            && one.slot == reading.slot
                            && one.is_of(&branch, &since)
                    })
                    .and_then(|one| one.told_at.clone());
                // **A Finding is of the commit it read.** One whose slot has
                // moved on, or is no longer stranded, is not shown.
                let at = kept.iter().position(|one| {
                    one.manifest_id == manifest
                        && one.slot == reading.slot
                        && stranded
                            .as_ref()
                            .is_some_and(|work| work.commit == one.commit)
                });
                let rescue = at.map(|at| kept.swap_remove(at));
                slots.push(PoolSlot {
                    manifest: manifest.clone(),
                    base: pool.base().to_string(),
                    reading,
                    session,
                    stranded,
                    rescue,
                    last_holder,
                    told,
                });
            }
        }
        slots
    }

    /// Each slot of one pool, off the runtime. The pool reads its slots side
    /// by side and each reading carries what its slot holds, where it shows
    /// work: `SlotReading::with_work` is the rule.
    async fn pool_read(&self, pool: SlotPool, how: Reading) -> Vec<SlotReading> {
        let vcs = Arc::clone(self.vcs());
        tokio::task::spawn_blocking(move || match how {
            #[cfg(test)]
            Reading::Live => vcs.slot_pool(&pool),
            Reading::Kept { whole } => vcs.slot_pool_kept(&pool, whole),
        })
            .await
            .expect("git panicked reading the pool")
    }

    /// A person's change to a repository's pool, from Cleanup's bay grid. The
    /// pool writes it beside the slots, so `armada worktree lease` honours it
    /// too and it outlives Fleet.
    pub(crate) fn change_slot_pool(
        &self,
        asked: ChangeSlotPool,
        manifest_id: Option<&ManifestId>,
    ) -> Result<SlotPoolChanged, Refusal> {
        let served = self.served_named(manifest_id)?;
        let raised = |code: &str, said: String| WireError::raised(code, said, self.run_id());
        if asked.act == SlotAct::Release {
            let released = self.session_released(&served, &asked);
            if released.is_ok() {
                self.hosts().slot_freed();
            }
            return released;
        }
        let change = match (asked.act, asked.slot) {
            (SlotAct::Add, _) => SlotChange::Add,
            (SlotAct::Remove, Some(n)) => SlotChange::Remove(n),
            (SlotAct::Close, Some(n)) => SlotChange::Close(n),
            (SlotAct::Open, Some(n)) => SlotChange::Open(n),
            (SlotAct::Release, _) => unreachable!("answered above"),
            (_, None) => {
                return Err(Refusal::Unacceptable(raised(
                    NO_SUCH_SLOT,
                    String::from("no slot named"),
                )))
            }
        };
        let slot = self
            .vcs()
            .change_slot_pool(&pool_of(&served), change)
            .map_err(|why| match why {
                SlotRefused::NoSuchSlot(n) => {
                    Refusal::Unacceptable(raised(NO_SUCH_SLOT, format!("no slot-{n}")))
                }
                SlotRefused::Held(by) => {
                    Refusal::IllegalMove(raised(SLOT_HELD, format!("held by {by}")))
                }
                SlotRefused::Stranded(what) => {
                    Refusal::IllegalMove(raised(SLOT_STRANDED, format!("stranded, holding {what}")))
                }
                SlotRefused::Busy => {
                    Refusal::IllegalMove(raised(SLOT_BUSY, String::from("a lease is under way")))
                }
                SlotRefused::NotACheckout => Refusal::IllegalMove(raised(
                    SLOT_NOT_A_CHECKOUT,
                    String::from("not a checkout"),
                )),
                SlotRefused::Dirty(files) => Refusal::IllegalMove(raised(
                    SLOT_DIRTY,
                    format!(
                        "{} uncommitted, first {}",
                        files.len(),
                        files.first().map(String::as_str).unwrap_or_default()
                    ),
                )),
                SlotRefused::LastSlot => {
                    Refusal::IllegalMove(raised(SLOT_LAST, String::from("the last slot")))
                }
                SlotRefused::Vcs(said) => Refusal::Fault(raised(SLOT_UNCHANGED, said)),
            })?;
        // A slot added or reopened is one a waiting session can take.
        self.hosts().slot_freed();
        Ok(SlotPoolChanged {
            manifest_id: ManifestId::from(served.manifest().id()),
            slot,
            released: None,
        })
    }
}
