//! `GET /worktrees` answered from memory.
//!
//! The list is git over every checkout and slot, about a hundred subprocesses
//! and 2 to 3 seconds at load 40. So the last answer is kept and served, and one
//! task behind it reads again: when `Vcs::changes` or a Job's move says
//! something changed, and every [`STALE_AFTER`] while anyone asked in the last
//! [`WATCHED_FOR`]. Nobody asking reads nothing.
//!
//! **A read that something changed asks git about the slots that changed.** The
//! adapter hears the file system (`docs/spikes/032-what-moves-when-a-slot-changes.md`) and keeps the
//! rest of the pool as it read it. **Only the read every [`STALE_AFTER`] asks
//! about every slot**, which is what a change nobody heard of waits for: the
//! list is never wrong for longer than that. It reads HEAD and the refs of every
//! slot and `git status` only of a free slot whose watch cannot vouch for the
//! last answer: `adapters::leasing::watching` says when.
//!
//! **Only the list served to Bridge is kept.** The sweep, `armada clean`, a
//! release and a rescue decide from a slot, and read git themselves.
//! **Without a keeper every request is live**, which is what a test gets that
//! does not start one.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::WorktreesHeld;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tokio::time::Instant;

use crate::daemon::Fleet;
use crate::leasing::Reading;
use crate::wire::{worktree_held, worktree_slot};

/// A snapshot this old is read again, every slot, behind the request that finds
/// it and on the timer while anyone is asking. The owner's bound on how stale
/// the slot list may be (15 Oct 2026): a change no event announced is shown by
/// the next one.
pub const STALE_AFTER: Duration = Duration::from_secs(10);

/// How long after the last request the keeper keeps reading. Past it nothing has
/// kept the snapshot true, so a request is answered live.
pub const WATCHED_FOR: Duration = Duration::from_secs(120);

/// How often the keeper looks at whether anything changed. An atomic read.
const LOOK_EVERY: Duration = Duration::from_millis(250);

/// The least between a read ending and the next starting, so a Fleet that
/// changes something every moment is not reading every moment.
const MIN_GAP: Duration = Duration::from_secs(1);

/// The snapshot and what decides whether to take another.
#[derive(Default)]
pub(crate) struct Kept {
    inner: Mutex<Inner>,
    /// A request found the snapshot behind. Wakes a keeper asleep because
    /// nobody had asked.
    wake: Notify,
    /// One read at a time, whoever asks for it.
    reading: tokio::sync::Mutex<()>,
}

#[derive(Default)]
struct Inner {
    /// A keeper is running; without one nothing is served from here.
    keeper: bool,
    held: Option<Arc<WorktreesHeld>>,
    /// When the read behind `held` began, and when the last read ended.
    begun: Option<Instant>,
    /// When the last read of every slot began. A read of the slots that
    /// changed does not move it, so the timer is not pushed back by events.
    whole: Option<Instant>,
    ended: Option<Instant>,
    asked: Option<Instant>,
    /// Counted by [`Kept::changed`], and what it stood at when `held` began.
    changed: u64,
    changed_at_begin: u64,
    /// `Vcs::changes` when `held` began.
    adapter_at_begin: u64,
}

/// What a request is given.
pub(crate) enum Answer {
    /// The snapshot, which the keeper reads again if it is behind.
    Kept(Arc<WorktreesHeld>),
    /// Nothing true is kept.
    Read,
}

impl Kept {
    pub(crate) fn keeper_started(&self) {
        self.inner.lock().expect("not poisoned").keeper = true;
    }

    /// Something a snapshot shows changed. A counter, so anything that moves a
    /// slot or a Job may call it.
    pub(crate) fn changed(&self) {
        self.inner.lock().expect("not poisoned").changed += 1;
    }

    /// What a request is answered with, noting that someone asked.
    fn answer(&self, now: Instant, adapter: u64) -> Answer {
        let mut inner = self.inner.lock().expect("not poisoned");
        let was_watched = inner.watched(now);
        inner.asked = Some(now);
        // A keeper asleep for want of anyone asking is woken by the first who
        // does, and by anyone who finds the snapshot out of date.
        if !was_watched || inner.out_of_date(now, adapter) {
            self.wake.notify_one();
        }
        let Some(held) = inner.held.clone().filter(|_| inner.keeper) else {
            return Answer::Read;
        };
        if inner.begun.is_none_or(|begun| now.saturating_duration_since(begun) > WATCHED_FOR) {
            return Answer::Read;
        }
        Answer::Kept(held)
    }

