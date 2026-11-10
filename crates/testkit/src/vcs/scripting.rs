//! How a test scripts the fake and reads back what it was asked: the builders, the
//! `now_*` setters that work through `&self` once a Fleet holds the fake, and the
//! counts over [`Delivered`].

use std::sync::atomic::Ordering;

use adapter_traits::{
    BaseFetched, BaseOnTheRemote, BroughtUpToDate, KeptCurrent, Landing, NotMerged,
    RepositoryStanding, Standing, Worktree,
};

use super::{Delivered, Delivering, FakeVcs, Merging};
use crate::work_product::Holding;

impl FakeVcs {
    pub fn new() -> FakeVcs {
        FakeVcs::default()
    }

    /// How many times a pool has been read — [`Vcs::slot_pool`].
    pub fn pool_reads(&self) -> u64 {
        self.pool_reads.load(Ordering::Relaxed)
    }

    /// How many of those were [`Vcs::slot_pool_kept`] asked for every slot.
    pub fn whole_pool_reads(&self) -> u64 {
        self.whole_pool_reads.load(Ordering::Relaxed)
    }

    /// `done`, after counting it for [`Vcs::changes`].
    pub(super) fn noted<T>(&self, done: T) -> T {
        self.changed.fetch_add(1, Ordering::Relaxed);
        done
    }

    /// Seed a branch somebody else already made, so a collision can be
    /// exercised without a repository.
    pub fn with_existing_branch(self, branch: impl Into<String>) -> FakeVcs {
        self.branches
            .lock()
            .expect("not poisoned")
            .insert(branch.into());
        self
    }

    /// Make the next creation fail as a machine would.
    ///
    /// The argument is what it stands in for, and it is a fixed string chosen
    /// at the call site rather than a message assembled from data — a fake's
    /// failure is a script, not a diagnosis.
    pub fn refuse_next(&self, standing_in_for: &'static str) {
        *self.refuse_next.lock().expect("not poisoned") = Some(standing_in_for);
    }

    /// Every worktree this fake said it made, in order. **Nothing removes an
    /// entry**, for the same reason nothing removes a worktree.
    pub fn created(&self) -> Vec<Worktree> {
        self.created.lock().expect("not poisoned").clone()
    }

    /// The branch each lease that took a slot was cut from, in order: the
    /// pool's base as the lease was asked. Spike 022, slice 4.
    pub fn cut_from(&self) -> Vec<String> {
        self.cut_from.lock().expect("not poisoned").clone()
    }

    /// Every repair branch this fake was asked to delete, in order.
    pub fn deleted_branches(&self) -> Vec<String> {
        self.deleted_branches.lock().expect("not poisoned").clone()
    }

