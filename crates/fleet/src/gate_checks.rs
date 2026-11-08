//! The Checks Fleet runs to close a plan group's `checks_green` gate. `docs/concepts/session/the-plan.md`,
//! *Gates*.
//!
//! **A push of the Job's branch is the only thing that asks for a run**, and only while a group gated
//! on `checks_green` is open. The run is the manifest's test Check over the tests the push touched, as
//! an agent's `armada check test --changed` reads them, in one Fleet-held detached worktree per
//! repository whose `target/` stays warm.
//!
//! **A test red at the base on main as well does not hold the gate.** A failed run names its failures,
//! and Fleet runs only those at the commit the branch met main at, once per (repository, base, test).
//!
//! **One run at a time per repository**, behind every Check a Drone waits on. A newer push of a Job
//! replaces its queued older one, and a Job whose gate closed meanwhile is not run.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use checks_runner::{Priority, TouchedRun, Writing};
use core_model::{EdgeGrain, FieldValue, Level, ManifestId, Narrowing};
use ipc::TestsReaching;
use store::{KeptGate, KeptGateChecks, KeptSession, Store, CHECKS_GREEN};
use tokio::sync::Notify;
use verification::Exit;

use crate::daemon::Fleet;
use crate::places::Asking;

/// The manifest's Check whose tests close the gate.
const TEST_CHECK: &str = "test";

/// What a Job's push asks: where its worktree is and the branch pushed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Asked {
    pub job: String,
    pub worktree: String,
    pub branch: Option<String>,
}

#[derive(Default)]
struct Line {
    running: bool,
    queued: Vec<Asked>,
}

/// The runs owed, by repository, and what each base said of the tests asked of it. Never written down: a
/// Fleet that starts waits for the next push.
#[derive(Default)]
pub struct GateRuns {
    lines: Mutex<BTreeMap<String, Line>>,
    woken: Notify,
    /// Whether a test was red at a base, by (repository, base commit, test).
    base: Mutex<BTreeMap<(String, String, String), bool>>,
}

impl GateRuns {
    /// A push of `asked.job` in `repository`. Replaces the Job's run still queued.
    pub(crate) fn asked(&self, repository: &str, asked: Asked) {
        let mut lines = self.lines.lock().unwrap_or_else(PoisonError::into_inner);
        let line = lines.entry(repository.to_string()).or_default();
        match line.queued.iter_mut().find(|one| one.job == asked.job) {
            Some(older) => *older = asked,
            None => line.queued.push(asked),
        }
        drop(lines);
        self.woken.notify_one();
    }

    /// Resolves when a push was asked since the last wake.
    pub(crate) async fn woken(&self) {
        self.woken.notified().await;
    }

    /// The repositories with a run owed and none going, each marked as going.
    pub(crate) fn due(&self) -> Vec<String> {
        let mut lines = self.lines.lock().unwrap_or_else(PoisonError::into_inner);
        lines
            .iter_mut()
            .filter(|(_, line)| !line.running && !line.queued.is_empty())
            .map(|(repository, line)| {
                line.running = true;
                repository.clone()
            })
            .collect()
    }

    /// The oldest run owed in `repository`. None ends the repository's going: the next push starts it.
    pub(crate) fn next(&self, repository: &str) -> Option<Asked> {
        let mut lines = self.lines.lock().unwrap_or_else(PoisonError::into_inner);
        let line = lines.get_mut(repository)?;
        if line.queued.is_empty() {
            line.running = false;
            return None;
        }
        Some(line.queued.remove(0))
    }

    fn base_said(&self, repository: &str, base: &str, test: &str) -> Option<bool> {
        let held = self.base.lock().unwrap_or_else(PoisonError::into_inner);
        held.get(&(repository.to_string(), base.to_string(), test.to_string())).copied()
    }

    fn base_kept(&self, repository: &str, base: &str, test: &str, red: bool) {
        let mut held = self.base.lock().unwrap_or_else(PoisonError::into_inner);
        held.insert((repository.to_string(), base.to_string(), test.to_string()), red);
    }
}

