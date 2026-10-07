//! The hub Fleet puts on each repository's merge line: main's CI as slice one
//! kept it, and the open pull requests the forge lists. `docs/concepts/fleet.md`,
//! *What Fleet knows about main's CI*.
//!
//! **One listing a visit.** The pull requests are asked on the same interval and
//! rotation as main's head, so the forge is asked about a repository once per
//! interval whatever is open. **Held in memory only**: a restart reads them
//! again on the repository's next visit, and main's reading is kept anyway.
//!
//! **A pull request waits on main** when its `ci` failed and every check that
//! failed on it also fails on main. A failure on any other job is its own.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, CiState, Delivery, OpenPull, RecentlyMerged, Vcs, WorkProduct};
use core_model::JobId;
use core_model::Timestamp;
use ipc::{
    HubJob, HubMerged, HubPullCi, HubPullRequest, MainChecking, MainCiState, MainFailedJob,
    MainMerge, MainRun, MainRunState, MainStanding, MergeLineHub,
};
use store::{MainCi, MainState};

use crate::daemon::Fleet;
use crate::repositories::Served;

/// The branch name a log asked for under when it is main's own run. It is the
/// word the head sends whatever the repository calls its base, and a line never
/// queues the base, so it names no branch of `armada land`'s.
pub(crate) const MAIN: &str = "main";

/// A log read off the forge and held, read the way a file is.
pub(crate) struct HeldLog(Vec<u8>);

impl api::Follow for HeldLog {
    fn read(&self, from: u64, to_the_end: bool) -> api::Followed {
        let at = usize::try_from(from)
            .unwrap_or(usize::MAX)
            .min(self.0.len());
        crate::following::windowed(&self.0[at..], from, to_the_end)
    }

    /// It was written whole before it was asked for.
    fn writing(&self) -> bool {
        false
    }
}

/// An open pull request as last read, with the Job of ours that opened it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OpenPulled {
    pub(crate) pull: OpenPull,
    pub(crate) job: Option<JobId>,
}

/// A recently merged pull request as last read, with the Job of ours that
/// opened it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MergedPulled {
    pub(crate) pull: RecentlyMerged,
    pub(crate) job: Option<JobId>,
}

/// How many merged pull requests the hub carries.
const MERGED_SHOWN: usize = 5;

/// How many merged pull requests are listed to decide main's state from: **the
/// newest finished commit decides**, and the newest few may all still be running.
/// Their runs are read past [`MERGED_SHOWN`] only until one has finished.
const DECIDING_WINDOW: usize = 30;

/// How long a merge commit with no run on it is asked about again: the forge
/// starts its workflows a moment after a merge.
const RUN_MAY_START: Duration = Duration::from_secs(600);

/// What a commit's jobs add up to, for deciding main's state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Reduced {
    Green,
    Running,
    Red,
}

/// The CI run on one commit of main, as last read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MergedRun {
    /// Absent where nothing ran.
    pub(crate) run: Option<MainRun>,
    /// Nothing is still going, so it is not asked again.
    pub(crate) settled: bool,
    /// The jobs themselves, for the commit that decides main.
    pub(crate) runs: Vec<adapter_traits::CiRun>,
}

