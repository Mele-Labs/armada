//! A worktree pool: which Job holds which slot, by its id.
//!
//! **Mirrored to the disk the way the real pool keeps it** — the slot's
//! directory, and a `slot-<n>.lease` naming its holder — because a Fleet that
//! restarts gets a new `FakeVcs`, and the lease has to outlive the process
//! that took it exactly as the real one does. Where the root cannot be
//! written, it is held in memory alone.
//!
//! Faithful about what Fleet has to handle — a Job handed its own slot back, a
//! full pool, a slot somebody else took or that is gone, and a release the pool
//! refused — and about the path, which is `adapter_traits::slot_path`. The
//! pool's git rules are tested against a real repository in `adapters`.
//!
//! **A slot closed or removed is held in memory alone**: the real pool's
//! record of it is `adapters`', and a case about a restart does not ask.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;

use adapter_traits::{
    slot_path, RescueRefused, SlotChange, SlotHeld, SlotKept, SlotLeased, SlotPool, SlotReading,
    SlotRefused, SlotRescue, SlotRescued, SlotStanding, StrandedWork, Worktree, WorktreeSpec,
};

use super::{FakeVcs, FakeVcsError};

/// One slot: made or not, and who holds it.
#[derive(Clone, Debug, Default)]
struct Slot {
    made: bool,
    held_by: Option<String>,
    closed: bool,
    removed: bool,
    /// Its holder is gone and it holds this, and the diff a Scout is handed.
    stranded: Option<(StrandedWork, String)>,
}

impl Slot {
    /// Free, open and one of the pool's.
    fn leasable(&self) -> bool {
        self.held_by.is_none() && !self.closed && !self.removed && self.stranded.is_none()
    }

    /// As the disk has it.
    fn read(root: &str, slot: u32) -> Slot {
        let path = slot_path(root, slot);
        let held_by = std::fs::read_to_string(format!("{path}.lease"))
            .ok()
            .and_then(|text| {
                text.lines()
                    .find_map(|line| line.strip_prefix("holder job "))
                    .map(str::to_string)
            });
        Slot {
            made: Path::new(&path).is_dir(),
            held_by,
            ..Slot::default()
        }
    }

