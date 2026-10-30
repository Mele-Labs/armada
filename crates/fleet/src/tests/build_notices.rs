//! What Fleet tells sessions once a new build is on file: who is told, what is said, that it is
//! said once, and that a restart wait is settled only by the build it asked for. The git readings
//! behind the branch lines are `adapters`' to assert, in `tests::build_standing`.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Sessions};
use ipc::{
    ManifestId, SessionFact, SessionId, SessionOrigin, SessionReport, SessionRow, SetWaitingFor,
    StartSession, TakeHeld, WaitingAct, WaitingActKind, WaitingInput,
};
use store::{AttachmentState, Holder, KeptAttachment};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use super::main_fix::a_fleet_holding_bug;
use super::session_host::{Shared, StandIn};
use crate::build_notices::restart_asked;
use crate::daemon::Fleet;
use crate::tests::tmp::TempDir;

type Hosted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

pub(super) fn git(root: &Path, args: &[&str]) -> String {
    let done = Command::new("git")
        .args(["-c", "user.name=armada", "-c", "user.email=armada@example.invalid"])
        .args(args)
        .current_dir(root)
        .output()
        .expect("git runs");
    assert!(done.status.success(), "{args:?}: {}", String::from_utf8_lossy(&done.stderr));
    String::from_utf8_lossy(&done.stdout).trim().to_string()
}

pub(super) fn commit_on(root: &Path, branch: &str, from: &str, message: &str) -> String {
    git(root, &["checkout", "-q", "-B", branch, from]);
    git(root, &["commit", "-q", "--allow-empty", "-m", message]);
    git(root, &["rev-parse", "HEAD"])
}

pub(super) struct Rig {
    pub(super) fleet: Arc<Hosted>,
    pub(super) home: TempDir,
    /// The build: the tip of `armada/work`, which `armada/long` is two ahead of and
    /// `armada/other` shares nothing with.
    pub(super) build: String,
}

pub(super) fn a_rig() -> Rig {
    let home = TempDir::new();
    let stand_in = Arc::new(StandIn::default());
    let fleet = a_fleet_holding_bug(&home, false)
        .hosting_sessions_on(Arc::new(Shared(stand_in)), Duration::from_secs(600));
    let root = home.path();
    git(root, &["init", "-q", "-b", "main"]);
    git(root, &["commit", "-q", "--allow-empty", "-m", "base"]);
    let base = git(root, &["rev-parse", "HEAD"]);
    git(root, &["update-ref", "refs/remotes/origin/main", &base]);
    let build = commit_on(root, "armada/work", &base, "work");
    commit_on(root, "armada/long", &build, "more work");
    commit_on(root, "armada/other", &base, "somebody else's");
    Rig { fleet: Arc::new(fleet), home, build }
}

impl Rig {
    pub(super) fn support(&self) -> std::path::PathBuf {
        crate::runtime::support_dir(&self.home.path().to_string_lossy())
    }

    /// `scripts/restart` has installed the build: the commit, and the tree it came from.
    pub(super) fn on_file(&self, preview: bool) {
        self.on_file_at(&self.build, preview);
    }

    pub(super) fn on_file_at(&self, commit: &str, preview: bool) {
        let support = self.support();
        fs::create_dir_all(&support).unwrap();
        fs::write(support.join("restart-commit"), format!("{commit}\n")).unwrap();
        let _ = fs::remove_file(support.join("restart-source"));
        if preview {
            fs::write(support.join("restart-source"), "/Users/user/armada/.armada/preview\n").unwrap();
        }
    }

    /// `scripts/preview` has recorded the branches it left out of the build at `commit`, and why.
    pub(super) fn left_out(&self, commit: &str, reasons: &[(&str, &str)]) {
        let mut text = format!("{commit}\n");
        for (branch, why) in reasons {
            let tip = git(self.home.path(), &["rev-parse", branch]);
            text.push_str(&format!("{branch}\t{tip}\t{why}\n"));
        }
        fs::write(self.support().join("preview-left-out"), text).unwrap();
    }

    fn manifest(&self) -> String {
        self.fleet.repositories().first().expect("served").manifest().id().as_str().to_string()
    }

    /// Fleet restarted: nothing it kept in memory survives, the store does.
    pub(super) async fn restarts(&self) {
        *self.fleet.sweeping().lock().await = Default::default();
    }