/// How a run came out, for the verdict.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Ended {
    /// No test was touched, so none ran.
    NoTestTouched,
    Passed,
    /// The run failed. `failed` are the tests its summary named, none where it named none, and
    /// `red_at_base` are those of them that fail at the base on main too.
    Failed { failed: Vec<String>, red_at_base: Vec<String> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    Green { fact: String, already_red: Vec<String> },
    /// `failed` are the tests red here and not at the base.
    Red { failed: Vec<String>, already_red: Vec<String> },
}

/// What a run at the commit whose first seven characters are `sha7` comes to. **Green where every test
/// that failed is red at the base too**; a failure the summary named no test for holds the gate.
pub(crate) fn verdict(sha7: &str, ended: &Ended) -> Verdict {
    match ended {
        Ended::NoTestTouched => Verdict::Green { fact: format!("Checks passed at {sha7}: no test touched"), already_red: Vec::new() },
        Ended::Passed => Verdict::Green { fact: format!("Checks passed at {sha7}"), already_red: Vec::new() },
        Ended::Failed { failed, red_at_base } => {
            let (already_red, new): (Vec<String>, Vec<String>) = failed.iter().cloned().partition(|name| red_at_base.contains(name));
            match new.is_empty() && !already_red.is_empty() {
                true => Verdict::Green {
                    fact: format!("Checks passed at {sha7}, {} already red on main", already_red.len()),
                    already_red,
                },
                false => Verdict::Red { failed: new, already_red },
            }
        }
    }
}

fn short(sha: &str) -> String {
    sha.chars().take(7).collect()
}

/// The `(member, name)` of each test the map says reaches the changed paths, for the Check `key`: a
/// test-grain edge as it is, a file-grain edge read for the tests in the file, as `armada check`
/// reads them. Blocking: it reads source.
fn mapped_tests(answer: &TestsReaching, key: &str, root: &Path, dir: &str, narrowing: &Narrowing) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = Vec::new();
    if !answer.holds_edges {
        return found;
    }
    for group in answer.checks.iter().filter(|group| group.check == key) {
        for test in &group.tests {
            let names = match test.grain.domain() {
                EdgeGrain::Test => vec![(group.member.clone(), test.test.clone())],
                EdgeGrain::File => {
                    let file = match dir.is_empty() {
                        true => test.test.clone(),
                        false => test.test.strip_prefix(dir).and_then(|rest| rest.strip_prefix('/')).unwrap_or(&test.test).to_string(),
                    };
                    let read = checks_runner::touched(root, narrowing, std::slice::from_ref(&file));
                    let member = read.members.first().cloned().unwrap_or_default();
                    read.names.into_iter().map(|name| (member.clone(), name)).collect()
                }
            };
            for one in names {
                if !found.contains(&one) {
                    found.push(one);
                }
            }
        }
    }
    found
}

