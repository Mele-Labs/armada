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

fn git(root: &Path, args: &[&str]) -> String {
    let done = Command::new("git")
        .args(["-c", "user.name=armada", "-c", "user.email=armada@example.invalid"])
        .args(args)
        .current_dir(root)
        .output()
        .expect("git runs");
    assert!(done.status.success(), "{args:?}: {}", String::from_utf8_lossy(&done.stderr));
    String::from_utf8_lossy(&done.stdout).trim().to_string()
}

fn commit_on(root: &Path, branch: &str, from: &str, message: &str) -> String {
    git(root, &["checkout", "-q", "-B", branch, from]);
    git(root, &["commit", "-q", "--allow-empty", "-m", message]);
    git(root, &["rev-parse", "HEAD"])
}

struct Rig {
    fleet: Arc<Hosted>,
    home: TempDir,
    /// The build: the tip of `armada/work`, which `armada/long` is two ahead of and
    /// `armada/other` shares nothing with.
    build: String,
}

fn a_rig() -> Rig {
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
    fn support(&self) -> std::path::PathBuf {
        crate::runtime::support_dir(&self.home.path().to_string_lossy())
    }

    /// `scripts/restart` has installed the build: the commit, and the tree it came from.
    fn on_file(&self, preview: bool) {
        let support = self.support();
        fs::create_dir_all(&support).unwrap();
        fs::write(support.join("restart-commit"), format!("{}\n", self.build)).unwrap();
        let _ = fs::remove_file(support.join("restart-source"));
        if preview {
            fs::write(support.join("restart-source"), "/Users/user/armada/.armada/preview\n").unwrap();
        }
    }

    fn manifest(&self) -> String {
        self.fleet.repositories().first().expect("served").manifest().id().as_str().to_string()
    }

    /// Fleet restarted: nothing it kept in memory survives, the store does.
    async fn restarts(&self) {
        *self.fleet.sweeping().lock().await = Default::default();
    }

    async fn holds_branch(&self, session: &str, branch: &str) {
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

    async fn a_hosted_session(&self) -> String {
        self.fleet
            .start_session(StartSession {
                manifest_id: ManifestId::carried(self.manifest()),
                title: None,
                model: None,
                effort: None,
                mode: None,
                pilot: None,
                fork: None,
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

    async fn waits_to_run(&self, session: &str, command: &str) {
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
    async fn thread_of(&self, id: &str) -> Vec<String> {
        let rows = Arc::clone(&self.fleet).get_session(SessionId::carried(id)).await.unwrap().rows;
        rows.into_iter()
            .filter_map(|row| match row {
                SessionRow::Message { text, .. } => Some(text),
                _ => None,
            })
            .collect()
    }

    async fn told(&self, session: &str) -> bool {
        self.fleet.store().lock().await.build_notice_kept(&self.build, session).unwrap()
    }
}

#[tokio::test]
async fn a_new_commit_tells_a_session_holding_branches_once_and_a_restart_does_not_repeat_it() {
    let rig = a_rig();
    let session = rig.a_hosted_session().await;
    for branch in ["armada/work", "armada/long", "armada/other", "main"] {
        rig.holds_branch(&session, branch).await;
    }
    rig.on_file(true);

    rig.fleet.builds_told().await;

    let short = &rig.build[..7];
    let said = rig.thread_of(&session).await;
    assert_eq!(
        said,
        [format!(
            "Fleet and Bridge now run the preview at {short}.\n\
             armada/long is in it up to {short}; 1 commit since is not.\n\
             armada/other is not in it.\n\
             armada/work is in it."
        )]
    );
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
async fn a_restart_wait_is_settled_by_the_build_it_asked_for_and_a_mismatch_is_not() {
    let rig = a_rig();
    let preview = rig.a_hosted_session().await;
    let main = rig.a_hosted_session().await;
    rig.waits_to_run(&preview, "scripts/preview --adopt --restart").await;
    rig.waits_to_run(&main, "scripts/restart --main").await;
    rig.on_file(true);

    rig.fleet.builds_told().await;

    let short = &rig.build[..7];
    let settled = rig.thread_of(&preview).await;
    assert!(settled.iter().any(|one| one.starts_with("Fleet and Bridge now run the preview")), "{settled:?}");
    assert!(settled.iter().any(|one| one.contains(&format!("Restarted: preview at {short}"))), "{settled:?}");
    assert!(rig.waiting_on(&preview).await.is_empty(), "the wait is settled");

    let told = rig.thread_of(&main).await;
    assert_eq!(told.len(), 1, "told, and no more: {told:?}");
    assert!(told[0].starts_with("Fleet and Bridge now run the preview"));
    assert_eq!(rig.waiting_on(&main).await, ["restart"], "it asked for main, and this is the preview");
}

#[tokio::test]
async fn nobody_is_told_while_a_restart_is_under_way_or_before_there_is_a_build() {
    let rig = a_rig();
    let session = rig.a_hosted_session().await;
    rig.holds_branch(&session, "armada/work").await;
    rig.fleet.builds_told().await;
    assert!(rig.thread_of(&session).await.is_empty(), "no commit on file");

    rig.on_file(false);
    fs::write(rig.support().join("restart-build.status"), "running\nmain\n").unwrap();
    rig.fleet.builds_told().await;
    assert!(rig.thread_of(&session).await.is_empty(), "the wrapper is still working");

    fs::remove_file(rig.support().join("restart-build.status")).unwrap();
    rig.fleet.builds_told().await;
    let said = rig.thread_of(&session).await;
    assert!(said[0].starts_with(&format!("Fleet and Bridge now run main at {}.", &rig.build[..7])), "{said:?}");
}

#[tokio::test]
async fn a_terminal_session_that_is_not_listening_is_asked_again_with_nothing_kept() {
    let rig = a_rig();
    rig.a_terminal_session("term-1").await;
    rig.holds_branch("term-1", "armada/work").await;
    rig.on_file(true);

    rig.fleet.builds_told().await;
    assert!(!rig.told("term-1").await, "nothing kept for a notice not delivered");

    rig.the_mod_asks("term-1").await;
    rig.fleet.builds_told().await;
    let held = rig.the_mod_asks("term-1").await;
    let [said] = held.as_slice() else { panic!("one message held: {held:?}") };
    assert!(said.starts_with("Fleet: Fleet and Bridge now run the preview at"), "{said}");
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
