//! Fleet telling the Jobs whose branches read a wire type that a pull request changes it, against
//! real git in worktrees of one repository and a scripted forge. What the patches say is
//! `wire_types`' to read; under test here is who is told what, and how many times.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{FollowedPull, FromOutside, PatchedFile, PullQueue, RecentlyMerged};
use api::HostedSessions as _;
use ipc::{ManifestId, SessionId, SessionRow, StartSession};
use store::{AttachmentState, Holder, KeptAttachment};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use super::daemon::fittings;
use super::pull_notices::pull;
use super::session_host::{Shared, StandIn};
use super::slot_leases::{lease_record, By};
use super::tmp::TempDir;
use crate::daemon::Fleet;
use crate::slots::Concurrency;

type Hosted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const PATCH: &str = "@@ -10,6 +10,7 @@ pub struct SessionRow {\n     pub id: String,\n+    pub notice: Option<String>,\n     pub at: String,\n }\n";

struct Rig {
    fleet: Arc<Hosted>,
    home: TempDir,
}

fn git(at: &Path, args: &[&str]) {
    let ran = std::process::Command::new("git")
        .arg("-C")
        .arg(at)
        .args(["-c", "user.name=t", "-c", "user.email=t@t.invalid"])
        .args(args)
        .output()
        .expect("git runs");
    assert!(ran.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&ran.stderr));
}

fn a_rig() -> Rig {
    let home = TempDir::new();
    git(home.path(), &["-c", "init.defaultBranch=main", "init", "--quiet"]);
    for (path, text) in [
        ("crates/bridge-native/src/view.rs", "fn draw() {}\n"),
        ("crates/bridge-native/src/waiting.rs", "use ipc::SessionRow;\n"),
        ("crates/ipc/src/hosted_sessions.rs", "pub struct SessionRow {}\n"),
        ("docs/notes.md", "notes\n"),
    ] {
        let at = home.path().join(path);
        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
        std::fs::write(at, text).unwrap();
    }
    git(home.path(), &["add", "."]);
    git(home.path(), &["commit", "--quiet", "-m", "start"]);
    let mut fittings = fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.concurrency = Concurrency::of(2);
    let fleet = Fleet::assembled(fittings)
        .hosting_sessions_on(Arc::new(Shared(Arc::new(StandIn::default()))), Duration::from_secs(600));
    Rig { fleet: Arc::new(fleet), home }
}

impl Rig {
    fn root(&self) -> String {
        self.fleet.repositories().first().expect("served").root().to_string()
    }

    fn slot(&self, n: u32) -> PathBuf {
        PathBuf::from(adapter_traits::slot_path(&self.root(), n))
    }

    /// A hosted Job in slot `n`, whose branch has appended `line` to `file`.
    async fn a_job_in_slot(&self, n: u32, file: &str, line: &str) -> String {
        let at = self.slot(n);
        git(self.home.path(), &["worktree", "add", "--quiet", "-b", &format!("branch-{n}"), &at.to_string_lossy()]);
        let manifest = self.fleet.repositories().first().expect("served").manifest().id().as_str().to_string();
        let id = Arc::clone(&self.fleet)
            .start_session(StartSession {
                project: None,
                manifest_id: ManifestId::carried(manifest),
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
                on_your_words: None,
                workflow: None,
            })
            .await
            .expect("started")
            .id
            .as_str()
            .to_string();
        lease_record(&self.root(), n, &format!("branch-{n}"), By::Session(&id));
        let path = at.join(file);
        let mut text = std::fs::read_to_string(&path).unwrap();
        text.push_str(line);
        text.push('\n');
        std::fs::write(path, text).unwrap();
        id
    }

    /// Whose pull request `number` is: the Job holds it, so it is not told of its own change.
    async fn holds_pull(&self, id: &str, number: u64) {
        let at = self.fleet.now().as_str().to_string();
        let manifest = self.fleet.repositories().first().expect("served").manifest().id().as_str().to_string();
        self.fleet
            .store()
            .lock()
            .await
            .attach(
                &KeptAttachment {
                    holder: Holder::session(id),
                    kind: "pr".into(),
                    manifest_id: manifest,
                    target: number.to_string(),
                    state: AttachmentState::Standing,
                    detail: Default::default(),
                    since: at.clone(),
                    changed_at: at,
                },
                false,
            )
            .unwrap();
    }

