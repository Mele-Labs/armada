//! Handing a repository's red main to a Job. `docs/concepts/fleet.md`, *When
//! main goes red*.
//!
//! **Three ways in, one record.** A person dispatches a new Job, a person sends
//! the work back to an earlier one, or Fleet itself sends it back to the Job
//! whose pull request turned main red. Each ends with a row in `main_ci_fixes`,
//! which is what `hub.fixing` and a Job's `fixes_main` are read from, and what
//! says once per red per Job.
//!
//! **A send-back reuses what already sends work back.** A Job at its review
//! gate takes the facts as `request_changes` takes a note. A Job that has ended
//! is continued the way a redispatch continues one: a new Job on a fresh branch
//! from main, `redispatched_from` naming the first, since the registry never
//! reopens a Job under its own id. A Job still working is refused.
//!
//! **A take is claimed in the central store first** (`crate::red_claim`), so a
//! Fleet on another machine refuses a second fix. The claim comes before
//! `approve`, the send-back and the minting, and a Job proposed or minted for a
//! red that turns out to be held is stopped.
//!
//! **A claim only where a Check and a test are both known**, as `draft_fix`
//! claims one, so a Drone that hits the same test is pointed at this Job. It
//! names no files: a log gives a test's name and not its path, so nothing is
//! held off a file.

use adapter_traits::{AgentHarness, Delivery, Holder, Vcs, WorkProduct};
use core_model::{
    AcceptanceCriterion, Actor, Breakage, BreakageClaim, CriterionId, CriterionOrigin,
    CriterionSource, Facts, Job, JobId, JobStatus,
};
use store::{MainCi, MainFix, TakenHow};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::drafting::StatedBy;
use crate::main_ci::{MainChange, MainChanged};
use crate::red_claim::Taker;
use crate::redispatch::Carrying;
use crate::repositories::Served;
use crate::resume::Redirection;

/// The workflow a Job dispatched for a red runs under.
const FIX_WORKFLOW: &str = "bug";

/// The last lines of the failing job's log the Job is handed, beside its address.
const LOG_LINES: usize = 60;

/// Why a red could not be handed to a Job.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MainNotFixable {
    /// This Fleet does not serve the repository.
    NotServed,
    /// Main is not red, or Fleet has not read it.
    NotRed,
    /// A newer commit's CI is still running on main, and may already have
    /// fixed it.
    ChecksRunning,
    /// A Job is already working on this red.
    Taken(JobId),
    /// The repository holds no workflow to dispatch a fix under.
    NoWorkflow,
    /// A Job that is neither at its review nor over.
    CannotSendBack { job: JobId, status: JobStatus },
    /// A brief with nothing in it.
    Blank,
    /// Another machine's Job or a Session holds this red in the central store.
    /// `held_for` is whole seconds.
    TakenElsewhere { holder: Holder, held_for: i64 },
    /// The central store could not be asked or said no, so the red was not
    /// claimed. A fix nobody else can see is the double fix the claim stops.
    ClaimNotMade(String),
}

impl std::fmt::Display for MainNotFixable {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MainNotFixable::NotServed => write!(out, "this Fleet does not serve that repository"),
            MainNotFixable::NotRed => write!(out, "main is not red"),
            MainNotFixable::ChecksRunning => {
                write!(out, "new checks are running on main, and may have fixed it")
            }
            MainNotFixable::Taken(job) => {
                write!(out, "{} is already working on this red", job.as_str())
            }
            MainNotFixable::NoWorkflow => write!(
                out,
                "the repository holds no `{FIX_WORKFLOW}` workflow to dispatch a fix under"
            ),
            MainNotFixable::CannotSendBack { job, status } => write!(
                out,
                "{} is {}. Work goes back to a Job at its review or one that has ended",
                job.as_str(),
                status.as_wire()
            ),
            MainNotFixable::Blank => write!(out, "the brief is blank"),
            MainNotFixable::TakenElsewhere { holder, held_for } => write!(
                out,
                "{} {} on {} has held this red for {}. Take it over to replace that claim",
                match holder.kind {
                    adapter_traits::HolderKind::Session => "Session",
                    adapter_traits::HolderKind::Job => "Job",
                },
                holder.id,
                holder.machine,
                aged(*held_for)
            ),
            MainNotFixable::ClaimNotMade(said) => write!(out, "the red was not taken. {said}"),
        }
    }
}