    pub(super) async fn holds_branch(&self, session: &str, branch: &str) {
        let at = self.fleet.now().as_str().to_string();
        self.fleet
            .store()
            .lock()
            .await
            .attach(
                &KeptAttachment {
                    holder: Holder::session(session),
                    kind: "branch".into(),
                    manifest_id: self.manifest(),
                    target: branch.into(),
                    state: AttachmentState::Standing,
                    detail: Default::default(),
                    since: at.clone(),
                    changed_at: at,
                },
                false,
            )
            .unwrap();
    }

    pub(super) async fn a_hosted_session(&self) -> String {
        self.fleet
            .start_session(StartSession {
                manifest_id: ManifestId::carried(self.manifest()),
                title: None,
                model: None,
                effort: None,
                mode: None,
                root: None,
                fork: None,
                request: None,
                after: None,
                held: false,
                project_rows: Vec::new(),
            })
            .await
            .expect("started")
            .id
            .as_str()
            .to_string()
    }

    async fn a_terminal_session(&self, id: &str) {
        let cwd = self.fleet.repositories().first().unwrap().root().to_string();
        self.fleet
            .report_session(SessionReport {
                harness: adapters::HOSTED_HARNESS.into(),
                session_id: SessionId::carried(id),
                fact: SessionFact::Started {
                    cwd,
                    title: None,
                    origin: SessionOrigin::Terminal,
                    mod_version: None,
                },
            })
            .await
            .unwrap();
    }

    pub(super) async fn waits_to_run(&self, session: &str, command: &str) {
        self.fleet
            .waiting_for(
                None,
                SetWaitingFor {
                    session_id: Some(SessionId::carried(session)),
                    items: vec![WaitingInput {
                        id: "restart".into(),
                        text: "Restart onto the new build".into(),
                        act: Some(WaitingAct { kind: WaitingActKind::Run, target: command.into() }),
                        options: Vec::new(),
                        steps: Vec::new(),
                    }],
                },
            )
            .await
            .unwrap();
    }

    async fn waiting_on(&self, session: &str) -> Vec<String> {
        let record = Arc::clone(&self.fleet).get_session(SessionId::carried(session)).await.unwrap();
        record.session.waiting_for.into_iter().map(|one| one.id).collect()
    }

    async fn the_mod_asks(&self, id: &str) -> Vec<String> {
        self.fleet
            .take_held_messages(TakeHeld { session_id: id.into(), wait_ms: None })
            .await
            .unwrap()
            .messages
    }

    /// Every message in a hosted session's thread.
    pub(super) async fn thread_of(&self, id: &str) -> Vec<String> {
        let rows = Arc::clone(&self.fleet).get_session(SessionId::carried(id)).await.unwrap().rows;
        rows.into_iter()
            .filter_map(|row| match row {
                SessionRow::Message { text, .. } => Some(text),
                _ => None,
            })
            .collect()
    }

    pub(super) async fn told(&self, session: &str) -> bool {
        self.fleet.store().lock().await.build_notice_kept(&self.build, session).unwrap()
    }
}

#[tokio::test]
async fn a_new_commit_tells_a_session_that_asked_for_it_once_with_its_branches() {
    let rig = a_rig();
    let session = rig.a_hosted_session().await;
    rig.waits_to_run(&session, "scripts/preview --restart").await;
    for branch in ["armada/work", "armada/long", "armada/other", "main"] {
        rig.holds_branch(&session, branch).await;
    }
    rig.on_file(true);

    rig.fleet.builds_told().await;

    let short = &rig.build[..7];
    let said = rig.thread_of(&session).await;
    let [said] = said.as_slice() else { panic!("one answer: {said:?}") };
    let notice = format!(
        "Fleet and Bridge now run the preview at {short}.\n\
         armada/long is in it up to {short}; 1 commit since is not.\n\
         armada/other is not in it.\n\
         armada/work is in it."
    );
    assert!(said.contains(&notice), "{said}");
    assert!(rig.told(&session).await);

    rig.fleet.builds_told().await;
    rig.restarts().await;
    rig.fleet.builds_told().await;
    assert_eq!(rig.thread_of(&session).await.len(), 1, "told once for this build");
}