impl MergedRun {
    /// `None` where nothing ran: a commit that proves nothing either way.
    pub(crate) fn class(&self) -> Option<Reduced> {
        Some(match self.run.as_ref()?.state {
            MainRunState::Failed => Reduced::Red,
            MainRunState::Passed => Reduced::Green,
            MainRunState::Running => Reduced::Running,
        })
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
    /// Read one repository's open pull requests. **A forge that will not answer
    /// keeps the last reading**, rather than drawing every pull request as gone.
    pub(crate) async fn notice_pulls(&self, served: &Served) {
        let listed = self
            .forge_asked(served.root(), |vcs: &V, root: &str| {
                vcs.open_pull_requests(root)
            })
            .await;
        let Some(listed) = listed else { return };
        let read: Vec<OpenPulled> = {
            let store = self.store().lock().await;
            listed
                .into_iter()
                .map(|pull| OpenPulled {
                    job: self.job_of_pull(&store, served, pull.number),
                    pull,
                })
                .collect()
        };
        self.sweeping()
            .lock()
            .await
            .pulls
            .insert(served.root().to_string(), read);
    }

    /// The newest merged pull requests, **a second forge call on the same
    /// visit**. A forge that will not answer keeps the last reading.
    pub(crate) async fn notice_merged(&self, served: &Served) {
        let Some(base) = served.manifest().base().map(str::to_string) else {
            return;
        };
        let listed = self
            .forge_asked(served.root(), move |vcs: &V, root: &str| {
                vcs.recently_merged_pull_requests(root, &base, DECIDING_WINDOW)
            })
            .await;
        let Some(listed) = listed else { return };
        let read: Vec<MergedPulled> = {
            let store = self.store().lock().await;
            listed
                .into_iter()
                .map(|pull| MergedPulled {
                    job: self.job_of_pull(&store, served, pull.number),
                    pull,
                })
                .collect()
        };
        self.sweeping()
            .lock()
            .await
            .merged
            .insert(served.root().to_string(), read);
    }

    /// The CI run on main's head and on each merged pull request's commit, newest
    /// first: **the commits main's state is decided from**, returned with what
    /// was read. **One `ci_runs_on` a commit, then again only while a job is
    /// unfinished** (or nothing ran yet and the merge is minutes old), and a
    /// settled reading is kept. A forge that will not answer keeps what was read.
    /// The head and the newest [`MERGED_SHOWN`] are always read; older merges
    /// only while none of the newer has finished, up to [`DECIDING_WINDOW`]. Usually
    /// none after the first visit.
    pub(crate) async fn notice_merged_runs(
        &self,
        served: &Served,
        head: &str,
    ) -> Vec<(String, Option<MergedRun>)> {
        let root = served.root().to_string();
        let (listed, kept) = {
            let sweep = self.sweeping().lock().await;
            (
                sweep.merged.get(&root).cloned().unwrap_or_default(),
                sweep.main_runs.get(&root).cloned().unwrap_or_default(),
            )
        };
        let now = self.now();
        let mut asked: Vec<(String, Option<String>)> = vec![(head.to_string(), None)];
        for one in &listed {
            let Some(commit) = one
                .pull
                .commit
                .as_ref()
                .map(|it| it.as_written().to_string())
            else {
                continue;
            };
            let merged_at = Some(one.pull.merged_at.as_written().to_string());
            match asked.iter_mut().find(|(seen, _)| *seen == commit) {
                Some(seen) => seen.1 = merged_at,
                None => asked.push((commit, merged_at)),
            }
        }
        let mut read = BTreeMap::new();
        let mut candidates: Vec<(String, Option<MergedRun>)> = Vec::new();
        for (at, (commit, merged_at)) in asked.into_iter().enumerate() {
            let decided = candidates.iter().any(|(_, run)| {
                run.as_ref()
                    .and_then(MergedRun::class)
                    .is_some_and(|it| it != Reduced::Running)
            });
            if at > MERGED_SHOWN && decided {
                break;
            }
            if let Some(settled) = kept.get(&commit).filter(|it| it.settled) {
                read.insert(commit.clone(), settled.clone());
                candidates.push((commit, Some(settled.clone())));
                continue;
            }
            let asking = commit.clone();
            let answer = self
                .forge_asked(&root, move |vcs: &V, root: &str| {
                    vcs.ci_runs_on(root, &asking)
                })
                .await;
            let Some(runs) = answer else {
                let old = kept.get(&commit).cloned();
                if let Some(old) = &old {
                    read.insert(commit.clone(), old.clone());
                }
                candidates.push((commit, old));
                continue;
            };
            let unfinished = runs.iter().any(|run| run.state == CiState::Pending);
            let failed: Vec<String> = runs
                .iter()
                .filter(|run| run.state == CiState::Failed)
                .map(|run| run.name.as_written().to_string())
                .collect();
            let may_start = runs.is_empty()
                && merged_at.is_some_and(|at| {
                    crate::converging::elapsed(&Timestamp::from_rfc3339(at), &now) < RUN_MAY_START
                });
            let run = match (failed.is_empty(), unfinished, runs.is_empty()) {
                (_, _, true) => None,
                (false, _, _) => Some(MainRun {
                    state: MainRunState::Failed,
                    failed,
                }),
                (true, true, _) => Some(MainRun {
                    state: MainRunState::Running,
                    failed: Vec::new(),
                }),
                (true, false, _) => Some(MainRun {
                    state: MainRunState::Passed,
                    failed: Vec::new(),
                }),
            };
            let reading = MergedRun {
                run,
                settled: !unfinished && !may_start,
                runs,
            };
            read.insert(commit.clone(), reading.clone());
            candidates.push((commit, Some(reading)));
        }
        self.sweeping().lock().await.main_runs.insert(root, read);
        candidates
    }

    /// The log of one job that failed on a merged commit's run on main, for
    /// `observe_land_check` under `main@<commit>`. **`None` unless that run is
    /// one the hub shows as failed for that job.**
    pub(crate) async fn main_run_log(
        &self,
        root: String,
        commit: String,
        job: String,
    ) -> Option<api::LandOutput> {
        let shown = self
            .sweeping()
            .lock()
            .await
            .main_runs
            .get(&root)
            .and_then(|runs| runs.get(&commit))
            .and_then(|it| it.run.as_ref())
            .is_some_and(|run| run.failed.contains(&job));
        shown.then_some(())?;
        let asked = commit.clone();
        let runs = self
            .forge_asked(&root, move |vcs: &V, root: &str| {
                vcs.ci_runs_on(root, &asked)
            })
            .await?;
        let wanted = job.clone();
        let run = runs
            .into_iter()
            .find(|run| run.state == CiState::Failed && run.name.as_written() == wanted)?;
        let text = self
            .forge_asked(&root, move |vcs: &V, root: &str| vcs.ci_log(root, &run))
            .await?;
        Some(api::LandOutput {
            root,
            branch: format!("{MAIN}@{commit}"),
            name: job,
            follow: Arc::new(HeldLog(text.as_written().as_bytes().to_vec())),
        })
    }

    /// The log of one job that failed on a served repository's main, for
    /// `observe_land_check` under [`MAIN`]. `check` is the Check a failed job
    /// maps to, or the job's own name. **`None` unless main is red for that
    /// job**; the forge is asked for the job's log now, bounded.
    pub(crate) async fn main_log(&self, root: String, check: String) -> Option<api::LandOutput> {
        self.repositories()
            .served()
            .iter()
            .any(|served| served.root() == root)
            .then_some(())?;
        let reading = self.store().lock().await.main_ci(&root).ok().flatten()?;
        let job = reading
            .failed
            .iter()
            .find(|job| job.name == check || job.check.as_deref() == Some(check.as_str()))?;
        let (name, commit) = (job.name.clone(), reading.red_commit().to_string());
        let runs = self
            .forge_asked(&root, move |vcs: &V, root: &str| {
                vcs.ci_runs_on(root, &commit)
            })
            .await?;
        let run = runs
            .into_iter()
            .find(|run| run.state == CiState::Failed && run.name.as_written() == name)?;
        let text = self
            .forge_asked(&root, move |vcs: &V, root: &str| vcs.ci_log(root, &run))
            .await?;
        Some(api::LandOutput {
            root,
            branch: MAIN.to_string(),
            name: check,
            follow: Arc::new(HeldLog(text.as_written().as_bytes().to_vec())),
        })
    }

    /// The Job of ours whose pull request this is, in this repository.
    pub(crate) fn job_of_pull(
        &self,
        store: &store::Store,
        served: &Served,
        number: u64,
    ) -> Option<JobId> {
        store
            .jobs_with_pull_request_number(number)
            .unwrap_or_default()
            .into_iter()
            .find(|job| {
                self.served_by_id(job)
                    .is_ok_and(|it| it.root() == served.root())
            })
    }

    /// Each served repository's hub, by root. A repository Fleet has read
    /// nothing for is left out.
    pub(crate) async fn merge_hubs(&self) -> Vec<(String, MergeLineHub)> {
        let roots: Vec<String> = self
            .repositories()
            .served()
            .iter()
            .map(|served| served.root().to_string())
            .collect();
        let (pulls, merged, runs) = {
            let sweep = self.sweeping().lock().await;
            (
                sweep.pulls.clone(),
                sweep.merged.clone(),
                sweep.main_runs.clone(),
            )
        };
        let store = self.store().lock().await;
        let titled = |job: &JobId| {
            store.load_job(job).ok().map(|found| HubJob {
                id: ipc::JobId::from(job),
                title: found.title().as_str().to_string(),
            })
        };
        roots
            .into_iter()
            .filter_map(|root| {
                let main = store.main_ci(&root).ok().flatten();
                let mut hub = hub_of(
                    main.as_ref(),
                    pulls.get(&root),
                    merged.get(&root),
                    runs.get(&root),
                    &titled,
                )?;
                hub.fixing = main
                    .as_ref()
                    .and_then(|main| self.fixing_of(&store, main))
                    .and_then(|job| titled(&job));
                Some((root, hub))
            })
            .collect()
    }
}

/// The hub for one repository, or `None` where there is nothing to say.
pub(crate) fn hub_of(
    main: Option<&MainCi>,
    pulls: Option<&Vec<OpenPulled>>,
    merged: Option<&Vec<MergedPulled>>,
    runs: Option<&BTreeMap<String, MergedRun>>,
    titled: &dyn Fn(&JobId) -> Option<HubJob>,
) -> Option<MergeLineHub> {
    let no_runs = BTreeMap::new();
    let runs = runs.unwrap_or(&no_runs);
    if main.is_none() && pulls.is_none() && merged.is_none() {
        return None;
    }
    // A red stays red until a green, so a fix still running does not clear it.
    let red = main.is_some_and(|it| it.red_at.is_some() || it.state == MainState::Red);
    let failing_on_main: Vec<&str> = match main {
        Some(it) if red => it.failed.iter().map(|job| job.name.as_str()).collect(),
        _ => Vec::new(),
    };
    Some(MergeLineHub {
        main: main.map(|it| standing(it, red, merged.map_or(&[][..], Vec::as_slice), runs, titled)),
        pull_requests: pulls
            .map(|read| {
                read.iter()
                    .map(|one| request(one, &failing_on_main, titled))
                    .collect()
            })
            .unwrap_or_default(),
        merged: merged
            .map(|read| {
                read.iter()
                    .take(MERGED_SHOWN)
                    .map(|one| merged_of(one, runs, titled))
                    .collect()
            })
            .unwrap_or_default(),
        fixing: None,
    })
}

fn standing(
    main: &MainCi,
    red: bool,
    merged: &[MergedPulled],
    runs: &BTreeMap<String, MergedRun>,
    titled: &dyn Fn(&JobId) -> Option<HubJob>,
) -> MainStanding {
    MainStanding {
        state: match (red, main.state) {
            (true, _) => MainCiState::Red,
            (false, MainState::Green) => MainCiState::Green,
            (false, MainState::Running) => MainCiState::Running,
            (false, MainState::NothingRan | MainState::Red) => MainCiState::NothingRan,
        },
        commit: main.commit.clone(),
        read_at: ipc::Instant::carried(main.read_at.as_str()),
        red_since: main
            .red_at
            .as_ref()
            .filter(|_| red)
            .map(|at| ipc::Instant::carried(at.as_str())),
        failed: main
            .failed
            .iter()
            .filter(|_| red)
            .map(|job| MainFailedJob {
                name: job.name.clone(),
                check: job.check.clone(),
                log_url: job.log_url.clone(),
                tests: job.tests.clone(),
            })
            .collect(),
        merge: main.merge.as_ref().filter(|_| red).map(|merge| MainMerge {
            number: merge.number,
            url: merge.url.clone(),
            branch: merge.branch.clone(),
            job: merge.job.as_ref().and_then(titled),
        }),
        red_commit: main
            .red_commit
            .clone()
            .or_else(|| (red && main.state == MainState::Red).then(|| main.commit.clone())),
        checking: match main.newer_running > 0 && (red || main.state == MainState::Green) {
            true => checking(main, merged, runs, titled),
            false => Vec::new(),
        },
    }
}

/// The commits newer than the one that decided main whose run is going, newest
/// first: main's head, then any newest merge still running, each with the pull
/// request that merged it.
fn checking(
    main: &MainCi,
    merged: &[MergedPulled],
    runs: &BTreeMap<String, MergedRun>,
    titled: &dyn Fn(&JobId) -> Option<HubJob>,
) -> Vec<MainChecking> {
    fn commit_of(one: &MergedPulled) -> Option<&str> {
        one.pull.commit.as_ref().map(|it| it.as_written())
    }
    let decided = main.decided();
    let running = |commit: &str| {
        runs.get(commit)
            .and_then(|it| it.run.as_ref())
            .is_some_and(|run| run.state == MainRunState::Running)
    };
    let pull_of = |one: &MergedPulled| MainMerge {
        number: one.pull.number,
        url: Some(one.pull.url.as_written().to_string()),
        branch: Some(one.pull.branch.as_written().to_string()),
        job: one.job.as_ref().and_then(titled),
    };
    let head = (main.commit != decided && running(&main.commit)).then(|| MainChecking {
        commit: main.commit.clone(),
        pull_request: merged
            .iter()
            .find(|one| commit_of(one) == Some(main.commit.as_str()))
            .map(pull_of),
    });
    let newer = merged
        .iter()
        .take_while(|one| commit_of(one) != Some(decided))
        .filter_map(|one| Some((one, commit_of(one)?)))
        .filter(|(_, commit)| *commit != main.commit && running(commit))
        .map(|(one, commit)| MainChecking {
            commit: commit.to_string(),
            pull_request: Some(pull_of(one)),
        });
    head.into_iter().chain(newer).collect()
}

fn request(
    one: &OpenPulled,
    failing_on_main: &[&str],
    titled: &dyn Fn(&JobId) -> Option<HubJob>,
) -> HubPullRequest {
    let pull = &one.pull;
    let own_failure = pull
        .failing
        .iter()
        .any(|name| !failing_on_main.contains(&name.as_written()));
    HubPullRequest {
        number: pull.number,
        title: pull.title.as_written().to_string(),
        branch: pull.branch.as_written().to_string(),
        url: pull.url.as_written().to_string(),
        author: pull.author.as_ref().map(|it| it.as_written().to_string()),
        ci: pull.ci.map(|state| match state {
            CiState::Passed => HubPullCi::Passed,
            CiState::Pending => HubPullCi::Running,
            CiState::Failed if !pull.failing.is_empty() && !own_failure => HubPullCi::WaitingOnMain,
            CiState::Failed => HubPullCi::Failed,
        }),
        job: one.job.as_ref().and_then(titled),
    }
}

fn merged_of(
    one: &MergedPulled,
    runs: &BTreeMap<String, MergedRun>,
    titled: &dyn Fn(&JobId) -> Option<HubJob>,
) -> HubMerged {
    let pull = &one.pull;
    HubMerged {
        number: pull.number,
        title: pull.title.as_written().to_string(),
        branch: pull.branch.as_written().to_string(),
        url: pull.url.as_written().to_string(),
        author: pull.author.as_ref().map(|it| it.as_written().to_string()),
        merged_at: ipc::Instant::carried(pull.merged_at.as_written()),
        commit: pull.commit.as_ref().map(|it| it.as_written().to_string()),
        job: one.job.as_ref().and_then(titled),
        main_run: pull
            .commit
            .as_ref()
            .and_then(|it| runs.get(it.as_written()))
            .and_then(|it| it.run.clone()),
    }
}
