//! A Skill Trigger, a Skill added step and a Drone added step: a side Drone
//! that works on a branch cut from the Job's, in a slot of its own.
//! `docs/concepts/trigger.md`, *A Skill or a Drone step*.
//!
//! **The repair's machinery, with one Drone and no Command.** The queue, the
//! slot, the branch, the record and the owner's choice are `crate::repairing`
//! and `crate::placing_a_fix`; this is what differs. The Drone is told its task
//! and not a failure, runs once, and there is nothing to run again afterwards:
//! what it committed is the answer.
//!
//! | The Drone | The firing |
//! |---|---|
//! | committed changes | `fix_ready`, placed with `choose_trigger_fix` |
//! | changed nothing and ended | `passed` |
//! | would not start, or ran past its budget | `failed`, `held` where it blocks |
//!
//! **In flight it reads `running` with a Drone already on it** (`tries` is 1),
//! which is how a Command that is running reads apart from it, and how a
//! restart finds it again. **Self repair does not apply**: a Drone already
//! fixes its own failures.

use adapter_traits::{AgentHarness, Delivery, Prompt, SlotPool, Vcs, WorkProduct, Worktree};
use config::settings as keys;
use core_model::{
    AddedKind, DroneId, Job, Level, RepairRecord, StepId, TriggerResolution, TriggerState,
    TriggerWhen,
};

use crate::daemon::Fleet;
use crate::prompts::Prompts;
use crate::trigger_hold::Hold;
use crate::trigger_repair::{Subject, Waiting};

/// What the side Drone is sent to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Side {
    /// Run this skill.
    Skill(String),
    /// Do what this brief says.
    Brief(String),
}

impl Side {
    pub(crate) fn of_addition(kind: &AddedKind) -> Option<Side> {
        match kind {
            AddedKind::Skill { skill } => Some(Side::Skill(skill.clone())),
            AddedKind::Drone { brief } => Some(Side::Brief(brief.clone())),
            AddedKind::Script { .. } => None,
        }
    }
}

fn place(prompts: &Prompts, when: TriggerWhen, step: &StepId) -> String {
    match when {
        TriggerWhen::StepStarts => {
            prompts.fill(keys::PROMPT_SIDE_BEFORE_STEP, &[("step", step.as_str())])
        }
        TriggerWhen::StepPasses => {
            prompts.fill(keys::PROMPT_SIDE_AFTER_STEP, &[("step", step.as_str())])
        }
        TriggerWhen::PrOpened => prompts.get(keys::PROMPT_SIDE_AFTER_PR).to_string(),
    }
}

/// The Drone's one turn. The first line is what a Drone harness and a fake
/// both key on, as the repair's is, so **the heading stays Fleet's** whatever
/// the words below it say.
pub(crate) fn brief(
    prompts: &Prompts,
    side: &Side,
    title: &str,
    branch: Option<&str>,
    when: TriggerWhen,
    step: &StepId,
) -> String {
    let place = place(prompts, when, step);
    let context = match branch {
        Some(branch) => prompts.fill(
            keys::PROMPT_SIDE_CONTEXT_ON_BRANCH,
            &[("job", title), ("branch", branch), ("place", &place)],
        ),
        None => prompts.fill(
            keys::PROMPT_SIDE_CONTEXT,
            &[("job", title), ("place", &place)],
        ),
    };
    let rest = prompts.get(keys::PROMPT_SIDE_REST);
    match side {
        Side::Skill(skill) => format!(
            "RUN THE SKILL `{skill}`\n\n{}\n\n{rest}",
            prompts.fill(
                keys::PROMPT_SIDE_SKILL,
                &[("skill", skill), ("context", &context)]
            )
        ),
        Side::Brief(task) => format!("RUN THE STEP\n\n{task}\n\n{context}\n\n{rest}"),
    }
}

/// A side run's words as they ship: each the default of a `prompts.side…` key.
pub(crate) const SKILL: &str = "Run the `{skill}` skill on this branch. {context}";
pub(crate) const CONTEXT: &str = "This runs for the Job \"{job}\", {place}.";
pub(crate) const CONTEXT_ON_BRANCH: &str =
    "This runs for the Job \"{job}\" on `{branch}`, {place}.";