    fn the_forge_says(&self, queue: PullQueue) {
        let mut queued: FollowedPull = pull(7, Vec::new());
        queued.queue = queue;
        self.fleet.vcs().main_ci.followed_are(Some(vec![queued]));
        self.fleet.vcs().now_pull_request_patches(Some(vec![
            PatchedFile { path: "crates/ipc/src/hosted_sessions.rs".into(), patch: Some(PATCH.into()), blob: None },
            PatchedFile { path: "crates/ipc/src/tests/hosted.rs".into(), patch: Some(PATCH.replace("SessionRow", "Other")), blob: None },
        ]));
    }

    async fn reads(&self) {
        let served = self.fleet.repositories().served().remove(0);
        self.fleet.pulls_told(&served).await;
    }

    /// What Fleet said to a Job about the wire, in order.
    async fn wire_words_to(&self, id: &str) -> Vec<String> {
        let rows = Arc::clone(&self.fleet).get_session(SessionId::carried(id)).await.unwrap().rows;
        rows.into_iter()
            .filter_map(|row| match row {
                SessionRow::Message { notice: Some(notice), text, .. } if notice == "wire_changed" => Some(text),
                _ => None,
            })
            .collect()
    }
}

#[tokio::test]
async fn a_queued_wire_change_is_told_to_the_job_whose_branch_reads_the_type_and_to_no_other() {
    let rig = a_rig();
    let owner = rig.a_job_in_slot(1, "crates/bridge-native/src/view.rs", "use ipc::SessionRow;").await;
    rig.holds_pull(&owner, 7).await;
    let reader = rig.a_job_in_slot(2, "crates/bridge-native/src/view.rs", "use ipc::SessionRow;").await;
    let other_code = rig.a_job_in_slot(3, "crates/bridge-native/src/view.rs", "fn other() {}").await;
    let elsewhere = rig.a_job_in_slot(4, "docs/notes.md", "more").await;
    rig.the_forge_says(PullQueue::Waiting);

    rig.reads().await;

    let told = rig.wire_words_to(&reader).await;
    let [said] = told.as_slice() else { panic!("one word: {told:?}") };
    assert!(said.starts_with("Wire change in the merge queue: #7. It changes 1 type your branch reads."), "{said}");
    assert!(said.contains("SessionRow: adds notice. Read at crates/bridge-native/src/view.rs:2"), "{said}");
    assert!(!said.contains("Other"), "a test file of the wire is not the wire: {said}");
    assert!(rig.wire_words_to(&owner).await.is_empty(), "the pull request's own Job");
    assert!(rig.wire_words_to(&other_code).await.is_empty(), "a branch that reads nothing changed");
    assert!(rig.wire_words_to(&elsewhere).await.is_empty(), "a branch that changes nothing the wire is read from");
}

#[tokio::test]
async fn a_branch_is_told_of_a_type_only_where_its_own_changed_files_read_it() {
    let rig = a_rig();
    // `waiting.rs` names the type on main; this branch never touches it.
    let bystander = rig.a_job_in_slot(2, "crates/bridge-native/src/view.rs", "fn other() {}").await;
    let reader = rig.a_job_in_slot(3, "crates/bridge-native/src/view.rs", "use ipc::SessionRow;").await;
    rig.the_forge_says(PullQueue::Waiting);

    rig.reads().await;

    assert!(rig.wire_words_to(&bystander).await.is_empty(), "a file the branch did not change is not its read");
    let told = rig.wire_words_to(&reader).await;
    let [said] = told.as_slice() else { panic!("one word: {told:?}") };
    assert!(said.contains("Read at crates/bridge-native/src/view.rs:2"), "{said}");
    assert!(!said.contains("waiting.rs"), "{said}");
}

