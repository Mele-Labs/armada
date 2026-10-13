//! A repair Drone on a Job's own branch, sent by its Commander. `#2170`.
//! `docs/concepts/commander.md`, *The repair*, has the design.
//!
//! Not `crate::repairing`, which is a failed Trigger's, on a side branch the
//! owner places. This writes on the Job's branch in the Job's worktree and the
//! stopped step restarts over it, so the step's own Checks and Judge rule on
//! the work. It never calls `healing::healed`: the loop bound in
//! `crate::commanding_wakes` is the only count a repair spends.
//!
//! **A Job whose preparation failed has no stopped step**, and it is put back
//! the other way: queued, to be prepared and begun again as the Commander's
//! move, with the first step's baseline read before the repair so its file
//! counts as that step's work. `Stuck::unprepared` says which Jobs.

use std::sync::Arc;

use adapter_traits::{
    AgentHarness, Delivery, Grant, Prompt, Repair, Toolbelt, Vcs, WorkProduct, Worktree,
};
use config::settings as keys;
use core_model::{
    Actor, DroneId, DroneStanding, Job, JobId, Level, Recourse, StepId, StepState, Target,
};
use ipc::Breakage;

use crate::adrift::Adrift;
use crate::commanding_wakes::Trouble;
use crate::daemon::Fleet;
use crate::healing::holds_a_marker;
use crate::prompts::Prompts;
use crate::repair_hold::Held;
use crate::resume::Ending;

/// Where a Check is declared. Held off at launch; the gate refuses the same
/// paths in the diff (`verification::forbidden`).
const NOT_THE_DRONES: [&str; 2] = ["armada.yml", ".armada"];

/// A repair as it is sent: where it works and which step it puts back. There is
/// no branch to name, so it cannot be aimed anywhere but at the Job's own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobRepair {
    worktree: Worktree,
    step: StepId,
    broke: Breakage,
    back: Back,
}

/// Where the Job goes once the repair is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Back {
    /// The step the breakage stopped is restarted.
    ToTheStep,
    /// No step had begun: the Job is queued and met at its first.
    ToTheFirst,
}

impl JobRepair {
    /// `None` where the Job has no step to put back: the one that stopped, or
    /// else the one it is on, or the first of a Job that never began one.
    pub fn onto(job: &Job, worktree: Worktree, broke: Breakage) -> Option<JobRepair> {
        let begun = job
            .steps()
            .iter()
            .any(|step| step.state() != StepState::NotStarted);
        let (step, back) = match job.stopped_on() {
            Some((step, _)) => (step.clone(), Back::ToTheStep),
            None if !begun => (
                job.workflow().steps().first()?.id().clone(),
                Back::ToTheFirst,
            ),
            None => (job.current_step_id().cloned()?, Back::ToTheStep),
        };
        Some(JobRepair {
            worktree,
            step,
            broke,
            back,
        })
    }

    /// The Job's own worktree, on the Job's own branch.
    pub fn worktree(&self) -> &Worktree {
        &self.worktree
    }

    /// The step restarted once the repair is in, or begun for the first time.
    pub fn restarts(&self) -> &StepId {
        &self.step
    }

    pub fn broke(&self) -> Breakage {
        self.broke
    }

    /// `belt` and what this fault needs: the index for a conflict or a broken
    /// worktree, the declared bootstrap for the other two. Never a Manifest.
    pub fn belted(&self, belt: Toolbelt, bootstrap: &[String], manifests: &[String]) -> Toolbelt {
        let belt = match self.broke {
            Breakage::MergeConflict | Breakage::Worktree => {
                belt.and(Grant::RepairTheWorktree(Repair::TheIndex))
            }
            Breakage::Dependencies | Breakage::MissingBinary => bootstrap.iter().fold(belt, |belt, run| {
                belt.and(Grant::RepairTheWorktree(Repair::TheInstall(run.clone())))
            }),
        };
        NOT_THE_DRONES
            .iter()
            .map(|path| path.to_string())
            .chain(manifests.iter().cloned())
            .fold(belt, |belt, path| belt.holding_off(&path))
    }

