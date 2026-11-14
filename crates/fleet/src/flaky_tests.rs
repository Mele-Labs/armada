//! A failing test that threw a pull request out of the merge queue, fixed by one Job on one Mac
//! while every other Job it threw out waits on that claim and is queued again when the fix merges.
//! `docs/concepts/fleet.md`, *A test that fails the merge queue*.
//!
//! **Fleet judges, in this order**, the owner's rulings of 13 and 14 Nov 2026
//! (`.claude/decisions/2026-11-13-claims-store.md`): a test main fails too waits on main's own fix;
//! a test another Job holds is waited on; a test the pull request's changes do not reach, or one
//! that failed the queue for another pull request too, is a flake that Fleet claims and starts the
//! fix Job for, **whatever the Autopilot switch says**; and a test the change does reach is the
//! Job's to answer. **Fleet claims that test only once the Job says it is flaky**
//! (`call_test_flaky`), so a Job's own legitimate fix is never blocked.
//!
//! **A map that never saw the test cannot say a change misses it.** The test map can be wrong,
//! and a wrong "does not reach" sends a fix Job at a test the pull request broke, so with no edge
//! of the test at all the Job is asked instead.
//!
//! **Kept on the Jobs' ledgers**: a fixer holds a `test` row, a Job that waits a `test_wait` row
//! naming its pull request, and a Job asked a `test_asked` row. The claims pass
//! (`crate::job_claims`) claims and renews a fixer's test, asks for a waiter's so the store lists
//! the wait, and ends the wait when the claim does.

use std::collections::BTreeMap;
use std::sync::Arc;

use adapter_traits::{AgentHarness, ClaimKind, Claimant, Claimed, Delivery, Holder, HolderKind, Vcs, WorkProduct};
use api::HostedSessions;
use config::settings as keys;
use ipc::{CloseSession, ManifestId, SessionId, SessionVoice, StartSession};
use store::{AttachmentState, KeptAttachment};

use crate::daemon::Fleet;
use crate::job_claims::words::holder_named;
use crate::job_claims::{key_of, JobHeld, Wait, WaitedOn, Wanted};
use crate::job_claims::wanting::JobRead;
use crate::red_claim::repository_key_of;
use crate::repositories::Served;
use crate::signing_in::this_machine;

/// A Job fixing a failing test holds this row, its target the test.
pub(crate) const TEST: &str = "test";
/// A Job waiting on a failing test holds this row, its target the test and its detail the pull
/// request to queue again.
pub(crate) const TEST_WAIT: &str = "test_wait";
/// A Job told its change reaches a failing test holds this row until it answers.
pub(crate) const TEST_ASKED: &str = "test_asked";

const PULL: &str = "pull";

/// `call_test_flaky` from no Job, or with no log line.
const NOT_CALLABLE: &str = "fleet.test_not_callable";
/// `call_test_flaky` about a test Fleet did not ask the Job about.
const NOT_ASKED: &str = "fleet.test_not_asked";
/// The red commit a wait on main's own fix is on.
const RED: &str = "red";

/// How many of a queue run's failing tests are judged. A run failing more is not a flake.
const TESTS_JUDGED: usize = 3;

/// How long a test's failure of another pull request's queue run counts as a second signal.
const FAILED_ELSEWHERE_FOR_MS: i64 = 24 * 60 * 60 * 1000;

/// One wait on a failing test, as its ledger row holds it and the pass reads it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct TestWait {
    pub pull: u64,
    /// The red commit, where it waits on main's own fix.
    pub red: Option<String>,
    /// Whether main is still red, for a wait on main's fix.
    pub red_standing: bool,
    /// A live Job on this Mac fixing the test, for a project with no store.
    pub here: Option<Claimant>,
}

impl TestWait {
    pub(crate) fn read(detail: &BTreeMap<String, String>) -> Option<TestWait> {
        let pull = detail.get(PULL)?.parse().ok()?;
        Some(TestWait { pull, red: detail.get(RED).cloned(), ..TestWait::default() })
    }
}