#[tokio::test]
async fn a_wire_change_is_told_once_at_the_queue_and_a_merge_follows_in_one_line() {
    let rig = a_rig();
    let reader = rig.a_job_in_slot(2, "crates/bridge-native/src/view.rs", "use ipc::SessionRow;").await;
    rig.the_forge_says(PullQueue::Waiting);

    rig.reads().await;
    rig.reads().await;
    assert_eq!(rig.wire_words_to(&reader).await.len(), 1, "told once for the pull request");

    rig.fleet.vcs().main_ci.followed_are(Some(Vec::new()));
    // This repository names no base for `notice_merged` to list from, so the listing is set as it would leave it.
    let root = rig.root();
    rig.fleet.sweeping().lock().await.merged.insert(
        root,
        vec![crate::main_hub::MergedPulled {
            pull: RecentlyMerged {
                number: 7,
                title: FromOutside::verbatim("Carry the notice"),
                branch: FromOutside::verbatim("armada/7"),
                url: FromOutside::verbatim("https://forge.invalid/armada/pull/7"),
                author: None,
                merged_at: FromOutside::verbatim(rig.fleet.now().as_str()),
                commit: None,
            },
        }],
    );
    rig.reads().await;
    rig.reads().await;

    let told = rig.wire_words_to(&reader).await;
    assert_eq!(told.len(), 2, "{told:?}");
    assert_eq!(told[1], "#7 merged: its wire change is on main.");
}

#[tokio::test]
async fn a_merge_is_not_told_to_a_branch_that_already_has_it() {
    let rig = a_rig();
    let before = rig.a_job_in_slot(2, "crates/bridge-native/src/view.rs", "use ipc::SessionRow;").await;
    std::fs::write(rig.home.path().join("docs/notes.md"), "notes\nmerged\n").unwrap();
    git(rig.home.path(), &["commit", "--quiet", "-am", "Merge pull request #7"]);
    let merge = String::from_utf8(
        std::process::Command::new("git").arg("-C").arg(rig.home.path()).args(["rev-parse", "HEAD"]).output().unwrap().stdout,
    )
    .unwrap()
    .trim()
    .to_string();
    let after = rig.a_job_in_slot(3, "crates/bridge-native/src/view.rs", "use ipc::SessionRow;").await;
    rig.the_forge_says(PullQueue::Waiting);
    rig.fleet.vcs().main_ci.followed_are(Some(Vec::new()));
    let root = rig.root();
    rig.fleet.sweeping().lock().await.merged.insert(
        root,
        vec![crate::main_hub::MergedPulled {
            pull: RecentlyMerged {
                number: 7,
                title: FromOutside::verbatim("Carry the notice"),
                branch: FromOutside::verbatim("armada/7"),
                url: FromOutside::verbatim("https://forge.invalid/armada/pull/7"),
                author: None,
                merged_at: FromOutside::verbatim(rig.fleet.now().as_str()),
                commit: Some(FromOutside::verbatim(&merge)),
            },
        }],
    );

    rig.reads().await;

    let told = rig.wire_words_to(&before).await;
    let [said] = told.as_slice() else { panic!("one word: {told:?}") };
    assert!(said.starts_with("Wire change merged into main: #7."), "{said}");
    assert!(rig.wire_words_to(&after).await.is_empty(), "a branch cut after the merge already reads the change");
}

#[tokio::test]
async fn each_merge_of_the_last_hours_is_asked_of_the_forge_once_however_many_there_are() {
    let rig = a_rig();
    // Read nothing it changes, so it is never told, and every merge stays waiting on it.
    let _elsewhere = rig.a_job_in_slot(4, "docs/notes.md", "more").await;
    rig.the_forge_says(PullQueue::Waiting);
    rig.fleet.vcs().main_ci.followed_are(Some(Vec::new()));
    let merged: Vec<_> = (1..=40)
        .map(|number| crate::main_hub::MergedPulled {
            pull: RecentlyMerged {
                number,
                title: FromOutside::verbatim("A merge"),
                branch: FromOutside::verbatim(&format!("armada/{number}")),
                url: FromOutside::verbatim(&format!("https://forge.invalid/armada/pull/{number}")),
                author: None,
                merged_at: FromOutside::verbatim(rig.fleet.now().as_str()),
                commit: None,
            },
        })
        .collect();
    rig.fleet.sweeping().lock().await.merged.insert(rig.root(), merged);

    rig.reads().await;
    rig.reads().await;

    let asked = rig
        .fleet
        .vcs()
        .delivered()
        .into_iter()
        .filter(|one| matches!(one, testkit::Delivered::ReadPullRequestPatches { .. }))
        .count();
    assert_eq!(asked, 40, "a second reading asks again of none");
}
