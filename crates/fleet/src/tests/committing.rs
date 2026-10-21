//! Commit to repository: a copy of a person's own piece committed by a Job of its own, on its own
//! branch with a pull request for that alone, and theirs left where it is kept. `crate::committing`.

use std::path::{Path, PathBuf};

use adapter_traits::Opened;
use ipc::{
    CommitToRepository, CommittedToRepository, FlightControl, FlightControlKind, FlightControls,
    FlightControlsRead, PieceToCommit, SaveRepositoryFlightControls, SessionId, WorkflowId,
};
use testkit::{CommitScope, Delivered, Delivering, FakeVcs};

use super::session_host::{rig_fitted, rig_placing, Rig};
use crate::committing::{EXISTS, NOT_YOURS};
use crate::flight_controls::repository::IN_REPOSITORY;

fn own(id: &str, key: Option<&str>) -> FlightControl {
    FlightControl {
        id: id.into(),
        name: None,
        glyph: None,
        key: key.map(String::from),
        hidden: false,
        text: Some(format!("bin/{id}")),
        on_text: None,
        off_text: None,
        kind: Some(FlightControlKind::Shell),
    }
}

fn kit(rig: &Rig) -> PathBuf {
    PathBuf::from(&rig.fleet.host().kit_home)
}

/// A Job in the rig's repository holding `lease`, a slot and its branch, with the slot's tree made.
async fn a_job_holding(rig: &Rig, lease: Option<(u32, &str)>) -> (SessionId, PathBuf) {
    let id = rig.start().await;
    let mut hosting = rig.fleet.store().lock().await.hosting(id.as_str()).expect("read").expect("hosted");
    hosting.lease_slot = lease.map(|(slot, _)| slot);
    hosting.lease_branch = lease.map(|(_, branch)| branch.to_string());
    rig.fleet.store().lock().await.keep_hosting(&hosting).expect("kept");
    let tree = PathBuf::from(adapter_traits::slot_path(&rig.root, lease.map_or(0, |(slot, _)| slot)));
    std::fs::create_dir_all(&tree).expect("a tree");
    (id, tree)
}

/// The person's own tiles for the rig's repository, saved where #2627 keeps them, and the file's
/// bytes as written.
async fn yours(rig: &Rig, job: Vec<FlightControl>) -> (PathBuf, Vec<u8>) {
    let controls = FlightControls { version: 1, job, deck: Vec::new() };
    rig.fleet
        .repository_flight_controls_saved(SaveRepositoryFlightControls { repository: rig.root.clone(), controls })
        .await
        .expect("saved");
    let path = crate::run_memory::places::file_for(&kit(rig), crate::flight_controls::repository::FOLDER, &rig.root);
    let bytes = std::fs::read(&path).expect("kept for the person");
    (path, bytes)
}

fn asking(rig: &Rig, piece: PieceToCommit, overwrite: bool) -> CommitToRepository {
    CommitToRepository { repository: rig.root.clone(), piece, overwrite }
}

/// The tree of the Job a commit made: its slot, as its hosting holds it.
async fn tree_of(rig: &Rig, committed: &CommittedToRepository) -> PathBuf {
    let hosting = rig.fleet.store().lock().await.hosting(committed.session_id.as_str()).expect("read").expect("hosted");
    assert_eq!(hosting.lease_branch.as_deref(), Some(committed.branch.as_str()), "the Job holds the branch");
    PathBuf::from(adapter_traits::slot_path(&rig.root, hosting.lease_slot.expect("a slot")))
}

fn tiles(job: &[&str]) -> PieceToCommit {
    PieceToCommit::FlightControls { job: job.iter().map(|id| id.to_string()).collect(), deck: Vec::new() }
}

fn refused_with(refusal: api::Refusal) -> (String, String) {
    match refusal {
        api::Refusal::Unacceptable(error) => (error.code, error.message),
        other => panic!("not a 422: {other:?}"),
    }
}

