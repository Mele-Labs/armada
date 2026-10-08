//! A failed Trigger with `repair` on: a repair Drone on a branch of its own,
//! the Command run again there, and the fix held for the owner to place.
//! `docs/concepts/fleet.md`, *A failed Trigger's repair*; the rules it applies
//! are `crate::trigger_repair`'s.
//!
//! **Non-blocking.** Nothing here moves the Job or its step: a failure queues
//! a firing, and [`repair_next`](Fleet::repair_next), which Fleet's own loop
//! calls, works one at a time. **The repair Drone holds a pool slot**, because
//! it writes and builds and a build wants the warm `target/` a slot keeps. The
//! slot is given back when the fix is held or the repair ends, so a fix
//! waiting on the owner holds no bay.

use std::path::Path;

use adapter_traits::{
    AgentHarness, CommitTime, Delivery, Grant, Prompt, SlotLeased, Toolbelt, Vcs, WorkProduct,
    Worktree,
};
use core_model::{Job, JobId, Level, RepairRecord, TriggerState};
use verification::Exit;

use crate::daemon::Fleet;
use crate::trigger_repair::{self, AfterRerun, Subject, Waiting};

/// Why no slot was leased for a repair.
enum Leasing {
    /// Every slot is held. The repair waits for one.
    Full,
    /// Said in the words a person reads.
    Not(String),
}

fn code_of(exit: &Exit) -> Option<i32> {
    match exit {
        Exit::Code(code) => Some(*code),
        _ => None,
    }
}

