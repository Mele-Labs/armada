//! A red Check in a merge line turn, asked of the base before the branch is
//! blamed. `docs/capabilities/merge-line.md`, *Choosing what reruns*.
//!
//! **The base is asked once per Check and commit.** Green on the base: the
//! Job's log says so the moment it is known and the turn runs the Check once
//! more on the branch, so a failure is real only after that rerun also fails.
//! Red on the base: the failure is main's and nothing is rerun. A timeout on
//! the base is main's for the turn and is not remembered, since a slow run is
//! not a broken commit.

use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct, Worktree};
use core_model::{
    Attempt, CheckOutcome, Component, Envelope, FieldValue, Job, Level, ResolvedCheck, StepCheck,
};
use store::Blame;
use verification::Ran;

use crate::check_output::kept_for_a_commit;
use crate::checking;
use crate::daemon::Fleet;
use crate::repositories::Served;

/// What the base answered for one Check at one commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum BaseSays {
    Green,
    Red,
}

/// What asking came to.
enum Asked {
    Says(BaseSays, String),
    /// Past the limit on the base. Never remembered.
    TimedOut(String),
    /// The base could not be run: no checkout, or the Check never started.
    Unasked,
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
    /// What a gate's red Checks come to once each is asked of the base: `None`
    /// where every one passed on its rerun, otherwise the sentence the refusal
    /// carries. **Whose failure it was is left for the turn**, in
    /// [`Lines::blamed`](crate::taking_turns::Lines).
    pub(crate) async fn read_the_reds(
        &self,
        job: &Job,
        served: &Served,
        worktree: &Worktree,
        head: &str,
        touched: &[String],
        checks: &[ResolvedCheck],
        reds: Vec<StepCheck>,
    ) -> Option<String> {
        let (mut branch, mut main) = (Vec::new(), Vec::new());
        for red in reds {
            let named = shown(&red);
            let Some(check) = checks.iter().find(|one| one.label() == red.name) else {
                branch.push(named);
                continue;
            };
            match self.asked_of_the_base(job, served, check, touched).await {
                Asked::Says(BaseSays::Red, at) => {
                    main.push(format!("{named} fails on the base at {} too", short(&at)));
                }
                Asked::TimedOut(at) => main.push(format!(
                    "{named} timed out on the base at {} itself, past its limit",
                    short(&at)
                )),
                Asked::Says(BaseSays::Green, at) => {
                    self.said_about_a_check(
                        job,
                        Level::Warn,
                        &format!(
                            "{} failed and the base is green for it, so the turn runs it once \
                             more and goes on",
                            red.name
                        ),
                        &red,
                        &at,
                    );
                    match self
                        .rerun_on_the_branch(job, served, worktree, head, touched, check)
                        .await
                    {
                        Ok(None) => self.said_about_a_check(
                            job,
                            Level::Info,
                            &format!("{} passed on the rerun, so the turn goes on", red.name),
                            &red,
                            &at,
                        ),
                        Ok(Some(again)) => branch.push(shown(&again)),
                        Err(said) => branch.push(said),
                    }
                }
                Asked::Unasked => branch.push(named),
            }
        }
        if branch.is_empty() && main.is_empty() {
            return None;
        }
        let blamed = if branch.is_empty() {
            Blame::Base
        } else {
            Blame::Branch
        };
        if let Ok(mut by) = self.lines().blamed.lock() {
            by.insert(job.id().clone(), blamed);
        }
        Some(match (branch.is_empty(), main.is_empty()) {
            (false, true) => format!("{} did not pass", branch.join(", ")),
            (true, false) => format!("{}, so it is the base's to fix", main.join(", ")),
            _ => format!(
                "{} did not pass on the branch, and {}",
                branch.join(", "),
                main.join(", ")
            ),
        })
    }

    /// Run one Check again over the worktree. `None` is a pass; `Some` is the
    /// row of the run that failed, with the log of this run on it.
    async fn rerun_on_the_branch(
        &self,
        job: &Job,
        served: &Served,
        worktree: &Worktree,
        head: &str,
        touched: &[String],
        check: &ResolvedCheck,
    ) -> Result<Option<StepCheck>, String> {
        let (ran, printed) = self
            .ran_over(job, served, worktree, std::slice::from_ref(check), touched)
            .await?;
        if ran.advances() {
            return Ok(None);
        }
        // A key of its own, or the rerun's log would write over the first run's.
        let key = format!("{head}-again");
        let kept = kept_for_a_commit(served.records_root(), &key, &ran.recorded(), &printed);
        Ok(kept.into_iter().find(|row| !row.outcome.advances()))
    }

    async fn asked_of_the_base(
        &self,
        job: &Job,
        served: &Served,
        check: &ResolvedCheck,
        touched: &[String],
    ) -> Asked {
        let Ok((_, checkout)) = self.base_to_show_from(job).await else {
            return Asked::Unasked;
        };
        let at = checkout.commit().to_string();
        let key = (
            served.root().to_string(),
            at.clone(),
            check.label().to_string(),
        );
        let remembered = self
            .lines()
            .asked
            .lock()
            .ok()
            .and_then(|had| had.get(&key).cloned());
        if let Some(says) = remembered {
            return Asked::Says(says, at);
        }
        let completed = checking::ran(
            std::slice::from_ref(check),
            touched,
            false,
            checking::Reading::Whole,
            Path::new(checkout.path()),
            self.budget().duration(),
            &self.checks_room_for(job, crate::places::Asking::Gate).await,
            &crate::underway::Announcing::nowhere(),
            &self.main_checkout_ports(served).await,
            &self.main_checkout_port_env(served).await,
            None,
            &checking::Stop::never(),
            None,
            Attempt::FIRST,
            None,
        )
        .await;
        let observed: Vec<_> = completed.iter().map(|one| one.observed.clone()).collect();
        let Ok(ran) = Ran::against(std::slice::from_ref(check), &observed) else {
            return Asked::Unasked;
        };
        let says = match ran.recorded().first().map(|row| row.outcome) {
            Some(CheckOutcome::Passed) => BaseSays::Green,
            Some(CheckOutcome::Failed | CheckOutcome::Signalled) => BaseSays::Red,
            Some(CheckOutcome::TimedOut) => return Asked::TimedOut(at),
            _ => return Asked::Unasked,
        };
        if let Ok(mut had) = self.lines().asked.lock() {
            had.insert(key, says.clone());
        }
        Asked::Says(says, at)
    }

    /// A line in the Job's log about one Check, naming it and its log.
    fn said_about_a_check(&self, job: &Job, level: Level, said: &str, check: &StepCheck, at: &str) {
        let mut envelope = Envelope::new(
            self.now(),
            level,
            Component::Fleet,
            self.run().clone(),
            said,
        )
        .in_job(job.id().as_ulid().clone())
        .with_field("check", FieldValue::Str(check.name.clone()))
        .with_field("base", FieldValue::Str(at.to_string()));
        if let Some(log) = &check.output_path {
            envelope = envelope.with_field("log", FieldValue::Str(log.clone()));
        }
        self.noted_in_the_log(job.id(), &envelope);
    }
}

fn shown(row: &StepCheck) -> String {
    match &row.output_path {
        Some(path) => format!("{} ({path})", row.name),
        None => row.name.clone(),
    }
}

fn short(sha: &str) -> &str {
    sha.get(..10).unwrap_or(sha)
}
