//! What the forge says about main's CI, read for every repository this Fleet
//! serves. `docs/concepts/fleet.md`, *What Fleet knows about main's CI*.
//!
//! **The forge's facts first.** A failing job is named as the forge names it,
//! with its log and the merge that turned main red. A Manifest Check is added
//! only where one maps, and a test only where the log names one; unmapped and
//! unread are ordinary and never guessed at.
//!
//! **One repository a turn, on the sweep's interval, rotating.** Its open pull
//! requests are listed on the same visit, `crate::main_hub`. The head is a
//! ref lookup; the jobs are asked once it has moved and again only while some
//! are unfinished. **A red stays red until a green**, so a fix still running
//! does not end it. Nothing here acts: the change rides on the turn.

use std::sync::Arc;

use adapter_traits::{AgentHarness, CiRun, CiState, Delivery, Vcs, WorkProduct};
use store::{MainCi, MainFailedJob, MainMerge, MainState};

use crate::adrift::Adrift;
use crate::converging::elapsed;
use crate::daemon::Fleet;
use crate::repositories::Served;

/// How many failed jobs have their log read in one reading. A job past it is
/// read on the next, and one never read names no test.
const LOGS_A_READING: usize = 8;

/// What happened to main's CI that something later may act on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MainChange {
    /// It failed, where it was not failing before or fails something new.
    WentRed,
    /// It was red and a newer commit's CI was running on top of it, and that
    /// run ended red on the same jobs. **The red is back, and may be acted on.**
    BackToRed,
    /// It was red and a commit has passed everything.
    WentGreen,
}

/// A change in a repository's main, with the reading it came to.
///
/// **The culprit rides on it where it was found**: the merge's pull request
/// number, and the Fleet Job that opened that pull request, if any. It is asked
/// once per commit, in the reading that finds the red.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MainChanged {
    pub change: MainChange,
    pub now: MainCi,
}

/// What a commit's jobs add up to.
enum Reduced {
    Green,
    Running,
    Red,
    NothingRan,
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
    /// Read one repository's main, if it is time to. **Empty nearly every
    /// turn**: too soon, nothing served names a base, the head has not moved
    /// and nothing is running, or the forge would not answer.
    pub(crate) async fn notice_main(&self) -> Result<Vec<MainChanged>, Adrift> {
        let Some((served, base)) = self.main_due().await else {
            return Ok(Vec::new());
        };
        let root = served.root().to_string();
        self.notice_pulls(&served).await;
        self.notice_merged(&served).await;
        self.notice_merged_runs(&served).await;
        let Some(head) = self
            .forge_asked(&root, {
                let base = base.clone();
                move |vcs: &V, root: &str| vcs.base_head_on_the_forge(root, &base)
            })
            .await
        else {
            return Ok(Vec::new());
        };
        let before = self
            .store()
            .lock()
            .await
            .main_ci(&root)
            .map_err(Adrift::Reading)?;
        if before
            .as_ref()
            .is_some_and(|it| it.commit == head && it.base == base && it.unfinished == 0)
        {
            return Ok(Vec::new());
        }
        let Some(runs) = self
            .forge_asked(&root, {
                let head = head.clone();
                move |vcs: &V, root: &str| vcs.ci_runs_on(root, &head)
            })
            .await
        else {
            return Ok(Vec::new());
        };
        let unfinished = runs
            .iter()
            .filter(|run| run.state == CiState::Pending)
            .count();
        let failing: Vec<&CiRun> = runs
            .iter()
            .filter(|run| run.state == CiState::Failed)
            .collect();
        let reduced = match (failing.is_empty(), unfinished, runs.is_empty()) {
            (false, _, _) => Reduced::Red,
            (true, 0, true) => Reduced::NothingRan,
            (true, 0, false) => Reduced::Green,
            (true, _, _) => Reduced::Running,
        };
        let now = self.now();
        let streak = before.as_ref().and_then(|it| it.red_at.clone());
        let carried = |state| MainCi {
            repository: root.clone(),
            base: base.clone(),
            commit: head.clone(),
            state,
            read_at: now.clone(),
            red_at: streak.clone(),
            // The red's own commit, kept while a newer one runs on top of it.
            red_commit: streak.as_ref().and_then(|_| {
                before.as_ref().map(|it| match it.state {
                    MainState::Red => it.commit.clone(),
                    _ => it.red_commit().to_string(),
                })
            }),
            unfinished: unfinished as u32,
            failed: before
                .as_ref()
                .map(|it| it.failed.clone())
                .unwrap_or_default(),
            merge: before.as_ref().and_then(|it| it.merge.clone()),
        };
        let (reading, change) = match reduced {
            Reduced::Green => (
                MainCi {
                    red_at: None,
                    red_commit: None,
                    failed: Vec::new(),
                    merge: None,
                    ..carried(MainState::Green)
                },
                streak.as_ref().map(|_| MainChange::WentGreen),
            ),
            Reduced::Running => (carried(MainState::Running), None),
            // Nothing ran: nothing proved, so a red is not ended by it. A red that was
            // held is no longer: the run it waited on is not coming.
            Reduced::NothingRan => (
                carried(match (&streak, before.as_ref()) {
                    (Some(_), Some(it)) if it.state != MainState::Running => it.state,
                    _ => MainState::NothingRan,
                }),
                None,
            ),
            Reduced::Red => {
                // A reading that was only running carries the red's old jobs, and
                // is no proof that this commit failed the same ones.
                let same_commit = before
                    .as_ref()
                    .is_some_and(|it| it.commit == head && it.state == MainState::Red);
                let failed = self
                    .failed_jobs(&served, &head, &failing, before.as_ref())
                    .await;
                let continues = streak.is_some()
                    && (same_commit
                        || failed.iter().all(|job| {
                            before
                                .as_ref()
                                .is_some_and(|it| it.failed.iter().any(|was| was.name == job.name))
                        }));
                let kept = before.as_ref().and_then(|it| it.merge.clone());
                let merge = match continues || (same_commit && kept.is_some()) {
                    true => kept,
                    false => self.the_merge(&served, &head).await,
                };
                (
                    MainCi {
                        state: MainState::Red,
                        red_at: streak.clone().filter(|_| continues).or(Some(now.clone())),
                        red_commit: Some(head.clone()),
                        failed,
                        merge,
                        ..carried(MainState::Red)
                    },
                    match continues {
                        false => Some(MainChange::WentRed),
                        true => before
                            .as_ref()
                            .filter(|it| it.held())
                            .map(|_| MainChange::BackToRed),
                    },
                )
            }
        };
        self.store()
            .lock()
            .await
            .record_main_ci(&reading)
            .map_err(Adrift::Writing)?;
        Ok(change
            .map(|change| MainChanged {
                change,
                now: reading,
            })
            .into_iter()
            .collect())
    }

