//! The acts that take something away, and what each one leaves.
//!
//! Cut out of [`daemon`](mod@crate::daemon) on that module's own three-part
//! sentence — what Fleet is made of, the slots it works in, and the things it
//! can be asked. `resume` and `reviewing` hold the rest; these are a ladder
//! rather than leftovers, each rung defined against the one below it.
//!
//! | Act | Asked by | Goes | Survives |
//! |---|---|---|---|
//! | [`kill_drone`](Fleet::kill_drone) | a person | the process, and the step it was on | the Job, its worktree, every step that advanced |
//! | [`kill_process`](Fleet::kill_process) | a person | one process of the Job and what it started | the Drone, its step, the Job — unless the process was the Drone |
//! | [`kill_processes`](Fleet::kill_processes) | a person | every process of the Job, and the step | what `kill_drone` leaves |
//! | [`drone_at_rest`](Fleet::drone_at_rest) | Fleet | what `kill_drone` takes | what it leaves |
//! | [`ended_unanswered`](Fleet::ended_unanswered) | Fleet | the same | the same |
//! | [`kill_job`](Fleet::kill_job) | a person | the Job, at `killed` | the record, and the worktree until `armada clean` |
//! | [`forget_job`](Fleet::forget_job) | a person | the record | nothing this owns; the worktree is `armada clean`'s |
//!
//! **Reading them apart is why they are together.** Each doc below says what
//! it is *not*: a kill that is not terminal, a terminal that is not a verdict,
//! a deletion that is not a way to stop something still running, and a reap
//! nobody asked for.
//!
//! **That last one needs defending, and `Ended` is the defence — for
//! `drone_at_rest`.** [`ended_unanswered`](Fleet::ended_unanswered) is the one
//! exception, argued on its own doc: what it takes away was never the Drone's
//! silence to answer for.
//!
//! [`stopped_by_hand`](Fleet::stopped_by_hand) is here because `kill_drone` is
//! its only caller and the two are one act as an operator means it.

use std::num::NonZeroU32;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::{
    Actor, EscalationTrigger, Job, JobId, StepLevelTrigger, StepState, StepTarget, Target,
};
use ipc::{WireError, WireValue};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::drone::{aftermath, Aftermath, Ending};
use crate::group::end_the_process;
use crate::permitting::{Refusing, Waiting};
use crate::resources::Tree;
use crate::working::{StoodDown, Working};
use store::ExtraEnded;