pub(crate) const BEFORE_STEP: &str = "before step `{step}` starts";
pub(crate) const AFTER_STEP: &str = "after step `{step}` passes";
pub(crate) const AFTER_PR: &str = "after the pull request opens";
pub(crate) const REST: &str =
    "Change what is needed and stop when it is done. Do not commit, push or open a \
                pull request: Fleet does that.";

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
    /// What a hold or a fix is for, where it is a side run and not a Command.
    pub(crate) async fn side_of(&self, job: &Job, hold: &Hold) -> Option<Side> {
        match hold {
            Hold::Addition(added) => Side::of_addition(&added.kind),
            Hold::Firing { firing, .. } => {
                let frozen = self.store().lock().await.frozen_triggers(job.id()).ok()?;
                frozen
                    .into_iter()
                    .find(|one| {
                        one.name == firing.name
                            && one.when == firing.when
                            && one.step == firing.step
                    })
                    .and_then(|one| match one.resolution {
                        TriggerResolution::Skill { name } => Some(Side::Skill(name)),
                        TriggerResolution::Drone { brief } => Some(Side::Brief(brief)),
                        _ => None,
                    })
            }
        }
    }

    /// Put a side run on the queue: the firing reads `running` with a Drone
    /// on it, and `repair_next` works it once a slot is free. **Queued and not
    /// waited for**, as a repair is.
    pub(crate) async fn side_queued(
        &self,
        job: &Job,
        subject: Subject,
        name: String,
        when: TriggerWhen,
        step: StepId,
        side: &Side,
    ) {
        let record = RepairRecord {
            tries: 1,
            ..RepairRecord::default()
        };
        let told = brief(
            &self.prompts(),
            side,
            job.title().as_str(),
            job.branch().map(|branch| branch.as_str()),
            when,
            &step,
        );
        let waiting = Waiting {
            job: job.id().clone(),
            subject,
            trigger: name,
            step,
            command: String::new(),
            exit: None,
            stdout: String::new(),
            stderr: String::new(),
            record,
            side: Some(told),
        };
        self.repair_kept(job, &waiting, TriggerState::Running, &waiting.record, false)
            .await;
        self.trigger_repairs()
            .lock()
            .expect("not poisoned")
            .push(waiting);
    }

    /// The Drone's turn, then what it left: `Ok(true)` where it changed files.
    async fn side_run_made(
        &self,
        job: &Job,
        waiting: &Waiting,
        told: &str,
        worktree: &Worktree,
    ) -> Result<bool, String> {
        let prompt = Prompt::assembled(told)
            .map_err(|why| format!("the brief would not render: {why:?}"))?;
        let belt = self.repair_belt(job).await;
        let config = self
            .spawn_config_with(job, &waiting.step, worktree, prompt, None, belt)
            .await
            .map_err(|why| format!("the Drone would not configure: {why:?}"))?;
        let drone = DroneId::carried(self.mint().ulid());
        let ran = self.repair_run_spending(&config).await;
        // Counted against the Job whether or not it came right.
        if let Ok((_, spend)) = &ran {
            let _ = self.record_spend(job.id(), &drone, spend).await;
        }
        let (said, spend) = ran?;
        // A transcript that closed with no turn ending is a Drone that died or
        // gave up, and what it left is not an answer.
        if spend.cost_micros.is_none() && spend.turns == 0 {
            return Err(String::from("the Drone stopped before it finished"));
        }
        if let Some(said) = said {
            self.logged(
                job.id(),
                self.trigger_line(
                    job,
                    Level::Info,
                    &format!("Drone for `{}`: {said}", waiting.trigger),
                ),
            );
        }
        self.the_fix_committed(worktree, &format!("Run `{}`", waiting.trigger))?;
        Ok(self
            .work()
            .changed_files(worktree)
            .map(|changed| !changed.paths().is_empty())
            .unwrap_or(false))
    }

    /// Run the Drone on the slot it holds, give the slot back, and settle the
    /// firing: `fix_ready`, `passed` or `failed`.
    pub(crate) async fn side_run_over(
        &self,
        job: &Job,
        waiting: &Waiting,
        told: &str,
        held: (SlotPool, u32, Worktree),
        mut record: RepairRecord,
    ) {
        let (pool, slot, worktree) = held;
        record.tries = 1;
        let came_to = self.side_run_made(job, waiting, told, &worktree).await;
        if matches!(came_to, Ok(true)) {
            record.files = self
                .work()
                .changed_files(&worktree)
                .map(|changed| changed.paths())
                .unwrap_or_default();
        }
        let _ = self.vcs().park_slot(
            &pool,
            slot,
            &crate::repairing::holder_of(job.id(), &waiting.subject),
        );
        let (state, said) = match came_to {
            Ok(true) => (
                TriggerState::FixReady,
                String::from("the Drone changed files and the change waits on your choice"),
            ),
            Ok(false) => (
                TriggerState::Passed,
                String::from("the Drone changed nothing"),
            ),
            Err(why) => (TriggerState::Failed, why),
        };
        self.repair_ended(job, waiting, state, &record, &said).await;
    }

    /// The owner's Rerun on a held side run: the Drone goes again on a branch
    /// of its own, and the hold waits through it.
    pub(crate) async fn side_rerun(&self, job: &Job, hold: &Hold, side: &Side) {
        self.side_queued(
            job,
            hold.subject(),
            hold.name(),
            hold.when(),
            hold.step().clone(),
            side,
        )
        .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_skill_is_named_on_the_first_line_and_told_where_it_runs() {
        let told = brief(
            &Prompts::shipped(),
            &Side::Skill("simplify".into()),
            "Fix the reader",
            Some("armada/fix-the-reader"),
            TriggerWhen::StepPasses,
            &StepId::new("implement"),
        );
        assert!(told.starts_with("RUN THE SKILL `simplify`\n"));
        assert!(told.contains(
            "\"Fix the reader\" on `armada/fix-the-reader`, after step `implement` passes"
        ));
        assert!(told.contains("Do not commit, push or open a pull request"));
    }

    #[test]
    fn a_drone_step_carries_its_brief_whole() {
        let told = brief(
            &Prompts::shipped(),
            &Side::Brief("Add a changelog line.".into()),
            "Fix the reader",
            None,
            TriggerWhen::PrOpened,
            &StepId::new("summarise"),
        );
        assert!(told.starts_with("RUN THE STEP\n\nAdd a changelog line."));
        assert!(told.contains("after the pull request opens"));
    }
}
