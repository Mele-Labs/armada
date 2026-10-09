//! Promoting a mod: a branch of the repository holding it under
//! `packages/mods/`, made in a leased slot, with the owner's checkout exactly as
//! it was and nothing pushed. Real git, because what is under test is what git
//! holds afterwards.

use std::path::Path;

use adapters::GitVcs;
use api::{Mods, Refusal};
use ipc::{PromoteMod, ScaffoldMod};
use testkit::{FakeHarness, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::daemon::fitted_over;
use crate::tests::reclaim::{a_repository, branches, commit, git};
use crate::tests::tmp::TempDir;

fn a_fleet_over_a_repository(home: &TempDir, with_packages: bool) -> Fleet<FakeHarness, GitVcs, FakeWorkProduct> {
    a_repository(home);
    if with_packages {
        std::fs::create_dir_all(home.path().join("packages")).expect("packages/");
        std::fs::write(home.path().join("packages/README"), "packages\n").expect("a file");
        git(home.path(), &["add", "packages/README"]);
        commit(home.path(), "packages/");
    }
    Fleet::assembled(fitted_over(
        home,
        FakeWorkProduct::changed(&[]),
        FakeHarness::that_listens(),
        GitVcs::new(),
    ))
}

async fn scaffolded(fleet: &Fleet<FakeHarness, GitVcs, FakeWorkProduct>, name: &str) {
    fleet
        .scaffold_mod(ScaffoldMod { name: name.into(), description: Some("Softer".into()), kind: None })
        .await
        .expect("scaffolded");
}

fn promoting(name: &str) -> PromoteMod {
    PromoteMod { name: name.into(), manifest_id: None }
}

fn held(home: &Path, branch: &str, path: &str) -> String {
    git(home, &["show", &format!("{branch}:{path}")])
}

#[tokio::test]
async fn a_mod_is_copied_onto_a_new_branch_and_the_checkout_is_left_alone() {
    let home = TempDir::new();
    let fleet = a_fleet_over_a_repository(&home, true);
    scaffolded(&fleet, "calm").await;
    let before = branches(home.path());

    let promoted = fleet.promote_mod(promoting("calm")).await.expect("promoted");
    assert!(promoted.branch.starts_with("armada/mod-calm-"), "{}", promoted.branch);

    let tree = git(home.path(), &["ls-tree", "-r", "--name-only", &promoted.branch]);
    let mod_files: Vec<&str> = tree.lines().filter(|path| path.starts_with("packages/mods/")).collect();
    assert_eq!(mod_files, ["packages/mods/calm/mod.toml", "packages/mods/calm/theme.css"], "the two files and not the mod's .git");
    assert_eq!(
        held(home.path(), &promoted.branch, "packages/mods/calm/theme.css"),
        std::fs::read_to_string(home.path().join("mods/calm/theme.css")).expect("the source")
    );
    assert!(held(home.path(), &promoted.branch, "packages/mods/calm/mod.toml").contains("name = \"calm\""));
    assert_eq!(git(home.path(), &["rev-parse", &promoted.branch]).trim(), promoted.commit);

    assert_eq!(git(home.path(), &["rev-parse", "--abbrev-ref", "HEAD"]).trim(), "main", "the checkout is still on main");
    assert_eq!(git(home.path(), &["status", "--porcelain", "--untracked-files=no"]), "", "and has no change in it");
    assert!(!home.path().join("packages/mods").exists(), "nothing was written in the checkout");
    let after = branches(home.path());
    assert_eq!(after.len(), before.len() + 1, "one new branch: {after:?}");
    assert_eq!(git(home.path(), &["remote"]), "", "there is no remote to push to");

    let again = fleet.promote_mod(promoting("calm")).await.expect("the slot was given back, so a second promotes too");
    assert_ne!(again.branch, promoted.branch);
}

#[tokio::test]
async fn an_invalid_mod_or_a_repository_without_packages_is_refused_and_leaves_no_branch() {
    let home = TempDir::new();
    let fleet = a_fleet_over_a_repository(&home, true);
    scaffolded(&fleet, "calm").await;
    std::fs::write(home.path().join("mods/calm/theme.css"), ":root { --accent: url(x); }").expect("edit");
    let before = branches(home.path());

    let refused = fleet.promote_mod(promoting("calm")).await.expect_err("invalid");
    assert!(matches!(&refused, Refusal::IllegalMove(error) if error.code == "fleet.mod_not_promotable" && error.message.contains("`url(`")), "{refused:?}");
    let missing = fleet.promote_mod(promoting("nothing")).await.expect_err("no mod");
    assert_eq!(missing.error().code, "fleet.no_such_mod");
    let climbing = fleet.promote_mod(promoting("../calm")).await.expect_err("not a slug");
    assert_eq!(climbing.status(), 422);
    assert_eq!(branches(home.path()), before);

    let bare = TempDir::new();
    let fleet = a_fleet_over_a_repository(&bare, false);
    scaffolded(&fleet, "calm").await;
    let refused = fleet.promote_mod(promoting("calm")).await.expect_err("no packages/");
    assert!(matches!(&refused, Refusal::IllegalMove(error) if error.message.contains("packages/")), "{refused:?}");
    assert_eq!(branches(bare.path()), ["main"]);
}