fn ids(read: FlightControlsRead) -> Vec<String> {
    match read {
        FlightControlsRead::Read { controls } => controls.job.into_iter().map(|tile| tile.id).collect(),
        FlightControlsRead::NotRead { why } => panic!("not read: {why}"),
    }
}

fn plant(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("a folder")).expect("made");
    std::fs::write(path, text).expect("written");
}

/// **A Job of its own, with no Job open**: its own slot and branch, one commit holding the tile
/// beside the ones main holds, pushed, and a pull request for that alone. The person's file and
/// the checkout are byte for byte what they were.
#[tokio::test]
async fn committing_with_no_job_open_makes_a_job_of_its_own_with_one_commit_and_its_pull_request() {
    let rig = rig_placing(None);
    let (mine, before) = yours(&rig, vec![own("specs", Some("s")), own("deploy", None)]).await;
    let on_main = "version = 1\n[[job]]\nid = \"lint\"\nkind = \"shell\"\ntext = \"make lint\"\n";
    plant(&Path::new(&rig.root).join(IN_REPOSITORY), on_main);
    let made = rig.fleet.vcs().committed().len();

    let committed = rig.fleet.committed_to_repository(asking(&rig, tiles(&["specs"]), false)).await.expect("committed");

    let tree = tree_of(&rig, &committed).await;
    assert_ne!(tree, PathBuf::from(&rig.root));
    let session = rig.fleet.store().lock().await.session(committed.session_id.as_str()).expect("read").expect("kept");
    assert_eq!(session.title.as_deref(), Some("Commit the tile specs"));
    assert_eq!(committed.paths, [IN_REPOSITORY.to_string()]);
    let commits = rig.fleet.vcs().committed();
    assert_eq!(commits.len(), made + 1, "one commit");
    assert_eq!((commits[made].branch.as_str(), commits[made].message.as_str()), (committed.branch.as_str(), "Add the tile specs"));
    assert_eq!(commits[made].scope, CommitScope::Paths(vec![IN_REPOSITORY.to_string()]));
    let on_branch = crate::flight_controls::repository::parse_repository(&std::fs::read_to_string(tree.join(IN_REPOSITORY)).expect("in the tree")).expect("reads");
    assert_eq!(on_branch.job.iter().map(|tile| tile.id.as_str()).collect::<Vec<_>>(), ["lint", "specs"]);

    let delivered = rig.fleet.vcs().delivered();
    assert!(delivered.contains(&Delivered::Pushed { branch: committed.branch.clone() }), "{delivered:?}");
    let opened = delivered.iter().find_map(|one| match one {
        Delivered::OpenedForReview { review, .. } => Some(review.title().to_string()),
        _ => None,
    });
    assert_eq!(opened.as_deref(), Some("Commit the tile specs"));
    assert_eq!(committed.pull_request.as_deref(), Some("https://forge.invalid/armada/pull/1"));
    assert_eq!(committed.pull_request_not_opened, None);
    let held = rig.fleet.store().lock().await.attachments_of(&store::Holder::session(committed.session_id.as_str())).expect("read");
    assert!(held.iter().any(|row| row.kind == "pr" && row.target == "1"), "the Job holds its pull request");

    assert_eq!(std::fs::read(&mine).expect("still kept"), before, "the person's copy is untouched");
    assert_eq!(std::fs::read_to_string(Path::new(&rig.root).join(IN_REPOSITORY)).expect("on main"), on_main, "the checkout is untouched");
}

