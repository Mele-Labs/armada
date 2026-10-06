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

use std::sync::Arc;

use adapter_traits::{AgentHarness, CiState, Delivery, OpenPull, Vcs, WorkProduct};
use core_model::JobId;
use ipc::{
    HubJob, HubPullCi, HubPullRequest, MainCiState, MainFailedJob, MainMerge, MainStanding,
    MergeLineHub,
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
        let (name, commit) = (job.name.clone(), reading.commit.clone());
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
        let pulls = self.sweeping().lock().await.pulls.clone();
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
                let hub = hub_of(main.as_ref(), pulls.get(&root), &titled)?;
                Some((root, hub))
            })
            .collect()
    }
}

/// The hub for one repository, or `None` where there is nothing to say.
pub(crate) fn hub_of(
    main: Option<&MainCi>,
    pulls: Option<&Vec<OpenPulled>>,
    titled: &dyn Fn(&JobId) -> Option<HubJob>,
) -> Option<MergeLineHub> {
    if main.is_none() && pulls.is_none() {
        return None;
    }
    // A red stays red until a green, so a fix still running does not clear it.
    let red = main.is_some_and(|it| it.red_at.is_some() || it.state == MainState::Red);
    let failing_on_main: Vec<&str> = match main {
        Some(it) if red => it.failed.iter().map(|job| job.name.as_str()).collect(),
        _ => Vec::new(),
    };
    Some(MergeLineHub {
        main: main.map(|it| standing(it, red, titled)),
        pull_requests: pulls
            .map(|read| {
                read.iter()
                    .map(|one| request(one, &failing_on_main, titled))
                    .collect()
            })
            .unwrap_or_default(),
        fixing: None,
    })
}

fn standing(main: &MainCi, red: bool, titled: &dyn Fn(&JobId) -> Option<HubJob>) -> MainStanding {
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
    }
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