/// A claim's age, to its largest whole unit.
fn aged(seconds: i64) -> String {
    match seconds {
        0..=59 => format!("{seconds} s"),
        60..=3599 => format!("{} min", seconds / 60),
        3600..=86_399 => format!("{} h", seconds / 3600),
        _ => format!("{} d", seconds / 86_400),
    }
}

/// Whether a take of a red still counts: the Job has not been stopped.
pub(crate) fn stopped(status: JobStatus) -> bool {
    matches!(
        status,
        JobStatus::Killed | JobStatus::CompletedFailed | JobStatus::Rejected
    )
}

/// The facts of a red, one line each, as the band shows them.
pub(crate) fn brief_of(main: &MainCi) -> String {
    let mut lines = Vec::new();
    for job in &main.failed {
        lines.push(format!(
            "{} fails on main.",
            job.check.as_deref().unwrap_or(&job.name)
        ));
        lines.extend(job.tests.iter().map(|test| format!("Test: {test}")));
    }
    if let Some(merge) = &main.merge {
        lines.push(format!(
            "Merged in #{}{}.",
            merge.number,
            merge
                .branch
                .as_ref()
                .map_or(String::new(), |branch| format!(" ({branch})"))
        ));
    }
    lines.join("\n")
}

/// The first failed job's tests that can be claimed: only under a Check.
fn claimable(main: &MainCi) -> Option<(&str, &[String])> {
    let first = main.failed.first()?;
    let check = first.check.as_deref()?;
    (!first.tests.is_empty()).then_some((check, first.tests.as_slice()))
}

