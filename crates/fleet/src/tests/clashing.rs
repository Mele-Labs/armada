//! Fleet telling two agents their changes conflict, against real git in worktrees of one repository:
//! a file both wrote that merges clean says nothing, the same lines are named to both sides once,
//! a holder that ends takes its clashes with it, and main moving is told to the holder it conflicts
//! with. What git decides is `adapters`' to assert; under test here is who is told what, and when.

use std::path::PathBuf;
use std::sync::Arc;

use api::{HostedSessions as _, Needs as _, Sessions as _};
use core_model::Timestamp;
use ipc::{SessionFact, SessionId, SessionOrigin, SessionReport, TakeHeld};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use super::concurrency::{approved, one_scoped_step};
use super::daemon::{fittings, one};
use super::needs::git;
use super::tmp::TempDir;
use crate::clashing::{Opposed, Side, Who, Writes};
use crate::daemon::Fleet;
use crate::peers::SPACING;
use crate::slots::Concurrency;

type Hosted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

fn twenty_lines() -> String {
    (1..=20).map(|n| format!("line {n}\n")).collect()
}

struct Rig {
    fleet: Arc<Hosted>,
    home: TempDir,
}

fn a_rig() -> Rig {
    let home = TempDir::new();
    git(home.path(), &["-c", "init.defaultBranch=main", "init", "--quiet"]);
    std::fs::write(home.path().join("f.txt"), twenty_lines()).unwrap();
    std::fs::write(home.path().join("g.txt"), "g\n").unwrap();
    git(home.path(), &["add", "f.txt", "g.txt"]);
    git(home.path(), &["commit", "--quiet", "-m", "start"]);
    let mut fittings = fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.starting().workflows = one(one_scoped_step());
    fittings.concurrency = Concurrency::of(2);
    Rig {
        fleet: Arc::new(Fleet::assembled(fittings)),
        home,
    }
}

impl Rig {
    fn manifest(&self) -> String {
        self.fleet.repositories().first().expect("served").manifest().id().as_str().to_string()
    }

    fn slot(&self, n: u32) -> PathBuf {
        let root = self.fleet.repositories().first().expect("served").root().to_string();
        PathBuf::from(adapter_traits::slot_path(&root, n))
    }

    /// A terminal Session standing in a new slot, listening.
    async fn a_session_in_slot(&self, id: &str, n: u32) {
        let at = self.slot(n);
        git(
            self.home.path(),
            &["worktree", "add", "--quiet", "-b", &format!("branch-{n}"), &at.to_string_lossy()],
        );
        self.report(id, SessionFact::Started {
            cwd: at.to_string_lossy().to_string(),
            title: Some(format!("session {n}")),
            origin: SessionOrigin::Terminal,
            mod_version: None,
        })
        .await;
        assert!(self.the_mod_asks(id).await.is_empty(), "now it is listening");
    }

    async fn report(&self, id: &str, fact: SessionFact) {
        self.fleet
            .report_session(SessionReport {
                harness: adapters::HOSTED_HARNESS.into(),
                session_id: SessionId::carried(id),
                fact,
            })
            .await
            .expect("reported");
    }

    async fn writes(&self, id: &str, n: u32, file: &str, line: usize, to: &str) {
        let path = self.slot(n).join(file);
        let mut text: Vec<String> =
            std::fs::read_to_string(&path).unwrap().lines().map(str::to_string).collect();
        text[line - 1] = to.to_string();
        std::fs::write(&path, text.join("\n") + "\n").unwrap();
        self.report(id, SessionFact::Touched { paths: vec![file.to_string()] }).await;
    }

    async fn the_mod_asks(&self, id: &str) -> Vec<String> {
        self.fleet
            .take_held_messages(TakeHeld { session_id: id.into(), wait_ms: None })
            .await
            .unwrap()
            .messages
    }

