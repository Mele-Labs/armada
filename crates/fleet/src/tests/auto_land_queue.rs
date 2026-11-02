//! Auto land and the merge queue: a pull request is not queued behind one it conflicts with. The
//! decision is tested with a stand-in for git; the Fleet cases run real git against a bare
//! repository standing in for the forge's remote, and `FakeVcs` for the forge itself.

use std::path::Path;

use adapter_traits::{CiState, FollowedCheck, FollowedPull, FromOutside, Oid, PullQueue, PullRequestStanding, QueueEntry, QueueState};
use api::{HostedSessions, Sessions};
use ipc::{AttachmentReport, SessionFact, SessionId, SessionReport, SetSessionAutoLand};
use testkit::Delivered;

use super::pull_requesting::{facts, forge_ran, passed};
use super::session_host::{rig_fitted, Rig};
use super::tmp::TempDir;
use crate::session_host::queue_guard::{ahead_of, decide, Blocker, Guard, Tested};

const NUMBER: u64 = 12;
const BLOCKER: u64 = 11;

fn entry(number: u64, position: u32, head: Option<&str>) -> QueueEntry {
    QueueEntry { number, position, state: QueueState::Queued, head: head.and_then(Oid::parse) }
}

fn sha(digit: char) -> String {
    digit.to_string().repeat(40)
}

fn clash(files: &[&str]) -> Tested {
    Tested::Clash(files.iter().map(|one| one.to_string()).collect())
}

#[test]
fn a_pull_request_outside_the_queue_has_every_entry_ahead_of_it_and_a_queued_one_those_before_it() {
    let queue = [entry(30, 3, None), entry(10, 1, None), entry(20, 2, None)];

    let outside: Vec<u64> = ahead_of(99, &queue).iter().map(|one| one.number).collect();
    let second: Vec<u64> = ahead_of(20, &queue).iter().map(|one| one.number).collect();
    let first: Vec<u64> = ahead_of(10, &queue).iter().map(|one| one.number).collect();

    assert_eq!(outside, [10, 20, 30], "soonest first");
    assert_eq!(second, [10]);
    assert!(first.is_empty());
}

#[test]
fn the_first_entry_ahead_that_clashes_holds_a_pull_request_that_is_not_queued() {
    let queue = [entry(10, 1, None), entry(20, 2, None), entry(30, 3, None)];
    let mut tested = Vec::new();

    let guard = decide(99, &queue, |one| {
        tested.push(one.number);
        if one.number == 10 { Tested::Clean } else { clash(&["a.rs", "b.rs"]) }
    });

    assert_eq!(guard, Guard::Hold(Blocker { number: 20, files: vec!["a.rs".into(), "b.rs".into()] }));
    assert_eq!(tested, [10, 20], "nothing after the blocker is tested");
}

#[test]
fn a_queued_pull_request_that_clashes_with_one_ahead_is_to_be_taken_out_and_one_behind_is_ignored() {
    let queue = [entry(10, 1, None), entry(NUMBER, 2, None), entry(30, 3, None)];

    let guard = decide(NUMBER, &queue, |one| if one.number == 30 { clash(&["x"]) } else { Tested::Clean });
    assert_eq!(guard, Guard::Go, "the one behind it is not a blocker");

    let guard = decide(NUMBER, &queue, |_| clash(&["x"]));
    assert_eq!(guard, Guard::Dequeue(Blocker { number: 10, files: vec!["x".into()] }));
}

#[test]
fn what_could_not_be_tested_is_not_a_clash_and_is_not_a_clean_test() {
    let queue = [entry(10, 1, None), entry(20, 2, None)];

    assert_eq!(decide(99, &queue, |_| Tested::Unknown), Guard::Unchecked);
    assert_eq!(decide(99, &queue, |one| if one.number == 10 { Tested::Unknown } else { Tested::Clean }), Guard::Unchecked);
    assert_eq!(decide(99, &queue, |one| if one.number == 10 { Tested::Unknown } else { clash(&["f"]) }), Guard::Hold(Blocker { number: 20, files: vec!["f".into()] }));
    assert_eq!(decide(99, &[], |_| Tested::Unknown), Guard::Go, "an empty queue has nothing to test against");
}

fn git(repo: &Path, args: &[&str]) -> String {
    let run = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false"])
        .args(args)
        .output()
        .expect("git on PATH");
    assert!(run.status.success(), "git {args:?}: {run:?}");
    String::from_utf8_lossy(&run.stdout).trim().to_string()
}

fn twenty_lines(fifth: &str) -> String {
    (1..=20).map(|n| if n == 5 { format!("{fifth}\n") } else { format!("line {n}\n") }).collect()
}

/// The served repository, with a bare `origin` beside it that holds `refs/pull/<n>/head`.
struct Remote {
    root: String,
    /// Held so the remote outlives the test.
    _bare: TempDir,
}