/// What the Job is judged against: the one thing that went red.
fn criterion_of(main: &MainCi) -> AcceptanceCriterion {
    let first = main.failed.first();
    let text = match (first, claimable(main)) {
        (_, Some((check, [test, ..]))) => format!("`{test}` passes under the `{check}` Check."),
        (Some(job), _) => format!(
            "`{}` passes on main.",
            job.check.as_deref().unwrap_or(&job.name)
        ),
        (None, _) => String::from("Main's CI passes."),
    };
    AcceptanceCriterion {
        criterion_id: CriterionId::new("c1"),
        text,
        source: CriterionSource::Judge,
        origin: CriterionOrigin::Unsaid,
    }
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
    /// A person's press: a new Job for the red, or `send_to`'s work back to it.
    /// **The brief is on screen before the press, so the press is the approval.**
    /// `take_over` replaces another machine's claim on the red.
    pub async fn fix_main(
        &self,
        root: &str,
        send_to: Option<&JobId>,
        brief: Option<&str>,
        take_over: bool,
    ) -> Result<Job, Adrift> {
        let (served, main) = self.red_of(root).await?;
        if let Some(taken) = self.taking(&main).await {
            return Err(refused(root, MainNotFixable::Taken(taken)));
        }
        self.red_is_free(&main, take_over)
            .await
            .map_err(|why| refused(root, why))?;
        let brief = match brief {
            Some(text) if text.trim().is_empty() => {
                return Err(refused(root, MainNotFixable::Blank))
            }
            Some(text) => text.trim().to_string(),
            None => brief_of(&main),
        };
        let facts = self.with_the_log(&served, &main, brief).await;
        match send_to {
            None => self.dispatched_for(&served, &main, facts, take_over).await,
            Some(job) => {
                self.sent_back_for(&main, job, facts, TakenHow::SentBack, Actor::Human, take_over)
                    .await
            }
        }
    }

    /// `Commands::fix_main`'s answer: the Job, as its row.
    pub(crate) async fn fixed_main(
        &self,
        fix: ipc::FixMain,
    ) -> Result<ipc::JobSummary, api::Refusal> {
        let sent_to = fix.job.as_ref().map(ipc::JobId::to_domain);
        let job = self
            .fix_main(&fix.root, sent_to.as_ref(), fix.brief.as_deref(), fix.take_over)
            .await
            .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// The reading, if main is red there and Fleet serves the repository.
    pub(crate) async fn red_of(&self, root: &str) -> Result<(Served, MainCi), Adrift> {
        let served = self
            .repositories()
            .served()
            .into_iter()
            .find(|served| served.root() == root)
            .ok_or_else(|| refused(root, MainNotFixable::NotServed))?;
        let main = self
            .store()
            .lock()
            .await
            .main_ci(root)
            .map_err(Adrift::Reading)?
            .filter(|main| main.red_at.is_some() && !main.failed.is_empty())
            .ok_or_else(|| refused(root, MainNotFixable::NotRed))?;
        if main.held() {
            return Err(refused(root, MainNotFixable::ChecksRunning));
        }
        Ok((served, main))
    }

    /// The Job working on this red, if one is: the newest take whose Job has
    /// not been stopped.
    pub(crate) async fn taking(&self, main: &MainCi) -> Option<JobId> {
        let red_at = main.red_at.as_ref()?;
        let store = self.store().lock().await;
        let takes = store.main_fixes_of(&main.repository, red_at).ok()?;
        takes
            .into_iter()
            .filter(|take| take.ended_at.is_none())
            .map(|take| take.job)
            .find(|job| {
                store
                    .load_job(job)
                    .is_ok_and(|found| !stopped(found.status()))
            })
    }

    /// The brief, and under it the address of the failing job's log and its
    /// last lines as the forge gave them. **Absent where the forge would not
    /// give the text**: the address still goes.
    pub(crate) async fn with_the_log(&self, served: &Served, main: &MainCi, brief: String) -> String {
        let Some(job) = main.failed.first() else {
            return brief;
        };
        let (commit, name) = (main.red_commit().to_string(), job.name.clone());
        let text = async {
            let runs = self
                .forge_asked(served.root(), move |vcs: &V, root: &str| {
                    vcs.ci_runs_on(root, &commit)
                })
                .await?;
            let run = runs.into_iter().find(|run| {
                run.state == adapter_traits::CiState::Failed && run.name.as_written() == name
            })?;
            self.forge_asked(served.root(), move |vcs: &V, root: &str| {
                vcs.ci_log(root, &run)
            })
            .await
        }
        .await;
        let mut out = brief;
        if let Some(url) = &job.log_url {
            out.push_str(&format!("\n\nLog: {url}"));
        }
        if let Some(text) = text {
            let lines: Vec<&str> = text.as_written().lines().collect();
            let tail = &lines[lines.len().saturating_sub(LOG_LINES)..];
            out.push_str(&format!("\n\n{}", tail.join("\n")));
        }
        out
    }

    /// A new Job for the red, released at once.
    async fn dispatched_for(
        &self,
        served: &Served,
        main: &MainCi,
        facts: String,
        take_over: bool,
    ) -> Result<Job, Adrift> {
        let workflow = ipc::WorkflowId::carried(FIX_WORKFLOW);
        if !served.workflows().contains_key(&workflow.to_domain()) {
            return Err(refused(served.root(), MainNotFixable::NoWorkflow));
        }
        let first = main
            .failed
            .first()
            .ok_or_else(|| refused(served.root(), MainNotFixable::NotRed))?;
        let criterion = criterion_of(main);
        let proposal = ipc::ProposeJob {
            title: format!(
                "Fix {} on main",
                first.check.as_deref().unwrap_or(&first.name)
            ),
            workflow_id: workflow,
            owner_manifest_id: ipc::ManifestId::from(served.manifest().id()),
            origin: ipc::TopLevelOrigin::from(core_model::TopLevelOrigin::Manual),
            urgency: ipc::Urgency::from(core_model::Urgency::Normal),
            atomic: false,
            write_targets: None,
            dependencies: Vec::new(),
            model: None,
            acceptance_criteria: vec![ipc::ProposedCriterion {
                text: criterion.text,
                source: ipc::CriterionSource::from(criterion.source),
            }],
            subject: None,
            facts,
            attachments: Vec::new(),
            continue_from: None,
        };
        let job = self
            .proposed_job(proposal, StatedBy::APerson, None, Actor::Human)
            .await?;
        self.claimed_minted(main, &job, take_over).await?;
        let job = self.approve(job.id()).await?;
        self.recorded_take(main, job.id(), TakenHow::Dispatched)
            .await;
        Ok(job)
    }

    /// The work sent back to `job`, and the take recorded.
    async fn sent_back_for(
        &self,
        main: &MainCi,
        job: &JobId,
        facts: String,
        how: TakenHow,
        by: Actor,
        take_over: bool,
    ) -> Result<Job, Adrift> {
        let found = self.load(job).await?;
        let sent = match found.status() {
            JobStatus::AwaitingReview => {
                let note = Redirection::saying(&facts)
                    .ok_or_else(|| refused(&main.repository, MainNotFixable::Blank))?;
                self.claim_red(main, &Taker::Job(job.clone()), take_over)
                    .await
                    .map_err(|why| refused(&main.repository, why))?;
                self.request_changes(job, &note).await?
            }
            JobStatus::CompletedSuccess | JobStatus::CompletedFailed | JobStatus::Killed => {
                let carrying = Carrying {
                    facts: Facts::new(facts),
                    criteria: vec![criterion_of(main)],
                };
                let minted = self.mint_replacement(&found, Some(carrying), by).await?;
                self.claimed_minted(main, &minted, take_over).await?;
                if minted.status() == JobStatus::AwaitingApproval {
                    self.approve(minted.id()).await?
                } else {
                    minted
                }
            }
            status => {
                return Err(refused(
                    &main.repository,
                    MainNotFixable::CannotSendBack {
                        job: job.clone(),
                        status,
                    },
                ))
            }
        };
        self.recorded_take(main, sent.id(), how).await;
        Ok(sent)
    }

    /// Claim the red for a Job just made and not yet released. **A red that
    /// turns out held stops the Job**, so no unreleased proposal is left behind.
    async fn claimed_minted(&self, main: &MainCi, job: &Job, take_over: bool) -> Result<(), Adrift> {
        if let Err(why) = self
            .claim_red(main, &Taker::Job(job.id().clone()), take_over)
            .await
        {
            let _ = Fleet::kill_job(self, job.id()).await;
            return Err(refused(&main.repository, why));
        }
        Ok(())
    }

    /// Record that `job` has this red, and claim its failing tests.
    async fn recorded_take(&self, main: &MainCi, job: &JobId, how: TakenHow) {
        let (Some(red_at), Some(first)) = (main.red_at.clone(), main.failed.first()) else {
            return;
        };
        let now = self.now();
        let mut store = self.store().lock().await;
        let _ = store.keep_main_fix(&MainFix {
            repository: main.repository.clone(),
            red_at,
            job: job.clone(),
            how,
            check: first.check.clone().unwrap_or_else(|| first.name.clone()),
            test: first.tests.first().cloned(),
            merge: main.merge.as_ref().map(|merge| merge.number),
            taken_at: now.clone(),
            ended_at: None,
            fixed_in: None,
        });
        let (Some((check, tests)), Some(owner)) = (claimable(main), self.names().owner_of(job))
        else {
            return;
        };
        for test in tests {
            let _ = store.claim_breakage(
                &BreakageClaim {
                    fix: job.clone(),
                    repository: core_model::ManifestId::carried(core_model::Ulid::carried(
                        owner.clone(),
                    )),
                    breakage: Breakage {
                        check: check.to_string(),
                        test: test.clone(),
                        failure: format!("`{test}` fails under `{check}` on main."),
                    },
                    reported_by: job.clone(),
                    files: Vec::new(),
                },
                &now,
            );
        }
    }

    /// What main's change asks of Fleet: a red whose pull request a Job of ours
    /// opened is sent back to that Job, and a green ends every take.
    /// **Nothing here fails the turn**: a refusal is the band's two buttons.
    pub(crate) async fn main_acted_on(&self, changes: &[MainChanged]) {
        for changed in changes {
            match changed.change {
                MainChange::WentRed | MainChange::BackToRed => self.pick_up(&changed.now).await,
                MainChange::WentGreen => self.fixed(&changed.now).await,
            }
        }
    }

    /// Send a red back to the Job that caused it, **unless it is held**. **Once per red per Job**: the
    /// take is the guard, and a Job already on a red, any red, is left to it.
    async fn pick_up(&self, main: &MainCi) {
        // A newer run may have fixed it: BackToRed picks it up when that run ends red.
        if main.held() {
            return;
        }
        let Some(job) = main.merge.as_ref().and_then(|merge| merge.job.clone()) else {
            return;
        };
        {
            let store = self.store().lock().await;
            let taken = main.red_at.as_ref().is_some_and(|red_at| {
                store
                    .main_fixes_of(&main.repository, red_at)
                    .is_ok_and(|takes| takes.iter().any(|take| take.job == job))
            });
            if taken || store.is_fixing_main(&job).unwrap_or(true) {
                return;
            }
        }
        let Some(served) = self
            .repositories()
            .served()
            .into_iter()
            .find(|served| served.root() == main.repository)
        else {
            return;
        };
        // Held elsewhere, or the store silent: left to the band's buttons, which say why.
        if self.red_is_free(main, false).await.is_err() {
            return;
        }
        let facts = self.with_the_log(&served, main, brief_of(main)).await;
        let _ = self
            .sent_back_for(main, &job, facts, TakenHow::Took, Actor::Fleet, false)
            .await;
    }

    /// Main is green: every take ends, and the Job whose pull request put it
    /// there says it fixed main.
    async fn fixed(&self, main: &MainCi) {
        self.cleared_elsewhere(main).await;
        let commit = main.decided().to_string();
        let merged = self
            .forge_asked(&main.repository, move |vcs: &V, root: &str| {
                vcs.merged_by(root, &commit)
            })
            .await;
        let mut store = self.store().lock().await;
        let fixer = merged.and_then(|merged| {
            let number = merged.number;
            let found = store
                .jobs_with_pull_request_number(number)
                .unwrap_or_default()
                .into_iter()
                .find(|job| {
                    self.served_by_id(job)
                        .is_ok_and(|it| it.root() == main.repository)
                })?;
            Some((found, number))
        });
        let _ = store.end_main_fixes(
            &main.repository,
            &main.read_at,
            fixer.as_ref().map(|(job, number)| (job, *number)),
        );
    }

    /// The Job working on main's red for the hub: the newest take that has not
    /// been stopped, titled.
    pub(crate) fn fixing_of(&self, store: &store::Store, main: &MainCi) -> Option<JobId> {
        let red_at = main.red_at.as_ref()?;
        store
            .main_fixes_of(&main.repository, red_at)
            .ok()?
            .into_iter()
            .filter(|take| take.ended_at.is_none())
            .map(|take| take.job)
            .find(|job| {
                store
                    .load_job(job)
                    .is_ok_and(|found| !stopped(found.status()))
            })
    }

    /// A Job's part in main's red for its summary: working on it, or the one
    /// that fixed it. **Absent for a take that ended without this Job fixing
    /// main**, and for one whose Job was stopped.
    pub(crate) fn fixes_main_of(&self, store: &store::Store, job: &Job) -> Option<ipc::FixesMain> {
        let take = store.main_fix_of_job(job.id()).ok().flatten()?;
        let state = match (&take.ended_at, take.fixed_in) {
            (None, _) if stopped(job.status()) => return None,
            (None, _) => ipc::FixesMainState::Fixing,
            (Some(_), Some(_)) => ipc::FixesMainState::Fixed,
            (Some(_), None) => return None,
        };
        Some(ipc::FixesMain {
            state,
            check: take.check,
            test: take.test,
            merge: take.merge,
            fixed_in: take.fixed_in,
        })
    }
}

pub(crate) fn refused(root: &str, why: MainNotFixable) -> Adrift {
    Adrift::MainNotFixable {
        root: root.to_string(),
        why,
    }
}