#[tokio::test]
async fn a_session_with_neither_a_branch_nor_a_restart_wait_is_not_told() {
    let rig = a_rig();
    let bare = rig.a_hosted_session().await;
    let testing = rig.a_hosted_session().await;
    rig.waits_to_run(&testing, "scripts/preview --restart --dry-run").await;
    rig.on_file(true);

    rig.fleet.builds_told().await;

    for session in [&bare, &testing] {
        assert!(rig.thread_of(session).await.is_empty(), "{session}");
        assert!(!rig.told(session).await);
    }
    assert_eq!(rig.waiting_on(&testing).await, ["restart"], "a dry run is not a restart");
}

#[tokio::test]
async fn a_session_is_told_when_its_branch_enters_the_build_and_not_when_it_stays_put() {
    let rig = a_rig();
    let holding = rig.a_hosted_session().await;
    let elsewhere = rig.a_hosted_session().await;
    rig.holds_branch(&holding, "armada/work").await;
    rig.holds_branch(&holding, "armada/other").await;
    rig.holds_branch(&elsewhere, "feature/retry-budget").await;
    rig.on_file(true);

    rig.fleet.builds_told().await;

    let short = &rig.build[..7];
    let said = rig.thread_of(&holding).await;
    let [said] = said.as_slice() else { panic!("one notice: {said:?}") };
    assert!(
        said.contains(&format!("Fleet and Bridge now run the preview at {short}.\narmada/work is in it.")),
        "{said}"
    );
    assert!(!said.contains("armada/other"), "never in, so nothing dropped: {said}");
    assert!(rig.thread_of(&elsewhere).await.is_empty(), "git cannot place its branch");

    let root = rig.home.path();
    let rebuilt = commit_on(root, "preview", &rig.build, "another branch merged");
    rig.on_file_at(&rebuilt, true);
    rig.fleet.builds_told().await;
    assert_eq!(rig.thread_of(&holding).await.len(), 1, "its branch is where it was");

    let newer = commit_on(root, "armada/work", &rig.build, "more on work");
    let rebuilt = commit_on(root, "preview", &newer, "rebuilt with it");
    rig.on_file_at(&rebuilt, true);
    rig.fleet.builds_told().await;
    let said = rig.thread_of(&holding).await;
    assert_eq!(said.len(), 2, "{said:?}");
    assert!(said[1].ends_with("\narmada/work is in it."), "{}", said[1]);
}

#[tokio::test]
async fn a_session_is_told_when_its_branch_drops_out_of_the_build() {
    let rig = a_rig();
    let holding = rig.a_hosted_session().await;
    rig.holds_branch(&holding, "armada/work").await;
    rig.on_file(true);
    rig.fleet.builds_told().await;

    let root = rig.home.path();
    let base = git(root, &["rev-parse", "main"]);
    let without = commit_on(root, "preview", &base, "rebuilt without it");
    rig.on_file_at(&without, true);
    rig.fleet.builds_told().await;

    let said = rig.thread_of(&holding).await;
    assert_eq!(said.len(), 2, "{said:?}");
    assert!(said[1].ends_with("\narmada/work is not in it."), "{}", said[1]);
}

#[tokio::test]
async fn a_session_whose_branch_is_left_out_is_told_why_even_when_never_told_it_was_in() {
    let rig = a_rig();
    let holding = rig.a_hosted_session().await;
    rig.holds_branch(&holding, "armada/work").await;
    rig.holds_branch(&holding, "armada/other").await;
    rig.on_file(true);
    rig.left_out(&rig.build, &[("armada/other", "conflict: apps/a.ts, apps/b.ts")]);

    rig.fleet.builds_told().await;

    let short = &rig.build[..7];
    let said = rig.thread_of(&holding).await;
    let [said] = said.as_slice() else { panic!("one notice: {said:?}") };
    assert!(
        said.contains(&format!(
            "Fleet and Bridge now run the preview at {short}.\n\
             armada/other is not in it: conflict: apps/a.ts, apps/b.ts.\n\
             armada/work is in it."
        )),
        "{said}"
    );

    let root = rig.home.path();
    let rebuilt = commit_on(root, "preview", &rig.build, "another branch merged");
    rig.on_file_at(&rebuilt, true);
    rig.left_out(&rebuilt, &[("armada/other", "conflict: apps/a.ts, apps/b.ts")]);
    rig.fleet.builds_told().await;
    assert_eq!(rig.thread_of(&holding).await.len(), 1, "it was told this branch was out");
}

