//! Kit's Remove and Undo for a skill, against a fixture's own home: only a skill a fresh inventory
//! names as the person's own moves, and only a place Fleet trashed comes back. Nothing here runs a
//! command or reaches the network.

use std::path::{Path, PathBuf};

use ipc::{
    InstallKitPiece, KitInventory, KitPieceKind, RemoveKitPiece, RestoreKitPiece, SetupItem, SetupKindRow,
    WhatWasRead,
};
use adapters::kit_pieces::{Choice, Found, Piece};
use testkit::FakeWorkProduct;

use crate::kit_pieces::{chosen, own_skill};
use crate::tests::tmp::TempDir;

/// A skill of the person's own, in the folder the harness keeps them in.
fn skill(home: &TempDir, name: &str) -> PathBuf {
    let folder = adapters::kit_pieces::skills_folder(home.path()).join(name);
    std::fs::create_dir_all(&folder).expect("the skill's folder");
    std::fs::write(folder.join("SKILL.md"), format!("---\nname: {name}\n---\n")).expect("its file");
    folder
}

fn skills(inventory: &KitInventory) -> Vec<String> {
    match inventory.kinds.iter().find(|row| row.kind == "skills").map(|row| &row.read) {
        Some(WhatWasRead::Read { items, .. }) => items.iter().map(|item| item.name.clone()).collect(),
        other => panic!("skills were {other:?}"),
    }
}

fn removing(name: &str) -> RemoveKitPiece {
    RemoveKitPiece { kind: KitPieceKind::Skill, name: name.to_string() }
}

#[tokio::test]
async fn a_skill_of_the_persons_own_moves_to_the_trash_and_undo_puts_it_back() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));
    let folder = skill(&home, "docs");

    let removed = fleet.kit_piece_removed(removing("docs")).await.expect("removed");
    let trashed = removed.trashed.expect("where it went");
    assert_eq!(PathBuf::from(&trashed), home.path().join(".Trash/docs"));
    assert!(!folder.exists());
    assert!(!skills(&removed.setup).contains(&String::from("docs")));

    let back = fleet.kit_piece_restored(RestoreKitPiece { trashed: trashed.clone() }).await.expect("put back");
    assert!(folder.join("SKILL.md").is_file());
    assert!(skills(&back).contains(&String::from("docs")));
    let again = fleet.kit_piece_restored(RestoreKitPiece { trashed }).await;
    assert!(again.is_err(), "a place put back once is not Fleet's to move again");
}

#[tokio::test]
async fn a_skill_the_inventory_does_not_name_as_the_persons_own_is_refused() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));
    let elsewhere = home.path().join("projects/app/lint");
    std::fs::create_dir_all(&elsewhere).expect("a folder that is no skill");

    for name in ["lint", "nothing-by-this-name", "../projects/app/lint"] {
        assert!(fleet.kit_piece_removed(removing(name)).await.is_err(), "{name}");
    }
    assert!(elsewhere.is_dir());
}

fn row(name: &str, source: &Path, plugin: Option<&str>) -> SetupItem {
    SetupItem {
        name: name.to_string(),
        says: None,
        source: source.display().to_string(),
        style: None,
        plugin: plugin.map(str::to_string),
        uninstall: None,
    }
}

fn holding(items: Vec<SetupItem>) -> KitInventory {
    KitInventory {
        harness: String::from("h"),
        home: String::from("/home/user"),
        present: true,
        kinds: vec![SetupKindRow { kind: String::from("skills"), read: WhatWasRead::Read { items, unreadable: Vec::new() } }],
    }
}

#[test]
fn a_plugins_own_skill_is_never_removed_alone() {
    let home = Path::new("/home/user");
    let skills = adapters::kit_pieces::skills_folder(home);
    let brought = holding(vec![row("impeccable", &skills.join("impeccable"), Some("impeccable"))]);
    assert!(own_skill(&brought, home, "impeccable").is_err());
    let mine = holding(vec![row("impeccable", &skills.join("impeccable"), None)]);
    assert_eq!(own_skill(&mine, home, "impeccable"), Ok(skills.join("impeccable")));
}

#[test]
fn a_row_outside_the_persons_skills_folder_is_not_theirs_to_remove() {
    let home = Path::new("/home/user");
    let skills = adapters::kit_pieces::skills_folder(home);
    let outside = holding(vec![row("lint", Path::new("/home/user/projects/app/lint"), None)]);
    assert!(own_skill(&outside, home, "lint").is_err());
    let twice = holding(vec![row("docs", &skills.join("a/docs"), None), row("docs", &skills.join("b/docs"), None)]);
    assert!(own_skill(&twice, home, "docs").unwrap_err().contains("2 of your skills"));
}

#[tokio::test]
async fn undo_refuses_a_place_fleet_did_not_trash() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));
    let loose = home.path().join(".Trash/loose");
    std::fs::create_dir_all(&loose).expect("something in the Trash");

    let put_back = fleet.kit_piece_restored(RestoreKitPiece { trashed: loose.display().to_string() }).await;
    assert!(put_back.is_err());
    assert!(loose.is_dir());
}

fn a_skill(name: &str) -> Piece {
    Piece::Skill { name: name.to_string(), repository: String::from("someone/tools"), commands: Vec::new() }
}

fn two_skills() -> Found {
    Found::Several {
        repository: String::from("someone/tools"),
        choices: ["docs", "lint"].iter().map(|name| Choice { says: None, piece: a_skill(name) }).collect(),
    }
}

fn installing(kind: KitPieceKind, choice: Option<&str>) -> InstallKitPiece {
    InstallKitPiece { source: String::from("someone/tools"), kind, choice: choice.map(str::to_string) }
}

#[test]
fn an_install_of_one_of_several_runs_only_the_choice_a_fresh_look_found() {
    assert_eq!(chosen(two_skills(), &installing(KitPieceKind::Skill, Some("lint"))), Ok(a_skill("lint")));
}

#[test]
fn an_install_of_several_with_no_choice_a_gone_choice_or_another_kind_is_refused() {
    for (kind, choice, says) in [
        (KitPieceKind::Skill, None, "which one to install is not said"),
        (KitPieceKind::Skill, Some("format"), "no longer holds a skill named format"),
        (KitPieceKind::Plugin, Some("lint"), "no longer a plugin"),
    ] {
        let why = chosen(two_skills(), &installing(kind, choice)).unwrap_err();
        assert!(why.starts_with("Nothing was installed.") && why.contains(says), "{why}");
    }
}

#[test]
fn a_choice_named_against_a_source_that_is_now_one_piece_must_be_that_piece() {
    let one = || Found::Piece(a_skill("docs"));
    assert_eq!(chosen(one(), &installing(KitPieceKind::Skill, Some("docs"))), Ok(a_skill("docs")));
    assert_eq!(chosen(one(), &installing(KitPieceKind::Skill, None)), Ok(a_skill("docs")));
    let why = chosen(one(), &installing(KitPieceKind::Skill, Some("lint"))).unwrap_err();
    assert!(why.contains("no longer holds a skill named lint"), "{why}");
}