    async fn checks(&self, one: &str, other: &str) {
        let pair = Opposed::pair(&Who::Session(one.into()), &Who::Session(other.into()));
        self.fleet.pair_checked(&pair).await;
    }

    async fn clashes(&self) -> ipc::WriteClashes {
        self.fleet.list_write_clashes(None, None).await.unwrap()
    }
}

#[tokio::test]
async fn a_file_both_wrote_that_merges_clean_says_nothing() {
    let rig = a_rig();
    rig.a_session_in_slot("sess-a", 1).await;
    rig.a_session_in_slot("sess-b", 2).await;
    rig.writes("sess-a", 1, "f.txt", 1, "a's first").await;
    rig.writes("sess-b", 2, "f.txt", 20, "b's last").await;

    rig.checks("sess-a", "sess-b").await;

    assert!(rig.clashes().await.clashes.is_empty());
    assert!(rig.the_mod_asks("sess-a").await.is_empty());
    assert!(rig.the_mod_asks("sess-b").await.is_empty());
}

#[tokio::test]
async fn the_same_lines_are_named_to_both_sides_once_and_again_only_when_the_files_change() {
    let rig = a_rig();
    rig.a_session_in_slot("sess-a", 1).await;
    rig.a_session_in_slot("sess-b", 2).await;
    rig.writes("sess-a", 1, "f.txt", 5, "a").await;
    rig.writes("sess-b", 2, "f.txt", 5, "b").await;

    rig.checks("sess-a", "sess-b").await;

    let told_a = rig.the_mod_asks("sess-a").await;
    let told_b = rig.the_mod_asks("sess-b").await;
    let [said] = told_a.as_slice() else { panic!("one message: {told_a:?}") };
    assert!(said.starts_with("Fleet: Your changes to `f.txt` conflict with Session s-sess-b"), "{said}");
    assert!(said.contains("SendMessage to s-sess-b"), "how to reach it: {said}");
    let [said] = told_b.as_slice() else { panic!("one message: {told_b:?}") };
    assert!(said.contains("SendMessage to s-sess-a"), "{said}");
    let clashes = rig.clashes().await.clashes;
    let [clash] = clashes.as_slice() else { panic!("one clash: {clashes:?}") };
    assert_eq!(clash.paths, vec!["f.txt".to_string()]);
    assert_eq!(clash.one.label, "session 1");

    rig.checks("sess-a", "sess-b").await;
    assert!(rig.the_mod_asks("sess-a").await.is_empty(), "the same files are not said twice");
    assert!(rig.the_mod_asks("sess-b").await.is_empty());

    rig.writes("sess-a", 1, "g.txt", 1, "a's g").await;
    rig.writes("sess-b", 2, "g.txt", 1, "b's g").await;
    rig.checks("sess-a", "sess-b").await;
    let again = rig.the_mod_asks("sess-a").await;
    let [said] = again.as_slice() else { panic!("told again: {again:?}") };
    assert!(said.contains("`f.txt`") && said.contains("`g.txt`"), "{said}");
}

#[tokio::test]
async fn a_clash_that_no_longer_reproduces_is_cleared_without_a_word() {
    let rig = a_rig();
    rig.a_session_in_slot("sess-a", 1).await;
    rig.a_session_in_slot("sess-b", 2).await;
    rig.writes("sess-a", 1, "f.txt", 5, "a").await;
    rig.writes("sess-b", 2, "f.txt", 5, "b").await;
    rig.checks("sess-a", "sess-b").await;
    rig.the_mod_asks("sess-a").await;
    assert_eq!(rig.clashes().await.clashes.len(), 1);

    rig.writes("sess-b", 2, "f.txt", 5, "a").await;
    rig.checks("sess-a", "sess-b").await;

    assert!(rig.clashes().await.clashes.is_empty());
    assert!(rig.the_mod_asks("sess-a").await.is_empty());
}

