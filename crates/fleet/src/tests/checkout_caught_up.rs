//! The checkout Fleet serves is caught up whenever main moves, whoever merged.
//!
//! 15 Oct 2026: the owner's checkout sat days behind main,
//! because only a Job's own merge caught it up and Sessions and the merge queue
//! land most pull requests now.

use adapter_traits::{HeldBack, RepositoryStanding};

use crate::tests::main_ci::{a_fleet_over, the_hub, Fixture, MANIFEST, ONE, TWO};
use crate::tests::tmp::TempDir;

fn root(fleet: &Fixture) -> String {
    fleet.repositories().served()[0].root().to_string()
}

async fn held_back(fleet: &Fixture) -> Option<HeldBack> {
    fleet.sweeping().lock().await.held_back.get(&root(fleet)).cloned()
}

fn dirty() -> HeldBack {
    HeldBack {
        base: String::from("main"),
        on: None,
        uncommitted: vec![String::from("armada.yml")],
        clashing: Vec::new(),
        behind: 3,
        ahead: 0,
    }
}

#[tokio::test]
async fn main_moved_by_a_merge_no_job_made_catches_the_checkout_up() {
    let home = TempDir::new();
    let fleet = a_fleet_over(&home, MANIFEST);
    let vcs = fleet.vcs();
    vcs.main_ci.head_is(Some(TWO));
    vcs.checkout_on(ONE);
    vcs.repository_standing(RepositoryStanding::MovedOn {
        base: String::from("main"),
        commits: 1,
        head: String::from(TWO),
    });

    fleet.turn().await.unwrap();
    assert_eq!(vcs.repository_caught_up_to(), ["main"]);
    assert_eq!(held_back(&fleet).await, None);

    fleet.turn().await.unwrap();
    assert_eq!(
        vcs.repository_caught_up_to(),
        ["main"],
        "the checkout has main's tip now, so nothing is pulled again"
    );
}

#[tokio::test]
async fn a_checkout_that_already_has_main_is_not_touched() {
    let home = TempDir::new();
    let fleet = a_fleet_over(&home, MANIFEST);
    let vcs = fleet.vcs();
    vcs.main_ci.head_is(Some(ONE));
    vcs.checkout_on(ONE);

    fleet.turn().await.unwrap();
    assert!(vcs.repository_caught_up_to().is_empty());
}

#[tokio::test]
async fn a_dirty_checkout_is_held_back_and_said_until_it_catches_up() {
    let home = TempDir::new();
    let fleet = a_fleet_over(&home, MANIFEST);
    let vcs = fleet.vcs();
    vcs.main_ci.head_is(Some(TWO));
    vcs.checkout_on(ONE);
    vcs.repository_standing(RepositoryStanding::LeftAlone {
        why: String::from("`main` is carrying 1 uncommitted change(s)"),
    });
    vcs.held_back_by(Some(dirty()));

    fleet.turn().await.unwrap();
    assert_eq!(held_back(&fleet).await, Some(dirty()));
    let card = the_hub(&fleet).await.checkout.expect("the hub says it");
    assert_eq!(card.uncommitted, ["armada.yml"]);
    assert_eq!(card.behind, 3);

    vcs.repository_standing(RepositoryStanding::MovedOn {
        base: String::from("main"),
        commits: 3,
        head: String::from(TWO),
    });
    fleet.turn().await.unwrap();
    assert_eq!(held_back(&fleet).await, None, "caught up, so nothing to say");
    assert!(fleet
        .merge_hubs()
        .await
        .iter()
        .all(|(_, hub)| hub.checkout.is_none()));
}

fn on_another_branch() -> HeldBack {
    HeldBack {
        base: String::from("main"),
        on: Some(String::from("fix/slot-lease")),
        uncommitted: Vec::new(),
        clashing: Vec::new(),
        behind: 40,
        ahead: 0,
    }
}

#[tokio::test]
async fn on_another_branch_the_base_is_fast_forwarded_and_nobody_is_called() {
    let home = TempDir::new();
    let fleet = a_fleet_over(&home, MANIFEST);
    let vcs = fleet.vcs();
    vcs.main_ci.head_is(Some(TWO));
    vcs.checkout_on(ONE);
    vcs.repository_standing(RepositoryStanding::LeftAlone {
        why: String::from("the repository is on `fix/slot-lease` and not `main`"),
    });
    vcs.held_back_by(Some(on_another_branch()));

    fleet.turn().await.unwrap();
    assert_eq!(held_back(&fleet).await, None);
}

#[tokio::test]
async fn a_base_that_will_not_fast_forward_under_another_branch_is_said() {
    let home = TempDir::new();
    let fleet = a_fleet_over(&home, MANIFEST);
    let vcs = fleet.vcs();
    vcs.main_ci.head_is(Some(TWO));
    vcs.checkout_on(ONE);
    vcs.repository_standing(RepositoryStanding::LeftAlone {
        why: String::from("the repository is on `fix/slot-lease` and not `main`"),
    });
    vcs.held_back_by(Some(on_another_branch()));
    vcs.base_fast_forwards(false);

    fleet.turn().await.unwrap();
    assert_eq!(held_back(&fleet).await, Some(on_another_branch()));
}