    /// Whether a snapshot is being served, so an act that changed what it shows
    /// has something to correct.
    fn serving(&self) -> bool {
        let inner = self.inner.lock().expect("not poisoned");
        inner.keeper && inner.held.is_some()
    }

    #[cfg(test)]
    pub(crate) fn is_kept(&self) -> bool {
        self.inner.lock().expect("not poisoned").held.is_some()
    }

    /// Whether the snapshot satisfies `test`.
    #[cfg(test)]
    pub(crate) fn serves(&self, test: impl FnOnce(&WorktreesHeld) -> bool) -> bool {
        let inner = self.inner.lock().expect("not poisoned");
        inner.held.as_deref().is_some_and(test)
    }

    /// Make everything in it `by` older, for a test that cannot wait that long.
    #[cfg(test)]
    pub(crate) fn aged(&self, by: Duration) {
        let mut guard = self.inner.lock().expect("not poisoned");
        let inner = &mut *guard;
        for at in [&mut inner.begun, &mut inner.whole, &mut inner.ended, &mut inner.asked] {
            *at = at.and_then(|at| at.checked_sub(by));
        }
    }

    /// Whether the keeper should read now.
    fn due(&self, now: Instant, adapter: u64) -> bool {
        let inner = self.inner.lock().expect("not poisoned");
        match inner.held {
            // The first read, at boot; a failed one waits before another.
            None => inner
                .ended
                .is_none_or(|ended| now.saturating_duration_since(ended) >= STALE_AFTER),
            Some(_) => inner.watched(now) && inner.behind(now, adapter),
        }
    }

    fn watched(&self, now: Instant) -> bool {
        self.inner.lock().expect("not poisoned").watched(now)
    }

    /// Taken before a read, so a change during it leaves the snapshot behind
    /// rather than ahead.
    fn marks(&self, adapter: u64) -> (u64, u64) {
        (self.inner.lock().expect("not poisoned").changed, adapter)
    }

    /// Whether a read beginning at `now` must ask about every slot.
    fn owes_whole(&self, now: Instant) -> bool {
        self.inner.lock().expect("not poisoned").whole_is_old(now)
    }

    fn kept(&self, held: Arc<WorktreesHeld>, begun: Instant, whole: bool, marks: (u64, u64)) {
        let mut inner = self.inner.lock().expect("not poisoned");
        inner.held = Some(held);
        inner.begun = Some(begun);
        if whole {
            inner.whole = Some(begun);
        }
        inner.ended = Some(Instant::now());
        (inner.changed_at_begin, inner.adapter_at_begin) = marks;
    }

    fn failed(&self) {
        self.inner.lock().expect("not poisoned").ended = Some(Instant::now());
    }

    /// The snapshot, where a read began at or after `since` took it.
    fn since(&self, since: Instant) -> Option<Arc<WorktreesHeld>> {
        let inner = self.inner.lock().expect("not poisoned");
        let began_in_time = inner.begun.is_some_and(|begun| begun >= since);
        inner.held.clone().filter(|_| inner.keeper && began_in_time)
    }
}

impl Inner {
    fn watched(&self, now: Instant) -> bool {
        self.keeper
            && self
                .asked
                .is_some_and(|asked| now.saturating_duration_since(asked) <= WATCHED_FOR)
    }

    /// Changed since it was read, or old enough to be read again.
    fn out_of_date(&self, now: Instant, adapter: u64) -> bool {
        let moved = self.changed != self.changed_at_begin || adapter != self.adapter_at_begin;
        moved || self.whole_is_old(now)
    }

    fn whole_is_old(&self, now: Instant) -> bool {
        self.whole
            .is_none_or(|whole| now.saturating_duration_since(whole) >= STALE_AFTER)
    }

