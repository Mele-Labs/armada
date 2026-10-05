//! The merge line Fleet runs for each repository it serves: Jobs pressed to
//! merge under `merge_by: push` wait in a durable line, and one lands at a
//! time. `docs/capabilities/merge-line.md`.
//!
//! **A restart loses nothing, because nothing the line needs is in memory.**
//! The queue, each entry's place, nonce and outcome, the turn and the size a
//! turn takes are rows in `store`; [`Lines`] holds only a guard against a
//! second turn in this process, and answers that cost one rerun to lose.
//!
//! **Three recoveries, none a special case.** A turn killed mid-gate leaves its
//! entry waiting and the turn held by a process that is gone, so the next
//! Fleet takes it over. A merge left half-made is cleared first. A push
//! already made reads as `AlreadyMerged`, so the entry lands naming the merge.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use adapter_traits::{
    AgentHarness, Delivery, Landing, Mergeable, NotMerged, PushedOntoBase, Vcs, WhatBecameOfIt,
    WorkProduct,
};
use core_model::{Actor, JobId, Level};
use store::{Blame, Ended, LineEntry, LineState, TurnHolder};

use crate::adrift::Adrift;
use crate::asking_the_base::BaseSays;
use crate::daemon::Fleet;

/// What one turn takes from the front of the line. Batching is a later slice.
const TAKEN_AT_ONCE: usize = 1;

/// The most a turn will take once batching lands, and what a repository's
/// first turn is recorded as taking.
const CEILING: u32 = 8;

/// What this process holds in memory about the lines, never written down.
#[derive(Clone, Default)]
pub(crate) struct Lines {
    turning: Arc<Mutex<BTreeSet<String>>>,
    /// Whose failure the turn in flight read a red Check as, until it takes it.
    pub(crate) blamed: Arc<Mutex<BTreeMap<JobId, Blame>>>,
    /// What the base said of a Check at a commit, by repository, commit and
    /// Check. A timeout is never in here.
    pub(crate) asked: Arc<Mutex<BTreeMap<(String, String, String), BaseSays>>>,
}

/// A repository's turn in this process, given back however the turn ends.
pub(crate) struct Turning {
    lines: Lines,
    repository: String,
}

impl Lines {
    pub(crate) fn take(&self, repository: &str) -> Option<Turning> {
        if !self.guard().insert(repository.to_string()) {
            return None;
        }
        Some(Turning {
            lines: self.clone(),
            repository: repository.to_string(),
        })
    }

    pub(crate) fn is_turning(&self, repository: &str) -> bool {
        self.guard().contains(repository)
    }

    fn guard(&self) -> std::sync::MutexGuard<'_, BTreeSet<String>> {
        self.turning
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Drop for Turning {
    fn drop(&mut self) {
        self.lines.guard().remove(&self.repository);
    }
}

/// How one entry's turn ended, for the size the next takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Came {
    Landed,
    Red,
    /// Refused or dropped for a reason that says nothing about the line's pace.
    Other,
    /// Not ended: it goes again.
    Waiting,
}

