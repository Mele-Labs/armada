//! A Job held up by its own worktree or environment, repaired by a Drone before
//! a person is asked. `docs/concepts/fleet.md`, *Self-healing*, holds the rules:
//! two findings and one mechanism, Fleet reads whether it worked, and the
//! repairs are bounded per Job.

mod finding;

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;
use std::sync::Arc;

use adapter_traits::{
    AgentHarness, Delivery, DroneEvent, Grant, Prompt, Speaker, Toolbelt, Vcs, WorkProduct,
    Worktree,
};
use core_model::{Component, Envelope, FieldValue, Job, JobId, Level, StepId};
use tokio::io::{AsyncBufReadExt, BufReader};

pub(crate) use finding::{Finding, Kind};

use crate::daemon::Fleet;
use crate::drone::{self, Started};
use crate::gate::Ruling;
use crate::session::LiveSession;

/// Repairs of one kind a Job may have. Two, because the first can fail for a
/// reason the second does not (a network that was down), and a third is a
/// repair that does not hold.
pub(crate) const HEALS_PER_KIND: u32 = 2;

/// Repairs spent, per Job and kind. Held in memory: a Fleet that restarts
/// forgets, and the worst that costs is one more repair.
#[derive(Default)]
pub(crate) struct Heals(BTreeMap<(JobId, Kind), u32>);

impl Heals {
    /// Take one repair, or `false` where the Job has spent them.
    fn spend(&mut self, job: &JobId, kind: Kind) -> bool {
        let spent = self.0.entry((job.clone(), kind)).or_insert(0);
        *spent += 1;
        *spent <= HEALS_PER_KIND
    }
}

/// A repair that did not come right, in the words a person reads on the Job.
#[derive(Debug)]
pub struct NotHealed {
    pub found: String,
    pub tried: String,
}

impl fmt::Display for NotHealed {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            out,
            "the worktree was held up and a repair Drone did not put it right. Found: {}. \
             Tried: {}",
            self.found, self.tried
        )
    }
}