/// A wait that ended in a pass: the Job, the test and the pull request to queue again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WaitEnded {
    pub job: String,
    pub manifest_id: String,
    pub test: String,
    pub pull: u64,
}

/// What Fleet made of one failing test.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Judged {
    /// Main fails it too, at this commit.
    OnRed(String),
    /// Another Job holds it.
    Held(Claimant),
    /// Fleet started this Job to fix it, for this reason.
    Started(Claimant, String),
    /// It looks flaky, and the claim could not be made.
    NotClaimed(String),
    /// The pull request's change reaches it: the Job is asked.
    Reaches,
}

fn short(commit: &str) -> &str {
    commit.get(..7).unwrap_or(commit)
}

/// The test's own name, without the modules before it.
fn leaf(test: &str) -> &str {
    test.rsplit("::").next().unwrap_or(test)
}

/// The title of the Job started to fix `test`.
pub(crate) fn fix_title(test: &str) -> String {
    format!("Fix the flaky test {}", leaf(test))
}

/// What the Job whose pull request the queue threw out is told of one failing test.
pub(crate) fn judged_words(number: u64, base: &str, test: &str, judged: &Judged) -> String {
    match judged {
        Judged::OnRed(commit) => format!(
            "{test} fails on {base} too, at {}. Main's own fix covers it, so fix nothing for it here. Fleet queues #{number} again once {base} is green.",
            short(commit)
        ),
        Judged::Held(by) => format!(
            "{test} is being fixed by {}. Fix nothing for it here. Fleet queues #{number} again when that fix merges and {base} is green.",
            holder_named(by)
        ),
        Judged::Started(by, because) => format!(
            "{test} is flaky: {because}. Fleet started {} to fix it, so fix nothing for it here. Fleet queues #{number} again when that fix merges and {base} is green.",
            holder_named(by)
        ),
        Judged::NotClaimed(why) => format!("{test} looks flaky, and Fleet could not claim it, so no fix Job started. {why}"),
        Judged::Reaches => format!(
            "Your change reaches {test}. Fix it in this branch and push. If it is flaky instead, call `call_test_flaky` with the test and the log line that shows it, and Fleet claims it, starts the fix and queues #{number} again when that fix merges."
        ),
    }
}

/// The brief of the Job started to fix `test`.
fn fix_brief(test: &str, because: &str, log: &str) -> String {
    let mut said = format!(
        "{test} failed the merge queue and is flaky: {because}.\n\nFleet claimed it for this Job, so no Job on another Mac fixes it too. Make it pass every time: find why it fails only sometimes and fix that, never by loosening what it checks. Open a pull request with the fix. The claim ends when the fix merges and main is green, and every pull request waiting on it is queued again then."
    );
    if !log.trim().is_empty() {
        said.push_str("\n\n");
        said.push_str(log.trim());
    }
    said
}

/// The words when a wait ends and its pull request is queued again, or could not be.
fn requeued_words(test: &str, number: u64, refused: Option<&str>) -> String {
    match refused {
        None => format!("The fix for {test} merged. Fleet queued #{number} again."),
        Some(why) => format!("The fix for {test} merged, and #{number} could not be queued again: {why}"),
    }
}