/// **Opening the pull request failing loses nothing**: the Job keeps its branch, pushed, with the
/// commit on it, and the answer says why there is no pull request.
#[tokio::test]
async fn a_pull_request_that_does_not_open_leaves_the_jobs_branch_pushed_and_says_so() {
    let rig = rig_fitted(|fittings| {
        let vcs = std::mem::replace(&mut fittings.vcs, FakeVcs::new());
        fittings.vcs = vcs.delivering(Delivering {
            review: Opened::NoTool { why: String::from("gh is not installed") },
            ..Delivering::default()
        });
    });
    yours(&rig, vec![own("specs", None)]).await;

    let committed = rig.fleet.committed_to_repository(asking(&rig, tiles(&["specs"]), false)).await.expect("committed");

    assert_eq!(committed.pull_request, None);
    let said = committed.pull_request_not_opened.clone().expect("says why");
    assert_eq!(said, format!("{} is pushed. No pull request was opened: gh is not installed", committed.branch));
    assert!(rig.fleet.vcs().delivered().contains(&Delivered::Pushed { branch: committed.branch.clone() }));
    assert_eq!(rig.fleet.vcs().committed().last().expect("a commit").branch, committed.branch);
    let tree = tree_of(&rig, &committed).await;
    assert!(tree.join(IN_REPOSITORY).exists(), "the commit's file stays in the Job's tree");
    let session = rig.fleet.store().lock().await.session(committed.session_id.as_str()).expect("read").expect("kept");
    assert_eq!(session.state, store::SessionState::Live, "the Job stays");
}

/// **Against main's tree, before any Job is made**: a tile main already holds is refused without
/// `overwrite`, and replaced on the new Job's branch with it.
#[tokio::test]
async fn a_tile_main_already_holds_is_refused_without_overwrite_and_makes_no_job() {
    let rig = rig_placing(None);
    yours(&rig, vec![own("specs", None)]).await;
    let on_main = "version = 1\n[[job]]\nid = \"specs\"\nkind = \"shell\"\ntext = \"old\"\n";
    plant(&Path::new(&rig.root).join(IN_REPOSITORY), on_main);
    let (cut, made) = (rig.fleet.vcs().created().len(), rig.fleet.vcs().committed().len());

    let refusal = rig.fleet.committed_to_repository(asking(&rig, tiles(&["specs"]), false)).await.expect_err("refused");
    let (code, said) = refused_with(refusal);
    assert_eq!(code, EXISTS);
    assert_eq!(said, format!("{IN_REPOSITORY} on main already holds `specs` in job. Commit with overwrite to replace it."));
    assert_eq!(rig.fleet.vcs().created().len(), cut, "no Job's branch is cut");
    assert_eq!(rig.fleet.vcs().committed().len(), made, "nothing committed");

    let committed = rig.fleet.committed_to_repository(asking(&rig, tiles(&["specs"]), true)).await.expect("replaced");
    let text = std::fs::read_to_string(tree_of(&rig, &committed).await.join(IN_REPOSITORY)).expect("there");
    assert!(text.contains("bin/specs") && !text.contains("old"), "{text}");
    assert_eq!(std::fs::read_to_string(Path::new(&rig.root).join(IN_REPOSITORY)).expect("on main"), on_main);
}

#[tokio::test]
async fn a_tile_that_is_not_yours_is_refused() {
    let rig = rig_placing(None);
    yours(&rig, vec![own("specs", None)]).await;

    for (piece, says) in [(tiles(&["deploy"]), "have no `deploy` in job"), (tiles(&[]), "No tiles are chosen.")] {
        let refusal = rig.fleet.committed_to_repository(asking(&rig, piece, false)).await.expect_err("refused");
        let (code, said) = refused_with(refusal);
        assert_eq!(code, NOT_YOURS);
        assert!(said.contains(says), "{said}");
    }
    assert!(rig.fleet.vcs().created().is_empty(), "no Job's branch is cut");
}