#[tokio::test]
async fn a_holder_that_ends_takes_its_clashes_with_it() {
    let rig = a_rig();
    rig.a_session_in_slot("sess-a", 1).await;
    rig.a_session_in_slot("sess-b", 2).await;
    rig.writes("sess-a", 1, "f.txt", 5, "a").await;
    rig.writes("sess-b", 2, "f.txt", 5, "b").await;
    rig.checks("sess-a", "sess-b").await;
    assert_eq!(rig.clashes().await.clashes.len(), 1);

    rig.report("sess-a", SessionFact::Ended { reason: "prompt_input_exit".into() }).await;

    assert!(rig.clashes().await.clashes.is_empty());
    assert!(!rig.fleet.writes().holders().iter().any(|(who, ..)| *who == Who::Session("sess-a".into())));
}

#[tokio::test]
async fn a_holder_whose_worktree_is_gone_is_dropped_on_the_next_check() {
    let rig = a_rig();
    rig.a_session_in_slot("sess-a", 1).await;
    rig.a_session_in_slot("sess-b", 2).await;
    rig.writes("sess-a", 1, "f.txt", 5, "a").await;
    rig.writes("sess-b", 2, "f.txt", 5, "b").await;
    git(rig.home.path(), &["worktree", "remove", "--force", &rig.slot(1).to_string_lossy()]);

    rig.checks("sess-a", "sess-b").await;

    assert!(rig.clashes().await.clashes.is_empty());
    assert!(!rig.fleet.writes().holders().iter().any(|(who, ..)| *who == Who::Session("sess-a".into())));
    assert!(rig.the_mod_asks("sess-b").await.is_empty(), "nobody is left to talk to");
}

#[tokio::test]
async fn main_moving_onto_a_holders_lines_is_told_to_that_holder_only() {
    let rig = a_rig();
    rig.a_session_in_slot("sess-a", 1).await;
    rig.a_session_in_slot("sess-b", 2).await;
    rig.writes("sess-a", 1, "f.txt", 5, "a").await;
    rig.writes("sess-b", 2, "g.txt", 1, "b's g").await;
    let manifest = rig.manifest();
    let before = git_out(rig.home.path(), &["rev-parse", "HEAD"]);
    assert!(!rig.fleet.writes().main_seen(&manifest, &before), "the first sight is a baseline");

    std::fs::write(
        rig.home.path().join("f.txt"),
        twenty_lines().replace("line 5\n", "main's five\n"),
    )
    .unwrap();
    git(rig.home.path(), &["commit", "--quiet", "-am", "Merge pull request #7 from elsewhere"]);
    let after = git_out(rig.home.path(), &["rev-parse", "HEAD"]);
    assert!(rig.fleet.writes().main_seen(&manifest, &after));

    rig.fleet.clashes_checked().await;

    let told = rig.the_mod_asks("sess-a").await;
    let [said] = told.as_slice() else { panic!("one message: {told:?}") };
    assert!(said.contains("now conflicts with your changes to `f.txt`"), "{said}");
    assert!(said.contains("Merge pull request #7"), "where it landed: {said}");
    assert!(rig.the_mod_asks("sess-b").await.is_empty(), "its files are not main's");
    let clashes = rig.clashes().await.clashes;
    let [clash] = clashes.as_slice() else { panic!("one clash: {clashes:?}") };
    assert_eq!(clash.other.kind, ipc::ClashSide::Main);

    // The holder merged main in: it no longer conflicts, and the clash goes.
    git(&rig.slot(1), &["checkout", "--quiet", "--", "f.txt"]);
    git(&rig.slot(1), &["merge", "--quiet", &after]);
    let again = git_out(rig.home.path(), &["rev-parse", "HEAD"]);
    rig.fleet.writes().main_seen(&manifest, "somewhere-else");
    rig.fleet.writes().main_seen(&manifest, &again);
    rig.fleet.clashes_checked().await;
    assert!(rig.clashes().await.clashes.is_empty());
}

