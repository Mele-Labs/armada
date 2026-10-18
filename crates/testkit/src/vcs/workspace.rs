//! The `Vcs` half of `FakeVcs`: worktrees, branches, commits and slots. Split from `Delivery` to keep each file short to read by range.

use super::*;

impl Vcs for FakeVcs {
    type Error = FakeVcsError;
    type CommitError = FakeVcsError;

    fn create_worktree(&self, spec: &WorktreeSpec) -> Result<Worktree, Self::Error> {
        if let Some(standing_in_for) = self.refuse_next.lock().expect("not poisoned").take() {
            return Err(FakeVcsError::Refused { standing_in_for });
        }
        let branch = spec.branch();
        if !self
            .branches
            .lock()
            .expect("not poisoned")
            .insert(branch.clone())
        {
            return Err(FakeVcsError::BranchExists { branch });
        }
        let made = Worktree::at(spec.worktree_path(), branch);
        self.created
            .lock()
            .expect("not poisoned")
            .push(made.clone());
        Ok(made)
    }

    /// **`None` where no ref was scripted at all**, which is a repository that
    /// names no base — the same silence the real one answers with, and not the
    /// error a declared-but-missing branch raises.
    fn base_commit(
        &self,
        _repo_root: &str,
        declared: Option<&str>,
    ) -> Result<Option<String>, Self::Error> {
        let refs = self.refs.lock().expect("not poisoned");
        match declared {
            Some(name) => {
                refs.get(name)
                    .cloned()
                    .map(Some)
                    .ok_or_else(|| FakeVcsError::NoSuchRef {
                        r#ref: name.to_string(),
                    })
            }
            None => Ok(refs.values().next().cloned()),
        }
    }

    /// Every ref the fake holds, by name: `declared`, or the first where
    /// nothing is declared, is the base — `base_commit`'s own reading.
    fn branches(
        &self,
        _repo_root: &str,
        declared: Option<&str>,
    ) -> Result<Vec<adapter_traits::BranchListed>, Self::Error> {
        let refs = self.refs.lock().expect("not poisoned");
        let base = declared.or_else(|| refs.keys().next().map(String::as_str));
        let mut listed: Vec<adapter_traits::BranchListed> = refs
            .keys()
            .map(|name| adapter_traits::BranchListed {
                name: name.clone(),
                base: Some(name.as_str()) == base,
            })
            .collect();
        listed.sort_by_key(|branch| !branch.base);
        Ok(listed)
    }

    fn create_branch(&self, _repo_root: &str, name: &str, start: &str) -> Result<(), Self::Error> {
        commit::cut_branch(self, name, start)
    }

    /// **Answers the same checkout every time it is asked for one commit**,
    /// which is the sharing the real one promises. The `prepared` flag comes
    /// back as it was left, so a second Job on one base is told it need not
    /// prepare again.
    fn base_checkout(&self, spec: &BaseSpec) -> Result<BaseCheckout, Self::Error> {
        if let Some(standing_in_for) = self.refuse_next.lock().expect("not poisoned").take() {
            return Err(FakeVcsError::Refused { standing_in_for });
        }
        let mut bases = self.bases.lock().expect("not poisoned");
        let prepared = *bases.entry(spec.commit().to_string()).or_insert(false);
        Ok(BaseCheckout::at(spec.path(), spec.commit(), prepared))
    }

    fn drop_base_checkout(&self, spec: &BaseSpec) -> Result<(), Self::Error> {
        self.bases
            .lock()
            .expect("not poisoned")
            .remove(spec.commit());
        self.dropped_bases
            .lock()
            .expect("not poisoned")
            .push(spec.commit().to_string());
        Ok(())
    }

    fn commit_all(
        &self,
        worktree: &Worktree,
        message: &str,
        at: CommitTime,
    ) -> Result<Committed, Self::CommitError> {
        self.commit(worktree, CommitScope::All, message, at)
    }

