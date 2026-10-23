//! A `Vcs` that touches no disk.
//!
//! # Why this exists rather than a temporary repository per test
//!
//! Because a suite that shells out to git for every case is a suite people stop
//! running, and because the acceptance test is hermetic by definition — no
//! process spawned, no repository touched, no network opened. Fleet's own tests
//! ask *did a worktree get created before the Drone, and what did Fleet do when
//! one could not be*; neither question needs git to answer it.
//!
//! The cases that genuinely need git's opinion — an existing branch, an
//! administrative record that outlives its directory — are tested against a
//! real repository in `adapters`, where they are five tests rather than every
//! test.
//!
//! # What it is faithful about
//!
//! Every refusal a caller has to handle, and the derivation. Paths and
//! branch names come from the same [`WorktreeSpec`] the real implementation
//! uses, so a test asserting on a path is asserting on the derivation that
//! ships rather than on a second copy of it — the second-vocabulary defect.
//!
//! It is **not** faithful about the filesystem: nothing is created but a leased
//! slot's empty directory, so a test that wants to read a file out of a
//! worktree wants the real one.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use adapter_traits::{
    Base, BaseCheckout, BaseFetched, BaseMergedIn, BaseOnTheRemote, BaseSpec, BranchMerged, BroughtUpToDate,
    Change, CommitTime, Committed, Delivery, KeptCurrent, Landable,
    Merged, NotCloned, NotDelivered, NotMerged, Pushed, PushedOntoBase,
    RepositoryStanding, SlotKept, SlotLeased, SlotPool, SlotReading, SlotStanding, WorkShown,
    Standing, UncheckedHead, Vcs, WhatBecameOfIt, Worktree, WorktreeSpec,
};

use crate::work_product::Holding;

mod checkout_sends;
mod commit;
mod delivered;
mod delivering;
mod failed;
mod forge;
mod holding;
mod main_ci;
mod manifest_edits;
mod merging;
mod pull_requests;
mod scripting;
mod slots;
mod workspace;

use commit::Willing;
pub use commit::{CommitScope, FakeCommit};
pub use delivering::Delivering;
pub use delivered::Delivered;
pub use failed::FakeVcsError;
pub use holding::Hold;
use main_ci::MainCiScript;
pub use merging::Merging;
use merging::Trees;
use slots::FakeSlots;