impl Remote {
    fn at(root: &str) -> Remote {
        let bare = TempDir::new();
        git(bare.path(), &["init", "--quiet", "--bare"]);
        let repo = Path::new(root);
        git(repo, &["-c", "init.defaultBranch=main", "init", "--quiet"]);
        std::fs::write(repo.join("f.txt"), twenty_lines("line 5")).unwrap();
        git(repo, &["add", "f.txt"]);
        git(repo, &["commit", "--quiet", "-m", "start"]);
        git(repo, &["remote", "add", "origin", &bare.path().to_string_lossy()]);
        Remote { root: root.to_string(), _bare: bare }
    }

    /// Pull request `number`'s head: the base with line 5 as `fifth` says, pushed as the forge keeps it.
    fn head(&self, number: u64, fifth: &str) -> String {
        let repo = Path::new(&self.root);
        git(repo, &["checkout", "--quiet", "--detach", "main"]);
        std::fs::write(repo.join("f.txt"), twenty_lines(fifth)).unwrap();
        git(repo, &["commit", "--quiet", "-am", fifth]);
        let head = git(repo, &["rev-parse", "HEAD"]);
        git(repo, &["push", "--quiet", "--force", "origin", &format!("HEAD:refs/pull/{number}/head")]);
        git(repo, &["checkout", "--quiet", "main"]);
        head
    }
}

fn manifest_with_a_base() -> config::Manifest {
    config::Manifest::parse(Path::new("armada.yml"), "version: 1\nid: 01FIXTUREMANIFEST\nbase: main\n").expect("parses")
}

fn flag(id: &SessionId, on: bool) -> SetSessionAutoLand {
    SetSessionAutoLand { id: id.clone(), on }
}

fn pull(number: u64, head: &str, queue: PullQueue) -> FollowedPull {
    FollowedPull {
        number,
        branch: FromOutside::verbatim(format!("armada/{number}")),
        url: FromOutside::verbatim(format!("https://forge.invalid/armada/pull/{number}")),
        author: None,
        head: Some(head.to_string()),
        conflicting: false,
        queue,
        checks: vec![FollowedCheck { name: FromOutside::verbatim("ci"), state: CiState::Passed, required: true, hung: false, log_url: None }],
        title: None,
        auto_merge: false,
    }
}

/// A job with Auto land on that opened pull request 12, in a repository whose base is `main`.
async fn landing_pull_request_12() -> (Rig, Remote) {
    let rig = rig_fitted(|fittings| fittings.starting().manifest = manifest_with_a_base());
    let remote = Remote::at(&rig.root);
    let id = rig.start().await;
    rig.fleet.set_session_auto_land(flag(&id, true)).await.expect("on");
    let attachment = AttachmentReport {
        kind: "pr".into(),
        target: NUMBER.to_string(),
        detail: [("created".to_string(), "true".to_string())].into(),
    };
    rig.fleet
        .report_session(SessionReport { harness: adapters::HOSTED_HARNESS.into(), session_id: id, fact: SessionFact::Attached { attachment } })
        .await
        .expect("attached");
    rig.fleet.vcs().now_pull_request(Some(facts(PullRequestStanding::Open)));
    forge_ran(&rig.fleet, passed());
    (rig, remote)
}

async fn read_the_pull_reading(rig: &Rig) {
    let served = rig.fleet.repositories().served().remove(0);
    rig.fleet.pulls_told(&served).await;
}

fn said_to_the_job(rig: &Rig) -> String {
    (0..rig.stand_in.starts().len())
        .flat_map(|at| rig.stand_in.process(at).sent.lock().unwrap().clone())
        .collect::<Vec<_>>()
        .join("\n")
}

fn times_told(rig: &Rig, words: &str) -> usize {
    said_to_the_job(rig).matches(words).count()
}

fn queue_is(rig: &Rig, queue: Vec<QueueEntry>) {
    rig.fleet.vcs().main_ci.queue_is(Some(queue));
}

const WAITS: &str = "#12 waits: it conflicts with #11";

/// The incident: queued behind a pull request it conflicts with in one line, a pull request is
/// held, told once, and queued only when the one ahead is gone.
#[tokio::test]
async fn a_pull_request_that_clashes_with_one_in_the_queue_is_held_told_once_and_asked_when_it_clears() {
    let (rig, remote) = landing_pull_request_12().await;
    let ahead = remote.head(BLOCKER, "the blocker's line");
    let mine = remote.head(NUMBER, "my line");
    rig.fleet.vcs().main_ci.followed_are(Some(vec![pull(NUMBER, &mine, PullQueue::Outside)]));
    queue_is(&rig, vec![entry(BLOCKER, 1, Some(&ahead))]);

    read_the_pull_reading(&rig).await;
    read_the_pull_reading(&rig).await;

    assert_eq!(rig.fleet.vcs().times_asked_for_auto_merge(), 0, "not while the blocker is queued");
    assert_eq!(times_told(&rig, WAITS), 1, "told once: {}", said_to_the_job(&rig));
    assert!(said_to_the_job(&rig).contains("f.txt"), "the files are named");

    queue_is(&rig, Vec::new());
    read_the_pull_reading(&rig).await;

    assert_eq!(rig.fleet.vcs().times_asked_for_auto_merge(), 1, "asked once the blocker left, at the same commit");
    assert_eq!(times_told(&rig, WAITS), 1);
}

