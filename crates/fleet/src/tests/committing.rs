//! Commit to repository: a copy of a person's own piece committed on a Job's branch, and theirs
//! left where it is kept. `crate::committing`.

use std::path::{Path, PathBuf};

use ipc::{
    CommitToRepository, FlightControl, FlightControlKind, FlightControls, FlightControlsRead,
    PieceToCommit, SaveRepositoryFlightControls, SessionId, WorkflowId,
};
use testkit::CommitScope;

use super::session_host::{rig_placing, Rig};
use crate::committing::{EXISTS, NOT_YOURS, NO_JOB};
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

fn asking(rig: &Rig, session_id: Option<SessionId>, piece: PieceToCommit, overwrite: bool) -> CommitToRepository {
    CommitToRepository { repository: rig.root.clone(), session_id, piece, overwrite }
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

/// **A copy**: the chosen tile joins the branch's own file, and the person's file is byte for
/// byte what it was.
#[tokio::test]
async fn committing_tiles_adds_the_chosen_ones_on_the_jobs_branch_and_leaves_yours_untouched() {
    let rig = rig_placing(None);
    let (mine, before) = yours(&rig, vec![own("specs", Some("s")), own("deploy", None)]).await;
    let (id, tree) = a_job_holding(&rig, Some((4, "job/one"))).await;
    plant(&tree.join(IN_REPOSITORY), "version = 1\n[[job]]\nid = \"lint\"\nkind = \"shell\"\ntext = \"make lint\"\n");
    let made = rig.fleet.vcs().committed().len();

    let committed = rig.fleet.committed_to_repository(asking(&rig, Some(id), tiles(&["specs"]), false)).await.expect("committed");

    assert_eq!((committed.branch.as_str(), committed.paths.as_slice()), ("job/one", [IN_REPOSITORY.to_string()].as_slice()));
    assert!(committed.commit.is_some());
    let commits = rig.fleet.vcs().committed();
    assert_eq!(commits.len(), made + 1);
    assert_eq!(commits[made].branch, "job/one");
    assert_eq!(commits[made].message, "Add the tile specs");
    assert_eq!(commits[made].scope, CommitScope::Paths(vec![IN_REPOSITORY.to_string()]));
    let on_branch = crate::flight_controls::repository::parse_repository(&std::fs::read_to_string(tree.join(IN_REPOSITORY)).expect("in the tree")).expect("reads");
    assert_eq!(on_branch.job.iter().map(|tile| tile.id.as_str()).collect::<Vec<_>>(), ["lint", "specs"]);
    assert_eq!(std::fs::read(&mine).expect("still kept"), before, "the person's copy is untouched");
    assert!(!Path::new(&rig.root).join(IN_REPOSITORY).exists(), "the checkout is untouched");
}

#[tokio::test]
async fn committing_with_no_jobs_branch_is_refused_and_writes_nothing() {
    let rig = rig_placing(None);
    let (mine, before) = yours(&rig, vec![own("specs", None)]).await;
    let (unleased, _) = a_job_holding(&rig, None).await;
    let made = rig.fleet.vcs().committed().len();

    for (session_id, says) in [(None, "No Job is named."), (Some(unleased), "holds no branch in")] {
        let refusal = rig.fleet.committed_to_repository(asking(&rig, session_id, tiles(&["specs"]), false)).await.expect_err("refused");
        let (code, said) = refused_with(refusal);
        assert_eq!(code, NO_JOB);
        assert!(said.contains(says) && said.contains("Commit from a Job open in this repository."), "{said}");
    }
    assert_eq!(rig.fleet.vcs().committed().len(), made, "nothing committed");
    assert_eq!(std::fs::read(&mine).expect("still kept"), before);
    assert!(!Path::new(&rig.root).join(IN_REPOSITORY).exists());
}

#[tokio::test]
async fn a_tile_the_branch_already_holds_is_replaced_only_with_overwrite() {
    let rig = rig_placing(None);
    yours(&rig, vec![own("specs", None)]).await;
    let (id, tree) = a_job_holding(&rig, Some((4, "job/one"))).await;
    plant(&tree.join(IN_REPOSITORY), "version = 1\n[[job]]\nid = \"specs\"\nkind = \"shell\"\ntext = \"old\"\n");

    let refusal = rig.fleet.committed_to_repository(asking(&rig, Some(id.clone()), tiles(&["specs"]), false)).await.expect_err("refused");
    let (code, said) = refused_with(refusal);
    assert_eq!(code, EXISTS);
    assert!(said.contains("already holds `specs`"), "{said}");
    assert!(std::fs::read_to_string(tree.join(IN_REPOSITORY)).expect("there").contains("old"));

    rig.fleet.committed_to_repository(asking(&rig, Some(id), tiles(&["specs"]), true)).await.expect("replaced");
    let text = std::fs::read_to_string(tree.join(IN_REPOSITORY)).expect("there");
    assert!(text.contains("bin/specs") && !text.contains("old"), "{text}");
}

#[tokio::test]
async fn a_tile_that_is_not_yours_is_refused() {
    let rig = rig_placing(None);
    yours(&rig, vec![own("specs", None)]).await;
    let (id, tree) = a_job_holding(&rig, Some((4, "job/one"))).await;

    for (piece, says) in [(tiles(&["deploy"]), "have no `deploy` in job"), (tiles(&[]), "No tiles are chosen.")] {
        let refusal = rig.fleet.committed_to_repository(asking(&rig, Some(id.clone()), piece, false)).await.expect_err("refused");
        let (code, said) = refused_with(refusal);
        assert_eq!(code, NOT_YOURS);
        assert!(said.contains(says), "{said}");
    }
    assert!(!tree.join(IN_REPOSITORY).exists());
}

/// Kit's workflow file, as the person wrote it, into `.armada/workflows/` on the branch.
#[tokio::test]
async fn committing_a_workflow_copies_kits_file_onto_the_jobs_branch() {
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
    let (id, tree) = a_job_holding(&rig, Some((4, "job/one"))).await;
    let piece = PieceToCommit::Workflow { workflow_id: WorkflowId::carried("quick") };

    let committed = rig.fleet.committed_to_repository(asking(&rig, Some(id), piece.clone(), false)).await.expect("committed");

    assert_eq!(committed.paths, [".armada/workflows/quick.yml"]);
    assert_eq!(std::fs::read_to_string(tree.join(".armada/workflows/quick.yml")).expect("on the branch"), text);
    assert_eq!(rig.fleet.vcs().committed().last().expect("a commit").message, "Add the workflow quick");
    assert_eq!(std::fs::read_to_string(&file).expect("still in Kit"), text, "the person's copy is untouched");

    let missing = PieceToCommit::Workflow { workflow_id: WorkflowId::carried("absent") };
    let (code, _) = refused_with(rig.fleet.committed_to_repository(asking(&rig, None, missing, false)).await.expect_err("refused"));
    assert_eq!(code, NO_JOB, "a Job is asked for before anything else");
}

/// Kit's review tool file into `.armada/review-tools/`, refused where the branch has the id.
#[tokio::test]
async fn committing_a_review_tool_copies_kits_file_and_refuses_one_the_branch_holds() {
    let rig = rig_placing(None);
    let text = "---\nkind: tool\nname: Security\n---\nLook for secrets in the diff.\n";
    let file = kit(&rig).join("review-tools").join("security.md");
    plant(&file, text);
    let (id, tree) = a_job_holding(&rig, Some((4, "job/one"))).await;
    let piece = PieceToCommit::ReviewTool { id: "security".into() };

    let committed = rig.fleet.committed_to_repository(asking(&rig, Some(id.clone()), piece.clone(), false)).await.expect("committed");
    assert_eq!(committed.paths, [".armada/review-tools/security.md"]);
    assert_eq!(std::fs::read_to_string(tree.join(".armada/review-tools/security.md")).expect("on the branch"), text);
    assert_eq!(rig.fleet.vcs().committed().last().expect("a commit").message, "Add the review tool security");
    assert_eq!(std::fs::read_to_string(&file).expect("still in Kit"), text);

    let (code, _) = refused_with(rig.fleet.committed_to_repository(asking(&rig, Some(id.clone()), piece, false)).await.expect_err("refused"));
    assert_eq!(code, EXISTS);
    let other = PieceToCommit::ReviewTool { id: "absent".into() };
    let (code, said) = refused_with(rig.fleet.committed_to_repository(asking(&rig, Some(id), other, false)).await.expect_err("refused"));
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