    /// The repository to read this turn, and the base its Manifest names.
    async fn main_due(&self) -> Option<(Served, String)> {
        let now = self.now();
        let mut sweep = self.sweeping().lock().await;
        if let Some(last) = sweep.main_last.as_ref() {
            if elapsed(last, &now) < self.noticing().interval() {
                return None;
            }
        }
        sweep.main_last = Some(now);
        let naming: Vec<(Served, String)> = self
            .repositories()
            .served()
            .into_iter()
            .filter_map(|served| {
                let base = served.manifest().base()?.to_string();
                Some((served, base))
            })
            .collect();
        if naming.is_empty() {
            return None;
        }
        let at = sweep.main_next % naming.len();
        sweep.main_next = at + 1;
        naming.into_iter().nth(at)
    }

    /// One forge ask, off the runtime's own thread.
    pub(crate) async fn forge_asked<T: Send + 'static>(
        &self,
        root: &str,
        ask: impl FnOnce(&V, &str) -> T + Send + 'static,
    ) -> T {
        let (vcs, root) = (Arc::clone(self.vcs()), root.to_string());
        tokio::task::spawn_blocking(move || ask(&vcs, &root))
            .await
            .expect("the forge process panicked")
    }

    /// Each failed job as the forge names it, a Check where the Manifest maps
    /// one, and the tests its log names. **A job already read for this commit
    /// is kept as it was read**, so a rereading costs no second log.
    async fn failed_jobs(
        &self,
        served: &Served,
        head: &str,
        failing: &[&CiRun],
        before: Option<&MainCi>,
    ) -> Vec<MainFailedJob> {
        let mut logs_left = LOGS_A_READING;
        let mut failed = Vec::new();
        for run in failing {
            let name = run.name.as_written().to_string();
            let kept = before
                .filter(|it| it.red_commit() == head)
                .and_then(|it| it.failed.iter().find(|was| was.name == name));
            if let Some(kept) = kept {
                failed.push(kept.clone());
                continue;
            }
            let tests = match logs_left {
                0 => Vec::new(),
                _ => {
                    logs_left -= 1;
                    let asked = (*run).clone();
                    match self
                        .forge_asked(served.root(), move |vcs, root| vcs.ci_log(root, &asked))
                        .await
                    {
                        Some(log) => checks_runner::failing_tests_in(log.as_written()),
                        None => Vec::new(),
                    }
                }
            };
            failed.push(MainFailedJob {
                check: served
                    .manifest()
                    .check_for_ci_job(&name)
                    .map(str::to_string),
                log_url: run.log_url.as_ref().map(|url| url.as_written().to_string()),
                name,
                tests,
            });
        }
        failed
    }

    /// The pull request that merged `head`, and the Job of ours that opened
    /// it. **Asked once a commit, for as long as this process lives**: a direct
    /// push names none, and asking again would not change that.
    async fn the_merge(&self, served: &Served, head: &str) -> Option<MainMerge> {
        let key = format!("{}@{head}", served.root());
        if !self.sweeping().lock().await.culprit_asked.insert(key) {
            return None;
        }
        let commit = head.to_string();
        let pull = self
            .forge_asked(served.root(), move |vcs, root| vcs.merged_by(root, &commit))
            .await?;
        let job = {
            let store = self.store().lock().await;
            self.job_of_pull(&store, served, pull.number)
        };
        Some(MainMerge {
            number: pull.number,
            url: pull.url.map(|url| url.as_written().to_string()),
            branch: pull.branch.map(|branch| branch.as_written().to_string()),
            job,
        })
    }
}