    /// Out of date, and the last read ended long enough ago to start another.
    fn behind(&self, now: Instant, adapter: u64) -> bool {
        let spaced = self
            .ended
            .is_none_or(|ended| now.saturating_duration_since(ended) >= MIN_GAP);
        spaced && self.out_of_date(now, adapter)
    }
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
    /// The whole list, unscoped: from memory where a keeper holds a true one,
    /// otherwise read now.
    pub(crate) async fn worktrees_answer(&self) -> Result<WorktreesHeld, Refusal> {
        let asked = Instant::now();
        match self.worktrees_memory().answer(asked, self.vcs().changes()) {
            Answer::Kept(held) => Ok((*held).clone()),
            // A request that finds a read under way takes its result, if it
            // began within `STALE_AFTER` of the ask.
            Answer::Read => {
                let from = asked.checked_sub(STALE_AFTER).unwrap_or(asked);
                Ok((*self.read_worktrees(from).await?).clone())
            }
        }
    }

    /// Read the list and keep it, unless a read that began at or after `since`
    /// finished while this one waited its turn.
    pub(crate) async fn read_worktrees(
        &self,
        since: Instant,
    ) -> Result<Arc<WorktreesHeld>, Refusal> {
        let _one_at_a_time = self.worktrees_memory().reading.lock().await;
        if let Some(held) = self.worktrees_memory().since(since) {
            return Ok(held);
        }
        let begun = Instant::now();
        let whole = self.worktrees_memory().owes_whole(begun);
        let marks = self.worktrees_memory().marks(self.vcs().changes());
        match self.worktrees_read(Reading::Kept { whole }).await {
            Ok(live) => {
                let held = Arc::new(live);
                self.worktrees_memory().kept(Arc::clone(&held), begun, whole, marks);
                Ok(held)
            }
            Err(why) => {
                self.worktrees_memory().failed();
                Err(why)
            }
        }
    }

    /// After an act on a slot or a checkout. Reads again before returning where
    /// a keeper is serving, so the surface that pressed the act and reads the
    /// list is not shown the list from before it.
    pub(crate) async fn worktrees_changed(&self) {
        // A rescue or a reclaim gives a slot back without passing the Job's ledger.
        self.hosts().slot_freed();
        self.worktrees_memory().changed();
        if self.worktrees_memory().serving() {
            // A read that fails leaves the snapshot behind, and the keeper tries again.
            let _ = self.read_worktrees(Instant::now()).await;
        }
    }

    /// The list read from git and the store now.
    ///
    /// **`Fleet::held_among` is the derivation and this only redacts it**: the
    /// five tests are written once in `crate::holding`, and a filter here that
    /// decided for itself what is safe would be a third opinion. The one thing
    /// dropped is a piloted Job's checkout, through `Holding::offerable` (#367).
    #[cfg(test)]
    pub(crate) async fn worktrees_live(&self) -> Result<WorktreesHeld, Refusal> {
        self.worktrees_read(Reading::Live).await
    }

    async fn worktrees_read(&self, how: Reading) -> Result<WorktreesHeld, Refusal> {
        // One read of the Jobs for both halves, which then run side by side.
        let (loaded, _) = self.every_job().await.map_err(|why| self.refusal(why))?;
        let jobs = Arc::new(loaded.jobs);
        let (holding, slots) = tokio::join!(
            Fleet::held_among(self, Arc::clone(&jobs)),
            Fleet::pool_slots_among(self, &jobs, how),
        );
        Ok(WorktreesHeld {
            worktrees: holding
                .iter()
                .filter(|one| one.offerable())
                .map(worktree_held)
                .collect(),
            slots: slots.iter().map(worktree_slot).collect(),
        })
    }
}

/// Keep `GET /worktrees` true: read once now, so the first request is not the
/// slow one, then whenever the snapshot is behind and someone is asking.
pub fn keep_worktrees_warm<H, V, W>(fleet: Arc<Fleet<H, V, W>>) -> JoinHandle<()>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    fleet.worktrees_memory().keeper_started();
    tokio::spawn(async move {
        loop {
            let now = Instant::now();
            if fleet.worktrees_memory().due(now, fleet.vcs().changes()) {
                // Whatever it came to, `due` decides whether to try again.
                if let Ok(held) = fleet.read_worktrees(now).await {
                    fleet.strandings_attended(&held).await;
                }
            }
            if fleet.worktrees_memory().watched(Instant::now()) {
                tokio::select! {
                    _ = tokio::time::sleep(LOOK_EVERY) => {}
                    _ = fleet.worktrees_memory().wake.notified() => {}
                }
            } else {
                fleet.worktrees_memory().wake.notified().await;
            }
        }
    })
}