    /// Put a ref at a commit, so a base can be resolved without a repository.
    ///
    /// Called a second time with a different id, this is a base branch that
    /// moved — which is the only way to reach the second base checkout, and so
    /// the only way to test that a moved base does not photograph the old one.
    pub fn with_ref_at(self, r#ref: impl Into<String>, commit: impl Into<String>) -> FakeVcs {
        self.refs
            .lock()
            .expect("not poisoned")
            .insert(r#ref.into(), commit.into());
        self
    }

    /// Script what the repository looks like from the delivery side.
    pub fn delivering(self, delivering: Delivering) -> FakeVcs {
        *self.delivery.lock().expect("not poisoned") = delivering;
        self
    }

    /// Say that somebody has merged, or closed, the pull request — **after the
    /// Job that opened it has finished**, which is the only order the real
    /// thing happens in.
    ///
    /// `&self` and not `self`, unlike every other setter here: the fake is
    /// inside a Fleet by the time a person could merge anything, so a
    /// consuming builder could not express the case at all.
    pub fn now_landed(&self, landed: Landing) {
        self.delivery.lock().expect("not poisoned").landed = landed;
    }

    /// Say the pull request merged, and when, as the forge would.
    ///
    /// `&self` for [`now_landed`](FakeVcs::now_landed)'s reason.
    pub fn now_merged_at(&self, landed: Landing, at: &str) {
        let mut delivery = self.delivery.lock().expect("not poisoned");
        delivery.landed = landed;
        delivery.merged_at = Some(at.to_string());
    }

    /// Say what the forge does when it is asked to merge.
    ///
    /// `&self` for [`now_landed`](FakeVcs::now_landed)'s reason: nothing merges
    /// a pull request until the Job that opened it is at its gate, by which
    /// time the fake is inside a Fleet.
    pub fn merging(&self, merging: Merging) {
        *self.merging.lock().expect("not poisoned") = merging;
    }

    /// How many times the forge has been asked to merge. **The write this fake
    /// exists to make visible**, so a test asserting that a refused press
    /// merged nothing is asserting on the whole run rather than on a status.
    pub fn times_asked_to_merge(&self) -> usize {
        self.counted(|it| matches!(it, Delivered::Merged { .. }))
    }

    /// How many times a merge commit was pushed onto the base. Apart from
    /// [`times_asked_to_merge`](FakeVcs::times_asked_to_merge), so a test can
    /// tell which road a Manifest sent the work down.
    pub fn times_pushed_onto_the_base(&self) -> usize {
        self.counted(|it| matches!(it, Delivered::MergedByPush { .. }))
    }

    /// Say what each push onto the base answers, in turn, before
    /// [`merging`](FakeVcs::merging) answers the rest.
    pub fn pushing_in_turn(&self, answers: Vec<Merging>) {
        *self.pushes_in_turn.lock().expect("not poisoned") = answers.into();
    }

    /// Say that merging a moved base into a branch refuses with `why`.
    pub fn merging_the_base_in_refuses(&self, why: NotMerged) {
        *self.merging_the_base_in.lock().expect("not poisoned") = Some(why);
    }

    /// Say what keeping the branch current will come to.
    ///
    /// `&self` for [`now_landed`](FakeVcs::now_landed)'s reason: nothing keeps
    /// a branch current until the Job that opened its pull request has
    /// finished, by which time the fake is inside a Fleet.
    pub fn now_kept_current(&self, outcome: KeptCurrent) {
        self.delivery.lock().expect("not poisoned").kept_current = outcome;
    }

    /// Say what a spawn's own catch-up — `standing` and `bring_up_to_date` —
    /// answers from here on. `&self`, for [`now_landed`](FakeVcs::now_landed)'s
    /// reason: a test carrying a Job past a resolved conflict needs the
    /// ordinary catch-up to answer differently once the Fleet holding this
    /// fake exists.
    pub fn now_behind(&self, standing: Standing, rebase: Option<BroughtUpToDate>) {
        let mut delivery = self.delivery.lock().expect("not poisoned");
        delivery.standing = standing;
        delivery.rebase = rebase;
    }

    /// Say what bringing the repository up to the merged branch comes to.
    pub fn repository_standing(&self, standing: RepositoryStanding) {
        self.delivery.lock().expect("not poisoned").repository = standing;
    }

    /// Say which commit the served repository has checked out. A catch-up that
    /// names a commit moves it there.
    pub fn checkout_on(&self, commit: &str) {
        self.delivery.lock().expect("not poisoned").checkout = Some(commit.to_string());
    }

    /// Say whether the local base branch fast-forwards while another is checked out.
    pub fn base_fast_forwards(&self, does: bool) {
        self.delivery.lock().expect("not poisoned").base_fast_forwards = does;
    }

    /// Say what asking why the served repository was held back comes to.
    pub fn held_back_by(&self, held: Option<adapter_traits::HeldBack>) {
        self.delivery.lock().expect("not poisoned").held_back = held;
    }

    /// Say what the one pull a held checkout is offered comes to.
    pub fn pulling_comes_to(&self, pulled: adapter_traits::CheckoutPulled) {
        self.delivery.lock().expect("not poisoned").pulled = pulled;
    }

    /// Say what reading a file's uncommitted change comes to.
    pub fn uncommitted_change_is(&self, change: Result<String, String>) {
        self.delivery.lock().expect("not poisoned").uncommitted_change = change;
    }

    /// The paths whose uncommitted change was read, in order.
    pub fn uncommitted_changes_read(&self) -> Vec<String> {
        self.delivered
            .lock()
            .expect("not poisoned")
            .iter()
            .filter_map(|it| match it {
                Delivered::ReadAnUncommittedChange { path, .. } => Some(path.clone()),
                _ => None,
            })
            .collect()
    }

    /// How many times the served checkout was asked to pull itself level.
    pub fn times_the_checkout_was_pulled(&self) -> usize {
        self.counted(|it| matches!(it, Delivered::PulledTheCheckout { .. }))
    }

    /// How many times a branch has been asked to be kept current against a
    /// base that moved. **The one call here a person watching the pull
    /// request's branch sees happen**, so a test that asserts once is
    /// asserting the whole of the rule.
    pub fn times_kept_current(&self) -> usize {
        self.counted(|it| matches!(it, Delivered::KeptCurrent { .. }))
    }

    /// Every branch the repository was asked to catch up to, in order.
    pub fn repository_caught_up_to(&self) -> Vec<String> {
        self.delivered
            .lock()
            .expect("not poisoned")
            .iter()
            .filter_map(|it| match it {
                Delivered::CaughtTheRepositoryUp { base } => Some(base.clone()),
                _ => None,
            })
            .collect()
    }

    pub(super) fn counted(&self, which: impl Fn(&Delivered) -> bool) -> usize {
        self.delivered
            .lock()
            .expect("not poisoned")
            .iter()
            .filter(|it| which(it))
            .count()
    }

    /// How many times the forge has been asked what became of a pull request.
    /// **The whole design of the asking is how rarely it happens**, so this is
    /// what a test about the rotation counts.
    pub fn times_asked_what_became_of_it(&self) -> usize {
        self.delivered
            .lock()
            .expect("not poisoned")
            .iter()
            .filter(|done| matches!(done, Delivered::AskedWhatBecameOfIt { .. }))
            .count()
    }

    /// A rebase that writes these files, into the worktree this
    /// [`Holding`](crate::Holding) is a handle on.
    ///
    /// **What a conflicted rebase does.** `git rebase --autostash` puts markers
    /// into the files it could not merge, and markers are content: a footprint
    /// taken after one reads differently from a footprint taken before, on the
    /// same paths. Nothing here says the rebase conflicted — the answer is
    /// [`Delivering::rebase`]'s to script, and this is what it left behind.
    ///
    /// The two fakes are otherwise independent, and this is the one place they
    /// are not. It is needed because a test cannot see *when* Fleet takes a
    /// step's baseline unless the rebase between the two readings moves
    /// something.
    /// Say that the base branch on this machine holds commits the remote's has
    /// not got, or the other way round.
    ///
    /// `Agreed` without this, because a base level with its remote is the
    /// ordinary case and a fake that defaulted to skew would put a caveat into
    /// every pull request every other test reads.
    pub fn base_apart_from_the_remote(self, remote: &str, ahead: usize, behind: usize) -> FakeVcs {
        *self.base_on_the_remote.lock().expect("not poisoned") = Some(BaseOnTheRemote::Apart {
            remote: remote.to_string(),
            ahead,
            behind,
        });
        self
    }

    /// Say that the remote cannot be fetched from: `said` is git's sentence.
    /// `&self`, for [`now_landed`](FakeVcs::now_landed)'s reason.
    pub fn now_unable_to_fetch(&self, remote: &str, said: &str) {
        *self.base_fetch.lock().expect("not poisoned") = Some(BaseFetched::Failed {
            remote: remote.to_string(),
            said: said.to_string(),
        });
    }

    pub fn writing_into(self, holding: Holding, files: &[&str]) -> FakeVcs {
        *self.rebase_writes.lock().expect("not poisoned") =
            Some((holding, files.iter().map(|path| path.to_string()).collect()));
        self
    }

    /// Everything this fake was asked to do to a branch, in order. **Nothing
    /// removes an entry**, so a test asserting that no push happened is
    /// asserting on the whole run rather than on the last call.
    ///
    /// **Without the asks for laps** either, for the same reason as the fetches.
    ///
    /// **Without the fetches**, which come before every catch-up and would put
    /// a call in front of each one a test is about. `delivered_with_fetches`
    /// is the order they came in.
    pub fn delivered(&self) -> Vec<Delivered> {
        self.delivered_with_fetches()
            .into_iter()
            .filter(|it| !matches!(it, Delivered::FetchedBase { .. } | Delivered::AskedForLaps { .. }))
            .collect()
    }

    /// Say what the forge holds of the laps a commit's CI run recorded, from here on. `&self`, for
    /// [`now_landed`](FakeVcs::now_landed)'s reason.
    pub fn laps_on_the_forge(&self, held: Result<adapter_traits::LapsOnForge, adapter_traits::NotAsked>) {
        self.delivery.lock().expect("not poisoned").recorded_laps = held;
    }

    /// Every commit the forge was asked for laps of, in order. Kept out of
    /// [`delivered`](FakeVcs::delivered), which a poll that asks every tick would fill.
    pub fn asked_for_laps_of(&self) -> Vec<String> {
        self.delivered_with_fetches()
            .into_iter()
            .filter_map(|it| match it {
                Delivered::AskedForLaps { commit } => Some(commit),
                _ => None,
            })
            .collect()
    }

    /// [`delivered`](FakeVcs::delivered), and each fetch of a base in the
    /// place it was asked.
    pub fn delivered_with_fetches(&self) -> Vec<Delivered> {
        self.delivered.lock().expect("not poisoned").clone()
    }
}
