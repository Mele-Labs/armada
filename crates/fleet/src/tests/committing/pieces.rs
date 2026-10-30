//! Commit to repository for a Workspace, a lifecycle, a rule and autonomy defaults, and the removal
//! of one: found by what a file declares, written under Kit's own name, kept to the repository its
//! scope belongs to, and deleted on the Job's branch by the same commit path. `crate::committing`.

use std::path::{Path, PathBuf};

use ipc::{CommitToRepository, PieceScope, PieceToCommit};
use testkit::CommitScope;

use super::{asking, finished, kit, plant, refused_with, tree_of};
use crate::committing::{EXISTS, NOT_YOURS, REFUSED};
use crate::repositories::{Located, SetUp};
use crate::tests::session_host::{rig_placing, Rig};

const TICKET: &str = "version: 1\nlifecycle_id: ticket\nname: Ticket\nstates:\n  - { id: todo, label: To do, read: { tracker: [] } }\n  - { id: closed, label: Closed, read: { tracker: [] } }\nends: [closed]\n";
const NUDGE: &str = "version: 1\nrule_id: nudge\nname: Nudge\nwhen: { job: landed }\ndo:\n  - tell_agent: { wake: true }\n";

fn lifecycle(id: &str, scope: PieceScope) -> PieceToCommit {
    PieceToCommit::Lifecycle { id: id.to_string(), scope }
}

fn rule(id: &str, scope: PieceScope) -> PieceToCommit {
    PieceToCommit::LifecycleRule { id: id.to_string(), scope }
}

fn within(id: &str) -> PieceScope {
    PieceScope::Within { id: id.to_string() }
}

fn removing(root: &str, piece: PieceToCommit) -> CommitToRepository {
    CommitToRepository { repository: root.to_string(), piece, overwrite: false, remove: true }
}

fn on(rig: &Rig, root: &str, piece: PieceToCommit) -> CommitToRepository {
    CommitToRepository { repository: root.to_string(), ..asking(rig, piece, false) }
}

/// A second served repository, whose Manifest id is `01OTHERMANIFEST`.
fn another(rig: &Rig) -> String {
    let served = rig.fleet.repositories().first().expect("served");
    let root = format!("{}-other", rig.root);
    std::fs::create_dir_all(&root).expect("a folder");
    let manifest = config::Manifest::parse(&Path::new(&root).join("armada.yml"), "version: 1\nid: 01OTHERMANIFEST\n").expect("a Manifest");
    rig.fleet
        .repositories()
        .add(Located { root: root.clone(), records_root: root.clone(), set_up: Some(SetUp::of(manifest, (*served.workflows()).clone())) })
        .expect("served");
    root
}

/// The Workspace `my-store` as a file, holding the rig's repository and the other.
fn my_store(rig: &Rig) -> String {
    format!("version: 1\nworkspace_id: my-store\nname: My store\nrepositories:\n  - {}\n  - 01OTHERMANIFEST\n", rig.manifest.as_str())
}

fn kit_file(rig: &Rig, folder: &str, name: &str) -> PathBuf {
    kit(rig).join(folder).join(name)
}

/// Kit's file goes in under its own name, as written, and Kit's is as it was.
#[tokio::test]
async fn committing_a_lifecycle_copies_kits_file_under_its_own_name() {
    let rig = rig_placing(None);
    let file = kit_file(&rig, "lifecycles", "work-tickets.yml");
    plant(&file, TICKET);
    let made = rig.fleet.vcs().committed().len();

    let committed = rig.fleet.committed_to_repository(asking(&rig, lifecycle("ticket", PieceScope::Everywhere), false)).await.expect("committed");
    finished(&rig, &committed).await;

    assert_eq!(committed.paths, [".armada/lifecycles/work-tickets.yml"]);
    let tree = tree_of(&rig, &committed).await;
    assert_eq!(std::fs::read_to_string(tree.join(".armada/lifecycles/work-tickets.yml")).expect("on the branch"), TICKET);
    let commits = rig.fleet.vcs().committed();
    assert_eq!((commits.len(), commits[made].message.as_str()), (made + 1, "Add the lifecycle ticket"));
    assert_eq!(commits[made].scope, CommitScope::Paths(vec![".armada/lifecycles/work-tickets.yml".to_string()]));
    let session = rig.fleet.store().lock().await.session(committed.session_id.as_str()).expect("read").expect("kept");
    assert_eq!(session.title.as_deref(), Some("Commit the lifecycle ticket"));
    assert_eq!(std::fs::read_to_string(&file).expect("still in Kit"), TICKET);
    assert!(!Path::new(&rig.root).join(".armada/lifecycles").exists(), "the checkout is untouched");
}