impl std::error::Error for NotHealed {}

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
    /// Repair an index a Drone's resolution left unstaged, where there is one.
    /// Asked before a Drone is put on the worktree and before the gate reads
    /// the diff.
    ///
    /// `conflicted` is what a catch-up has just reported and a Drone is still
    /// to resolve: those paths are the Drone's work and are left alone.
    pub(crate) async fn index_healed(
        &self,
        job: &Job,
        step: &StepId,
        worktree: &Worktree,
        conflicted: &[String],
    ) -> Result<(), NotHealed> {
        let Some(finding) = self.unstaged_in(worktree, conflicted) else {
            return Ok(());
        };
        self.healed(job, step, worktree, &finding).await?;
        // Read again by Fleet: a Drone saying it staged them is a signal.
        match self.unstaged_in(worktree, conflicted) {
            None => Ok(()),
            Some(left) => Err(NotHealed {
                found: finding.said(),
                tried: format!("a repair Drone ran and {} is still unmerged", left.said()),
            }),
        }
    }

    fn unstaged_in(&self, worktree: &Worktree, conflicted: &[String]) -> Option<Finding> {
        // A read that fails is no finding: the gate's own reading fails the
        // same way and says so.
        let unmerged = self.work().unmerged_paths(worktree).ok()?;
        Finding::index_in(Path::new(worktree.path()), &unmerged, conflicted)
    }

    /// Repair the install a failed Check named, where `ruling` carries one.
    /// `true` means a repair Drone ran to its end and the caller should run the
    /// gate again; the Check is what says whether it worked.
    pub(crate) async fn install_healed(
        &self,
        job: &Job,
        step: &StepId,
        worktree: &Worktree,
        ruling: &Ruling,
    ) -> bool {
        if ruling.advanced() {
            return false;
        }
        let printed = ruling
            .output()
            .iter()
            .map(|kept| format!("{}\n{}", kept.output.stdout, kept.output.stderr))
            .collect::<Vec<_>>()
            .join("\n");
        let Some(finding) = Finding::install_in(&printed) else {
            return false;
        };
        match self.healed(job, step, worktree, &finding).await {
            Ok(()) => true,
            Err(not) => {
                self.noted_not_healed(job.id(), step, &not);
                false
            }
        }
    }

    /// Put a repair Drone on the worktree for `finding` and wait for it.
    async fn healed(
        &self,
        job: &Job,
        step: &StepId,
        worktree: &Worktree,
        finding: &Finding,
    ) -> Result<(), NotHealed> {
        let not = |tried: String| NotHealed {
            found: finding.said(),
            tried,
        };
        let allowed = self
            .heals()
            .lock()
            .expect("not poisoned")
            .spend(job.id(), finding.kind());
        if !allowed {
            return Err(not(format!(
                "{HEALS_PER_KIND} repairs of this kind already ran on this Job"
            )));
        }
        let bootstrap: Vec<String> = match self.effective_manifest(job).await {
            Ok((manifest, _)) => manifest
                .prepared_by()
                .iter()
                .map(|command| command.run().to_string())
                .collect(),
            Err(_) => Vec::new(),
        };
        let repairs = finding.repairs(&bootstrap);
        if repairs.is_empty() {
            return Err(not(String::from(
                "the Manifest declares no bootstrap command to run again",
            )));
        }
        // Reading and the repair's own commands, and no file writes: a repair
        // Drone that could edit would be a second Drone doing the step.
        let belt = repairs.into_iter().fold(
            Toolbelt::evidence_only()
                .and(Grant::ReadTheWorktree)
                .and(Grant::ReadTheRepository),
            |belt, repair| belt.and(Grant::RepairTheWorktree(repair)),
        );
        let prompt = Prompt::assembled(&finding.told(&bootstrap))
            .map_err(|why| not(format!("the brief would not render: {why:?}")))?;
        let config = self
            .spawn_config_with(job, step, worktree, prompt, None, belt)
            .await
            .map_err(|why| not(format!("the repair Drone would not configure: {why:?}")))?;
        self.noted_healing(
            job.id(),
            step,
            "a repair Drone is being put on the worktree before a person is asked",
            finding,
            None,
        );
        let said = self.repair_run(&config).await.map_err(not)?;
        self.noted_healing(
            job.id(),
            step,
            "the repair Drone came back",
            finding,
            said.as_deref(),
        );
        Ok(())
    }

    /// Start the repair Drone, read it until its turn ends, and end it. What it
    /// last said comes back for the log and is read as nothing else.
    pub(crate) async fn repair_run(
        &self,
        config: &adapter_traits::DroneSpawnConfig,
    ) -> Result<Option<String>, String> {
        self.repair_run_spending(config).await.map(|(said, _)| said)
    }

    /// [`repair_run`](Fleet::repair_run) with what the Drone spent, for a run
    /// that counts against its Job: a side Drone's.
    pub(crate) async fn repair_run_spending(
        &self,
        config: &adapter_traits::DroneSpawnConfig,
    ) -> Result<(Option<String>, store::DroneSpend), String> {
        let started = std::time::Instant::now();
        let Started {
            session,
            transcript,
            mut complaints,
            ..
        } = drone::start(self.harness().as_ref(), config)
            .await
            .map_err(|why| format!("the repair Drone would not start: {why:?}"))?;
        tokio::spawn(async move {
            let _ = tokio::io::copy(&mut complaints, &mut tokio::io::sink()).await;
        });
        let harness = Arc::clone(self.harness());
        let reading = async move {
            let (mut said, mut ended) = (None, Vec::new());
            let mut lines = BufReader::new(transcript).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                for event in harness.read(&line) {
                    match event {
                        DroneEvent::Said {
                            text,
                            by: Speaker::Drone,
                        } => said = Some(text),
                        DroneEvent::Ended { .. } => {
                            ended.push(event);
                            return (said, ended);
                        }
                        _ => {}
                    }
                }
            }
            (said, ended)
        };
        let budget = self.budget().duration();
        let outcome = tokio::time::timeout(budget, reading).await;
        let _ = session.terminate().await;
        let (said, ended) =
            outcome.map_err(|_| format!("the repair Drone ran past {}s", budget.as_secs()))?;
        Ok((said, crate::allowance::spent(&ended, started.elapsed())))
    }

    fn noted_healing(
        &self,
        job: &JobId,
        step: &StepId,
        said: &str,
        finding: &Finding,
        drone_said: Option<&str>,
    ) {
        let mut envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            said,
        )
        .in_job(job.as_ulid().clone())
        .at_step(step.as_str())
        .with_field("found", FieldValue::Str(finding.said()));
        if let Some(text) = drone_said {
            envelope = envelope.with_field("drone_said", FieldValue::Str(text.to_string()));
        }
        self.noted_in_the_log(job, &envelope);
    }

    pub(crate) fn noted_not_healed(&self, job: &JobId, step: &StepId, not: &NotHealed) {
        let envelope = Envelope::new(
            self.now(),
            Level::Warn,
            Component::Fleet,
            self.run().clone(),
            "a repair Drone did not put the worktree right",
        )
        .in_job(job.as_ulid().clone())
        .at_step(step.as_str())
        .with_field("found", FieldValue::Str(not.found.clone()))
        .with_field("tried", FieldValue::Str(not.tried.clone()));
        self.noted_in_the_log(job, &envelope);
    }
}