/// Version control that remembers what it was asked for and creates nothing.
///
/// **`Mutex` rather than `RefCell`, and that is not a style choice.** A Fleet
/// implements `api::Daemon`, which is `Send + Sync`, so every seam it holds has
/// to be `Sync` too — and a fake that is not cannot stand in for the real
/// adapter at the one boundary the acceptance test drives. The three fakes in
/// this crate each carry the same note.
#[derive(Debug, Default)]
pub struct FakeVcs {
    branches: Mutex<BTreeSet<String>>,
    /// The worktree pool each repository leases a Job's slot from.
    slots: FakeSlots,
    created: Mutex<Vec<Worktree>>,
    refuse_next: Mutex<Option<&'static str>>,
    committed: Mutex<Vec<FakeCommit>>,
    /// What the next commit answers. Both halves are scripted rather than
    /// inferred: a fake that guessed whether a worktree it never created has
    /// anything in it would be asserting against its own guess.
    commits: Mutex<Willing>,
    /// Commits already on the branch that the base has not got. Zero unless a
    /// test says otherwise — see [`FakeVcs::with_commits_on_the_branch`].
    ahead: Mutex<usize>,
    /// What delivery answers. Scripted for the same reason: there is no
    /// repository here to be behind anything, and no remote to push to.
    delivery: Mutex<Delivering>,
    /// What the forge says about the base branch's newest commit.
    pub main_ci: MainCiScript,
    delivered: Mutex<Vec<Delivered>>,
    /// What the rebase leaves in the worktree, and where. Absent for every case
    /// that does not care — see [`FakeVcs::writing_into`].
    rebase_writes: Mutex<Option<(Holding, Vec<String>)>>,
    /// Whether the base branch and the remote's agree. **Its own field rather
    /// than a sixth on [`Delivering`]**, which is that type's own rule: every
    /// field there is written out at every call site, and a repository whose
    /// base is level with its remote is what all but one of them mean. See
    /// [`FakeVcs::base_apart_from_the_remote`]. Absent is `Agreed`, which is
    /// what keeps this fake's `Default` and the domain type's lack of one both
    /// intact.
    base_on_the_remote: Mutex<Option<BaseOnTheRemote>>,
    /// What fetching the base from the remote came to. **Its own field**, for
    /// [`base_on_the_remote`](FakeVcs::base_on_the_remote)'s reason. Absent is
    /// `Local`: a repository with no remote, which is what every test that does
    /// not care about the fetch means.
    base_fetch: Mutex<Option<BaseFetched>>,
    /// What the forge answers a merge with. **Its own field rather than a tenth
    /// on [`Delivering`]**, for [`base_on_the_remote`](FakeVcs::base_on_the_remote)'s
    /// reason and with a second: a merge only ever happens after the Fleet
    /// holding this fake exists, so it is scripted through `&self` and a
    /// consuming builder could not express the case at all.
    merging: Mutex<Merging>,
    /// What a form's edit to the Manifest comes to.
    manifest_edits: Mutex<manifest_edits::ManifestEdits>,
    /// What sending the served checkout's uncommitted files as a pull request comes to.
    checkout_sends: Mutex<checkout_sends::CheckoutSends>,
    /// What the forge says about one pull request and does when it is readied.
    pull_requests: Mutex<pull_requests::PullRequests>,
    /// Answers `merge_by_push` gives before `merging` does, first first — a
    /// base that moves once and then holds still is two answers.
    pushes_in_turn: Mutex<VecDeque<Merging>>,
    /// What merging a moved base into a branch refuses with, or `None` where it
    /// merges clean.
    merging_the_base_in: Mutex<Option<NotMerged>>,
    /// The tree the Job's branch carries. Every merge this fake makes into it
    /// moves it, so `merge_by_push` can refuse a head no Check read.
    trees: Mutex<Trees>,
    /// What commit each ref is at. **Scripted**, for
    /// [`commits`](FakeVcs::commits)' reason: there is no repository here to
    /// have a history, and a fake that invented one id per name would make
    /// *the base moved* untestable, which is the case the base checkout exists
    /// to get right.
    refs: Mutex<BTreeMap<String, String>>,
    /// Every base checkout this fake has been asked for and not dropped, keyed
    /// by commit, with whether it has been marked prepared.
    ///
    /// **A map rather than a list**, so asking twice for one commit answers the
    /// same checkout — which is the property `Vcs::base_checkout` promises and
    /// the one a caller relying on the sharing has to be able to assert.
    bases: Mutex<BTreeMap<String, bool>>,
    /// Every base checkout this fake has been asked to drop, in order.
    dropped_bases: Mutex<Vec<String>>,
    /// The branch each lease that took a slot was cut from, in order.
    cut_from: Mutex<Vec<String>>,
    /// Every repair branch this fake was asked to delete, in order.
    deleted_branches: Mutex<Vec<String>>,
    /// What leasing a bay to read a pull request in answers: the head the checkout is at, or the
    /// sentence the forge refused with. Absent is a head of forty `a`s.
    review_head: Mutex<Option<Result<String, String>>>,
    /// Every bay leased to read a pull request, as `(pull request, holder)`, and every one given
    /// back, as `(slot, holder)`.
    review_leases: Mutex<Vec<(u64, String)>>,
    review_releases: Mutex<Vec<(u32, String)>>,
    /// What [`Vcs::changes`] counts: every act on a slot or a checkout.
    changed: AtomicU64,
    /// How many times a pool was read. A snapshot that answers from memory
    /// leaves this where it was.
    pool_reads: AtomicU64,
    whole_pool_reads: AtomicU64,
    /// Held shut by [`FakeVcs::hold_pushes`]: a push waits on it.
    pushes_held: Mutex<Option<Arc<Hold>>>,
}