#[tokio::test]
async fn each_kind_goes_to_its_own_folder() {
    let rig = rig_placing(None);
    plant(&kit_file(&rig, "rules", "nudge.yml"), NUDGE);
    plant(&kit_file(&rig, "autonomy", "mine.yml"), "version: 1\nautonomy_id: mine\ndefaults:\n  draft: asks\n");
    plant(&kit_file(&rig, "workspaces", "my-store.yml"), &my_store(&rig));
    for (piece, path) in [
        (rule("nudge", PieceScope::Everywhere), ".armada/rules/nudge.yml"),
        (PieceToCommit::Autonomy { id: "mine".into(), scope: PieceScope::Everywhere }, ".armada/autonomy/mine.yml"),
        (PieceToCommit::Workspace { id: "my-store".into() }, ".armada/workspaces/my-store.yml"),
    ] {
        let committed = rig.fleet.committed_to_repository(asking(&rig, piece, false)).await.expect("committed");
        assert_eq!(committed.paths, [path]);
        finished(&rig, &committed).await;
    }
}

/// **The file on the branch that declares the piece is the one replaced, whatever it is called.**
#[tokio::test]
async fn a_file_main_holds_for_the_piece_is_replaced_in_place_and_only_with_overwrite() {
    let rig = rig_placing(None);
    plant(&kit_file(&rig, "lifecycles", "work-tickets.yml"), TICKET);
    let held = Path::new(&rig.root).join(".armada/lifecycles/old-name.yaml");
    plant(&held, &TICKET.replace("name: Ticket", "name: Older"));
    let piece = lifecycle("ticket", PieceScope::Everywhere);

    let (code, said) = refused_with(rig.fleet.committed_to_repository(asking(&rig, piece.clone(), false)).await.expect_err("refused"));
    assert_eq!((code.as_str(), said.as_str()), (EXISTS, ".armada/lifecycles/old-name.yaml on main already holds `ticket`. Commit with overwrite to replace it."));
    assert!(rig.fleet.vcs().created().is_empty(), "no Job's branch is cut");

    let committed = rig.fleet.committed_to_repository(asking(&rig, piece, true)).await.expect("replaced");
    finished(&rig, &committed).await;
    assert_eq!(committed.paths, [".armada/lifecycles/old-name.yaml"]);
    assert_eq!(std::fs::read_to_string(tree_of(&rig, &committed).await.join(".armada/lifecycles/old-name.yaml")).expect("there"), TICKET);
}

/// **Main's file under the name Kit's goes by, declaring something else, is never written over.**
#[tokio::test]
async fn a_file_that_declares_something_else_under_the_same_name_is_never_written_over() {
    let rig = rig_placing(None);
    plant(&kit_file(&rig, "lifecycles", "tickets.yml"), TICKET);
    plant(&Path::new(&rig.root).join(".armada/lifecycles/tickets.yml"), &TICKET.replace("lifecycle_id: ticket", "lifecycle_id: ticket_v2"));
    for overwrite in [false, true] {
        let asked = asking(&rig, lifecycle("ticket", PieceScope::Everywhere), overwrite);
        let (code, said) = refused_with(rig.fleet.committed_to_repository(asked).await.expect_err("refused"));
        assert_eq!(code, EXISTS);
        assert!(said.contains("holds something else"), "{said}");
    }
    assert!(rig.fleet.vcs().created().is_empty());
}

