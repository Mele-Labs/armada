//! `merge_by: push` over a base that moved: the base merged into the Job's
//! branch in its own worktree, the Job's Checks run again over the merge, and
//! the push asked again, up to [`ROUNDS`] times. `docs/concepts/manifest.md`, *How work lands*.
//!
//! **The Checks are every Manifest Check the Job's workflow gated on**, from
//! every step: the step a Job holds at before merging is usually a hand-off
//! that declares none, and the tree the merge changes is what the earlier
//! steps' Checks read. The built-ins are about a Drone's own change and are
//! not asked again.
//!
//! **Nothing is pushed that was not gated.** The push refuses a head whose tree
//! the Checks never passed on, such as one `crate::currency` merged into, and
//! that head is gated where it stands first. A red round puts the branch back.

use std::sync::Arc;

use adapter_traits::{
    AgentHarness, BaseMergedIn, Delivery, Landable, NotMerged, PushedOntoBase, UncheckedHead, Vcs,
    WorkProduct, Worktree,
};
use adapters::onto_base::ROUNDS;
use config::MergeBy;
use core_model::{Component, Envelope, FieldValue, Job, Level, ResolvedCheck};
use verification::Ran;

use crate::adrift::Adrift;
use crate::check_output::kept_for_a_commit;
use crate::checking;
use crate::daemon::Fleet;
use crate::gate::Ruling;
use crate::repositories::Served;

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
    /// Push the Job's branch onto its base, bringing it up and gating it again
    /// each time the base has moved past it.
    ///
    /// **Held against the Job's other acts for the whole loop**, with the hold
    /// `crate::rechecking` takes: a Drone put on the worktree mid-run would
    /// change the tree the Checks are reading.
    pub(crate) async fn pushed_onto_the_base(
        &self,
        job: &Job,
        served: &Served,
        pull_request: Option<u64>,
    ) -> Result<PushedOntoBase, Adrift> {
        let Some(_held) = self.rechecking().take(job.id()) else {
            return Err(Adrift::ChecksRunningAgain {
                job: job.id().clone(),
            });
        };
        let refused = |why| Adrift::NotMerged {
            job: job.id().clone(),
            why,
        };
        let mut rounds = 0;
        loop {
            let answer = self.pushed_once(job, served, pull_request).await;
            let moved = match &answer {
                Err(NotMerged::BaseMoved { said } | NotMerged::Unchecked { said }) => said.clone(),
                _ => return answer.map_err(refused),
            };
            if rounds == ROUNDS {
                return Err(refused(NotMerged::BaseMoved {
                    said: format!(
                        "it moved during each of {ROUNDS} rounds of bringing the branch up and \
                         gating it again; press merge again when it is quieter. Last: {moved}"
                    ),
                }));
            }
            rounds += 1;
            match answer {
                Err(NotMerged::BaseMoved { .. }) => {
                    self.brought_up_and_gated(job, served, rounds).await
                }
                _ => self.gated_where_it_stands(job, served, rounds).await,
            }
            .map_err(refused)?;
        }
    }

    /// One push, under `merge_end`, for `crate::currency`'s reason.
    async fn pushed_once(
        &self,
        job: &Job,
        served: &Served,
        pull_request: Option<u64>,
    ) -> Result<PushedOntoBase, NotMerged> {
        let nothing_to_check = gated_on(job).is_empty();
        let checked = self.checked_tree(job).await;
        let (vcs, root, handle, declared) = (
            Arc::clone(self.vcs()),
            served.root().to_string(),
            job.handle(),
            self.target_of(served, job.id()).await,
        );
        let _at_the_merge_end = self.merge_end().lock().await;
        tokio::task::spawn_blocking(move || {
            let landable = match (nothing_to_check, checked.as_deref()) {
                (true, _) => Landable::NothingToCheck,
                (false, Some(tree)) => Landable::Checked(tree),
                (false, None) => Landable::Unchecked,
            };
            vcs.merge_by_push(&root, &handle, declared.as_deref(), pull_request, landable)
        })
        .await
        .unwrap_or_else(|stopped| {
            Err(NotMerged::Refused {
                said: stopped.to_string(),
            })
        })
    }

    /// Merge the moved base into the branch and run the Job's Checks over it.
    /// `Ok` is a green branch holding the base; anything else has put it back.
    async fn brought_up_and_gated(
        &self,
        job: &Job,
        served: &Served,
        round: u32,
    ) -> Result<(), NotMerged> {
        let worktree = self
            .surviving_worktree(job)
            .map_err(|gone| NotMerged::BaseMoved {
                said: format!("the branch could not be brought up to it: {gone}"),
            })?;
        let merged = {
            let (vcs, root, owned, declared) = (
                Arc::clone(self.vcs()),
                served.root().to_string(),
                worktree.clone(),
                self.target_of(served, job.id()).await,
            );
            let _at_the_merge_end = self.merge_end().lock().await;
            tokio::task::spawn_blocking(move || {
                vcs.merge_the_moved_base_in(&root, &owned, declared.as_deref())
            })
            .await
            .unwrap_or_else(|stopped| {
                Err(NotMerged::Refused {
                    said: stopped.to_string(),
                })
            })
        };
        let merged = match merged {
            Ok(merged) => merged,
            Err(why) => {
                self.said_about_the_round(job, Level::Warn, round, None, Some(&why));
                return Err(why);
            }
        };
        self.said_about_the_round(job, Level::Info, round, Some(&merged), None);
        let failed = self
            .gated_again(job, served, &worktree, &merged.head, &merged.touched)
            .await;
        let Some(failed) = failed else {
            self.kept_as_checked(job, &merged.tree).await;
            return Ok(());
        };
        let why = NotMerged::GateFailed {
            said: format!(
                "{failed}, against {} with {} merged in; nothing was pushed",
                short(&merged.head),
                merged.base
            ),
        };
        let put_back = self.vcs().put_back(&worktree, &merged);
        self.said_about_the_round(job, Level::Warn, round, Some(&merged), Some(&why));
        if let Err(cause) = put_back {
            self.said_not_put_back(job, &cause.said());
        }
        Err(why)
    }

    /// Run the Job's Checks over the branch's head where it already holds the
    /// base, but carries a tree they never passed on.
    ///
    /// **A red is not put back**, unlike a round's: Fleet did not make this
    /// head in this press, and the remote branch already holds it. Its tree
    /// stays unchecked, so every press refuses it until the branch moves.
    async fn gated_where_it_stands(
        &self,
        job: &Job,
        served: &Served,
        round: u32,
    ) -> Result<(), NotMerged> {
        let worktree = self
            .surviving_worktree(job)
            .map_err(|gone| NotMerged::Refused {
                said: format!("the branch's Checks could not be run: {gone}"),
            })?;
        let read = {
            let (vcs, root, owned, declared, checked) = (
                Arc::clone(self.vcs()),
                served.root().to_string(),
                worktree.clone(),
                self.target_of(served, job.id()).await,
                self.checked_tree(job).await,
            );
            let _at_the_merge_end = self.merge_end().lock().await;
            tokio::task::spawn_blocking(move || {
                vcs.the_unchecked_head(&root, &owned, declared.as_deref(), checked.as_deref())
            })
            .await
            .unwrap_or_else(|stopped| {
                Err(NotMerged::Refused {
                    said: stopped.to_string(),
                })
            })
        };
        let head = match read {
            Ok(head) => head,
            Err(why) => {
                self.said_about_the_head(job, Level::Warn, round, None, Some(&why));
                return Err(why);
            }
        };
        self.said_about_the_head(job, Level::Info, round, Some(&head), None);
        let failed = self
            .gated_again(job, served, &worktree, &head.head, &head.touched)
            .await;
        let Some(failed) = failed else {
            self.kept_as_checked(job, &head.tree).await;
            return Ok(());
        };
        let why = NotMerged::GateFailed {
            said: format!(
                "{failed}, against {}, which its Checks had not read; nothing was pushed",
                short(&head.head)
            ),
        };
        self.said_about_the_head(job, Level::Warn, round, Some(&head), Some(&why));
        Err(why)
    }

    /// Under `merge_by: push`, write down the tree a gate's Checks passed on,
    /// where they were every Manifest Check the workflow declares. **Held, never
    /// raised**: a tree not written is one the push runs the Checks over again.
    pub(crate) async fn kept_what_the_gate_checked(&self, job: &Job, ruling: &Ruling) {
        let Ok(served) = self.served_by(job) else {
            return;
        };
        let wanted = gated_on(job);
        if served.manifest().merge_by() != MergeBy::Push || wanted.is_empty() {
            return;
        }
        let passed = |check: &ResolvedCheck| {
            ruling
                .checks()
                .iter()
                .any(|row| row.name == check.key() && row.outcome.advances())
        };
        if !wanted.iter().all(passed) {
            return;
        }
        let Ok(worktree) = self.surviving_worktree(job) else {
            return;
        };
        let vcs = Arc::clone(self.vcs());
        let read = tokio::task::spawn_blocking(move || vcs.tree_as_it_stands(&worktree)).await;
        if let Ok(Ok(tree)) = read {
            self.kept_as_checked(job, &tree).await;
        }
    }

    async fn checked_tree(&self, job: &Job) -> Option<String> {
        self.store()
            .lock()
            .await
            .checked_tree_for(job.id())
            .ok()
            .flatten()
    }

    async fn kept_as_checked(&self, job: &Job, tree: &str) {
        let _ = self
            .store()
            .lock()
            .await
            .record_checked_tree(job.id(), tree);
    }

    /// Run the Job's Checks over the worktree, announced on the step it holds
    /// at. `None` is green; `Some` names what went red, once each red Check has
    /// been asked of the base — `crate::asking_the_base`.
    async fn gated_again(
        &self,
        job: &Job,
        served: &Served,
        worktree: &Worktree,
        head: &str,
        touched: &[String],
    ) -> Option<String> {
        let checks = gated_on(job);
        let (ran, printed) = match self.ran_over(job, served, worktree, &checks, touched).await {
            Ok(ran) => ran,
            Err(said) => return Some(said),
        };
        if ran.advances() {
            return None;
        }
        let kept = kept_for_a_commit(served.records_root(), head, &ran.recorded(), &printed);
        let reds = kept
            .into_iter()
            .filter(|check| !check.outcome.advances())
            .collect();
        self.read_the_reds(job, served, worktree, head, touched, &checks, reds)
            .await
    }

    /// One run of `checks` over the worktree, on the step the Job holds at.
    pub(crate) async fn ran_over(
        &self,
        job: &Job,
        served: &Served,
        worktree: &Worktree,
        checks: &[ResolvedCheck],
        touched: &[String],
    ) -> Result<(Ran, Vec<(String, checks_runner::Output)>), String> {
        let Some(step) = job.current_step_id() else {
            return Err(String::from(
                "the Job stands at no step to run its Checks on",
            ));
        };
        let attempt = self
            .store()
            .lock()
            .await
            .step_attempt(job.id(), step)
            .unwrap_or(core_model::Attempt::FIRST);
        let announcing = self.announcing(served, job, step, attempt);
        let completed = checking::ran(
            checks,
            touched,
            false,
            checking::Reading::Whole,
            std::path::Path::new(worktree.path()),
            self.budget().duration(),
            &self.checks_room_for(job, crate::places::Asking::Gate).await,
            &announcing,
            &self.port_map(job).await,
            &self.port_env(job).await,
            None,
            &checking::Stop::never(),
            None,
            attempt,
            None,
            None,
        )
        .await;
        drop(announcing);
        let observed: Vec<_> = completed.iter().map(|one| one.observed.clone()).collect();
        let ran = Ran::against(checks, &observed)
            .map_err(|why| format!("its Checks could not be read: {why}"))?;
        let printed = completed
            .into_iter()
            .filter_map(|one| one.printed)
            .collect();
        Ok((ran, printed))
    }

    /// A line in the Job's log for one round: the merge, or what stopped it.
    fn said_about_the_round(
        &self,
        job: &Job,
        level: Level,
        round: u32,
        merged: Option<&BaseMergedIn>,
        refused: Option<&NotMerged>,
    ) {
        let said = match (merged, refused) {
            (Some(_), None) => {
                "the base moved past what was gated, so it was merged into the Job's branch and \
                 its Checks run again over the merge"
            }
            (_, Some(NotMerged::GateFailed { .. })) => {
                "the Job's Checks did not pass with the moved base merged in, so nothing was \
                 pushed and the branch was put back"
            }
            _ => "the moved base could not be merged into the Job's branch, so nothing was pushed",
        };
        let mut envelope = Envelope::new(
            self.now(),
            level,
            Component::Fleet,
            self.run().clone(),
            said,
        )
        .in_job(job.id().as_ulid().clone())
        .with_field("round", FieldValue::Int(i64::from(round)));
        if let Some(merged) = merged {
            envelope = envelope
                .with_field("onto", FieldValue::Str(merged.onto.clone()))
                .with_field("commit", FieldValue::Str(merged.head.clone()));
        }
        if let Some(refused) = refused {
            envelope = envelope
                .with_field("refused", FieldValue::Str(refused.kind().to_string()))
                .with_field("cause", FieldValue::Str(refused.said()));
        }
        self.noted_in_the_log(job.id(), &envelope);
    }

    /// A line in the Job's log for a round over a head its Checks had not read.
    fn said_about_the_head(
        &self,
        job: &Job,
        level: Level,
        round: u32,
        head: Option<&UncheckedHead>,
        refused: Option<&NotMerged>,
    ) {
        let said = match (head, refused) {
            (Some(_), None) => {
                "the Job's branch already holds the base, but at a head its Checks have not \
                 passed on, so they run over it before the push"
            }
            (_, Some(NotMerged::GateFailed { .. })) => {
                "the Job's Checks did not pass on the branch's head, so nothing was pushed"
            }
            _ => "the branch's head could not be read for its Checks, so nothing was pushed",
        };
        let mut envelope = Envelope::new(
            self.now(),
            level,
            Component::Fleet,
            self.run().clone(),
            said,
        )
        .in_job(job.id().as_ulid().clone())
        .with_field("round", FieldValue::Int(i64::from(round)));
        if let Some(head) = head {
            envelope = envelope.with_field("commit", FieldValue::Str(head.head.clone()));
        }
        if let Some(refused) = refused {
            envelope = envelope
                .with_field("refused", FieldValue::Str(refused.kind().to_string()))
                .with_field("cause", FieldValue::Str(refused.said()));
        }
        self.noted_in_the_log(job.id(), &envelope);
    }

    fn said_not_put_back(&self, job: &Job, cause: &str) {
        let envelope = Envelope::new(
            self.now(),
            Level::Warn,
            Component::Fleet,
            self.run().clone(),
            "the Job's branch still holds the base its Checks failed on, and was not put back",
        )
        .in_job(job.id().as_ulid().clone())
        .with_field("cause", FieldValue::Str(cause.to_string()));
        self.noted_in_the_log(job.id(), &envelope);
    }
}

/// Every Manifest Check the Job's workflow declares, once each, in the order
/// its steps first name them.
fn gated_on(job: &Job) -> Vec<ResolvedCheck> {
    let mut seen = std::collections::BTreeSet::new();
    job.workflow()
        .steps()
        .iter()
        .flat_map(|step| step.checks())
        .filter(|check| matches!(check, ResolvedCheck::ManifestCheck { .. }))
        .filter(|check| seen.insert(check.key().to_string()))
        .cloned()
        .collect()
}

fn short(sha: &str) -> &str {
    sha.get(..10).unwrap_or(sha)
}