/// Kit's workflow file, as the person wrote it, into `.armada/workflows/` on the branch.
#[tokio::test]
async fn committing_a_workflow_copies_kits_file_onto_its_jobs_branch() {
    let rig = rig_placing(None);
    let file = kit(&rig).join("workflows").join("quick.yml");
    let text = "version: 1\nworkflow_id: quick\nname: Quick\nsteps:\n  - id: implement\n    label: \"Implement\"\n    \
                evidence: {submitted: {type: diff}}\n    mechanical_checks:\n      - type: every_manifest_check\n    \
                delivers: false\n    advance_gate: auto\n";
    plant(&file, text);
    let served = rig.fleet.repositories().first().expect("served");
    let written = [config::Written::in_kit(file.clone(), text.to_string())];
    let (workflows, _, files) = config::Catalogue::of(written, &config::Roster::offering_nothing()).resolve(served.manifest()).into_parts();
    assert_eq!(files.len(), 1, "the workflow fits the rig's Manifest");
    served.catalogued(crate::repositories::Catalogued { workflows, left_out: Vec::new(), files });
    let piece = PieceToCommit::Workflow { workflow_id: WorkflowId::carried("quick") };

    let committed = rig.fleet.committed_to_repository(asking(&rig, piece, false)).await.expect("committed");

    assert_eq!(committed.paths, [".armada/workflows/quick.yml"]);
    let tree = tree_of(&rig, &committed).await;
    assert_eq!(std::fs::read_to_string(tree.join(".armada/workflows/quick.yml")).expect("on the branch"), text);
    assert_eq!(rig.fleet.vcs().committed().last().expect("a commit").message, "Add the workflow quick");
    assert_eq!(std::fs::read_to_string(&file).expect("still in Kit"), text, "the person's copy is untouched");

    let missing = PieceToCommit::Workflow { workflow_id: WorkflowId::carried("absent") };
    let (code, _) = refused_with(rig.fleet.committed_to_repository(asking(&rig, missing, false)).await.expect_err("refused"));
    assert_eq!(code, NOT_YOURS);
}

/// Kit's review tool file into `.armada/review-tools/`, refused where main has the id.
#[tokio::test]
async fn committing_a_review_tool_copies_kits_file_and_refuses_one_main_holds() {
    let rig = rig_placing(None);
    let text = "---\nkind: tool\nname: Security\n---\nLook for secrets in the diff.\n";
    let file = kit(&rig).join("review-tools").join("security.md");
    plant(&file, text);
    let piece = PieceToCommit::ReviewTool { id: "security".into() };

    let committed = rig.fleet.committed_to_repository(asking(&rig, piece.clone(), false)).await.expect("committed");
    assert_eq!(committed.paths, [".armada/review-tools/security.md"]);
    let tree = tree_of(&rig, &committed).await;
    assert_eq!(std::fs::read_to_string(tree.join(".armada/review-tools/security.md")).expect("on the branch"), text);
    assert_eq!(rig.fleet.vcs().committed().last().expect("a commit").message, "Add the review tool security");
    assert_eq!(std::fs::read_to_string(&file).expect("still in Kit"), text);

    plant(&Path::new(&rig.root).join(".armada/review-tools/security.md"), "---\nkind: tool\nname: Security\n---\nOlder.\n");
    let (code, _) = refused_with(rig.fleet.committed_to_repository(asking(&rig, piece.clone(), false)).await.expect_err("refused"));
    assert_eq!(code, EXISTS);
    plant(&Path::new(&rig.root).join(".armada/review-tools/security.md"), text);
    let (code, said) = refused_with(rig.fleet.committed_to_repository(asking(&rig, piece, true)).await.expect_err("refused"));
    assert_eq!((code.as_str(), said.as_str()), (EXISTS, ".armada/review-tools/security.md on main already holds this as written."));
    let other = PieceToCommit::ReviewTool { id: "absent".into() };
    let (code, said) = refused_with(rig.fleet.committed_to_repository(asking(&rig, other, false)).await.expect_err("refused"));
    assert_eq!((code.as_str(), said.as_str()), (NOT_YOURS, "Your Kit holds no review tool `absent`."));
}

#[tokio::test]
async fn the_repositorys_own_tiles_are_read_from_the_named_jobs_tree_and_else_the_checkout() {
    let rig = rig_placing(None);
    plant(&Path::new(&rig.root).join(IN_REPOSITORY), "version = 1\n[[job]]\nid = \"on-main\"\nkind = \"shell\"\ntext = \"make\"\n");
    let (id, tree) = a_job_holding(&rig, Some((4, "job/one"))).await;
    plant(&tree.join(IN_REPOSITORY), "version = 1\n[[job]]\nid = \"on-branch\"\nkind = \"shell\"\ntext = \"make\"\n");

    let read = rig.fleet.repository_flight_controls_read(rig.root.clone(), None).await.expect("read");
    assert_eq!(ids(read.in_repository), ["on-main"]);
    assert_eq!(ids(read.read), Vec::<String>::new(), "the person's own are apart");
    let read = rig.fleet.repository_flight_controls_read(rig.root.clone(), Some(id)).await.expect("read");
    assert_eq!(ids(read.in_repository), ["on-branch"]);
}