fn git_out(repo: &std::path::Path, args: &[&str]) -> String {
    let run = std::process::Command::new("git").arg("-C").arg(repo).args(args).output().unwrap();
    String::from_utf8_lossy(&run.stdout).trim().to_string()
}

#[tokio::test]
async fn a_jobs_drone_is_queued_the_clash_and_a_note_reaches_a_session_that_has_one() {
    let rig = a_rig();
    let job = approved(&rig.fleet, &rig.home, "fix the reader").await;
    rig.a_session_in_slot("sess-b", 2).await;
    let opposed = Opposed::pair(&Who::Job(job.clone()), &Who::Session("sess-b".into()));
    let now = rig.fleet.now();
    rig.fleet
        .writes()
        .found(&opposed, Some(["f.txt".to_string()].into()), None, &now);

    rig.fleet.clash_announced(&opposed).await;

    let brief = rig.fleet.peer_news_for_the_brief(&job).await.expect("the Drone is owed it");
    assert!(brief.text().contains("Merging the changes below would conflict"), "{}", brief.text());
    assert!(brief.text().contains("`f.txt` conflict with Session s-sess-b"), "{}", brief.text());
    assert!(brief.text().contains("call `leave_note` with `s-sess-b`"), "{}", brief.text());
    let told = rig.the_mod_asks("sess-b").await;
    assert!(told[0].contains("conflict with Job") && told[0].contains("redirect_drone"), "{told:?}");

    rig.fleet
        .leave_note(&job, &ipc::mcp::LeaveNote { to: "s-sess-b".into(), note: "I take the reader, you take the writer".into() })
        .await
        .expect("a note to a Session in a clash with this Job");
    rig.fleet.notes_sent().await;
    let heard = rig.the_mod_asks("sess-b").await;
    assert!(heard[0].contains("> I take the reader, you take the writer"), "{heard:?}");
    assert!(heard[0].contains("not Armada's") && heard[0].contains("redirect_drone"), "{heard:?}");

    let again = rig
        .fleet
        .leave_note(&job, &ipc::mcp::LeaveNote { to: "s-sess-b".into(), note: "and one more".into() })
        .await
        .expect_err("inside the spacing");
    assert!(again.because.contains("under two minutes"), "{}", again.because);
    let _ = SPACING;
}

#[tokio::test]
async fn a_note_to_a_session_with_no_clash_is_refused() {
    let rig = a_rig();
    let job = approved(&rig.fleet, &rig.home, "fix the reader").await;
    rig.a_session_in_slot("sess-b", 2).await;

    let refused = rig
        .fleet
        .leave_note(&job, &ipc::mcp::LeaveNote { to: "s-sess-b".into(), note: "hello".into() })
        .await
        .expect_err("no conflict");
    assert!(refused.because.contains("no conflict"), "{}", refused.because);
    let nobody = rig
        .fleet
        .leave_note(&job, &ipc::mcp::LeaveNote { to: "s-nobody0".into(), note: "hello".into() })
        .await
        .expect_err("no such Session");
    assert!(nobody.because.contains("no live Session"), "{}", nobody.because);
}

#[test]
fn a_second_holder_on_a_file_is_due_a_check_after_the_wait_and_not_before() {
    let writes = Writes::default();
    let (a, b) = (Who::Session("a".into()), Who::Session("b".into()));
    let at = Timestamp::from_rfc3339("2026-10-12T09:00:00.000Z");
    writes.wrote(&a, "m", ["f.txt".to_string()], &at);
    assert!(writes.due(&Timestamp::from_rfc3339("2026-10-12T09:10:00.000Z")).is_empty(), "alone on it");

    writes.wrote(&b, "m", ["f.txt".to_string()], &at);
    assert!(writes.due(&Timestamp::from_rfc3339("2026-10-12T09:00:10.000Z")).is_empty(), "inside the wait");
    let due = writes.due(&Timestamp::from_rfc3339("2026-10-12T09:00:31.000Z"));
    assert_eq!(due, vec![Opposed::pair(&a, &b)]);

    writes.taken(&due[0]);
    assert!(writes.due(&Timestamp::from_rfc3339("2026-10-12T09:05:00.000Z")).is_empty(), "taken");
    writes.wrote(&a, "m", ["other.txt".to_string()], &at);
    assert_eq!(writes.due(&Timestamp::from_rfc3339("2026-10-12T09:05:00.000Z")).len(), 1, "a later write by either asks again");
}