    fn commit_paths(
        &self,
        worktree: &Worktree,
        paths: &[&str],
        message: &str,
        at: CommitTime,
    ) -> Result<Committed, Self::CommitError> {
        let paths = paths.iter().map(|path| path.to_string()).collect();
        self.commit(worktree, CommitScope::Paths(paths), message, at)
    }

    /// Clones nothing; [`FakeVcs::refuse_next`] stands in for git refusing.
    fn clone_repository(
        &self,
        _url: &str,
        _destination: &str,
        _within: std::time::Duration,
    ) -> Result<(), NotCloned> {
        match self.refuse_next.lock().expect("not poisoned").take() {
            Some(standing_in_for) => Err(NotCloned::Refused {
                said: standing_in_for.to_string(),
            }),
            None => Ok(()),
        }
    }

    /// The same refusals [`create_worktree`](Vcs::create_worktree) scripts.
    fn lease_slot(
        &self,
        pool: &SlotPool,
        spec: &WorktreeSpec,
        job_id: &str,
    ) -> Result<SlotLeased, Self::Error> {
        self.noted(self.leased(pool, spec, job_id))
    }

    fn lease_existing_slot(
        &self,
        pool: &SlotPool,
        branch: &str,
        job_id: &str,
    ) -> Result<SlotLeased, Self::Error> {
        let leased = self.slots.lease_existing(pool, branch, job_id);
        self.noted(Ok(leased))
    }

    fn park_slot(
        &self,
        pool: &SlotPool,
        slot: u32,
        job_id: &str,
    ) -> Result<adapter_traits::SlotParked, adapter_traits::SlotParkRefused> {
        self.noted(self.slots.park(pool, slot, job_id))
    }

    fn delete_repair_branch(
        &self,
        spec: &WorktreeSpec,
    ) -> Result<bool, adapter_traits::BranchKept> {
        self.deleted_branches
            .lock()
            .expect("not poisoned")
            .push(spec.branch());
        Ok(true)
    }

    fn delete_merged_branch(
        &self,
        _pool: &SlotPool,
        branch: &str,
    ) -> Result<bool, adapter_traits::BranchKept> {
        self.deleted_branches
            .lock()
            .expect("not poisoned")
            .push(branch.to_string());
        Ok(true)
    }

    fn branch_is_merged(&self, _pool: &SlotPool, _branch: &str) -> bool {
        true
    }

    fn park_hosted_slot(
        &self,
        pool: &SlotPool,
        slot: u32,
        session_id: &str,
    ) -> Result<adapter_traits::SlotParked, adapter_traits::SlotParkRefused> {
        self.noted(self.slots.park(pool, slot, session_id))
    }

    fn park_ended_session_slot(
        &self,
        pool: &SlotPool,
        slot: u32,
        session_id: &str,
    ) -> Result<adapter_traits::SlotParked, adapter_traits::SlotParkRefused> {
        self.noted(self.slots.park_left(pool, slot, session_id))
    }

    fn release_session_slot(
        &self,
        pool: &SlotPool,
        slot: u32,
        holder: &str,
    ) -> Result<adapter_traits::SlotParked, adapter_traits::SlotParkRefused> {
        self.noted(self.slots.park(pool, slot, holder))
            .map_err(|refused| match refused {
                adapter_traits::SlotParkRefused::HeldByAnother(now) => {
                    adapter_traits::SlotParkRefused::HolderChanged(now)
                }
                adapter_traits::SlotParkRefused::NotLeased => {
                    adapter_traits::SlotParkRefused::HolderChanged(String::from("nobody"))
                }
                other => other,
            })
    }

    fn hand_session_slot(
        &self,
        pool: &SlotPool,
        slot: u32,
        from: &str,
        to_session: &str,
    ) -> Result<(), adapter_traits::SlotParkRefused> {
        self.noted(self.slots.hand(pool, slot, from, to_session))
    }

