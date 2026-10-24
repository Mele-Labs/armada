//! Promote: a mod put into the repository by a Job of its own, which pushes its branch, opens its
//! pull request and is told to see it merged. `crate::mods::promote`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use adapter_traits::Opened;
use api::Mods;
use ipc::{ModPromoted, ModPullRequest, PromoteMod};
use testkit::{CommitScope, Delivered, Delivering, FakeVcs};

use super::session_host::{eventually, rig_fitted, rig_placing, Rig};

const TOML: &str = "name = \"calm\"\nkind = \"theme\"\nversion = \"0.1.0\"\ndescription = \"Softer greys\"\n";
const CSS: &str = ":root {\n  --bg-base: #101418;\n  --accent: #4A9EDB;\n}\n";
const PATHS: [&str; 2] = ["packages/mods/calm/mod.toml", "packages/mods/calm/theme.css"];

/// The rig's repository given a `packages/` folder, and the theme mod `calm` in the machine's mods.
fn a_mod_to_promote(rig: &Rig) {
    std::fs::create_dir_all(Path::new(&rig.root).join("packages")).expect("packages/");
    let at = Path::new(&rig.fleet.host().mods_dir).join("calm");
    std::fs::create_dir_all(&at).expect("a folder");
    std::fs::write(at.join("mod.toml"), TOML).expect("mod.toml");
    std::fs::write(at.join("theme.css"), CSS).expect("theme.css");
}

async fn promoted(rig: &Rig) -> ModPromoted {
    Arc::clone(&rig.fleet).promote_mod(PromoteMod { name: "calm".into(), manifest_id: None }).await.expect("promoted")
}

/// The tree of the Job a promote made: its slot, as its hosting holds it.
async fn tree_of(rig: &Rig, promoted: &ModPromoted) -> PathBuf {
    let hosting = rig.fleet.store().lock().await.hosting(promoted.session_id.as_str()).expect("read").expect("hosted");
    assert_eq!(hosting.lease_branch.as_deref(), Some(promoted.branch.as_str()), "the Job holds the branch");
    PathBuf::from(adapter_traits::slot_path(&rig.root, hosting.lease_slot.expect("a slot")))
}

/// **A Job of its own that sees it merged**: the mod's two files committed alone on the Job's
/// branch, pushed, a pull request opened and held on the Job's row, and the Job told what to do
/// with it. The checkout is never written.
#[tokio::test]
async fn promoting_a_mod_opens_its_pull_request_in_a_job_of_its_own_and_briefs_that_job() {
    let rig = rig_placing(None);
    a_mod_to_promote(&rig);
    let made = rig.fleet.vcs().committed().len();

    let promoted = promoted(&rig).await;

    let tree = tree_of(&rig, &promoted).await;
    let session = rig.fleet.store().lock().await.session(promoted.session_id.as_str()).expect("read").expect("kept");
    assert_eq!(session.title.as_deref(), Some("Add the calm theme mod"));
    let commits = rig.fleet.vcs().committed();
    assert_eq!(commits.len(), made + 1, "one commit");
    assert_eq!((commits[made].branch.as_str(), commits[made].message.as_str()), (promoted.branch.as_str(), "Add the calm theme mod"));
    assert_eq!(commits[made].scope, CommitScope::Paths(PATHS.map(String::from).to_vec()));
    assert_eq!(std::fs::read_to_string(tree.join(PATHS[0])).expect("in the tree"), TOML);
    assert_eq!(std::fs::read_to_string(tree.join(PATHS[1])).expect("in the tree"), CSS);

    let delivered = rig.fleet.vcs().delivered();
    assert!(delivered.contains(&Delivered::Pushed { branch: promoted.branch.clone() }), "{delivered:?}");
    let opened = delivered.iter().find_map(|one| match one {
        Delivered::OpenedForReview { review, .. } => Some(review.title().to_string()),
        _ => None,
    });
    assert_eq!(opened.as_deref(), Some("Add the calm theme mod"));
    assert_eq!(promoted.pull_request.as_deref(), Some("https://forge.invalid/armada/pull/1"));
    assert_eq!(promoted.pull_request_not_opened, None);
    let held = rig.fleet.store().lock().await.attachments_of(&store::Holder::session(promoted.session_id.as_str())).expect("read");
    assert!(held.iter().any(|row| row.kind == "pr" && row.target == "1"), "the Job holds its pull request");

    let briefed = || async {
        rig.stand_in.process_of(promoted.session_id.as_str()).is_some_and(|process| {
            process.sent.lock().unwrap().iter().any(|line| {
                line.contains("https://forge.invalid/armada/pull/1") && line.contains("finish when it merges")
            })
        })
    };
    eventually(briefed).await;
    assert!(!Path::new(&rig.root).join("packages/mods/calm").exists(), "the checkout is untouched");
}