/// The test waits `held` has after a pass, brought up to date from what the store answered.
/// **A wait ends** when the Job won the test's claim (nobody holds it now), when no store applies
/// and no Job on this Mac fixes it, or, for a wait on main, when main is green. A store that did
/// not answer leaves the wait as it was. Answers the waits that ended.
pub(crate) fn test_waits_kept(job: &JobRead, held: &mut JobHeld, since: &str) -> Vec<WaitEnded> {
    let mut ended = Vec::new();
    for (test, wait) in &job.test_waits {
        let wanted = Wanted::test(test);
        let on = match &wait.red {
            Some(_) if wait.red_standing => Some(WaitedOn::Main),
            Some(_) => None,
            None => match (held.elsewhere.get(&wanted), &wait.here) {
                (Some(by), _) => Some(WaitedOn::Job(by.clone())),
                (None, Some(here)) => Some(WaitedOn::Job(here.clone())),
                (None, None) if held.sent.contains(&wanted) || held.storeless => None,
                (None, None) => continue,
            },
        };
        match on {
            Some(on) => {
                let same = held.waits.get(test).is_some_and(|was| was.on == on);
                if !same {
                    let since = held.waits.get(test).map_or(since.to_string(), |was| was.since.clone());
                    held.waits.insert(test.clone(), Wait { on, since, requeues: Some(wait.pull) });
                }
            }
            None => {
                held.waits.remove(test);
                ended.push(WaitEnded { job: job.id.clone(), manifest_id: job.manifest_id.clone(), test: test.clone(), pull: wait.pull });
            }
        }
    }
    held.waits.retain(|test, wait| wait.requeues.is_none() || job.test_waits.contains_key(test));
    ended
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
    /// The failing tests of a pull request the merge queue threw out, each judged for its owners:
    /// the words that go under the drop notice. Empty where the run named no test.
    pub(crate) async fn ejection_judged(self: &Arc<Self>, served: &Served, number: u64, owners: &[String], tests: &[String], log: &str) -> String {
        if owners.is_empty() || tests.is_empty() {
            return String::new();
        }
        let base = served.manifest().base().unwrap_or("main").to_string();
        let mut words = Vec::new();
        for test in tests.iter().take(TESTS_JUDGED) {
            let elsewhere = self.queue_failure_noted(served.root(), test, number);
            let judged = self.test_judged(served, number, owners, test, elsewhere, log).await;
            words.push(judged_words(number, &base, test, &judged));
        }
        words.join("\n\n")
    }

    async fn test_judged(self: &Arc<Self>, served: &Served, number: u64, owners: &[String], test: &str, elsewhere: Option<u64>, log: &str) -> Judged {
        let manifest = served.manifest().id().as_str().to_string();
        if let Some(commit) = self.red_failing(served.root(), test).await {
            self.waits_kept_for(&manifest, owners, test, number, Some(&commit)).await;
            return Judged::OnRed(commit);
        }
        if let Some(by) = self.test_held(served, test, owners).await {
            self.waits_kept_for(&manifest, owners, test, number, None).await;
            return Judged::Held(by);
        }
        let because = match elsewhere {
            Some(other) => Some(format!("it failed the merge queue for #{other} too")),
            None if self.change_misses(served, number, test).await => Some(format!("#{number}'s changes do not reach it")),
            None => None,
        };
        match because {
            Some(because) => self.flake_claimed(served, number, owners, test, &because, log).await,
            None => {
                self.asked_kept(&manifest, owners, test, number).await;
                Judged::Reaches
            }
        }
    }

    /// `test` is a flake: claim it, start its fix Job and have `owners` wait on it.
    pub(crate) async fn flake_claimed(self: &Arc<Self>, served: &Served, number: u64, owners: &[String], test: &str, because: &str, log: &str) -> Judged {
        let judged = match self.test_held(served, test, owners).await {
            Some(by) => Judged::Held(by),
            None => self.fix_started(served, test, because, log).await,
        };
        if matches!(judged, Judged::Held(_) | Judged::Started(..)) {
            self.waits_kept_for(served.manifest().id().as_str(), owners, test, number, None).await;
        }
        judged
    }

    /// The red commit main is failing `test` at, where main is red on it.
    async fn red_failing(&self, root: &str, test: &str) -> Option<String> {
        let main = self.store().lock().await.main_ci(root).ok().flatten()?;
        let fails = main.red_at.is_some() && main.failed.iter().any(|job| job.tests.iter().any(|one| one == test));
        fails.then(|| main.red_commit().to_string())
    }

    /// Note that the queue threw out `number` for `test`, and answer another pull request it threw
    /// out for it lately.
    fn queue_failure_noted(&self, root: &str, test: &str, number: u64) -> Option<u64> {
        let now = self.now().epoch_millis().unwrap_or_default();
        self.job_claims().with(|kept| {
            let seen = kept.queue_failed.entry((root.to_string(), test.to_string())).or_default();
            seen.retain(|_, at| now.saturating_sub(*at) < FAILED_ELSEWHERE_FOR_MS);
            let other = seen.keys().copied().find(|other| *other != number);
            seen.insert(number, now);
            other
        })
    }

    /// Whether the test map knows `test` and says none of the pull request's changed files reach it.
    async fn change_misses(&self, served: &Served, number: u64, test: &str) -> bool {
        let repository = served.manifest().id().clone();
        if !self.store().lock().await.test_map_knows(&repository, test).unwrap_or(false) {
            return false;
        }
        let named = number.to_string();
        let Some(diff) = self.forge_asked(served.root(), move |vcs: &V, root: &str| vcs.pull_request_diff(root, &named)).await else {
            return false;
        };
        if diff.files.is_empty() {
            return false;
        }
        match self.store().lock().await.tests_reaching(&repository, &diff.files) {
            Ok(reaching) => !reaching.edges.iter().any(|edge| edge.test == test),
            Err(_) => false,
        }
    }

    /// This Mac's live Job fixing `test` in `manifest`, other than `owners`.
    async fn fixer_here(&self, manifest: &str, test: &str, owners: &[String]) -> Option<Claimant> {
        let store = self.store().lock().await;
        let rows = store.standing_in_order(TEST, manifest, Some(test)).ok()?;
        let job = rows.into_iter().map(|row| row.holder.id).find(|id| {
            !owners.contains(id) && store.session(id).ok().flatten().is_some_and(|session| session.ended_at.is_none())
        })?;
        let title = store.session(&job).ok().flatten().and_then(|session| session.title).unwrap_or_default();
        drop(store);
        let configured: String = self.machine_settings().now().get(keys::MACHINE_NAME);
        let machine = this_machine(&configured);
        Holder::new(HolderKind::Session, &job, &machine, 0).map(|holder| Claimant::new(holder, "", &title))
    }

    /// Who holds `test`'s claim, other than `owners`: this Mac's own fixer first, then the store.
    /// **A claim this Mac's ended Job left is no holder**. A store that does not answer is none:
    /// the claim that follows meets it.
    async fn test_held(&self, served: &Served, test: &str, owners: &[String]) -> Option<Claimant> {
        let manifest = served.manifest().id().as_str().to_string();
        if let Some(here) = self.fixer_here(&manifest, test, owners).await {
            return Some(here);
        }
        let named = test.to_string();
        let read = self
            .at_the_store_switched(served.root(), true, move |opened| {
                let (Some(repository), Some(key)) = (repository_key_of(&opened.slug), key_of(&Wanted::test(&named), &opened.slug)) else { return Ok(None) };
                Ok(opened.store.read_claim(&repository, ClaimKind::Test, &key)?.map(|held| (held, opened.machine.clone())))
            })
            .await;
        let (held, machine) = read.ok().flatten().flatten()?;
        if owners.contains(&held.holder.id) || (held.holder.machine == machine && !self.live_here(&held.holder.id).await) {
            return None;
        }
        Some(held)
    }

    async fn live_here(&self, job: &str) -> bool {
        self.store().lock().await.session(job).ok().flatten().is_some_and(|session| session.ended_at.is_none())
    }

    /// Start the Job that fixes `test`, claimed for it first in the store so a Fleet on another Mac
    /// racing for the same test starts none. **First claim wins**: a Job started here that loses the
    /// claim is closed before it is briefed.
    async fn fix_started(self: &Arc<Self>, served: &Served, test: &str, because: &str, log: &str) -> Judged {
        let manifest = served.manifest().id().as_str().to_string();
        let started = self
            .start_session(StartSession {
                project: None,
                manifest_id: ManifestId::carried(&manifest),
                title: Some(fix_title(test)),
                model: None,
                effort: None,
                mode: None,
                root: None,
                fork: None,
                request: None,
                after: None,
                held: false,
                project_rows: Vec::new(),
                on_your_words: None,
            })
            .await;
        let Ok(started) = started else { return Judged::NotClaimed("The fix Job did not start.".to_string()) };
        let job = started.id.as_str().to_string();
        let close = |job: &str| CloseSession { session_id: SessionId::carried(job) };
        let fixer = match self.test_claimed_for(served, test, &job).await {
            Ok(Some((Claimed::Won, fixer))) => fixer,
            Ok(None) => match self.fixer_here(&manifest, test, &[]).await {
                Some(fixer) => fixer,
                None => {
                    let _ = self.closed_and_parked(close(&job)).await;
                    return Judged::NotClaimed("The fix Job could not be named.".to_string());
                }
            },
            Ok(Some((Claimed::Held(by), _))) => {
                let _ = self.closed_and_parked(close(&job)).await;
                return Judged::Held(by);
            }
            Err(why) => {
                let _ = self.closed_and_parked(close(&job)).await;
                return Judged::NotClaimed(why.said());
            }
        };
        let voice = SessionVoice::Session { id: "fleet".to_string(), title: "Fleet".to_string(), mac: None };
        if self.first_message_from(&job, voice, &fix_brief(test, because, log)).await.is_err() {
            self.test_given_back(served, test).await;
            let _ = self.closed_and_parked(close(&job)).await;
            return Judged::NotClaimed("The fix Job did not take its brief.".to_string());
        }
        Judged::Started(fixer, because.to_string())
    }

    /// Claim `test` for `job`, and keep the row that makes the claims pass renew it. `Ok(None)` where
    /// no store applies: the row alone holds it on this Mac. The claimant as the store has it.
    async fn test_claimed_for(&self, served: &Served, test: &str, job: &str) -> Result<Option<(Claimed, Claimant)>, adapter_traits::StoreError> {
        let manifest = served.manifest().id().as_str().to_string();
        self.ledger_row_kept(job, TEST, &manifest, test, BTreeMap::new()).await;
        let title = fix_title(test);
        let (named, id, since) = (test.to_string(), job.to_string(), self.unix_seconds());
        let claimed = self
            .at_the_store_switched(served.root(), true, move |opened| {
                let (Some(repository), Some(key)) = (repository_key_of(&opened.slug), key_of(&Wanted::test(&named), &opened.slug)) else { return Ok(None) };
                let holder = Holder::new(HolderKind::Session, &id, &opened.machine, since)
                    .ok_or_else(|| adapter_traits::StoreError::Refused("the holder's id could not be written to the store".to_string()))?;
                let claimant = Claimant::new(holder, &opened.login, &title);
                let won = match opened.store.claim(&repository, ClaimKind::Test, &key, &claimant)? {
                    // This Mac's own row for a Job that ended is replaced, not waited on.
                    Claimed::Held(held) if held.holder.machine == opened.machine && held.holder.id != id => {
                        return Ok(Some((Claimed::Held(held), claimant)));
                    }
                    other => other,
                };
                Ok(Some((won, claimant)))
            })
            .await
            .map(Option::flatten);
        let claimed = match claimed {
            Ok(Some((Claimed::Held(held), claimant))) if held.holder.machine == claimant.holder.machine && !self.live_here(&held.holder.id).await => {
                self.test_taken_over(served, test, claimant).await
            }
            other => other,
        };
        if !matches!(claimed, Ok(None) | Ok(Some((Claimed::Won, _)))) {
            self.ledger_row_settled(job, TEST, &manifest, test).await;
        }
        claimed
    }

    async fn test_taken_over(&self, served: &Served, test: &str, claimant: Claimant) -> Result<Option<(Claimed, Claimant)>, adapter_traits::StoreError> {
        let named = test.to_string();
        self.at_the_store_switched(served.root(), true, move |opened| {
            let (Some(repository), Some(key)) = (repository_key_of(&opened.slug), key_of(&Wanted::test(&named), &opened.slug)) else { return Ok(None) };
            Ok(Some((opened.store.replace_claim(&repository, ClaimKind::Test, &key, &claimant)?, claimant)))
        })
        .await
        .map(Option::flatten)
    }

    /// Give back `test`'s claim, for a fix that could not go on.
    async fn test_given_back(&self, served: &Served, test: &str) {
        let named = test.to_string();
        let _ = self
            .at_the_store_switched(served.root(), true, move |opened| {
                let (Some(repository), Some(key)) = (repository_key_of(&opened.slug), key_of(&Wanted::test(&named), &opened.slug)) else { return Ok(()) };
                opened.store.release_claim(&repository, ClaimKind::Test, &key)
            })
            .await;
    }

    /// Each owner waits on `test` for `number`: on main's fix where `red` names its commit.
    async fn waits_kept_for(&self, manifest: &str, owners: &[String], test: &str, number: u64, red: Option<&str>) {
        let mut detail = BTreeMap::from([(PULL.to_string(), number.to_string())]);
        if let Some(red) = red {
            detail.insert(RED.to_string(), red.to_string());
        }
        for owner in owners {
            self.ledger_row_kept(owner, TEST_WAIT, manifest, test, detail.clone()).await;
            self.ledger_row_settled(owner, TEST_ASKED, manifest, test).await;
        }
    }

    async fn asked_kept(&self, manifest: &str, owners: &[String], test: &str, number: u64) {
        let detail = BTreeMap::from([(PULL.to_string(), number.to_string())]);
        for owner in owners {
            self.ledger_row_kept(owner, TEST_ASKED, manifest, test, detail.clone()).await;
        }
    }

    async fn ledger_row_kept(&self, job: &str, kind: &str, manifest: &str, target: &str, detail: BTreeMap<String, String>) {
        let now = self.now().as_str().to_string();
        let row = KeptAttachment {
            holder: store::Holder::session(job),
            kind: kind.to_string(),
            manifest_id: manifest.to_string(),
            target: target.to_string(),
            state: AttachmentState::Standing,
            detail,
            since: now.clone(),
            changed_at: now,
        };
        let _ = self.store().lock().await.attach(&row, false);
    }

    async fn ledger_row_settled(&self, job: &str, kind: &str, manifest: &str, target: &str) {
        let now = self.now().as_str().to_string();
        let _ = self.store().lock().await.settle(&store::Holder::session(job), kind, manifest, target, AttachmentState::Spent, &now);
    }

    /// What a pass needs of each test wait: whether main is still red, for a wait on main, and this
    /// Mac's own fixer, for a project with no store.
    pub(crate) async fn test_waits_read(&self, job: &mut JobRead) {
        if job.test_waits.is_empty() {
            return;
        }
        let red = self.store().lock().await.main_ci(&job.root).ok().flatten().is_some_and(|main| main.red_at.is_some());
        let tests: Vec<String> = job.test_waits.keys().cloned().collect();
        for test in tests {
            let here = self.fixer_here(&job.manifest_id, &test, std::slice::from_ref(&job.id)).await;
            if let Some(wait) = job.test_waits.get_mut(&test) {
                wait.red_standing = red;
                wait.here = here;
            }
        }
    }

    /// A fixer's tests end once its pull request merged and main read green after it: the row is
    /// settled, so the pass gives the claim back.
    pub(crate) async fn fixes_ended(&self, jobs: &mut [JobRead]) {
        for job in jobs.iter_mut().filter(|job| !job.tests.is_empty()) {
            let Some(merged_at) = job.merged_at.clone() else { continue };
            let green_since = self
                .store()
                .lock()
                .await
                .main_ci(&job.root)
                .ok()
                .flatten()
                .is_some_and(|main| main.red_at.is_none() && main.read_at.as_str() > merged_at.as_str());
            if !green_since {
                continue;
            }
            for test in std::mem::take(&mut job.tests) {
                self.ledger_row_settled(&job.id, TEST, &job.manifest_id, &test).await;
            }
        }
    }

    /// Each ended wait: its row settled, and its pull request queued again once no other wait holds
    /// it, with a word to the Job.
    pub(crate) async fn test_waits_ended(self: &Arc<Self>, ended: Vec<WaitEnded>) {
        for one in ended {
            self.ledger_row_settled(&one.job, TEST_WAIT, &one.manifest_id, &one.test).await;
            let still = self.store().lock().await.attachments_of(&store::Holder::session(&one.job)).unwrap_or_default().into_iter().any(|row| {
                row.kind == TEST_WAIT && row.state == AttachmentState::Standing && row.detail.get(PULL) == Some(&one.pull.to_string())
            });
            if still {
                continue;
            }
            let manifest = ManifestId::carried(&one.manifest_id);
            let refused = match self.auto_merge_asked_for(&manifest, one.pull, None).await {
                Ok(None) => None,
                Ok(Some(blocker)) => Some(format!("it clashes with #{} ahead of it in the queue.", blocker.number)),
                Err(why) => Some(why.error().message.clone()),
            };
            let _ = self.told_by_fleet(&one.job, &requeued_words(&one.test, one.pull, refused.as_deref()), ipc::Notice::Claims).await;
        }
    }

    /// Whether `job` waits on a failing test for `number`, so nothing queues it before the fix merges.
    pub(crate) async fn waits_on_a_fix(&self, job: &str, number: u64) -> bool {
        let rows = self.store().lock().await.attachments_of(&store::Holder::session(job)).unwrap_or_default();
        rows.iter().any(|row| row.kind == TEST_WAIT && row.state == AttachmentState::Standing && row.detail.get(PULL) == Some(&number.to_string()))
    }

    /// `call_test_flaky`: the Job Fleet asked about `test` says it is flaky. **Only then is the test
    /// claimed**, so a Job's own fix of a test its change broke is never blocked by a claim.
    pub(crate) async fn test_called_flaky(self: &Arc<Self>, caller: Option<api::Caller>, call: ipc::CallTestFlaky) -> Result<ipc::TestCalledFlaky, api::Refusal> {
        let refuse = |why: &str| self.hosted_refusal(NOT_CALLABLE, why);
        let id = caller
            .as_ref()
            .and_then(|caller| self.session_holding(caller))
            .or_else(|| {
                let named = call.session_id.as_ref()?.as_str().trim().to_string();
                (!named.is_empty()).then_some(named)
            })
            .ok_or_else(|| refuse("a test is called flaky by the Job Fleet asked, and this call is not from one"))?;
        let (test, line) = (call.test.trim().to_string(), call.log_line.trim().to_string());
        if line.is_empty() {
            return Err(refuse("name the line of the failing run's log that shows the test is flaky"));
        }
        let Some((manifest, pull)) = self.test_asked_of(&id, &test).await else {
            let why = format!("Fleet did not ask job {id} about {test}");
            return Err(api::Refusal::IllegalMove(ipc::WireError::raised(NOT_ASKED, &why, self.run_id())));
        };
        let served = self.served_named(Some(&ManifestId::carried(&manifest)))?;
        let because = format!("the Job that owns #{pull} says so, citing \"{line}\"");
        let judged = self.flake_claimed(&served, pull, std::slice::from_ref(&id), &test, &because, &line).await;
        if matches!(judged, Judged::NotClaimed(_)) {
            self.ledger_row_settled(&id, TEST_ASKED, &manifest, &test).await;
        }
        let base = served.manifest().base().unwrap_or("main").to_string();
        Ok(ipc::TestCalledFlaky { pull, said: judged_words(pull, &base, &test, &judged) })
    }

    /// The pull request a Job was asked about for `test`, where it was and has not answered.
    pub(crate) async fn test_asked_of(&self, job: &str, test: &str) -> Option<(String, u64)> {
        let rows = self.store().lock().await.attachments_of(&store::Holder::session(job)).ok()?;
        rows.into_iter()
            .find(|row| row.kind == TEST_ASKED && row.target == test && row.state == AttachmentState::Standing)
            .and_then(|row| Some((row.manifest_id.clone(), row.detail.get(PULL)?.parse().ok()?)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn by() -> Claimant {
        Claimant::new(Holder::new(HolderKind::Session, "j1", "home", 0).expect("a holder"), "nick", "Fix the flaky test two_macs_acting")
    }

    #[test]
    fn a_fix_job_is_titled_by_the_tests_own_name() {
        assert_eq!(fix_title("tests::two_macs::two_macs_acting"), "Fix the flaky test two_macs_acting");
        assert_eq!(fix_title("An Empty List Folds"), "Fix the flaky test An Empty List Folds");
    }

    #[test]
    fn each_judgement_says_what_ends_the_wait_and_only_the_asked_one_names_the_act() {
        let started = judged_words(12, "main", "tests::a", &Judged::Started(by(), "#12's changes do not reach it".into()));
        assert!(started.contains("Job \"Fix the flaky test two_macs_acting\" on nick's Mac home"), "{started}");
        assert!(started.contains("queues #12 again when that fix merges"), "{started}");
        let reaches = judged_words(12, "main", "tests::a", &Judged::Reaches);
        assert!(reaches.contains("call_test_flaky"), "{reaches}");
        let red = judged_words(12, "main", "tests::a", &Judged::OnRed("abcdef0123".into()));
        assert!(red.contains("at abcdef0"), "{red}");
    }

    fn waiting(pull: u64) -> JobRead {
        JobRead { id: "w".into(), test_waits: BTreeMap::from([("tests::a".to_string(), TestWait { pull, ..TestWait::default() })]), ..JobRead::default() }
    }

    #[test]
    fn a_test_wait_stands_while_another_holds_it_and_ends_once_the_job_wins_it() {
        let job = waiting(12);
        let mut held = JobHeld::default();
        held.elsewhere.insert(Wanted::test("tests::a"), by());
        assert!(test_waits_kept(&job, &mut held, "t0").is_empty());
        assert_eq!(held.waits["tests::a"].requeues, Some(12));
        held.elsewhere.clear();
        assert!(test_waits_kept(&job, &mut held, "t1").is_empty(), "a store that did not answer leaves it");
        assert!(held.waits.contains_key("tests::a"));
        held.sent.insert(Wanted::test("tests::a"));
        let ended = test_waits_kept(&job, &mut held, "t2");
        assert_eq!(ended.iter().map(|one| one.pull).collect::<Vec<_>>(), vec![12]);
        assert!(held.waits.is_empty());
    }

    #[test]
    fn a_wait_on_main_ends_when_main_is_green() {
        let mut job = waiting(7);
        job.test_waits.get_mut("tests::a").expect("a wait").red = Some("abc".into());
        job.test_waits.get_mut("tests::a").expect("a wait").red_standing = true;
        let mut held = JobHeld::default();
        assert!(test_waits_kept(&job, &mut held, "t0").is_empty());
        assert_eq!(held.waits["tests::a"].on, WaitedOn::Main);
        job.test_waits.get_mut("tests::a").expect("a wait").red_standing = false;
        assert_eq!(test_waits_kept(&job, &mut held, "t1").len(), 1);
    }
}