#[test]
fn different_repositories_never_pair_and_a_holder_leaving_forgets_its_pairs() {
    let writes = Writes::default();
    let (a, b, c) = (Who::Session("a".into()), Who::Session("b".into()), Who::Session("c".into()));
    let at = Timestamp::from_rfc3339("2026-10-12T09:00:00.000Z");
    let late = Timestamp::from_rfc3339("2026-10-12T10:00:00.000Z");
    writes.wrote(&a, "one", ["f.txt".to_string()], &at);
    writes.wrote(&b, "two", ["f.txt".to_string()], &at);
    assert!(writes.due(&late).is_empty());

    writes.wrote(&c, "one", ["f.txt".to_string()], &at);
    let pair = Opposed::pair(&a, &c);
    writes.found(&pair, Some(["f.txt".to_string()].into()), None, &at);
    writes.left(&a);
    assert!(writes.due(&late).is_empty());
    assert!(writes.reading(&pair).is_none());
    assert!(matches!(pair.1, Side::Holder(_)));
}

#[test]
fn a_terminal_mods_paths_are_named_from_the_worktree_and_none_climbs_out() {
    let slot = "/r/.armada/slots/slot-1";
    let told = |cwd: &str, paths: &[&str]| {
        let paths: Vec<String> = paths.iter().map(|path| path.to_string()).collect();
        crate::clashing::inside_worktree(slot, cwd, &paths)
    };
    assert_eq!(told(slot, &["a.rs", "./b.rs", "../x", "/etc/hosts"]), vec!["a.rs", "b.rs"]);
    assert_eq!(told(&format!("{slot}/crates/fleet"), &["a.rs"]), vec!["crates/fleet/a.rs"]);
    assert!(told("/r/elsewhere", &["a.rs"]).is_empty(), "a directory outside the slot");
}

#[test]
fn a_drones_edit_call_is_a_write_in_its_worktree_and_a_path_elsewhere_is_not() {
    use adapter_traits::{CallDetail, DroneEvent};
    use crate::transcript::Tap as _;

    let writes = Arc::new(Writes::default());
    let who = Who::Job(core_model::JobId::carried(core_model::Ulid::carried("01TEST00000000000000000001")));
    let tap = crate::clashing::WritesSeen {
        harness: Arc::new(FakeHarness::that_listens()),
        writes: Arc::clone(&writes),
        clock: Arc::new(super::planted::Ticking::from_nine()),
        who: who.clone(),
        manifest: "m".into(),
        worktree: "/r/.armada/slots/slot-3".into(),
        home: "/home/x".into(),
    };
    let edit = |path: &str| DroneEvent::Called {
        tool: testkit::EDITS.to_string(),
        call: "c".to_string(),
        detail: CallDetail::of(path),
    };

    tap.saw(&[edit("/r/.armada/slots/slot-3/src/a.rs"), edit("/etc/hosts"), edit("src/b.rs")]);

    let held = writes.holders();
    let [(holder, manifest, paths)] = held.as_slice() else { panic!("one holder: {held:?}") };
    assert_eq!((holder, manifest.as_str()), (&who, "m"));
    assert_eq!(paths.iter().map(String::as_str).collect::<Vec<_>>(), vec!["src/a.rs", "src/b.rs"]);
}