/// **A pull request that does not open loses nothing**: the answer says why and names the Job,
/// which keeps its branch pushed with the commit on it. Its agent is not started, since it has no
/// pull request to watch.
#[tokio::test]
async fn a_pull_request_that_does_not_open_answers_why_and_still_names_the_job() {
    let rig = rig_fitted(|fittings| {
        let vcs = std::mem::replace(&mut fittings.vcs, FakeVcs::new());
        fittings.vcs = vcs.delivering(Delivering {
            review: Opened::NoTool { why: String::from("gh is not installed") },
            ..Delivering::default()
        });
    });
    a_mod_to_promote(&rig);

    let promoted = promoted(&rig).await;

    assert_eq!(promoted.pull_request, None);
    let said = promoted.pull_request_not_opened.clone().expect("says why");
    assert_eq!(said, format!("{} is pushed. No pull request was opened: gh is not installed", promoted.branch));
    assert!(rig.fleet.vcs().delivered().contains(&Delivered::Pushed { branch: promoted.branch.clone() }));
    assert!(tree_of(&rig, &promoted).await.join(PATHS[1]).exists(), "the commit's files stay in the Job's tree");
    let session = rig.fleet.store().lock().await.session(promoted.session_id.as_str()).expect("read").expect("kept");
    assert_eq!(session.state, store::SessionState::Live, "the Job stays");
    assert!(rig.stand_in.process_of(promoted.session_id.as_str()).is_none(), "nothing was sent to it");
}

/// **A promote outlives the window that pressed it**: `list_mods` carries the open pull request and
/// its Job on the mod's row, so a Bridge read again is not offered Promote, and a second promote is
/// refused while that pull request is open. Once it merges the row carries none.
#[tokio::test]
async fn an_open_pull_request_is_listed_on_its_mod_and_refuses_a_second_promote() {
    let rig = rig_placing(None);
    a_mod_to_promote(&rig);
    let promoted = promoted(&rig).await;
    assert_eq!(promoted.title, "Add the calm theme mod");

    let listed = rig.fleet.list_mods().await.expect("listed");
    let calm = listed.mods.iter().find(|one| one.name == "calm").expect("calm is listed");
    let open = ModPullRequest {
        address: String::from("https://forge.invalid/armada/pull/1"),
        number: 1,
        title: String::from("Add the calm theme mod"),
        session_id: promoted.session_id.clone(),
    };
    assert_eq!(calm.pull_request.as_ref(), Some(&open));

    let again = Arc::clone(&rig.fleet).promote_mod(PromoteMod { name: "calm".into(), manifest_id: None }).await.expect_err("refused");
    assert_eq!((again.status(), again.error().code.as_str()), (409, "fleet.mod_not_promotable"));
    assert!(again.error().message.contains("https://forge.invalid/armada/pull/1"), "{}", again.error().message);

    let holder = store::Holder::session(promoted.session_id.as_str());
    let manifest = rig.fleet.repositories().served()[0].manifest().id().as_str().to_string();
    rig.fleet.store().lock().await.settle(&holder, "pr", &manifest, "1", store::AttachmentState::Spent, "2026-10-23T12:00:00Z").expect("merged");
    let merged = rig.fleet.list_mods().await.expect("listed");
    assert_eq!(merged.mods.iter().find(|one| one.name == "calm").expect("calm").pull_request, None);
}