    /// The Drone's one turn. The heading stays Fleet's, whatever the body says:
    /// it is the line a harness and a fake key on.
    pub fn told(&self, prompts: &Prompts, title: &str) -> String {
        format!(
            "REPAIR THE JOB\n\n{}",
            prompts.fill(
                keys::PROMPT_JOB_REPAIR,
                &[
                    ("job", title),
                    ("branch", self.worktree.branch()),
                    ("step", self.step.as_str()),
                    ("broke", found(self.broke)),
                ],
            )
        )
    }

    /// The commit Fleet makes of what the Drone wrote.
    pub fn commit_message(&self) -> String {
        format!("Repair the worktree: {}", named(self.broke))
    }
}

/// What broke, in a word, for the Job's log and the commit.
pub fn named(broke: Breakage) -> &'static str {
    match broke {
        Breakage::MergeConflict => "merge conflict",
        Breakage::Dependencies => "dependencies",
        Breakage::MissingBinary => "missing binary",
        Breakage::Worktree => "worktree",
    }
}

/// What the Drone is told is wrong, and the one thing to do about it.
fn found(broke: Breakage) -> &'static str {
    match broke {
        Breakage::MergeConflict => {
            "a merge left conflicts in the worktree. Resolve each file so it keeps what the Job \
             needs from both sides, leave no conflict marker in it, and stage it with `git add`"
        }
        Breakage::Dependencies => {
            "a dependency is missing or will not install. Install it with the repository's own \
             commands, run again with a forced reinstall where one takes the flag"
        }
        Breakage::MissingBinary => {
            "a binary a command needs is missing or not on the path. Find which, and install it \
             with the repository's own commands"
        }
        Breakage::Worktree => {
            "the worktree is unusable. Read `git status` and the files to see why, and put it \
             right"
        }
    }
}

/// The repair's body as it ships: `prompts.jobRepair`'s default.
pub(crate) const SHIPPED: &str = "\
This is the Job \"{job}\", on `{branch}`, and its step `{step}` stopped because {broke}.

Change only what the breakage needs, in this worktree. Do not edit `armada.yml` or \
`.armada`, and do not commit, push or open a pull request: Fleet commits what you changed \
and puts the step's Drone back on it. The Job's own Checks and Judge decide whether the \
work is done. Then stop.";

/// How a repair came to an end, in the words the Job's log carries.
enum Came {
    HandedIn,
    Nothing(String),
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
    /// A Commander's repair: refused now where the step could not be restarted
    /// after it, and otherwise on its way, answered with the Drone's id.
    /// Offered exactly where `restart_step` is, by the same reading
    /// (`Stuck::of`), so the act and the restart it ends in cannot disagree.
    pub(crate) async fn repair_sent(
        self: Arc<Self>,
        job_id: &JobId,
        broke: Breakage,
    ) -> Result<ipc::RepairSent, Adrift> {
        let job = self.load(job_id).await?;
        self.held_for_a_person(&job)?;
        let worktree = self.surviving_worktree(&job)?;
        let reason = self.last_reason(job_id).await?;
        let no_step = || Adrift::NoStepStopped {
            job: job_id.clone(),
        };
        let stuck = self
            .why_stuck(&job, reason.as_ref(), &[])
            .await
            .ok_or_else(no_step)?;
        if stuck.standing().drone != DroneStanding::Gone {
            return Err(Adrift::DroneStillThere {
                job: job_id.clone(),
            });
        }
        // A Job that broke before its first step has no restart to offer, and
        // a preparation that failed is the one such Job a repair answers.
        let before_a_step = stuck.unprepared();
        if !before_a_step && !stuck.recourse().contains(&Recourse::RestartStep) {
            return Err(no_step());
        }
        let repair = JobRepair::onto(&job, worktree, broke).ok_or_else(no_step)?;
        let held = self
            .repairing()
            .taking(job_id)
            .ok_or_else(|| Adrift::RepairUnderway {
                job: job_id.clone(),
            })?;
        if before_a_step {
            self.baseline_before_the_repair(job_id, &repair).await?;
        }
        let drone = DroneId::carried(self.mint().ulid());
        let sent = ipc::RepairSent {
            job_id: ipc::JobId::from(job_id),
            step_id: ipc::StepId::from(repair.restarts()),
            drone_id: ipc::DroneId::from(&drone),
            broke,
        };
        // Before the Drone starts, so a Fleet that dies with it out finds the hold.
        self.store()
            .lock()
            .await
            .note_repair_out(&store::RepairOut {
                job_id: job_id.clone(),
                step: repair.restarts().as_str().to_string(),
                broke: named(broke).to_string(),
                drone_id: drone.as_str().to_string(),
            })
            .map_err(Adrift::Writing)?;
        self.noted_setting(
            job_id,
            "the Job's Commander sent a repair Drone onto the Job's branch",
            &[
                ("broke", named(broke).to_string()),
                ("step", repair.restarts().as_str().to_string()),
                ("branch", repair.worktree().branch().to_string()),
                ("drone", drone.as_str().to_string()),
            ],
        )
        .await;
        let fleet = Arc::clone(&self);
        tokio::spawn(async move {
            fleet.repair_made(held, &job, &repair, &drone).await;
        });
        Ok(sent)
    }