/// The size after a turn: halved after a red, doubled after a green, never
/// below one nor above [`CEILING`]. `None` where it did not move.
pub(crate) fn size_after(was: u32, came: Came) -> Option<(u32, &'static str)> {
    let moved = match came {
        Came::Red => ((was / 2).max(1), "halved after a red"),
        Came::Landed => (was.saturating_mul(2).min(CEILING), "doubled after a green"),
        Came::Other | Came::Waiting => return None,
    };
    (moved.0 != was).then_some(moved)
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
    /// Run one turn on a repository's line if it is free: finish what a dead
    /// turn left landed, take the turn, land the front of the line, give the
    /// turn up. `true` is a turn that ended an entry.
    ///
    /// **The turn is released only by finishing.** A future dropped mid-gate, or
    /// a process killed, leaves the row held, which the next caller reads as a
    /// turn to take over.
    pub(crate) async fn drive_the_line(&self, repository: &str) -> Result<bool, Adrift> {
        let Some(_guard) = self.lines().take(repository) else {
            return Ok(false);
        };
        let unfinished = self
            .store()
            .lock()
            .await
            .landings_not_finished(repository)
            .map_err(Adrift::Writing)?;
        for landing in &unfinished {
            self.finished_landing(landing).await;
        }
        if !self.has_the_turn(repository).await? {
            return Ok(false);
        }
        let (waiting, size) = {
            let store = self.store().lock().await;
            (
                store.waiting_in_line(repository).map_err(Adrift::Writing)?,
                store.line_size(repository).map_err(Adrift::Writing)?,
            )
        };
        let size = size.map_or(CEILING, |kept| kept.size);
        let running: Vec<i64> = waiting
            .iter()
            .take(TAKEN_AT_ONCE)
            .map(|one| one.id)
            .collect();
        self.store()
            .lock()
            .await
            .order_the_line(repository, &running, &[])
            .map_err(Adrift::Writing)?;
        let mut ended = false;
        let mut came = Came::Waiting;
        for entry in waiting.iter().take(TAKEN_AT_ONCE) {
            came = self.ran_the_turn_of(entry).await;
            ended |= came != Came::Waiting;
        }
        let at = self.now();
        let mut store = self.store().lock().await;
        if let Some((moved, why)) = size_after(size, came) {
            store
                .keep_line_size(repository, moved, why, &at)
                .map_err(Adrift::Writing)?;
        }
        store
            .release_turn(repository, self.run().as_str())
            .map_err(Adrift::Writing)?;
        // No turn is running now, so nothing is waiting on one.
        let behind: Vec<(i64, store::HeldBack)> = waiting
            .iter()
            .map(|one| (one.id, store::HeldBack::None))
            .collect();
        store
            .order_the_line(repository, &[], &behind)
            .map_err(Adrift::Writing)?;
        Ok(ended)
    }

    /// Hold the repository's turn: free, ours from a turn this process
    /// dropped, or taken from a holder whose process is gone. A holder still
    /// running keeps it.
    async fn has_the_turn(&self, repository: &str) -> Result<bool, Adrift> {
        let me = self.holder();
        let at = self.now();
        let mut store = self.store().lock().await;
        let Some(held) = store
            .hold_turn(repository, &me, &at)
            .map_err(Adrift::Writing)?
        else {
            return Ok(true);
        };
        // Our own run behind our own guard is a turn this process abandoned,
        // such as a press whose caller went away.
        if held.run != me.run && !gone(&held) {
            return Ok(false);
        }
        store
            .take_over_turn(repository, &held.run, &me, &at)
            .map_err(Adrift::Writing)
    }

    fn holder(&self) -> TurnHolder {
        let pid = std::process::id();
        let started = match crate::process::holder_of(pid) {
            Ok(crate::process::Holder::Held(at)) => at.as_str().to_string(),
            _ => String::new(),
        };
        TurnHolder {
            run: self.run().as_str().to_string(),
            pid,
            started,
        }
    }

    /// Land one entry, or say why not. **Written to the entry before anything
    /// is done with the answer**, so a Fleet killed after this still knows.
    async fn ran_the_turn_of(&self, entry: &LineEntry) -> Came {
        let job = match self.load(&entry.job_id).await {
            Ok(job) => job,
            Err(why) => return self.dropped(entry, &why.to_string()).await,
        };
        if let Err(why) = self.at_the_gate(&job) {
            return self.dropped(entry, &why.to_string()).await;
        }
        let Ok(served) = self.served_by(&job) else {
            return self
                .dropped(entry, "the repository is no longer served")
                .await;
        };
        self.settled_the_worktree(&job).await;
        match self
            .pushed_onto_the_base(&job, &served, number_of(&entry.pull_request))
            .await
        {
            Ok(pushed) => {
                let came = self.ended_landed(entry, &job, pushed).await;
                self.finished_landing(entry).await;
                came
            }
            Err(Adrift::NotMerged { why, .. }) => self.ended_refused(entry, &job, why).await,
            // Somebody else is running this Job's Checks: it goes again.
            Err(Adrift::ChecksRunningAgain { .. }) => Came::Waiting,
            Err(other) => self.dropped(entry, &other.to_string()).await,
        }
    }

    /// Clear a merge a killed turn left in the Job's worktree. Held, never
    /// raised: where it cannot, the gate refuses the worktree and says so.
    async fn settled_the_worktree(&self, job: &core_model::Job) {
        let Ok(worktree) = self.surviving_worktree(job) else {
            return;
        };
        let vcs = Arc::clone(self.vcs());
        let _ = tokio::task::spawn_blocking(move || vcs.settle_worktree(&worktree)).await;
    }

    async fn dropped(&self, entry: &LineEntry, said: &str) -> Came {
        let ended = Ended {
            state: LineState::Stopped,
            kind: None,
            said: Some(said.to_string()),
            merge_commit: None,
            base: None,
            blamed: None,
        };
        self.ended_with(entry, ended).await;
        Came::Other
    }

    async fn ended_landed(
        &self,
        entry: &LineEntry,
        job: &core_model::Job,
        pushed: PushedOntoBase,
    ) -> Came {
        let said = match pushed.merged {
            adapter_traits::Merged::Taken => "the merge commit was pushed onto the base",
            adapter_traits::Merged::AlreadyMerged => {
                "the base already held the branch, so the press moved the record rather than \
                 the base"
            }
        };
        self.said_about_the_merge(job, Level::Info, said, &entry.pull_request, None);
        let ended = Ended {
            state: LineState::Landed,
            kind: None,
            said: None,
            merge_commit: pushed.merge,
            base: Some(pushed.base),
            blamed: None,
        };
        self.ended_with(entry, ended).await;
        Came::Landed
    }

    async fn ended_refused(
        &self,
        entry: &LineEntry,
        job: &core_model::Job,
        why: NotMerged,
    ) -> Came {
        self.said_about_the_merge(
            job,
            Level::Warn,
            "the merge commit was not pushed onto the base, and the Job is where it was",
            &entry.pull_request,
            Some(&why),
        );
        let blamed = self
            .lines()
            .blamed
            .lock()
            .ok()
            .and_then(|mut by| by.remove(job.id()));
        let red = matches!(why, NotMerged::GateFailed { .. });
        let ended = Ended {
            state: LineState::Refused,
            kind: Some(why.kind().to_string()),
            said: Some(why.detail().to_string()),
            merge_commit: None,
            base: None,
            blamed,
        };
        self.ended_with(entry, ended).await;
        if red {
            Came::Red
        } else {
            Came::Other
        }
    }

    async fn ended_with(&self, entry: &LineEntry, ended: Ended) {
        let at = self.now();
        let _ = self
            .store()
            .lock()
            .await
            .end_line_entry(entry.id, &entry.nonce, &ended, &at);
    }

    /// Move the Job on after its landing: the pull request's record, then the
    /// approval, as a press that merged on the forge does. **Finished last**,
    /// so a landing a Fleet died before finishing is finished by the next, and
    /// held rather than raised, since the work is on the base either way.
    pub(crate) async fn finished_landing(&self, entry: &LineEntry) {
        // As the landing wrote it, not as the turn read it before.
        let Ok(Some(entry)) = self.store().lock().await.line_entry(entry.id) else {
            return;
        };
        let entry = &entry;
        let Ok(job) = self.load(&entry.job_id).await else {
            let _ = self.store().lock().await.finish_line_entry(entry.id);
            return;
        };
        let read = WhatBecameOfIt {
            landing: Landing::Merged {
                url: entry.pull_request.clone(),
            },
            base: entry.base.clone(),
            number: number_of(&entry.pull_request),
            title: None,
            mergeable: Mergeable::Unreadable,
            merged_at: Some(self.now().as_str().to_string()),
        };
        let _ = self.settled_landing(job.id(), read).await;
        // A Job a Fleet moved on before it died is left alone.
        if self.at_the_gate(&job).is_ok() {
            let by = Actor::from_wire(&entry.actor).unwrap_or(Actor::Human);
            if let Err(why) = self.approved(job.id(), by).await {
                self.noted_adrift(&why);
            }
        }
        let _ = self.store().lock().await.finish_line_entry(entry.id);
    }

    /// Drive every line that has work and no turn running here: what the sweep
    /// does, so entries a restart left are landed with nobody pressing.
    ///
    /// **Spawned, never awaited on the turn**, which is 250ms against a gate of
    /// minutes. `&Arc<Self>` for `probed`'s reason: only the loop holds one.
    pub(crate) async fn lines_driven(self: &Arc<Self>) {
        let Ok(roots) = self.store().lock().await.lines_with_work() else {
            return;
        };
        for root in roots {
            if self.lines().is_turning(&root) {
                continue;
            }
            let fleet = Arc::clone(self);
            tokio::spawn(async move { while let Ok(true) = fleet.drive_the_line(&root).await {} });
        }
    }
}

/// The pull request's number, off the end of its address.
fn number_of(url: &str) -> Option<u64> {
    url.rsplit('/').next().and_then(|tail| tail.parse().ok())
}

/// Whether the process that took a turn is gone: no process at its pid, or one
/// that started at another time, which is the pid recycled.
fn gone(held: &TurnHolder) -> bool {
    match crate::process::holder_of(held.pid) {
        Ok(crate::process::Holder::Vacant) => true,
        Ok(crate::process::Holder::Held(at)) => at.as_str() != held.started,
        // A probe that would not answer is not evidence the holder died.
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{size_after, Came, CEILING};

    #[test]
    fn a_red_halves_the_size_and_a_green_doubles_it_within_one_and_the_ceiling() {
        assert_eq!(size_after(8, Came::Red), Some((4, "halved after a red")));
        assert_eq!(size_after(1, Came::Red), None);
        assert_eq!(
            size_after(4, Came::Landed),
            Some((8, "doubled after a green"))
        );
        assert_eq!(size_after(CEILING, Came::Landed), None);
        assert_eq!(size_after(4, Came::Other), None);
    }
}