#[tokio::test]
async fn a_piece_kit_does_not_hold_is_not_yours_and_scope_is_part_of_what_is_held() {
    let rig = rig_placing(None);
    let (code, said) = refused_with(rig.fleet.committed_to_repository(asking(&rig, rule("nudge", PieceScope::Everywhere), false)).await.expect_err("refused"));
    assert_eq!((code.as_str(), said.as_str()), (NOT_YOURS, "Your Kit holds no rule `nudge`."));

    plant(&kit_file(&rig, "lifecycles", "tickets.yml"), TICKET);
    let (code, said) = refused_with(rig.fleet.committed_to_repository(asking(&rig, lifecycle("ticket", within("my-store")), false)).await.expect_err("refused"));
    assert_eq!((code.as_str(), said.as_str()), (NOT_YOURS, "Your Kit holds no lifecycle `ticket` within `my-store`."));
    assert!(rig.fleet.vcs().created().is_empty());
}

/// **A piece scoped within a Workspace goes to the Workspace's home**, the one repository whose
/// `.armada/workspaces/` holds the file, and a refusal names it.
#[tokio::test]
async fn a_piece_within_a_workspace_is_committed_to_its_home_and_nowhere_else() {
    let rig = rig_placing(None);
    let other = another(&rig);
    plant(&Path::new(&rig.root).join(".armada/workspaces/my-store.yml"), &my_store(&rig));
    plant(&kit_file(&rig, "rules", "nudge.my-store.yml"), &NUDGE.replace("name: Nudge", "name: Nudge\nscope: my-store"));
    let piece = rule("nudge", within("my-store"));

    let (code, said) = refused_with(rig.fleet.committed_to_repository(on(&rig, &other, piece.clone())).await.expect_err("refused"));
    assert_eq!(code, REFUSED);
    assert!(said.contains("scoped to the Workspace `my-store`") && said.contains(&format!("whose home is {}", rig.root)), "{said}");
    assert!(rig.fleet.vcs().created().is_empty(), "no Job's branch is cut");

    let committed = rig.fleet.committed_to_repository(on(&rig, &rig.root, piece)).await.expect("committed at home");
    assert_eq!(committed.paths, [".armada/rules/nudge.my-store.yml"]);
    finished(&rig, &committed).await;
}

#[tokio::test]
async fn a_piece_within_a_workspace_that_only_kit_holds_has_no_home_to_go_to() {
    let rig = rig_placing(None);
    plant(&kit_file(&rig, "workspaces", "solo.yml"), &my_store(&rig).replace("my-store", "solo"));
    plant(&kit_file(&rig, "rules", "nudge.solo.yml"), &NUDGE.replace("name: Nudge", "name: Nudge\nscope: solo"));
    let (code, said) = refused_with(rig.fleet.committed_to_repository(asking(&rig, rule("nudge", within("solo")), false)).await.expect_err("refused"));
    assert_eq!(code, REFUSED);
    assert!(said.contains("The Workspace `solo` is only in your Kit") && said.contains("Commit the Workspace"), "{said}");
    assert!(rig.fleet.vcs().created().is_empty());
}

/// A piece scoped within a repository's Manifest id is that repository's.
#[tokio::test]
async fn a_piece_within_a_repository_is_committed_to_that_repository() {
    let rig = rig_placing(None);
    let other = another(&rig);
    let mine = rig.manifest.as_str().to_string();
    plant(&kit_file(&rig, "rules", "nudge.mine.yml"), &NUDGE.replace("name: Nudge", &format!("name: Nudge\nscope: {mine}")));
    let piece = rule("nudge", within(&mine));

    let (code, said) = refused_with(rig.fleet.committed_to_repository(on(&rig, &other, piece.clone())).await.expect_err("refused"));
    assert_eq!(code, REFUSED);
    assert!(said.contains(&format!("scoped to the repository {}", rig.root)), "{said}");
    rig.fleet.committed_to_repository(on(&rig, &rig.root, piece)).await.expect("committed in its own repository");

    plant(&kit_file(&rig, "rules", "nudge.nowhere.yml"), &NUDGE.replace("name: Nudge", "name: Nudge\nscope: nowhere"));
    let (code, said) = refused_with(rig.fleet.committed_to_repository(asking(&rig, rule("nudge", within("nowhere")), false)).await.expect_err("refused"));
    assert_eq!(code, REFUSED);
    assert!(said.contains("neither a Workspace nor a repository Fleet serves"), "{said}");
}