    /// The Drone's run, then what it left: the step restarted over it, or the
    /// Commander woken because nothing was handed in.
    async fn repair_made(
        self: &Arc<Self>,
        held: Held,
        job: &Job,
        repair: &JobRepair,
        drone: &DroneId,
    ) {
        let ran = self.repair_run_over(job, repair, drone).await;
        // Given back before the restart, which a repair underway refuses, and
        // before a wake, whose turn may send another.
        drop(held);
        let came = match ran {
            Came::HandedIn => match repair.back {
                Back::ToTheStep => self.step_restarted_over(job, repair).await,
                Back::ToTheFirst => self.first_step_restored(job, repair).await,
            },
            nothing => nothing,
        };
        if let Came::Nothing(why) = came {
            self.nothing_handed_in(job.id(), repair.restarts(), named(repair.broke()), why)
                .await;
        }
        // Last, so a Fleet that dies before it is here finds the hold and deals
        // with it. A row left over a Job that has moved on is dropped at boot.
        self.repair_dealt_with(job.id(), drone.as_str()).await;
    }

    /// A repair that came to nothing: the Job's log says so and the Commander
    /// is woken, which the loop bound counts.
    pub(crate) async fn nothing_handed_in(
        self: &Arc<Self>,
        job: &JobId,
        step: &StepId,
        broke: &str,
        why: String,
    ) {
        self.noted_setting(
            job,
            "a repair Drone ended with nothing handed in",
            &[
                ("broke", broke.to_string()),
                ("step", step.as_str().to_string()),
                ("why", why),
            ],
        )
        .await;
        self.woken_for(job, Trouble::RepairHandedNothingIn { step: step.clone() })
            .await;
    }

    /// The Drone's row given back. A store that will not write leaves it for
    /// the next boot, which finds a Job that has moved on and drops it.
    pub(crate) async fn repair_dealt_with(&self, job: &JobId, drone: &str) {
        let _ = self.store().lock().await.repair_dealt_with(job, drone);
    }

    /// The first step's baseline, read before the repair writes anything. A
    /// step that has not begun has none, and the one read after the repair's
    /// commit would count the repair as the branch's, so its file would never
    /// reach the step's Checks and Judge as work. One already kept stands: an
    /// earlier repair that came to nothing may have left files, and they are
    /// the repair's too.
    async fn baseline_before_the_repair(
        &self,
        job: &JobId,
        repair: &JobRepair,
    ) -> Result<(), Adrift> {
        let kept = self
            .store()
            .lock()
            .await
            .step_baseline(job, repair.restarts());
        if matches!(kept, Ok(Some(_))) {
            return Ok(());
        }
        let read = self
            .work()
            .footprint(repair.worktree())
            .map_err(|cause| Adrift::WorkUnreadable {
                job: job.clone(),
                cause: Box::new(cause),
            })?;
        self.store()
            .lock()
            .await
            .keep_step_baseline(job, repair.restarts(), &read)
            .map_err(Adrift::Writing)
    }

