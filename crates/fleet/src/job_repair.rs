//! A repair Drone on a Job's own branch, sent by its Commander. `#2170`.
//! `docs/concepts/commander.md`, *The repair*, has the design.
//!
//! Not `crate::repairing`, which is a failed Trigger's, on a side branch the
//! owner places. This writes on the Job's branch in the Job's worktree and the
//! stopped step restarts over it, so the step's own Checks and Judge rule on
//! the work. It never calls `healing::healed`: the loop bound in
//! `crate::commanding_wakes` is the only count a repair spends.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use adapter_traits::{
    AgentHarness, Delivery, Grant, Prompt, Repair, Toolbelt, Vcs, WorkProduct, Worktree,
};
use config::settings as keys;
use core_model::{Actor, DroneId, DroneStanding, Job, JobId, Level, Recourse, StepId};
use ipc::Breakage;

use crate::adrift::Adrift;
use crate::commanding_wakes::Trouble;
use crate::daemon::Fleet;
use crate::healing::holds_a_marker;
use crate::prompts::Prompts;
use crate::resume::Ending;

/// Where a Check is declared. Held off at launch; the gate refuses the same
/// paths in the diff (`verification::forbidden`).
const NOT_THE_DRONES: [&str; 2] = ["armada.yml", ".armada"];

/// The Jobs a repair Drone is on, so a second is refused and a person's
/// restart does not put a step Drone beside it. In memory: a Fleet that
/// restarts ends the Drone with the process that read it.
#[derive(Clone, Default)]
pub(crate) struct Underway(Arc<Mutex<BTreeSet<JobId>>>);

impl Underway {
    fn taking(&self, job: &JobId) -> Option<Held> {
        let mut held = self.0.lock().expect("not poisoned");
        held.insert(job.clone()).then(|| Held {
            all: self.clone(),
            job: job.clone(),
        })
    }

    pub(crate) fn holds(&self, job: &JobId) -> bool {
        self.0.lock().expect("not poisoned").contains(job)
    }
}

/// A Job's hold, given back when the repair task ends however it ends.
pub(crate) struct Held {
    all: Underway,
    job: JobId,
}

impl Drop for Held {
    fn drop(&mut self) {
        self.all.0.lock().expect("not poisoned").remove(&self.job);
    }
}

/// A repair as it is sent: where it works and which step it puts back. There is
/// no branch to name, so it cannot be aimed anywhere but at the Job's own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobRepair {
    worktree: Worktree,
    step: StepId,
    broke: Breakage,
}

impl JobRepair {
    /// `None` where the Job has no step to put back: the one that stopped, or
    /// else the one it is on.
    pub fn onto(job: &Job, worktree: Worktree, broke: Breakage) -> Option<JobRepair> {
        let step = job
            .stopped_on()
            .map(|(step, _)| step.clone())
            .or_else(|| job.current_step_id().cloned())?;
        Some(JobRepair {
            worktree,
            step,
            broke,
        })
    }

    /// The Job's own worktree, on the Job's own branch.
    pub fn worktree(&self) -> &Worktree {
        &self.worktree
    }

    /// The step restarted once the repair is in.
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
        if !stuck.recourse().contains(&Recourse::RestartStep) {
            return Err(no_step());
        }
        let repair = JobRepair::onto(&job, worktree, broke).ok_or_else(no_step)?;
        let held = self
            .repairing()
            .taking(job_id)
            .ok_or_else(|| Adrift::RepairUnderway {
                job: job_id.clone(),
            })?;
        let drone = DroneId::carried(self.mint().ulid());
        let sent = ipc::RepairSent {
            job_id: ipc::JobId::from(job_id),
            step_id: ipc::StepId::from(repair.restarts()),
            drone_id: ipc::DroneId::from(&drone),
            broke,
        };
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
            Came::HandedIn => self.step_restarted_over(job, repair).await,
            nothing => nothing,
        };
        if let Came::Nothing(why) = came {
            self.noted_setting(
                job.id(),
                "a repair Drone ended with nothing handed in",
                &[
                    ("broke", named(repair.broke()).to_string()),
                    ("step", repair.restarts().as_str().to_string()),
                    ("why", why),
                ],
            )
            .await;
            self.woken_for(
                job.id(),
                Trouble::RepairHandedNothingIn {
                    step: repair.restarts().clone(),
                },
            )
            .await;
        }
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