    /// The record onto the disk, in the real pool's format. Best-effort: a
    /// root nothing can write is held in memory. **Never the directory**: a
    /// case that removes one is a case about a slot gone from under its Job.
    fn write(&self, root: &str, slot: u32) {
        let path = slot_path(root, slot);
        let record = match &self.held_by {
            Some(by) => format!("branch -\nholder job {by}\nsince 0\n"),
            None => String::new(),
        };
        if Path::new(root).is_dir() {
            if let Some(parent) = Path::new(&path).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(format!("{path}.lease"), record);
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct FakeSlots {
    /// By repository root, slot `n` at index `n - 1`.
    pools: Mutex<BTreeMap<String, Vec<Slot>>>,
    /// What the next release answers instead of giving the slot back.
    keep_next: Mutex<Option<String>>,
    /// What every release answers, where a case's Jobs end holding work.
    keep_every: Mutex<Option<String>>,
    released: Mutex<Vec<(u32, String)>>,
    completed: Mutex<Vec<(u32, String)>>,
    rescued: Mutex<Vec<(u32, SlotRescue)>>,
}

impl FakeSlots {
    /// Act on the pool at `pool`'s root, read from the disk the first time,
    /// and written back after.
    fn with<T>(&self, pool: &SlotPool, act: impl FnOnce(&mut Vec<Slot>) -> T) -> T {
        self.at(pool.repo_root(), pool.slots(), act)
    }

    fn at<T>(&self, root: &str, count: u32, act: impl FnOnce(&mut Vec<Slot>) -> T) -> T {
        let mut pools = self.pools.lock().expect("not poisoned");
        let slots = pools.entry(root.to_string()).or_default();
        while slots.len() < count as usize {
            let next = slots.len() as u32 + 1;
            slots.push(Slot::read(root, next));
        }
        let answer = act(slots);
        for (at, slot) in slots.iter().enumerate() {
            slot.write(root, at as u32 + 1);
        }
        answer
    }

    pub(super) fn lease(&self, pool: &SlotPool, spec: &WorktreeSpec, job: &str) -> SlotLeased {
        self.with(pool, |slots| {
            let mine = slots
                .iter()
                .position(|slot| slot.held_by.as_deref() == Some(job));
            let Some(at) = mine.or_else(|| slots.iter().position(Slot::leasable)) else {
                return SlotLeased::Full;
            };
            let reused = slots[at].made;
            slots[at].made = true;
            slots[at].held_by = Some(job.to_string());
            let slot = at as u32 + 1;
            let path = spec.clone().in_slot(slot).worktree_path();
            // The one directory this fake makes: a Drone needs a working
            // directory that is there, and nobody knows the slot before this.
            let _ = std::fs::create_dir_all(&path);
            SlotLeased::Took {
                slot,
                worktree: Worktree::at(path, spec.branch()),
                reused,
            }
        })
    }

    pub(super) fn open(&self, pool: &SlotPool, job: &str) -> bool {
        self.with(pool, |slots| {
            slots
                .iter()
                .any(|slot| slot.leasable() || slot.held_by.as_deref() == Some(job))
        })
    }

    pub(super) fn standing(&self, pool: &SlotPool, slot: u32, job: &str) -> SlotStanding {
        self.with(pool, |slots| match slots.get(slot as usize - 1) {
            None | Some(Slot { made: false, .. }) => SlotStanding::Gone,
            Some(Slot { held_by: None, .. }) => SlotStanding::Free,
            Some(Slot {
                held_by: Some(by), ..
            }) if by == job => SlotStanding::Held,
            Some(Slot {
                held_by: Some(by), ..
            }) => SlotStanding::HeldBy(by.clone()),
        })
    }

    pub(super) fn release(&self, pool: &SlotPool, slot: u32, job: &str) -> Result<(), SlotKept> {
        if let Some(why) = self.keep_next.lock().expect("not poisoned").take() {
            return Err(SlotKept(why));
        }
        if let Some(why) = self.keep_every.lock().expect("not poisoned").clone() {
            return Err(SlotKept(why));
        }
        self.with(pool, |slots| {
            let Some(at) = slots
                .get_mut(slot as usize - 1)
                .filter(|at| at.held_by.as_deref() == Some(job))
            else {
                return Err(SlotKept(format!("{job} does not hold slot-{slot}")));
            };
            at.held_by = None;
            Ok(())
        })?;
        self.released
            .lock()
            .expect("not poisoned")
            .push((slot, job.to_string()));
        Ok(())
    }

    /// Every slot: made or not and who holds it. Never warm, and never behind,
    /// because nothing here is a checkout git could count.
    pub(super) fn readings(&self, pool: &SlotPool) -> Vec<SlotReading> {
        let root = pool.repo_root().to_string();
        self.with(pool, |slots| {
            slots
                .iter()
                .enumerate()
                .filter(|(_, slot)| !slot.removed)
                .map(|(at, slot)| {
                    let number = at as u32 + 1;
                    SlotReading {
                        slot: number,
                        path: slot_path(&root, number),
                        held: match (&slot.held_by, slot.made, &slot.stranded) {
                            (Some(job), _, _) => SlotHeld::Job(job.clone()),
                            (None, true, Some((work, _))) => SlotHeld::Stranded(format!(
                                "{} uncommitted",
                                work.uncommitted.len()
                            )),
                            (None, true, None) => SlotHeld::Free,
                            (None, false, _) => SlotHeld::Unmade,
                        },
                        branch: slot
                            .stranded
                            .as_ref()
                            .and_then(|(work, _)| work.branch.clone()),
                        since: None,
                        warm: false,
                        behind: None,
                        closed: slot.closed,
                    }
                })
                .collect()
        })
    }

    /// Who holds each slot of the pool at `root` this fake has looked at, in
    /// order.
    pub(super) fn holders(&self, root: &str) -> Vec<Option<String>> {
        self.at(root, 0, |slots| {
            slots.iter().map(|slot| slot.held_by.clone()).collect()
        })
    }

    pub(super) fn hold(&self, root: &str, slot: u32, by: Option<&str>, made: bool) {
        let path = slot_path(root, slot);
        let _ = match made {
            true => std::fs::create_dir_all(&path),
            false => std::fs::remove_dir_all(&path),
        };
        self.at(root, slot, |slots| {
            slots[slot as usize - 1] = Slot {
                made,
                held_by: by.map(str::to_string),
                ..Slot::default()
            };
        });
    }

    /// The real pool's rules, on the fake's slots: add takes the lowest
    /// number unused, and only a free or unmade slot goes.
    pub(super) fn change(&self, pool: &SlotPool, change: SlotChange) -> Result<u32, SlotRefused> {
        self.with(pool, |slots| {
            let number = match change {
                SlotChange::Add => {
                    return Ok(match slots.iter().position(|slot| slot.removed) {
                        Some(at) => {
                            slots[at] = Slot::default();
                            at as u32 + 1
                        }
                        None => {
                            slots.push(Slot::default());
                            slots.len() as u32
                        }
                    });
                }
                SlotChange::Remove(n) | SlotChange::Close(n) | SlotChange::Open(n) => n,
            };
            let left = slots.iter().filter(|slot| !slot.removed).count();
            let Some(slot) = (number as usize)
                .checked_sub(1)
                .and_then(|at| slots.get_mut(at))
                .filter(|slot| !slot.removed)
            else {
                return Err(SlotRefused::NoSuchSlot(number));
            };
            match change {
                SlotChange::Close(_) => slot.closed = true,
                SlotChange::Open(_) => slot.closed = false,
                _ if left == 1 => return Err(SlotRefused::LastSlot),
                _ => match &slot.held_by {
                    Some(by) => return Err(SlotRefused::Held(format!("job {by}"))),
                    None => {
                        *slot = Slot {
                            removed: true,
                            ..Slot::default()
                        }
                    }
                },
            }
            Ok(number)
        })
    }

    /// Leave slot `slot` at `root` stranded, holding `work`.
    pub(super) fn strand(&self, root: &str, slot: u32, work: StrandedWork, diff: &str) {
        let _ = std::fs::create_dir_all(slot_path(root, slot));
        self.at(root, slot, |slots| {
            slots[slot as usize - 1] = Slot {
                made: true,
                stranded: Some((work, diff.to_string())),
                ..Slot::default()
            };
        });
    }

    fn stranded_in(
        &self,
        pool: &SlotPool,
        slot: u32,
    ) -> Result<(StrandedWork, String), RescueRefused> {
        self.with(pool, |slots| {
            let Some(at) = (slot as usize)
                .checked_sub(1)
                .and_then(|at| slots.get(at))
                .filter(|at| !at.removed)
            else {
                return Err(RescueRefused::NoSuchSlot(slot));
            };
            at.stranded.clone().ok_or_else(|| {
                RescueRefused::NotStranded(String::from(match (&at.held_by, at.made) {
                    (Some(_), _) => "held",
                    (None, true) => "free",
                    (None, false) => "not made",
                }))
            })
        })
    }

    pub(super) fn stranded_work(
        &self,
        pool: &SlotPool,
        slot: u32,
    ) -> Result<StrandedWork, RescueRefused> {
        self.stranded_in(pool, slot).map(|(work, _)| work)
    }

    pub(super) fn stranded_diff(&self, pool: &SlotPool, slot: u32) -> Result<String, RescueRefused> {
        self.stranded_in(pool, slot).map(|(_, diff)| diff)
    }

    /// The real pool's refusals that Fleet answers: a slot not stranded, and a
    /// stash with no branch to keep the work on. Either act frees the slot.
    pub(super) fn rescue(
        &self,
        pool: &SlotPool,
        slot: u32,
        rescue: SlotRescue,
    ) -> Result<SlotRescued, RescueRefused> {
        let (work, _) = self.stranded_in(pool, slot)?;
        let rescued = match &rescue {
            SlotRescue::Scrap => SlotRescued {
                branch: work.branch.clone(),
                branch_kept: work.unpushed > 0,
                committed: None,
            },
            SlotRescue::Stash { .. } => {
                if work.branch.is_none() {
                    return Err(RescueRefused::OnNoBranch);
                }
                SlotRescued {
                    branch: work.branch.clone(),
                    branch_kept: true,
                    committed: (!work.uncommitted.is_empty()).then(|| String::from("stashed")),
                }
            }
        };
        self.with(pool, |slots| slots[slot as usize - 1].stranded = None);
        self.rescued
            .lock()
            .expect("not poisoned")
            .push((slot, rescue));
        Ok(rescued)
    }

    pub(super) fn keep_every(&self, why: &str) {
        *self.keep_every.lock().expect("not poisoned") = Some(why.to_string());
    }

    pub(super) fn keep_next(&self, why: &str) {
        *self.keep_next.lock().expect("not poisoned") = Some(why.to_string());
    }

    pub(super) fn released(&self) -> Vec<(u32, String)> {
        self.released.lock().expect("not poisoned").clone()
    }

    pub(super) fn completed(&self, slot: u32, job: &str) {
        self.completed
            .lock()
            .expect("not poisoned")
            .push((slot, job.to_string()));
    }
}

impl FakeVcs {
    /// Which Job holds each slot of the pool at `root`, slot 1 first.
    pub fn slot_holders(&self, root: &str) -> Vec<Option<String>> {
        self.slots.holders(root)
    }

    /// Hand slot `slot` at `root` to somebody else, as an agent's lease would.
    pub fn hold_slot(&self, root: &str, slot: u32, by: &str) {
        self.slots.hold(root, slot, Some(by), true);
    }

    /// Give slot `slot` at `root` back, as an agent's release would.
    pub fn free_slot(&self, root: &str, slot: u32) {
        self.slots.hold(root, slot, None, true);
    }

    /// Take slot `slot` at `root` off the disk, as a person removing it would.
    pub fn lose_slot(&self, root: &str, slot: u32) {
        self.slots.hold(root, slot, None, false);
    }

    /// Make the next release refuse, as the pool does for a dirty tree or
    /// unlanded commits.
    pub fn keep_next_release(&self, why: &str) {
        self.slots.keep_next(why);
    }

    /// Make every release refuse, so a Job that ends keeps its slot — what the
    /// pool does when its work is on neither the remote nor the base.
    pub fn keep_every_release(&self, why: &str) {
        self.slots.keep_every(why);
    }

    /// Every slot given back, and by which Job, in order.
    pub fn released_slots(&self) -> Vec<(u32, String)> {
        self.slots.released()
    }

    /// Leave slot `slot` at `root` stranded, holding `work`, with `diff` as
    /// its change against the base.
    pub fn strand_slot(&self, root: &str, slot: u32, work: StrandedWork, diff: &str) {
        self.slots.strand(root, slot, work, diff);
    }

    /// Every stranded slot rescued, and how, in order.
    pub fn rescued_slots(&self) -> Vec<(u32, SlotRescue)> {
        self.slots.rescued.lock().expect("not poisoned").clone()
    }

    /// Every slot marked as held by a Job that completed, in order.
    pub fn completed_slots(&self) -> Vec<(u32, String)> {
        self.slots.completed.lock().expect("not poisoned").clone()
    }

    /// A lease, refused the ways `create_worktree` scripts: a machine that will
    /// not cooperate, and a branch somebody already made.
    pub(super) fn leased(
        &self,
        pool: &SlotPool,
        spec: &WorktreeSpec,
        job_id: &str,
    ) -> Result<SlotLeased, FakeVcsError> {
        if let Some(standing_in_for) = self.refuse_next.lock().expect("not poisoned").take() {
            return Err(FakeVcsError::Refused { standing_in_for });
        }
        let branch = spec.branch();
        let holds_one = self
            .slots
            .holders(pool.repo_root())
            .iter()
            .any(|holder| holder.as_deref() == Some(job_id));
        if !holds_one
            && self
                .branches
                .lock()
                .expect("not poisoned")
                .contains(&branch)
        {
            return Err(FakeVcsError::BranchExists { branch });
        }
        let leased = self.slots.lease(pool, spec, job_id);
        if let SlotLeased::Took { worktree, .. } = &leased {
            self.cut_from
                .lock()
                .expect("not poisoned")
                .push(pool.base().to_string());
            if self.branches.lock().expect("not poisoned").insert(branch) {
                self.created
                    .lock()
                    .expect("not poisoned")
                    .push(worktree.clone());
            }
        }
        Ok(leased)
    }
}