/// Who holds a repair's slot: the Job, and which of its firings or added
/// steps it is for.
pub(crate) fn holder_of(job: &JobId, subject: &Subject) -> String {
    format!("{}-repair-{subject}", job.as_str())
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
    /// Work the oldest failed Trigger waiting for a repair, through its tries.
    /// `false` where none waits. **One at a time**, so a machine runs one
    /// repair Drone for Triggers however many Jobs have one failing.
    pub async fn repair_next(&self) -> bool {
        let next = self.trigger_repairs().lock().expect("not poisoned").pop();
        let Some(waiting) = next else {
            return false;
        };
        self.repaired(waiting).await
    }

    /// `false` where every slot is taken: the repair goes to the back of the
    /// queue and is asked again on a later turn, as a Job waits on a slot.
    async fn repaired(&self, waiting: Waiting) -> bool {
        let Ok(job) = self.load(&waiting.job).await else {
            return true;
        };
        // An attempt a restart cut short is redone and not counted twice.
        let mut record = waiting.record.clone();
        record.tries = record.tries.saturating_sub(1);
        let held = match self.repair_slot_leased(&job, &waiting, &mut record).await {
            Ok(held) => held,
            Err(Leasing::Full) => {
                self.trigger_repairs()
                    .lock()
                    .expect("not poisoned")
                    .push(waiting);
                return false;
            }
            Err(Leasing::Not(why)) => {
                self.repair_ended(&job, &waiting, TriggerState::Failed, &record, &why)
                    .await;
                return true;
            }
        };
        if let Some(told) = waiting.side.as_deref() {
            self.side_run_over(&job, &waiting, told, held, record).await;
            return true;
        }
        let (pool, slot, worktree) = held;
        let came_to = self
            .tries_made(&job, &waiting, &worktree, &mut record)
            .await;
        // What the fix changes, read while the branch is still checked out:
        // the worktree is measured from the Job's branch it was cut from.
        if matches!(came_to, Ok(AfterRerun::HoldTheFix)) {
            record.files = self
                .work()
                .changed_files(&worktree)
                .map(|changed| changed.paths())
                .unwrap_or_default();
        }
        // Committed before this, so parking keeps the branch and gives the bay back.
        let _ = self
            .vcs()
            .park_slot(&pool, slot, &holder_of(job.id(), &waiting.subject));
        match came_to {
            Ok(AfterRerun::HoldTheFix) => {
                self.repair_ended(
                    &job,
                    &waiting,
                    TriggerState::FixReady,
                    &record,
                    "the Command passes on the repair branch and the fix waits on your choice",
                )
                .await;
            }
            Ok(_) => {
                let said = format!(
                    "the Command still fails after {} repairs",
                    core_model::REPAIR_TRIES
                );
                self.repair_ended(&job, &waiting, TriggerState::Failed, &record, &said)
                    .await;
            }
            Err(why) => {
                self.repair_ended(&job, &waiting, TriggerState::Failed, &record, &why)
                    .await;
            }
        }
        true
    }

    /// Lease a slot on a branch cut from the Job's, and make it ready to work in.
    async fn repair_slot_leased(
        &self,
        job: &Job,
        waiting: &Waiting,
        record: &mut RepairRecord,
    ) -> Result<(adapter_traits::SlotPool, u32, Worktree), Leasing> {
        let served = self
            .served_by(job)
            .map_err(|why| Leasing::Not(why.to_string()))?;
        let from = job.branch().ok_or_else(|| {
            Leasing::Not(String::from("the Job has no branch to cut a repair from"))
        })?;
        let spec = self
            .repair_spec(&served, job, &waiting.subject)
            .map_err(|why| Leasing::Not(format!("{why:?}")))?;
        self.cut_from().learn(&spec.branch(), from.as_str());
        let pool = crate::leasing::pool_cut_from(&served, Some(from));
        let holder = holder_of(job.id(), &waiting.subject);
        // A repair a restart interrupted has its branch already.
        let leased = match record.branch.clone() {
            Some(branch) => self.vcs().lease_existing_slot(&pool, &branch, &holder),
            None => self.vcs().lease_slot(&pool, &spec, &holder),
        };
        match leased {
            Ok(SlotLeased::Took { slot, worktree, .. }) => {
                let worktree = self.based(&served, worktree);
                record.branch = Some(worktree.branch().to_string());
                self.prepared(job, &worktree)
                    .await
                    .map_err(|why| Leasing::Not(why.to_string()))?;
                Ok((pool, slot, worktree))
            }
            Ok(SlotLeased::Full) => Err(Leasing::Full),
            Err(why) => Err(Leasing::Not(format!(
                "the repair branch could not be cut: {why}"
            ))),
        }
    }

    /// Up to [`core_model::REPAIR_TRIES`] Drones on the repair branch, the
    /// Command run again after each.
    async fn tries_made(
        &self,
        job: &Job,
        waiting: &Waiting,
        worktree: &Worktree,
        record: &mut RepairRecord,
    ) -> Result<AfterRerun, String> {
        let (mut exit, mut stdout, mut stderr) =
            (waiting.exit, waiting.stdout.clone(), waiting.stderr.clone());
        loop {
            record.tries += 1;
            self.repair_kept(job, waiting, TriggerState::Repairing, record, false)
                .await;
            let told =
                trigger_repair::brief(&waiting.trigger, &waiting.command, exit, &stdout, &stderr);
            let prompt = Prompt::assembled(&told)
                .map_err(|why| format!("the brief would not render: {why:?}"))?;
            let belt = self.repair_belt(job).await;
            let config = self
                .spawn_config_with(job, &waiting.step, worktree, prompt, None, belt)
                .await
                .map_err(|why| format!("the repair Drone would not configure: {why:?}"))?;
            let said = self.repair_run(&config).await?;
            if let Some(said) = said {
                self.logged(
                    job.id(),
                    self.trigger_line(
                        job,
                        Level::Info,
                        &format!("repair Drone for `{}`: {said}", waiting.trigger),
                    ),
                );
            }
            self.the_fix_committed(worktree, &format!("Repair Trigger `{}`", waiting.trigger))?;
            self.repair_kept(job, waiting, TriggerState::Rerunning, record, false)
                .await;
            let attempt = checks_runner::run(
                &waiting.command,
                Path::new(worktree.path()),
                self.budget().duration(),
            )
            .await;
            let after = AfterRerun::of(record.tries, matches!(attempt.exit, Exit::Code(0)));
            if after != AfterRerun::TryAgain {
                return Ok(after);
            }
            exit = code_of(&attempt.exit);
            stdout = attempt.output.stdout;
            stderr = attempt.output.stderr;
        }
    }

    /// What the Drone wrote, committed by Fleet. **Nothing to commit is fine**:
    /// the Command is run either way and says whether it is fixed.
    pub(crate) fn the_fix_committed(&self, worktree: &Worktree, message: &str) -> Result<(), String> {
        let at = CommitTime::seconds_since_epoch(
            self.now()
                .epoch_millis()
                .unwrap_or_default()
                .div_euclid(1_000),
        );
        self.vcs()
            .commit_all(worktree, message, at)
            .map(|_| ())
            .map_err(|why| format!("the repair could not be committed: {why}"))
    }

    /// Writes and builds, and runs what the Manifest declares and does not
    /// call destructive. **No dispatch, plan or repair grants**, and nothing
    /// that pushes: Fleet delivers.
    pub(crate) async fn repair_belt(&self, job: &Job) -> Toolbelt {
        let mut belt = Toolbelt::evidence_only()
            .and(Grant::ReadTheWorktree)
            .and(Grant::ReadTheRepository)
            .and(Grant::ChangeTheWorktree);
        let Ok(served) = self.served_by(job) else {
            return belt;
        };
        let (manifest, _) = self.effective_manifest_in(&served, job).await;
        for name in manifest.command_names() {
            if let Some(command) = manifest.command(&name) {
                if !command.is_destructive() {
                    belt = belt.and(Grant::RunADeclaredCommand(command.run().to_string()));
                }
            }
        }
        belt
    }

    pub(crate) async fn repair_kept(
        &self,
        job: &Job,
        waiting: &Waiting,
        state: TriggerState,
        record: &RepairRecord,
        ended: bool,
    ) {
        let record = RepairRecord {
            settled_at: Some(self.now()),
            ..record.clone()
        };
        match self
            .repair_settled(job, &waiting.subject, state, &record, ended)
            .await
        {
            Ok(()) => self.repair_moved(job, &waiting.subject).await,
            Err(why) => {
                let said = format!("a Trigger's repair could not be recorded: {why}");
                self.logged(job.id(), self.trigger_line(job, Level::Warn, &said));
            }
        }
    }

    pub(crate) async fn repair_ended(
        &self,
        job: &Job,
        waiting: &Waiting,
        state: TriggerState,
        record: &RepairRecord,
        said: &str,
    ) {
        // A failure that was not fixed still holds the Job where the Trigger blocks.
        let state = self
            .held_where_it_blocks(job, &waiting.subject, state)
            .await;
        let over = matches!(
            state,
            TriggerState::Failed | TriggerState::Held | TriggerState::Passed
        );
        self.repair_kept(job, waiting, state, record, over).await;
        let level = match state {
            TriggerState::Failed | TriggerState::Held => Level::Warn,
            _ => Level::Info,
        };
        let said = format!(
            "{} `{}` {}: {said}",
            waiting.subject.label(),
            waiting.trigger,
            state.as_wire()
        );
        self.logged(job.id(), self.trigger_line(job, level, &said));
        // A repair that ends passed lets the hold go by itself.
        if state == TriggerState::Passed {
            self.hold_released_by_repair(job.id(), &waiting.subject)
                .await;
            self.hold_let_go(job.id(), core_model::Actor::Fleet).await;
        }
        // The fix is on the Job's branch, or it did not fix anything: either
        // way the branch the repair Drone wrote on has done its work.
        if trigger_repair::branch_is_done_with(state, record.choice) {
            self.repair_branch_given_back(job, &waiting.subject, record)
                .await;
        }
    }
}
