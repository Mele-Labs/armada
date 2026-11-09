//! The merge line as a screen draws it: what `get_merge_lines` serves, read once for every client.
//!
//! Pure. The input is the lines Fleet read of each forge and the repositories it serves; the output
//! is one [`MergeLineView`] per line, in the same order. This is a port of what Bridge worked out in
//! TypeScript (`mergeLineViews`, `hubOf`, `mainOf`, `redOf` and `repositoryLabel`), with the rules
//! kept as they were. **Two departures**: a pull request of your own passes through whole, where
//! the TypeScript kept only the fields a panel drew, and a title is filled in on a Job where the
//! TypeScript filled it in on the row.

use ipc::{
    HubPullRequest, HubQueue, HubQueueState, HubSession, MainCiState, MainFailedJob, MainMerge, MainRedJob,
    MainStanding, MainView, MergeLine, MergeLineHub, MergeLineView, MergeLineViews, MergeLines, RepositorySummary,
};

/// Every line as a screen draws it, in the order Fleet serves the lines.
pub(crate) fn views(lines: &MergeLines, repositories: &[RepositorySummary]) -> MergeLineViews {
    MergeLineViews { lines: lines.lines.iter().map(|line| view_of(line, repositories)).collect() }
}

fn view_of(line: &MergeLine, repositories: &[RepositorySummary]) -> MergeLineView {
    let name = name_of(&line.root, repositories);
    let Some(hub) = &line.hub else {
        return MergeLineView {
            root: line.root.clone(),
            name,
            main: None,
            pulls: Vec::new(),
            landed: Vec::new(),
            checkout: None,
            read_at: None,
            quiet_since: None,
        };
    };
    MergeLineView {
        root: line.root.clone(),
        name,
        main: main_of(hub),
        pulls: pulls_of(&hub.pull_requests),
        landed: hub.merged.clone(),
        checkout: hub.checkout.clone(),
        read_at: hub.forge_read_at.clone(),
        quiet_since: hub.quiet_since.clone(),
    }
}

/// The repository's label, or its root where Fleet's listing has not caught up with its line.
fn name_of(root: &str, repositories: &[RepositorySummary]) -> String {
    match repositories.iter().find(|one| one.root == root) {
        Some(served) => label(served, repositories),
        None => root.to_string(),
    }
}

/// What to call one repository among the set Fleet serves: a word, then `parent/folder`, then the
/// whole root, each rung taken only because the one above it names something else too. **Every other
/// repository is a rival, set up or not**, since the labels share one namespace.
pub(crate) fn label(repository: &RepositorySummary, repositories: &[RepositorySummary]) -> String {
    let mut others = repositories.iter().filter(|other| other.root != repository.root);
    let word = word_of(repository);
    if !others.clone().any(|other| word_of(other) == word) {
        return word;
    }
    let placed = placed_of(&repository.root);
    if others.any(|other| placed_of(&other.root) == placed) { repository.root.clone() } else { placed }
}

/// The readable word for a repository before any widening: the Manifest's repository, else the
/// root's last segment.
fn word_of(repository: &RepositorySummary) -> String {
    match &repository.manifest {
        Some(manifest) => manifest.repository.clone(),
        None => folder_of(&repository.root),
    }
}

fn placed_of(root: &str) -> String {
    let mut parts = root.trim_end_matches('/').rsplit('/');
    let folder = parts.next().unwrap_or(root);
    format!("{}/{folder}", parts.next().unwrap_or(""))
}

fn folder_of(root: &str) -> String {
    match root.trim_end_matches('/').rsplit('/').next() {
        Some(last) if !last.is_empty() => last.to_string(),
        _ => root.to_string(),
    }
}

/// Another author's pull request stays only in the queue, quiet; the rest are listed queue first.
fn pulls_of(pulls: &[HubPullRequest]) -> Vec<HubPullRequest> {
    let mut kept: Vec<&HubPullRequest> = pulls.iter().filter(|pull| pull.mine || in_the_queue(pull)).collect();
    kept.sort_by_key(|pull| {
        let (rank, position) = match &pull.queue {
            Some(HubQueue { position: Some(position), .. }) => (0, *position),
            Some(HubQueue { state: HubQueueState::WaitingForCi, .. }) => (1, 0),
            _ => (2, 0),
        };
        (rank, position)
    });
    kept.into_iter().map(|pull| if pull.mine { pull.clone() } else { quiet_of(pull) }).collect()
}

/// In the forge's merge queue now, rather than waiting for ci to join it.
fn in_the_queue(pull: &HubPullRequest) -> bool {
    pull.queue.as_ref().is_some_and(|queue| queue.state != HubQueueState::WaitingForCi)
}

/// Another author's pull request in the queue: its place and nothing that calls for anyone. **No
/// checks, no conflict, no review and no hand**, and the queue state is the plain one, so nothing
/// downstream can raise a card or an alarm for it.
fn quiet_of(pull: &HubPullRequest) -> HubPullRequest {
    HubPullRequest {
        number: pull.number,
        title: pull.title.clone(),
        branch: pull.branch.clone(),
        url: pull.url.clone(),
        author: pull.author.clone(),
        ci: None,
        queue: Some(HubQueue {
            state: HubQueueState::InQueue,
            position: pull.queue.as_ref().and_then(|queue| queue.position),
            enqueued_at: None,
        }),
        opened_at: None,
        updated_at: None,
        draft: false,
        mergeable: None,
        review: None,
        additions: None,
        deletions: None,
        owner: None,
        merge_by_hand: false,
        mine: false,
    }
}

/// Main is green or red and nothing else: a commit still running, one nothing ran on, one not read
/// and a red that names no failed job have no mark.
fn main_of(hub: &MergeLineHub) -> Option<MainView> {
    let main = hub.main.as_ref()?;
    match main.state {
        MainCiState::Green => Some(MainView::Green { checking: main.checking.clone() }),
        MainCiState::Red => red_of(main, hub),
        MainCiState::Running | MainCiState::NothingRan => None,
    }
}

fn red_of(main: &MainStanding, hub: &MergeLineHub) -> Option<MainView> {
    let (first, rest) = main.failed.split_first()?;
    Some(MainView::Red {
        failed: job_of(first),
        also: rest.iter().map(job_of).collect(),
        since: main.red_since.clone(),
        merge: main.merge.as_ref().map(merge_of),
        taken_by: hub.fixing_session.as_ref().map(|session| HubSession {
            id: session.id.clone(),
            title: Some(session.title.clone().unwrap_or_else(|| session.id.clone())),
        }),
        elsewhere: hub.fixing_elsewhere.clone(),
        checking: main.checking.clone(),
    })
}

/// A Manifest Check where the job maps to one, otherwise the job's own name.
fn job_of(job: &MainFailedJob) -> MainRedJob {
    MainRedJob {
        check: job.check.clone().unwrap_or_else(|| job.name.clone()),
        unmapped: job.check.is_none(),
        test: job.tests.first().cloned(),
        log_url: job.log_url.clone(),
    }
}

/// The pull request that turned main red, with its Job named: the ledger's title, else the branch,
/// else the Job's id.
fn merge_of(merge: &MainMerge) -> MainMerge {
    MainMerge {
        session: merge.session.as_ref().map(|session| HubSession {
            id: session.id.clone(),
            title: Some(
                session
                    .title
                    .clone()
                    .or_else(|| merge.branch.clone())
                    .unwrap_or_else(|| session.id.clone()),
            ),
        }),
        ..merge.clone()
    }
}