#[tokio::test]
async fn a_reason_recorded_for_another_build_is_not_told() {
    let rig = a_rig();
    let holding = rig.a_hosted_session().await;
    rig.holds_branch(&holding, "armada/other").await;
    rig.on_file(true);
    let older = git(rig.home.path(), &["rev-parse", "main"]);
    rig.left_out(&older, &[("armada/other", "conflict: apps/a.ts")]);

    rig.fleet.builds_told().await;

    assert!(rig.thread_of(&holding).await.is_empty());
}

#[tokio::test]
async fn a_restart_wait_is_settled_by_the_build_it_asked_for_and_a_mismatch_is_not() {
    let rig = a_rig();
    let preview = rig.a_hosted_session().await;
    let main = rig.a_hosted_session().await;
    rig.waits_to_run(&preview, "scripts/preview --adopt --restart").await;
    rig.waits_to_run(&main, "scripts/restart --main").await;
    rig.on_file(true);

    rig.fleet.builds_told().await;

    let settled = rig.thread_of(&preview).await;
    assert_eq!(settled.len(), 1, "the wait is answered with the notice, and no more: {settled:?}");
    assert!(settled[0].contains("Fleet and Bridge now run the preview"), "{settled:?}");
    assert!(rig.waiting_on(&preview).await.is_empty(), "the wait is settled");

    assert!(rig.thread_of(&main).await.is_empty(), "it asked for the other build");
    assert_eq!(rig.waiting_on(&main).await, ["restart"], "it asked for main, and this is the preview");
}

#[tokio::test]
async fn nobody_is_told_while_a_restart_is_under_way_or_before_there_is_a_build() {
    let rig = a_rig();
    let session = rig.a_hosted_session().await;
    rig.waits_to_run(&session, "scripts/restart --main").await;
    rig.fleet.builds_told().await;
    assert!(rig.thread_of(&session).await.is_empty(), "no commit on file");

    rig.on_file(false);
    fs::write(rig.support().join("restart-build.status"), "running\nmain\n").unwrap();
    rig.fleet.builds_told().await;
    assert!(rig.thread_of(&session).await.is_empty(), "the wrapper is still working");

    fs::remove_file(rig.support().join("restart-build.status")).unwrap();
    rig.fleet.builds_told().await;
    let said = rig.thread_of(&session).await;
    assert!(said[0].contains(&format!("Fleet and Bridge now run main at {}.", &rig.build[..7])), "{said:?}");
}

#[tokio::test]
async fn a_terminal_session_that_is_not_listening_is_asked_again_with_nothing_kept() {
    let rig = a_rig();
    rig.a_terminal_session("term-1").await;
    rig.waits_to_run("term-1", "scripts/preview --restart").await;
    rig.on_file(true);

    rig.fleet.builds_told().await;
    assert!(!rig.told("term-1").await, "nothing kept for a notice not delivered");

    rig.the_mod_asks("term-1").await;
    rig.fleet.builds_told().await;
    let held = rig.the_mod_asks("term-1").await;
    let [said] = held.as_slice() else { panic!("one message held: {held:?}") };
    assert!(said.contains("Fleet and Bridge now run the preview at"), "{said}");
    assert!(rig.told("term-1").await);
}

#[test]
fn a_restart_command_is_read_as_bridge_reads_it() {
    use ipc::BuildSource::{Main, Preview};
    for (command, asked) in [
        ("scripts/preview --restart", Some((Preview, false))),
        ("scripts/preview --adopt --restart", Some((Preview, true))),
        ("  ./scripts/restart --main --adopt ", Some((Main, true))),
        ("/Users/user/armada/scripts/preview --restart", Some((Preview, false))),
        ("scripts/preview", None),
        ("scripts/preview --restart --dry-run", None),
        ("scripts/preview --main", None),
        ("scripts/restart", None),
        ("scripts/restart --restart", None),
        ("scripts/restart --main && echo done", None),
        ("scripts/previewer --restart", None),
        ("echo scripts/preview --restart", None),
        ("", None),
    ] {
        assert_eq!(restart_asked(command), asked, "{command}");
    }
}