/// The first group gated on `checks_green` that has not closed, as the store reads the Job's plan.
fn open_group(store: &Store, id: &str) -> Option<String> {
    let plan = store.session_plan(id).ok()??;
    plan.groups.into_iter().find(|group| group.gate.as_deref() == Some(CHECKS_GREEN) && group.closed.is_none()).map(|group| group.id)
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
    /// The Job pushed its branch. Queues a run where a group gated on `checks_green` is open, and does
    /// nothing otherwise.
    pub(crate) fn gate_check_asked(&self, store: &Store, session: &KeptSession, branch: Option<String>) {
        let Some(repository) = session.manifest_id.as_deref() else { return };
        if open_group(store, &session.id).is_none() {
            return;
        }
        let asked = Asked { job: session.id.clone(), worktree: self.worktree_of(&session.cwd), branch };
        self.gate_runs().asked(repository, asked);
    }

    /// Start each repository's runs owed, on a task of its own.
    pub(crate) fn gate_checks_started(self: &Arc<Self>) {
        for repository in self.gate_runs().due() {
            let fleet = Arc::clone(self);
            tokio::spawn(async move {
                while let Some(asked) = fleet.gate_runs().next(&repository) {
                    fleet.gate_check_ran(&repository, &asked).await;
                }
            });
        }
    }

    /// One run: the commit pushed, what it changed since the base, the tests those touch, and the
    /// verdict. A reading git or the manifest cannot give is said on the console and ends the run.
    async fn gate_check_ran(&self, repository: &str, asked: &Asked) {
        let Some(served) = self.repositories().serving(repository) else { return };
        let id = served.manifest().id().clone();
        let said = |why: String| {
            self.said(Level::Warn, "a gate's Checks did not run", &id, [("job", FieldValue::Str(asked.job.clone())), ("why", FieldValue::Str(why))])
        };
        let Some(check) = served.manifest().check(TEST_CHECK) else { return said(format!("the manifest declares no {TEST_CHECK} Check")) };
        let base = served.manifest().base().unwrap_or("main").to_string();
        let (worktree, branch) = (asked.worktree.clone(), asked.branch.clone());
        let read = tokio::task::spawn_blocking(move || {
            let sha = adapters::territory::head_of(Path::new(&worktree), branch.as_deref().unwrap_or("HEAD"))?;
            let met = adapters::gate_worktree::met_at(Path::new(&worktree), &base, &sha)?;
            let changed = adapters::gate_worktree::changed_between(Path::new(&worktree), &met, &sha)?;
            Ok::<_, adapters::territory::NotRead>((sha, met, changed))
        })
        .await;
        let (sha, met, changed) = match read {
            Ok(Ok(read)) => read,
            Ok(Err(why)) => return said(why.to_string()),
            Err(why) => return said(why.to_string()),
        };
        let _place = self.room_asking(Asking::GateChecked).place().await;
        if !check.covers(&changed) {
            return self.gate_checked(&asked.job, &sha, Ended::NoTestTouched).await;
        }
        let root = PathBuf::from(served.root());
        let dir = served.manifest().dir().to_string();
        let at = match checked_out(&root, &sha).await {
            Ok(at) => at,
            Err(why) => return said(why),
        };
        let manifest_dir = at.join(&dir);
        let from_map = match (check.narrow().filter(|narrowing| narrowing.under().is_some()), self.tests_reaching(&id, &changed).await) {
            (Some(narrowing), Ok(answer)) => {
                let (narrowing, manifest_dir, dir) = (narrowing.clone(), manifest_dir.clone(), dir.clone());
                tokio::task::spawn_blocking(move || mapped_tests(&answer, TEST_CHECK, &manifest_dir, &dir, &narrowing)).await.unwrap_or_default()
            }
            _ => Vec::new(),
        };
        let command = match checks_runner::touched_run_with(&manifest_dir, check.one_test(), check.narrow(), &changed, &from_map) {
            TouchedRun::None => return self.gate_checked(&asked.job, &sha, Ended::NoTestTouched).await,
            TouchedRun::NotApplicable => return said(format!("the {TEST_CHECK} Check cannot run a test by name")),
            TouchedRun::Tests { command, .. } => command,
        };
        if !self.gate_check_began(&asked.job, &sha).await {
            return;
        }
        let attempt = self.gate_command_ran(check, &command, &manifest_dir).await;
        if attempt.exit == Exit::Code(0) {
            return self.gate_checked(&asked.job, &sha, Ended::Passed).await;
        }
        let failed = checks_runner::failing_tests(&attempt.output);
        let red_at_base = self.red_at_base(&id, check, &root, &dir, &met, &failed).await;
        self.gate_checked(&asked.job, &sha, Ended::Failed { failed, red_at_base }).await;
    }

    /// Which of `failed` fail at `base` too: the base's answers kept, and only the tests it has not been
    /// asked about run, in one command, in the same worktree checked out there.
    async fn red_at_base(&self, repository: &ManifestId, check: &config::Check, root: &Path, dir: &str, base: &str, failed: &[String]) -> Vec<String> {
        let key = repository.as_str();
        let asking: Vec<&str> = failed.iter().map(String::as_str).filter(|name| self.gate_runs().base_said(key, base, name).is_none()).collect();
        if !asking.is_empty() {
            let Ok(at) = checked_out(root, base).await else { return Vec::new() };
            let manifest_dir = at.join(dir);
            let Some(run) = check.one_test() else { return Vec::new() };
            let Ok(command) = checks_runner::tests_in(&manifest_dir, run, check.narrow(), &asking) else { return Vec::new() };
            let attempt = self.gate_command_ran(check, &command, &manifest_dir).await;
            let red = checks_runner::failing_tests(&attempt.output);
            // A run that failed and named nothing did not get to the tests: it says nothing of them.
            if attempt.exit == Exit::Code(0) || !red.is_empty() {
                for name in &asking {
                    self.gate_runs().base_kept(key, base, name, red.iter().any(|one| one == name));
                }
            }
        }
        failed.iter().filter(|name| self.gate_runs().base_said(key, base, name) == Some(true)).cloned().collect()
    }

    /// `command` in `dir`, at the width the Check is given, behind a place the caller holds.
    async fn gate_command_ran(&self, check: &config::Check, command: &str, dir: &Path) -> checks_runner::Attempt {
        let width = self.places_width().narrowed_to(check.width());
        let command = checks_runner::resolve_width(command, width);
        let mut env = vec![(checks_runner::HELD_ENV.to_string(), "1".to_string())];
        env.extend(checks_runner::build_cap(width));
        checks_runner::run_until(&command, dir, self.budget().duration(), Writing::Nowhere, &env, std::future::pending(), Priority::Low).await
    }

    /// A run began at `sha`: kept as running and the Job drawn again. False where no group is open, and
    /// nothing is run.
    pub(crate) async fn gate_check_began(&self, id: &str, sha: &str) -> bool {
        let began = {
            let mut store = self.store().lock().await;
            let run = KeptGateChecks { commit: short(sha), state: "running".into(), failed: Vec::new(), already_red: Vec::new(), at: self.timeline().now() };
            open_group(&store, id).is_some() && store.keep_session_gate_checks(id, &run).is_ok()
        };
        if began {
            let _ = self.published_hosted(id).await;
        }
        began
    }

    /// A run at `sha` ended. Closes the first open `checks_green` gate where the verdict is green and
    /// keeps the run as red where it is not. Nothing is kept for a Job whose gate has closed.
    pub(crate) async fn gate_checked(&self, id: &str, sha: &str, ended: Ended) {
        let sha7 = short(sha);
        let now = self.timeline().now();
        let kept = {
            let mut store = self.store().lock().await;
            let Some(group_id) = open_group(&store, id) else { return };
            match verdict(&sha7, &ended) {
                Verdict::Green { fact, already_red } => {
                    let gate = KeptGate {
                        group_id,
                        gate: CHECKS_GREEN.to_string(),
                        version: 0,
                        closed_at: now,
                        fact,
                        source: format!("checks:{sha}"),
                        commit: Some(sha7),
                        already_red,
                        skipped: false,
                    };
                    store.close_session_gate(id, &gate).is_ok() && store.clear_session_gate_checks(id).is_ok()
                }
                Verdict::Red { failed, already_red } => {
                    store.keep_session_gate_checks(id, &KeptGateChecks { commit: sha7, state: "red".into(), failed, already_red, at: now }).is_ok()
                }
            }
        };
        if kept {
            let _ = self.published_hosted(id).await;
        }
    }
}

/// The repository's gate-check worktree at `sha`.
async fn checked_out(root: &Path, sha: &str) -> Result<PathBuf, String> {
    let (root, sha) = (root.to_path_buf(), sha.to_string());
    tokio::task::spawn_blocking(move || adapters::gate_worktree::checked_out(&root, &sha).map_err(|why| why.to_string()))
        .await
        .map_err(|why| why.to_string())?
}