#[tokio::test]
async fn a_held_pull_request_is_asked_once_the_job_resolves_the_clash_with_the_blocker_still_queued() {
    let (rig, remote) = landing_pull_request_12().await;
    let ahead = remote.head(BLOCKER, "the blocker's line");
    let mine = remote.head(NUMBER, "my line");
    rig.fleet.vcs().main_ci.followed_are(Some(vec![pull(NUMBER, &mine, PullQueue::Outside)]));
    queue_is(&rig, vec![entry(BLOCKER, 1, Some(&ahead))]);
    read_the_pull_reading(&rig).await;
    assert_eq!(rig.fleet.vcs().times_asked_for_auto_merge(), 0);

    let resolved = remote.head(NUMBER, "the blocker's line");
    rig.fleet.vcs().main_ci.followed_are(Some(vec![pull(NUMBER, &resolved, PullQueue::Outside)]));
    read_the_pull_reading(&rig).await;

    assert_eq!(rig.fleet.vcs().times_asked_for_auto_merge(), 1);
}

#[tokio::test]
async fn a_pull_request_that_clashes_with_nothing_in_the_queue_is_asked() {
    let (rig, remote) = landing_pull_request_12().await;
    let mine = remote.head(NUMBER, "my line");
    rig.fleet.vcs().main_ci.followed_are(Some(vec![pull(NUMBER, &mine, PullQueue::Outside)]));
    // The blocker's head is one the remote does not hold, so nothing can be tested: fail open.
    queue_is(&rig, vec![entry(BLOCKER, 1, Some(&sha('e')))]);
    read_the_pull_reading(&rig).await;
    assert_eq!(rig.fleet.vcs().times_asked_for_auto_merge(), 1, "a head that cannot be tested is not a clash");

    let (rig, remote) = landing_pull_request_12().await;
    let mine = remote.head(NUMBER, "my line");
    rig.fleet.vcs().main_ci.followed_are(Some(vec![pull(NUMBER, &mine, PullQueue::Outside)]));
    rig.fleet.vcs().main_ci.queue_is(None);
    read_the_pull_reading(&rig).await;
    assert_eq!(rig.fleet.vcs().times_asked_for_auto_merge(), 1, "a queue the forge would not list is no reason to wait");
}

#[tokio::test]
async fn a_queued_pull_request_that_now_clashes_with_one_ahead_is_taken_out_held_and_told() {
    let (rig, remote) = landing_pull_request_12().await;
    let ahead = remote.head(BLOCKER, "the blocker's line");
    let mine = remote.head(NUMBER, "my line");
    rig.fleet.vcs().main_ci.followed_are(Some(vec![pull(NUMBER, &mine, PullQueue::Waiting)]));

    queue_is(&rig, vec![entry(NUMBER, 1, Some(&mine)), entry(BLOCKER, 2, Some(&ahead))]);
    read_the_pull_reading(&rig).await;
    let dequeued = |delivered: &[Delivered]| delivered.iter().any(|one| matches!(one, Delivered::Dequeued { .. }));
    assert!(!dequeued(&rig.fleet.vcs().delivered()), "behind it, the other is no blocker");

    queue_is(&rig, vec![entry(BLOCKER, 1, Some(&ahead)), entry(NUMBER, 2, Some(&mine))]);
    read_the_pull_reading(&rig).await;

    let delivered = rig.fleet.vcs().delivered();
    assert!(dequeued(&delivered) && delivered.contains(&Delivered::Dequeued { pull_request: NUMBER.to_string() }), "{delivered:?}");
    assert_eq!(times_told(&rig, "#12 was taken out of the merge queue: it now conflicts with #11"), 1);

    // Out of the queue now, and held: not asked, not told again.
    rig.fleet.vcs().main_ci.followed_are(Some(vec![pull(NUMBER, &mine, PullQueue::Outside)]));
    queue_is(&rig, vec![entry(BLOCKER, 1, Some(&ahead))]);
    read_the_pull_reading(&rig).await;
    assert_eq!(rig.fleet.vcs().times_asked_for_auto_merge(), 0);
    assert_eq!(said_to_the_job(&rig).matches("conflicts with #11").count(), 1);

    queue_is(&rig, Vec::new());
    read_the_pull_reading(&rig).await;
    assert_eq!(rig.fleet.vcs().times_asked_for_auto_merge(), 1, "asked again once the blocker has gone");
}