/// A kill named a pid the Job's tree did not hold when Fleet read it at the
/// act. Declared beside the act, the contract's form for a code.
const NOT_THE_JOBS_PROCESS: &str = "fleet.not_the_jobs_process";

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
    /// End the Drone. **The Job survives**, and its worktree is held.
    ///
    /// Where the Job then stands is `crate::aftermath`'s answer and not this
    /// method's: a process that is gone having left no evidence pauses the Job
    /// for a person. That is not terminal, so nothing here ends a Job — and it
    /// is not `running` either, which is the state the milestone refuses.
    ///
    /// **The step moves too, and that is what makes the Job recoverable.** On
    /// both endings that leave a person holding it the step the Drone was on
    /// stops under `drone_killed`; on the third it does not, because evidence
    /// is waiting and something is already queued that will rule on it.
    /// [`Fleet::stopped_by_hand`] is the move, and `#313` is what it cost.
    pub async fn kill_drone(&self, job_id: &JobId) -> Result<Job, Adrift> {
        // Every Drone beside the kept one first, and outside its slot: the
        // step they work stops with it. Slice 5.
        self.end_the_crew(job_id, ExtraEnded::Killed).await;
        if let Some(slot) = self.slot_of(job_id).await {
            let mut working = slot.lock().await;
            if working.as_ref().is_some_and(|at_work| at_work.is(job_id)) {
                let ending = Ending::of(
                    &working
                        .as_ref()
                        .expect("the slot was just read as full")
                        .heard(),
                );
                let standing = self.load(job_id).await?.status();
                self.end_the_drone(&mut working).await;
                let job = self.load(job_id).await?;
                match aftermath(standing, &ending, self.left(job_id)) {
                    // **The step first, and the order is `crate::dispatch`'s.**
                    // The inner machine is frozen the moment the Job leaves
                    // `running`, so a step stopped after the move would be
                    // refused and `last_verdict` would stay unwritten — which
                    // is the whole of what this call used to leave behind.
                    Aftermath::JobMoves(target) => {
                        let job = self.stopped_by_hand(&job).await?;
                        self.move_job(&job, target, Actor::Human).await?;
                    }
                    // The Job had already stopped and stays where it is; its
                    // step has not, and it is the same reading that is now
                    // wrong. `escalated` freezes the inner machine, so this one
                    // crosses on `step_machine`'s named exception rather than
                    // on the order above.
                    Aftermath::AlreadyStopped => {
                        self.stopped_by_hand(&job).await?;
                    }
                    Aftermath::TheGateDecides => {}
                }
            }
        }
        self.load(job_id).await
    }

    /// End one process the Job holds, and every process under it. `#1647`.
    ///
    /// **The pid is a name, never a grant.** The Job's tree is read again here,
    /// at the act, and a pid not in it is refused with
    /// [`Adrift::NotTheJobsProcess`] and nothing is signalled. That one reading
    /// is also what catches a pid that exited after the screen was drawn and
    /// came back as somebody else's process.
    ///
    /// **A child is not the Drone, and nothing here reads it as one.** Killing
    /// a runaway `cargo` under the Drone ends that process and what it started,
    /// leaves the Drone, its step and the Job exactly as they were, and writes
    /// nothing to the record. The Drone sees a tool call fail and carries on;
    /// Fleet watches the Drone's own process and never its children, so there
    /// is no ending for anything to classify. What it started goes too, leaves
    /// first, because a build killed at its root leaves its compilers to the
    /// init process — out of the Job's tree, so out of Pulse and out of reach
    /// of every later kill.
    ///
    /// **The Drone's own pid is [`kill_drone`](Fleet::kill_drone)**, step and
    /// all, so the step never goes on thinking its Drone is alive. Its
    /// descendants go with it, as [`kill_processes`](Fleet::kill_processes)
    /// says, since every process of the Job is under that one.
    pub async fn kill_process(&self, job_id: &JobId, pid: u32) -> Result<Job, Adrift> {
        // A Drone beside the kept one is that Drone stopped alone; a process
        // under one is that process. Slice 5.
        if let Some((drone, own, under)) = self.beside_under(job_id, pid).await {
            if own {
                return self.kill_one_drone(job_id, &drone).await;
            }
            for pid in under.iter().rev().filter_map(|pid| NonZeroU32::new(*pid)) {
                end_the_process(pid);
            }
            return self.load(job_id).await;
        }
        let tree = self.tree_now(job_id).await?;
        match tree.as_ref().map(|tree| (tree, tree.under(pid))) {
            Some((tree, under)) if !under.is_empty() => self.ended(job_id, tree, &under).await,
            _ => Err(Adrift::NotTheJobsProcess {
                job: job_id.clone(),
                pid,
            }),
        }
    }

    /// End every process the Job holds. `#1647`.
    ///
    /// **[`kill_drone`](Fleet::kill_drone), and then what it misses.** The
    /// Drone is in the tree, so its step stops by hand exactly as there. What
    /// this adds is every descendant that left the Drone's process group — a
    /// tool that started its own, as a shell with job control does — which the
    /// group signal never reaches and which would otherwise outlive the Drone,
    /// reparented and invisible. Those go first, leaves before parents, so
    /// nothing is orphaned while the Drone is still ending.
    ///
    /// **Not [`kill_job`](Fleet::kill_job)**: the Job survives, with its
    /// worktree held, and a restart lands on the stopped step. Where the tree
    /// will not read, or the Drone is gone, this is `kill_drone` alone.
    pub async fn kill_processes(&self, job_id: &JobId) -> Result<Job, Adrift> {
        match self.tree_now(job_id).await? {
            Some(tree) => self.ended(job_id, &tree, &tree.under(tree.root)).await,
            None => self.kill_drone(job_id).await,
        }
    }

    /// Signal each of `under`, children before parents, and where the Drone is
    /// among them end it through [`kill_drone`](Fleet::kill_drone).
    async fn ended(&self, job_id: &JobId, tree: &Tree, under: &[u32]) -> Result<Job, Adrift> {
        let fleet = std::process::id();
        for pid in under.iter().rev() {
            if *pid != tree.root && *pid != fleet {
                if let Some(pid) = NonZeroU32::new(*pid) {
                    end_the_process(pid);
                }
            }
        }
        if !under.contains(&tree.root) {
            return self.load(job_id).await;
        }
        let job = self.kill_drone(job_id).await?;
        // A Drone this Fleet does not hold in a slot — one the record names
        // and adoption never took back — is out of `kill_drone`'s reach. It is
        // still the Job's, and still what was asked for.
        if tree.root_remains() {
            if let Some(root) = NonZeroU32::new(tree.root) {
                end_the_process(root);
            }
        }
        Ok(job)
    }

    /// Which refusal a process kill is: a pid outside the tree is its own,
    /// and everything else is [`Fleet::refusal`]'s.
    ///
    /// **A 409, not a 404 or a 422.** The usual way here is a row that was
    /// true when it was drawn — the process exited, or a build moved on — and
    /// the answer to that is the one every status conflict gets: read again,
    /// and decide again. The pid comes back on its own field.
    pub(crate) fn killing_refusal(&self, why: Adrift) -> Refusal {
        match &why {
            Adrift::NotTheJobsProcess { job, pid } => Refusal::IllegalMove(
                WireError::raised(NOT_THE_JOBS_PROCESS, why.to_string(), self.run_id())
                    .about_job(ipc::JobId::from(job))
                    .with_field("pid", WireValue::Int(i64::from(*pid))),
            ),
            _ => self.refusal(why),
        }
    }

    /// End the Job at `killed`. Terminal, and carrying no verdict.
    ///
    /// Legal from every non-terminal status, including those with no process
    /// under them — which is why it cannot be spelled as
    /// [`kill_drone`](Fleet::kill_drone).
    pub async fn kill_job(&self, job_id: &JobId) -> Result<Job, Adrift> {
        self.end_the_crew(job_id, ExtraEnded::Killed).await;
        if let Some(slot) = self.slot_of(job_id).await {
            let mut working = slot.lock().await;
            if working.as_ref().is_some_and(|at_work| at_work.is(job_id)) {
                self.end_the_drone(&mut working).await;
            }
        }
        // And every step the *record* still names one on, which the slot cannot
        // answer for: a Fleet that died holding a Drone leaves the pointer set,
        // and `redispatch` reaches this on a Job whose process this Fleet never
        // held. Without it a killed Job reads on the Board as one with a Drone
        // still on it. `drone_left` answers `Ok` for a step holding nothing, so
        // the ordinary path pays one load.
        self.every_exit_recorded(job_id).await?;
        let job = self.load(job_id).await?;
        let killed = self.move_job(&job, Target::Killed, Actor::Human).await?;
        // **The kill is not deferred and the admission after it is** — `#428`,
        // and the sharpest instance of it: killing one Job used to be able to
        // stop the next one starting, because `admit_next` runs a whole
        // dispatch and this one ran inside a request a client could abandon.
        // The Job is `killed` before this returns; the place it gave back is
        // filled by the turn.
        Ok(killed)
    }

    /// Delete the Job's whole record. **Real deletion**, through
    /// `Store::forget_job`, and only from a terminal status — a Job still in
    /// flight has no record to erase, only a status to move, and `kill_job` is
    /// the act that ends one that is not there yet.
    ///
    /// **It does not reclaim the worktree or the branch.** `armada clean`
    /// already owns that, on its own retention schedule; folding it in here
    /// would give one call two unrelated things to fail at.
    ///
    /// `Store::forget_job` runs through the same lock every other write
    /// takes — there is no second connection opened for it, which is what
    /// makes this safe to call from inside a live Fleet in the first place.
    pub async fn forget_job(&self, job_id: &JobId) -> Result<(), Adrift> {
        let job = self.load(job_id).await?;
        if !job.status().is_terminal() {
            return Err(Adrift::NotForgettable {
                job: job_id.clone(),
                status: job.status(),
            });
        }
        // Before the record goes: a lease held for a Job nobody can name again
        // would never be given back. Refused, it reads `kept` in `--status`.
        self.released_or_saved(&job).await;
        self.store()
            .lock()
            .await
            .forget_job(job_id)
            .map_err(Adrift::Writing)?;
        // The walk frames go with the rows that named them. Best effort: a
        // directory left behind is disk, and the record is already gone.
        let _ = std::fs::remove_dir_all(self.walk_frames(job_id));
        self.publish(ipc::Event::JobForgotten(ipc::JobForgotten {
            job_id: job_id.into(),
        }));
        Ok(())
    }

    /// Stop the step a person has just taken the Drone off.
    ///
    /// **What it has to get right is `Fleet::stopped_step`'s contract**, not
    /// the process's: a `stopped` row carrying `failed(<trigger>)` is what
    /// `restart_step` reads, and writing one without the gate having ruled on
    /// anything is the only place in Fleet that happens.
    ///
    /// **`Ok` and unchanged where no step is running**, which is not a fault
    /// and is the ordinary shape at a step boundary and on a Job whose steps
    /// have all advanced. `crate::drone_moves` answers the same way about a
    /// step holding no Drone, for the same reason: what a kill can be sure of
    /// is the process, and everything else is read off the record.
    pub(crate) async fn stopped_by_hand(&self, job: &Job) -> Result<Job, Adrift> {
        let why = StepLevelTrigger::of(EscalationTrigger::DroneKilled)
            .expect("`drone_killed` is step-level in the registry");
        // **Human, not Fleet.** This act exists because somebody pressed
        // something, and a row saying Fleet took the process away would claim a
        // decision it did not make. [`Fleet::stopped_at_rest`] is the row that
        // legitimately says Fleet, and the actor is the only difference between
        // the two calls.
        self.stopped_step_under(job, why, Actor::Human).await
    }

    /// Stop the step whose Drone was already gone when a person pressed
    /// restart. `#1034`.
    ///
    /// **Human, and there is no Drone to end.** It left on its own, or was
    /// lost to a Fleet restart, before anybody acted — `drone_killed` would
    /// claim a person killed a process that had already gone, which is
    /// exactly the misnaming [`stopped_by_hand`](Fleet::stopped_by_hand)'s own
    /// doc warns against one trigger over.
    pub(crate) async fn stopped_abandoned(&self, job: &Job) -> Result<Job, Adrift> {
        let why = StepLevelTrigger::of(EscalationTrigger::DroneGone)
            .expect("`drone_gone` is step-level in the registry");
        self.stopped_step_under(job, why, Actor::Human).await
    }

    /// Take away a Drone whose own run has ended, and stop the step it was on.
    ///
    /// **The only act on this ladder Fleet asks for**, and `DroneEvent::Ended`
    /// is what makes it defensible rather than a judgement: the Drone said its
    /// own run was over, and anything it does after saying so is outside what
    /// it told Fleet. The reading is `crate::silence`'s and so is its argument.
    ///
    /// **The cost is written down rather than discovered.** A Drone that
    /// reports its run has ended and then keeps working loses that work. It is
    /// accepted because the alternative is the row this reaps: a step saying
    /// `running` beneath a Drone that has said it is finished.
    ///
    /// **The order is `kill_drone`'s, and load-bearing for the same reason.**
    /// The process, then the step, then the Job — the inner machine freezes the
    /// moment the Job leaves `running`, so a step stopped after the move is
    /// refused and `last_verdict` stays unwritten. The step stops while the Job
    /// is still `running`, so this needs no `step_machine` exception of its own.
    ///
    /// **It will not hang on a process that will not go**, which is `#371`.
    /// `DroneSession::terminate` sends `SIGKILL`, uncatchable, and waits on
    /// Fleet's own direct child. The pipe is the other wait — a tool the Drone
    /// spawned can hold it open — and [`drained`](crate::Watching::drained)
    /// bounds it and says whether it got to the end. Both answers ride out on
    /// [`StoodDown`]: a slot handed back over a transcript cut short must not
    /// be handed back silently.
    pub(crate) async fn drone_at_rest(
        &self,
        target: Target,
        working: &mut Option<Working>,
    ) -> Result<Option<StoodDown>, Adrift> {
        let Some(at_work) = working.take() else {
            return Ok(None);
        };
        let job_id = at_work.standing().0;
        // **Not `boundary::stood_down`, and the difference is one log line.**
        // That one writes "the Drone was ended because its step ended", which
        // is the sentence this whole defect is about being false. What both
        // reach for is `stood_down_paying`, which is the ending and the spend
        // in the one order that makes the figure real; the reason a Drone was
        // ended belongs on `noted_stood_down` as a parameter rather than in a
        // second copy of that sentence here.
        let stood_down = self.stood_down_paying(at_work).await?;
        self.every_exit_recorded(&job_id).await?;
        let job = self.load(&job_id).await?;
        let job = self.stopped_at_rest(&job).await?;
        self.move_job(&job, target, Actor::Fleet).await?;
        // Ordinarily nothing, for `dispatch::reap`'s reason: the reading that
        // reaches here is one where `left` answered `Left::Nothing`. It is
        // called anyway because a Drone that submitted between that reading and
        // this line left evidence against a step that has now stopped, and a
        // drop nobody wrote down is the defect that pair closes.
        let dropped = self.empty_the_inbox(&job_id).await;
        self.dropped_with_the_job(&job_id, dropped);
        Ok(Some(stood_down))
    }

    /// Take away a Drone whose outstanding permission ask nobody answered
    /// past its own bound, stop the step it was on, and escalate the Job
    /// under the same trigger. `#801`.
    ///
    /// **A third rung beside [`kill_drone`](Fleet::kill_drone) and
    /// [`drone_at_rest`](Fleet::drone_at_rest), Fleet's own act like the
    /// second.** Unlike either, nothing here needs `crate::drone::aftermath`
    /// to classify what happened: a person's silence is the whole of why the
    /// Job stopped, so the step and the Job take the one trigger rather than
    /// folding an ending's classification into a step-level row apart from a
    /// Job-level reading.
    ///
    /// **The refusal is written before the process ends**, the same
    /// `refused_by_fleet` write the ordinary deny path uses, so the command
    /// is on the transcript beside this row's own `stopped` line.
    pub(crate) async fn ended_unanswered(
        &self,
        working: &mut Option<Working>,
        waiting: Waiting,
    ) -> Result<(), Adrift> {
        let Some(at_work) = working.as_ref() else {
            return Ok(());
        };
        let job_id = at_work.standing().0;
        let words = Refusing::Unanswered.to_the_drone(&waiting.command);
        at_work.refused_by_fleet(&waiting.tool, &waiting.call, &words);
        self.end_the_drone(working).await;
        let job = self.load(&job_id).await?;
        let why = StepLevelTrigger::of(EscalationTrigger::AskUnanswered)
            .expect("`ask_unanswered` is step-level in the registry");
        let job = self.stopped_step_under(&job, why, Actor::Fleet).await?;
        self.move_job(
            &job,
            Target::Escalated(EscalationTrigger::AskUnanswered),
            Actor::Fleet,
        )
        .await?;
        Ok(())
    }

    /// Stop the step Fleet has just taken a finished Drone off.
    ///
    /// **Fleet, and this is the one row where that is true of a Drone being
    /// taken away.** `drone_killed` is a person's decision and says so;
    /// `run_ended` is Fleet acting on the Drone's own last word, and a reader
    /// who opens it finds nobody to ask about it.
    ///
    /// **`crate::dispatch::reap` and `crate::daemon::answering`'s restart
    /// reconciliation call this too**, for `#792`'s reason: a Drone that ends
    /// its own run without `drone_at_rest` ever seeing it — because the
    /// process was already gone by the time Fleet asked, on a live turn or
    /// across a restart — used to leave its step `running` under a Job that
    /// had already moved past it. Both call this before `move_job`, in the
    /// order this method's own doc above already argues for.
    pub(crate) async fn stopped_at_rest(&self, job: &Job) -> Result<Job, Adrift> {
        let why = StepLevelTrigger::of(EscalationTrigger::RunEnded)
            .expect("`run_ended` is step-level in the registry");
        self.stopped_step_under(job, why, Actor::Fleet).await
    }

    /// Stop the step a Drone was working, under whichever trigger took it away.
    ///
    /// **What it has to get right is `Fleet::stopped_step`'s contract**, not
    /// the process's: a `stopped` row carrying `failed(<trigger>)` is what
    /// `restart_step` reads, and writing one without the gate having ruled on
    /// anything is the only place in Fleet that happens.
    ///
    /// **`Ok` and unchanged where no step is running**, which is not a fault
    /// and is the ordinary shape at a step boundary and on a Job whose steps
    /// have all advanced. `crate::drone_moves` answers the same way about a
    /// step holding no Drone, for the same reason: what an ending can be sure
    /// of is the process, and everything else is read off the record.
    async fn stopped_step_under(
        &self,
        job: &Job,
        why: StepLevelTrigger,
        by: Actor,
    ) -> Result<Job, Adrift> {
        let Some(step) = job
            .current_step()
            .filter(|row| row.state() == StepState::Running)
            .map(|row| row.step_id().clone())
        else {
            return Ok(job.clone());
        };
        self.move_step_by(job, &step, StepTarget::Stopped(why), by)
            .await
    }
}