    /// Run the Drone in the Job's worktree and commit what it wrote.
    async fn repair_run_over(&self, job: &Job, repair: &JobRepair, drone: &DroneId) -> Came {
        let at = std::path::Path::new(repair.worktree().path());
        let unmerged = self
            .work()
            .unmerged_paths(repair.worktree())
            .unwrap_or_default();
        let bootstrap: Vec<String> = match self.effective_manifest(job).await {
            Ok((manifest, _)) => manifest
                .prepared_by()
                .iter()
                .map(|command| command.run().to_string())
                .collect(),
            Err(_) => Vec::new(),
        };
        let belt = repair.belted(
            self.repair_belt(job).await,
            &bootstrap,
            &self.workspace_manifests(job),
        );
        let Ok(prompt) = Prompt::assembled(&repair.told(&self.prompts(), job.title().as_str()))
        else {
            return Came::Nothing(String::from("the brief would not render"));
        };
        let config = match self
            .spawn_config_with(job, repair.restarts(), repair.worktree(), prompt, None, belt)
            .await
        {
            Ok(config) => config,
            Err(why) => {
                return Came::Nothing(format!("the repair Drone would not configure: {why:?}"))
            }
        };
        let ran = self.repair_run_spending(&config).await;
        // Counted against the Job whether or not it came right, as a side
        // Drone's is.
        if let Ok((_, spend)) = &ran {
            let _ = self.record_spend(job.id(), drone, spend).await;
        }
        let (said, spend) = match ran {
            Ok(ran) => ran,
            Err(why) => return Came::Nothing(why),
        };
        // A transcript that closed with no turn ending is a Drone that died or
        // gave up, and what it left is not an answer.
        if spend.cost_micros.is_none() && spend.turns == 0 {
            return Came::Nothing(String::from("the Drone stopped before it finished"));
        }
        if let Some(said) = said {
            self.noted_setting(
                job.id(),
                "the repair Drone came back",
                &[("drone_said", said)],
            )
            .await;
        }
        if let Some(left) = unmerged.iter().find(|path| holds_a_marker(&at.join(path))) {
            return Came::Nothing(format!("{left} still holds a conflict marker"));
        }
        if let Err(why) = self.the_fix_committed(repair.worktree(), &repair.commit_message()) {
            return Came::Nothing(why);
        }
        match self.work().unmerged_paths(repair.worktree()) {
            Ok(left) if left.is_empty() => Came::HandedIn,
            Ok(left) => Came::Nothing(format!(
                "the index still holds {} as unmerged",
                left.join(", ")
            )),
            Err(why) => Came::Nothing(format!("the index could not be read: {why}")),
        }
    }

    /// The stopped step, put back as the Commander. The Job's Checks and Judge
    /// rule on it as they rule on any step.
    async fn step_restarted_over(&self, job: &Job, repair: &JobRepair) -> Came {
        match self
            .restart_step_by(job.id(), None, Ending::Unheard, Actor::Commander)
            .await
        {
            Ok(_) => {
                let said = format!(
                    "a repair Drone handed in and step {} was restarted",
                    repair.restarts().as_str()
                );
                self.logged(job.id(), self.trigger_line(job, Level::Info, &said));
                Came::HandedIn
            }
            Err(why) => Came::Nothing(format!("the step could not be restarted: {why}")),
        }
    }

    /// The Job of a failed preparation, queued as the Commander. It is prepared
    /// again and its first step begun as any Job's is: the preparation that
    /// failed is the first thing the repair is judged by.
    async fn first_step_restored(&self, job: &Job, repair: &JobRepair) -> Came {
        let record = match self.load(job.id()).await {
            Ok(record) => record,
            Err(why) => return Came::Nothing(format!("the Job could not be read: {why}")),
        };
        if let Err(why) = self.held_for_a_person(&record) {
            return Came::Nothing(format!("the Job is no longer held: {why}"));
        }
        match self.move_job(&record, Target::Queued, Actor::Commander).await {
            Ok(_) => {
                let said = format!(
                    "a repair Drone handed in and the Job was queued for its first step {}",
                    repair.restarts().as_str()
                );
                self.logged(job.id(), self.trigger_line(job, Level::Info, &said));
                Came::HandedIn
            }
            Err(why) => Came::Nothing(format!("the Job could not be queued: {why}")),
        }
    }

    /// Every workspace's Manifest as a path in the worktree, where that
    /// workspace's Checks are declared.
    fn workspace_manifests(&self, job: &Job) -> Vec<String> {
        let Ok(served) = self.served_by(job) else {
            return Vec::new();
        };
        let workspaces = served.workspaces();
        workspaces
            .iter()
            .map(|manifest| manifest.dir().to_string())
            .filter(|dir| !dir.is_empty() && dir != ".")
            .map(|dir| format!("{}/armada.yml", dir.trim_end_matches('/')))
            .collect()
    }
}