/// Two repositories holding one Workspace leave both set aside, so a second is refused.
#[tokio::test]
async fn a_workspace_a_repository_already_holds_is_not_committed_to_a_second() {
    let rig = rig_placing(None);
    let other = another(&rig);
    plant(&Path::new(&other).join(".armada/workspaces/my-store.yml"), &my_store(&rig));
    plant(&kit_file(&rig, "workspaces", "my-store.yml"), &my_store(&rig));
    let piece = PieceToCommit::Workspace { id: "my-store".into() };
    let (code, said) = refused_with(rig.fleet.committed_to_repository(asking(&rig, piece, false)).await.expect_err("refused"));
    assert_eq!(code, REFUSED);
    assert!(said.contains(&format!("already committed in {other}")), "{said}");
    assert!(rig.fleet.vcs().created().is_empty());
}

/// **A removal is a deletion committed by the Job's branch**: the file is gone from its tree, the
/// commit names the path, and the checkout and Kit are as they were.
#[tokio::test]
async fn removing_a_piece_deletes_the_file_that_declares_it_on_the_jobs_branch() {
    let rig = rig_placing(None);
    let held = ".armada/rules/stray-name.yml";
    plant(&Path::new(&rig.root).join(held), NUDGE);
    // The first Job's slot is the first the pool leases: the branch is cut from the base, which holds the file.
    let slot = PathBuf::from(adapter_traits::slot_path(&rig.root, 1));
    plant(&slot.join(held), NUDGE);
    plant(&slot.join(".armada/rules/other.yml"), &NUDGE.replace("rule_id: nudge", "rule_id: other"));
    let made = rig.fleet.vcs().committed().len();

    let committed = rig.fleet.committed_to_repository(removing(&rig.root, rule("nudge", PieceScope::Everywhere))).await.expect("committed");
    finished(&rig, &committed).await;

    assert_eq!(committed.paths, [held]);
    let tree = tree_of(&rig, &committed).await;
    assert_eq!(tree, slot, "the Job holds the slot the file was planted in");
    assert!(!tree.join(held).exists(), "deleted in the Job's tree");
    assert!(tree.join(".armada/rules/other.yml").exists(), "and nothing else");
    let commits = rig.fleet.vcs().committed();
    assert_eq!((commits.len(), commits[made].message.as_str()), (made + 1, "Remove the rule nudge"));
    assert_eq!(commits[made].scope, CommitScope::Paths(vec![held.to_string()]));
    let session = rig.fleet.store().lock().await.session(committed.session_id.as_str()).expect("read").expect("kept");
    assert_eq!(session.title.as_deref(), Some("Remove the rule nudge"));
    assert!(Path::new(&rig.root).join(held).exists(), "the checkout is untouched");
}

/// Removal is for the four kinds, and for what the repository holds under that scope.
#[tokio::test]
async fn removing_what_is_not_held_or_an_older_kind_is_refused_and_makes_no_job() {
    let rig = rig_placing(None);
    plant(&Path::new(&rig.root).join(".armada/rules/nudge.yml"), NUDGE);
    for (piece, said) in [
        (rule("absent", PieceScope::Everywhere), "main holds no rule `absent` to remove."),
        (rule("nudge", within("my-store")), "main holds no rule `nudge` within `my-store` to remove."),
        (PieceToCommit::Workspace { id: "my-store".into() }, "main holds no Workspace `my-store` to remove."),
        (
            PieceToCommit::ReviewTool { id: "security".into() },
            "Removing a review tool from a repository is not built. Only a Workspace, lifecycle, rule or autonomy defaults can be removed.",
        ),
        (
            PieceToCommit::FlightControls { job: vec!["specs".into()], deck: Vec::new() },
            "Removing tiles from a repository is not built. Only a Workspace, lifecycle, rule or autonomy defaults can be removed.",
        ),
    ] {
        let (code, refused) = refused_with(rig.fleet.committed_to_repository(removing(&rig.root, piece)).await.expect_err("refused"));
        assert_eq!((code.as_str(), refused.as_str()), (REFUSED, said));
    }
    assert!(rig.fleet.vcs().created().is_empty(), "no Job's branch is cut");
}