const WHOLE: &str = "version: 1\nid: 01FIXTUREMANIFEST\nchecks:\n  test:\n    run: make test\n";

/// Where Kit keeps the person's own `armada.yml` for the rig's repository.
fn kit_manifest(rig: &Rig) -> PathBuf {
    crate::run_memory::places::kit_manifest(&kit(rig), &rig.root)
}

/// **A copy**: Kit's `armada.yml` lands at the root of the Job's tree, and Kit's is as it was.
#[tokio::test]
async fn committing_the_manifest_copies_kits_file_to_the_workspace_root_and_leaves_kits() {
    let rig = rig_placing(None);
    plant(&kit_manifest(&rig), WHOLE);
    let made = rig.fleet.vcs().committed().len();

    let committed = rig.fleet.committed_to_repository(asking(&rig, PieceToCommit::Manifest, false)).await.expect("committed");

    assert_eq!(committed.paths, ["armada.yml".to_string()]);
    let tree = tree_of(&rig, &committed).await;
    let commits = rig.fleet.vcs().committed();
    assert_eq!((commits.len(), commits[made].message.as_str()), (made + 1, "Add armada.yml"));
    assert_eq!(commits[made].scope, CommitScope::Paths(vec!["armada.yml".to_string()]));
    assert_eq!(std::fs::read_to_string(tree.join("armada.yml")).expect("on the branch"), WHOLE);
    assert_eq!(std::fs::read_to_string(kit_manifest(&rig)).expect("still in Kit"), WHOLE, "the person's copy is untouched");
    assert!(!Path::new(&rig.root).join("armada.yml").exists(), "the checkout is untouched");
}

#[tokio::test]
async fn an_armada_yml_main_holds_is_replaced_only_with_overwrite() {
    let rig = rig_placing(None);
    plant(&kit_manifest(&rig), WHOLE);
    let on_main = Path::new(&rig.root).join("armada.yml");
    plant(&on_main, "version: 1\nid: 01FIXTUREOLDONE\n");

    let refusal = rig.fleet.committed_to_repository(asking(&rig, PieceToCommit::Manifest, false)).await.expect_err("refused");
    let (code, said) = refused_with(refusal);
    assert_eq!((code.as_str(), said.as_str()), (EXISTS, "armada.yml on main already exists. Commit with overwrite to replace it."));

    let committed = rig.fleet.committed_to_repository(asking(&rig, PieceToCommit::Manifest, true)).await.expect("replaced");
    assert_eq!(std::fs::read_to_string(tree_of(&rig, &committed).await.join("armada.yml")).expect("there"), WHOLE);
    assert!(std::fs::read_to_string(&on_main).expect("there").contains("OLDONE"), "the checkout is untouched");
}

#[tokio::test]
async fn the_manifest_is_refused_with_no_copy_in_kit_or_a_copy_that_is_not_whole() {
    let rig = rig_placing(None);
    let refusal = rig.fleet.committed_to_repository(asking(&rig, PieceToCommit::Manifest, false)).await.expect_err("refused");
    let (code, said) = refused_with(refusal);
    assert_eq!((code.as_str(), said.as_str()), (NOT_YOURS, "Your Kit holds no armada.yml for this repository."));

    plant(&kit_manifest(&rig), "checks:\n  test:\n    run: make quick\n");
    let refusal = rig.fleet.committed_to_repository(asking(&rig, PieceToCommit::Manifest, false)).await.expect_err("refused");
    let (code, said) = refused_with(refusal);
    assert_eq!(code, crate::committing::REFUSED);
    assert!(said.contains("is not a whole Manifest"), "{said}");
    assert!(rig.fleet.vcs().created().is_empty(), "no Job's branch is cut");
}