impl Delivery for FakeVcs {
    fn base(
        &self,
        _worktree: &Worktree,
        declared: Option<&str>,
    ) -> Result<Option<Base>, NotDelivered> {
        // Declared beats scripted, so a test can assert the key is honoured
        // without describing a repository at all.
        if let Some(declared) = declared {
            return Ok(Some(Base::Declared(declared.to_string())));
        }
        Ok(self.delivery.lock().expect("not poisoned").base.clone())
    }

    fn standing(&self, _worktree: &Worktree, _base: &Base) -> Result<Standing, NotDelivered> {
        Ok(self.delivery.lock().expect("not poisoned").standing)
    }

    fn ahead(&self, _worktree: &Worktree, _base: &Base) -> Result<usize, NotDelivered> {
        Ok(*self.ahead.lock().expect("not poisoned"))
    }

    fn fetch_base(&self, _in_repo: &str, base: &str) -> BaseFetched {
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::FetchedBase {
                base: base.to_string(),
            });
        self.base_fetch
            .lock()
            .expect("not poisoned")
            .clone()
            .unwrap_or(BaseFetched::Local)
    }

    fn base_on_the_remote(
        &self,
        _worktree: &Worktree,
        _base: &Base,
    ) -> Result<BaseOnTheRemote, NotDelivered> {
        Ok(self
            .base_on_the_remote
            .lock()
            .expect("not poisoned")
            .clone()
            .unwrap_or(BaseOnTheRemote::Agreed))
    }

    fn bring_up_to_date(
        &self,
        worktree: &Worktree,
        base: &Base,
    ) -> Result<BroughtUpToDate, NotDelivered> {
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::BroughtUpToDate {
                branch: worktree.branch().to_string(),
                base: base.name().to_string(),
            });
        // Before the answer is returned, because that is when git does it: the
        // worktree has already been written to by the time the caller reads
        // what the rebase came to.
        if let Some((holding, files)) = self.rebase_writes.lock().expect("not poisoned").as_ref() {
            holding.wrote(
                &files
                    .iter()
                    .map(|path| (path.as_str(), Change::Modified))
                    .collect::<Vec<(&str, Change)>>(),
            );
        }
        let brought = self
            .delivery
            .lock()
            .expect("not poisoned")
            .rebase
            .clone()
            .unwrap_or(BroughtUpToDate::Clean {
                base: base.name().to_string(),
                commits: 0,
            });
        if matches!(brought, BroughtUpToDate::Clean { commits, .. } if commits > 0) {
            self.trees.lock().expect("not poisoned").merged_into();
        }
        Ok(brought)
    }

    fn merge_branch(
        &self,
        worktree: &Worktree,
        branch: &str,
    ) -> Result<BranchMerged, NotDelivered> {
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::MergedBranch {
                branch: branch.to_string(),
                into: worktree.branch().to_string(),
            });
        Ok(self
            .delivery
            .lock()
            .expect("not poisoned")
            .branch_merge
            .clone())
    }

    fn push(&self, worktree: &Worktree) -> Result<Pushed, NotDelivered> {
        let held = self.pushes_held.lock().expect("not poisoned").clone();
        if let Some(held) = held {
            held.pass();
        }
        let pushed = self.delivery.lock().expect("not poisoned").push.clone();
        if pushed != Pushed::NoRemote {
            self.delivered
                .lock()
                .expect("not poisoned")
                .push(Delivered::Pushed {
                    branch: worktree.branch().to_string(),
                });
        }
        Ok(pushed)
    }

    fn push_forcing(&self, worktree: &Worktree) -> Result<Pushed, NotDelivered> {
        let pushed = self.delivery.lock().expect("not poisoned").push.clone();
        if pushed != Pushed::NoRemote {
            self.delivered
                .lock()
                .expect("not poisoned")
                .push(Delivered::PushedForcing {
                    branch: worktree.branch().to_string(),
                });
        }
        Ok(pushed)
    }

    fn landed(&self, _in_repo: &str, pull_request: &str) -> WhatBecameOfIt {
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::AskedWhatBecameOfIt {
                pull_request: pull_request.to_string(),
            });
        self.what_became_of_it()
    }

    fn merge_by_push(
        &self,
        _in_repo: &str,
        handle: &str,
        declared: Option<&str>,
        pull_request: Option<u64>,
        landable: Landable<'_>,
    ) -> Result<PushedOntoBase, NotMerged> {
        // Scripted the same way as `merge`, through the one `merging` field:
        // this fake has no remote base to merge onto.
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::MergedByPush {
                handle: handle.to_string(),
                pull_request,
            });
        let base = match (declared, &self.delivery.lock().expect("not poisoned").base) {
            (Some(declared), _) => declared.to_string(),
            (None, Some(base)) => base.name().to_string(),
            (None, None) => String::from("main"),
        };
        let next = self
            .pushes_in_turn
            .lock()
            .expect("not poisoned")
            .pop_front();
        let scripted = next.is_some();
        let answer = next.unwrap_or_else(|| self.merging.lock().expect("not poisoned").clone());
        let merged = match answer {
            Merging::Takes => Merged::Taken,
            Merging::AlreadyMerged => Merged::AlreadyMerged,
            Merging::Refuses(why) => return Err(why),
        };
        // Asked where the real one asks it: once the branch holds the base, and
        // before the push. A refusal leaves the scripted answer for the next.
        let tree = self.trees.lock().expect("not poisoned").now();
        let read = match landable {
            Landable::Checked(checked) => checked == tree,
            Landable::Unchecked => false,
            Landable::NothingToCheck => true,
        };
        if merged == Merged::Taken && !read {
            if scripted {
                self.pushes_in_turn
                    .lock()
                    .expect("not poisoned")
                    .push_front(Merging::Takes);
            }
            return Err(NotMerged::Unchecked {
                said: format!("the branch carries {tree}, which no run of its Checks passed on"),
            });
        }
        Ok(PushedOntoBase {
            base,
            merged,
            merge: Some(String::from("3e9a7c1000000000000000000000000000000000")),
        })
    }

    fn tree_as_it_stands(&self, _worktree: &Worktree) -> Result<String, NotDelivered> {
        Ok(self.trees.lock().expect("not poisoned").now())
    }

    fn the_unchecked_head(
        &self,
        _in_repo: &str,
        worktree: &Worktree,
        _declared: Option<&str>,
        _checked: Option<&str>,
    ) -> Result<UncheckedHead, NotMerged> {
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::ReadTheUncheckedHead {
                branch: worktree.branch().to_string(),
            });
        Ok(UncheckedHead {
            head: String::from("c0ffee0000000000000000000000000000000000"),
            tree: self.trees.lock().expect("not poisoned").now(),
            touched: vec![String::from("src/log.rs")],
        })
    }

    fn merge_the_moved_base_in(
        &self,
        _in_repo: &str,
        worktree: &Worktree,
        _declared: Option<&str>,
    ) -> Result<BaseMergedIn, NotMerged> {
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::MergedTheBaseIn {
                branch: worktree.branch().to_string(),
            });
        if let Some(why) = self
            .merging_the_base_in
            .lock()
            .expect("not poisoned")
            .clone()
        {
            return Err(why);
        }
        let mut trees = self.trees.lock().expect("not poisoned");
        trees.merged_into();
        Ok(BaseMergedIn {
            base: String::from("main"),
            onto: String::from("5b4ec82700000000000000000000000000000000"),
            head: String::from("e1f2a3b400000000000000000000000000000000"),
            tree: trees.now(),
            was: String::from("a1b2c3d400000000000000000000000000000000"),
            touched: vec![String::from("src/log.rs")],
        })
    }

    fn put_back(&self, worktree: &Worktree, _merged: &BaseMergedIn) -> Result<(), NotDelivered> {
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::PutBack {
                branch: worktree.branch().to_string(),
            });
        let mut trees = self.trees.lock().expect("not poisoned");
        trees.now = trees.was;
        Ok(())
    }

    fn settle_worktree(&self, worktree: &Worktree) -> Result<(), NotDelivered> {
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::SettledWorktree {
                branch: worktree.branch().to_string(),
            });
        Ok(())
    }

    fn base_tip(&self, _in_repo: &str, base: &str) -> Option<String> {
        self.refs.lock().expect("not poisoned").get(base).cloned()
    }

    fn kept_current(&self, spec: &WorktreeSpec, base: &str) -> KeptCurrent {
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::KeptCurrent {
                worktree: spec.worktree_path(),
                base: base.to_string(),
            });
        let kept = self
            .delivery
            .lock()
            .expect("not poisoned")
            .kept_current
            .clone();
        if matches!(kept, KeptCurrent::Rebased { .. }) {
            self.trees.lock().expect("not poisoned").merged_into();
        }
        kept
    }

    fn caught_the_repository_up(&self, _in_repo: &str, base: &str) -> RepositoryStanding {
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::CaughtTheRepositoryUp {
                base: base.to_string(),
            });
        let mut delivery = self.delivery.lock().expect("not poisoned");
        if let Some(head) = delivery.repository.caught_up_to() {
            if delivery.checkout.is_some() {
                delivery.checkout = Some(head.to_string());
            }
        }
        delivery.repository.clone()
    }

    fn checkout_has(&self, _in_repo: &str, _base: &str, commit: &str) -> bool {
        let delivery = self.delivery.lock().expect("not poisoned");
        delivery.checkout.as_deref().is_none_or(|on| on == commit)
    }

    fn base_fast_forwarded(&self, _in_repo: &str, _base: &str) -> bool {
        self.delivery
            .lock()
            .expect("not poisoned")
            .base_fast_forwards
    }

    fn held_back(&self, _in_repo: &str, _base: &str) -> Option<adapter_traits::HeldBack> {
        self.delivery
            .lock()
            .expect("not poisoned")
            .held_back
            .clone()
    }

    fn pulled_the_checkout(&self, in_repo: &str) -> adapter_traits::CheckoutPulled {
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::PulledTheCheckout {
                in_repo: in_repo.to_string(),
            });
        let mut delivery = self.delivery.lock().expect("not poisoned");
        // A pull that took leaves the checkout on the forge's tip, which is what
        // the next reading of main finds.
        if delivery.pulled.code == 0 && delivery.checkout.is_some() {
            delivery.checkout = self.main_ci.head();
        }
        delivery.pulled.clone()
    }

    fn uncommitted_change_of(&self, in_repo: &str, path: &str) -> Result<String, String> {
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::ReadAnUncommittedChange {
                in_repo: in_repo.to_string(),
                path: path.to_string(),
            });
        self.delivery
            .lock()
            .expect("not poisoned")
            .uncommitted_change
            .clone()
    }

    fn manifest_by_pull_request(&self, _in_repo: &str, _file: &str) -> bool {
        self.manifest_goes_by_pull_request()
    }

    fn manifest_on_the_base(
        &self,
        _in_repo: &str,
        _base: &str,
        _file: &str,
    ) -> Result<adapter_traits::ManifestOnTheBase, NotDelivered> {
        self.manifest_on_the_base_is()
    }

    fn send_manifest_edit(
        &self,
        _in_repo: &str,
        sending: &adapter_traits::ManifestEditToSend,
    ) -> Result<adapter_traits::ManifestEditSent, NotDelivered> {
        self.manifest_edit_sent(sending)
    }

    fn send_checkout_as_pull_request(
        &self,
        _in_repo: &str,
        sending: &adapter_traits::CheckoutToSend,
    ) -> Result<adapter_traits::CheckoutSent, NotDelivered> {
        self.checkout_sent(sending)
    }
}

#[cfg(test)]
#[path = "vcs/tests.rs"]
mod tests;