    fn lease_review_bay(
        &self,
        pool: &SlotPool,
        pull_request: u64,
        holder: &str,
    ) -> Result<adapter_traits::ReviewBay, String> {
        let head = match self.review_head.lock().expect("not poisoned").clone() {
            Some(Err(said)) => return Err(said),
            Some(Ok(head)) => head,
            None => "a".repeat(40),
        };
        let leased = self.slots.lease_existing(pool, &format!("review/{holder}"), holder);
        self.noted(());
        match leased {
            SlotLeased::Took { slot, worktree, .. } => {
                self.review_leases
                    .lock()
                    .expect("not poisoned")
                    .push((pull_request, holder.to_string()));
                Ok(adapter_traits::ReviewBay::Took {
                    slot,
                    path: worktree.path().to_string(),
                    head,
                })
            }
            SlotLeased::Full => Ok(adapter_traits::ReviewBay::Full),
        }
    }

    fn release_review_bay(&self, pool: &SlotPool, slot: u32, holder: &str) -> Result<(), String> {
        self.review_releases
            .lock()
            .expect("not poisoned")
            .push((slot, holder.to_string()));
        self.noted(self.slots.release(pool, slot, holder).map_err(|kept| kept.0))
    }

    fn paths_touched_by(&self, _pool: &SlotPool, authors: &[String], commits: usize) -> Vec<String> {
        self.history_read(authors, commits)
    }

    fn slot_standing(&self, pool: &SlotPool, slot: u32, job_id: &str) -> SlotStanding {
        self.slots.standing(pool, slot, job_id)
    }

    fn slot_after_merge(&self, _pool: &SlotPool, slot: u32, _merged: &str) -> adapter_traits::SlotAfterMerge {
        self.slots.after_merge(slot)
    }

    fn release_slot(&self, pool: &SlotPool, slot: u32, job_id: &str) -> Result<(), SlotKept> {
        self.noted(self.slots.release(pool, slot, job_id))
    }

    fn slot_pool(&self, pool: &SlotPool) -> Vec<SlotReading> {
        self.pool_reads.fetch_add(1, Ordering::Relaxed);
        self.slots
            .readings(pool)
            .into_iter()
            .map(|reading| {
                let slot = reading.slot;
                reading.with_work(|shown| match shown {
                    WorkShown::Stranded => self.stranded_work(pool, slot).ok(),
                    WorkShown::Session => self.session_work(pool, slot).ok(),
                })
            })
            .collect()
    }

    fn slot_pool_kept(&self, pool: &SlotPool, full: bool) -> Vec<SlotReading> {
        if full {
            self.whole_pool_reads.fetch_add(1, Ordering::Relaxed);
        }
        self.slot_pool(pool)
    }

    fn change_slot_pool(
        &self,
        pool: &SlotPool,
        change: adapter_traits::SlotChange,
    ) -> Result<u32, adapter_traits::SlotRefused> {
        self.noted(self.slots.change(pool, change))
    }

    fn stranded_work(
        &self,
        pool: &SlotPool,
        slot: u32,
    ) -> Result<adapter_traits::StrandedWork, adapter_traits::RescueRefused> {
        self.slots.stranded_work(pool, slot)
    }

    fn stranded_diff(
        &self,
        pool: &SlotPool,
        slot: u32,
    ) -> Result<String, adapter_traits::RescueRefused> {
        self.slots.stranded_diff(pool, slot)
    }

    fn rescue_slot(
        &self,
        pool: &SlotPool,
        slot: u32,
        rescue: adapter_traits::SlotRescue,
    ) -> Result<adapter_traits::SlotRescued, adapter_traits::RescueRefused> {
        self.noted(self.slots.rescue(pool, slot, rescue))
    }

    fn changes(&self) -> u64 {
        self.changed.load(Ordering::Relaxed)
    }
}
