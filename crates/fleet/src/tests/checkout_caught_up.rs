//! The checkout Fleet serves is caught up whenever main moves, whoever merged.
//!
//! 15 Oct 2026: the owner's checkout sat days behind main,
//! because only a Job's own merge caught it up and Sessions and the merge queue
//! land most pull requests now.

use adapter_traits::{CheckoutPulled, HeldBack, RepositoryStanding};
use api::{Commands, Queries, Refusal};
use ipc::{CatchUpCheckout, CheckoutDiffAsk, ManifestId};

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

// ---------------------------------------------------------- the one press the card offers

fn ask() -> CatchUpCheckout {
    CatchUpCheckout { manifest_id: ManifestId::carried("01FIXTUREMANIFEST") }
}

/// A fleet whose checkout is behind main and held by `held`, read once so the hub says so.
async fn held_by(home: &TempDir, held: HeldBack) -> Fixture {
    let fleet = a_fleet_over(home, MANIFEST);
    let vcs = fleet.vcs();
    vcs.main_ci.head_is(Some(TWO));
    vcs.checkout_on(ONE);
    vcs.repository_standing(RepositoryStanding::LeftAlone {
        why: String::from("`main` is carrying 1 uncommitted change(s)"),
    });
    vcs.held_back_by(Some(held));
    // Under another branch Fleet fast-forwards the local base itself where it can; these holds are the ones it could not.
    vcs.base_fast_forwards(false);
    fleet.turn().await.unwrap();
    assert!(held_back(&fleet).await.is_some(), "the hold was read");
    fleet
}

fn refused_with(answer: Result<ipc::CheckoutCaughtUp, Refusal>) -> (String, String) {
    match answer {
        Err(Refusal::IllegalMove(why)) => (why.code, why.message),
        other => panic!("expected a 409, got {other:?}"),
    }
}

#[tokio::test]
async fn a_safe_hold_runs_the_pull_and_the_hub_stops_saying_it() {
    let home = TempDir::new();
    let fleet = held_by(&home, dirty()).await;
    let vcs = fleet.vcs();
    vcs.pulling_comes_to(CheckoutPulled { code: 0, output: String::from("Fast-forward") });

    let pulled = fleet.catch_up_checkout(ask()).await.expect("the pull ran");

    assert_eq!((pulled.code, pulled.output.as_str()), (0, "Fast-forward"));
    assert_eq!(vcs.times_the_checkout_was_pulled(), 1);
    assert_eq!(held_back(&fleet).await, None, "the checkout has main's tip, so Fleet stops holding it");
    assert!(fleet.merge_hubs().await.iter().all(|(_, hub)| hub.checkout.is_none()));
}

/// A stash git could not put back is `code` 1 and git's words; the hold is read again, not assumed.
#[tokio::test]
async fn a_pull_that_did_not_take_is_an_answer_with_git_words_and_the_hold_is_read_again() {
    let home = TempDir::new();
    let fleet = held_by(&home, dirty()).await;
    let vcs = fleet.vcs();
    vcs.pulling_comes_to(CheckoutPulled { code: 1, output: String::from("Your local changes are stashed, however applying them resulted in conflicts.") });

    let pulled = fleet.catch_up_checkout(ask()).await.expect("an answer, not a refusal");

    assert_eq!(pulled.code, 1);
    assert!(pulled.output.contains("stashed"));
    assert_eq!(vcs.times_the_checkout_was_pulled(), 1);
}

#[tokio::test]
async fn a_hold_with_a_file_main_also_changed_is_refused_and_nothing_runs() {
    let home = TempDir::new();
    let fleet = held_by(&home, HeldBack { clashing: vec![String::from("armada.yml")], ..dirty() }).await;

    let (code, said) = refused_with(fleet.catch_up_checkout(ask()).await);

    assert_eq!(code, "fleet.checkout_not_catchable");
    assert!(said.contains("armada.yml"), "{said}");
    assert_eq!(fleet.vcs().times_the_checkout_was_pulled(), 0);
    assert!(held_back(&fleet).await.is_some(), "the hold stands");
}

#[tokio::test]
async fn a_hold_on_another_branch_is_refused_and_nothing_runs() {
    let home = TempDir::new();
    let fleet = held_by(&home, HeldBack { on: Some(String::from("my-own-branch")), ..dirty() }).await;

    let (code, said) = refused_with(fleet.catch_up_checkout(ask()).await);

    assert_eq!(code, "fleet.checkout_not_catchable");
    assert!(said.contains("my-own-branch"), "{said}");
    assert_eq!(fleet.vcs().times_the_checkout_was_pulled(), 0);
}

#[tokio::test]
async fn a_hold_with_commits_main_lacks_is_refused_and_nothing_runs() {
    let home = TempDir::new();
    let fleet = held_by(&home, HeldBack { ahead: 2, ..dirty() }).await;

    let (code, _) = refused_with(fleet.catch_up_checkout(ask()).await);

    assert_eq!(code, "fleet.checkout_not_catchable");
    assert_eq!(fleet.vcs().times_the_checkout_was_pulled(), 0);
}

/// The card was drawn, then the owner committed their file or Fleet caught up: the hold is read now, not as drawn.
#[tokio::test]
async fn a_hold_that_has_gone_since_the_card_was_drawn_is_refused_as_not_held() {
    let home = TempDir::new();
    let fleet = held_by(&home, dirty()).await;
    fleet.vcs().held_back_by(None);

    let (code, _) = refused_with(fleet.catch_up_checkout(ask()).await);

    assert_eq!(code, "fleet.checkout_not_held");
    assert_eq!(fleet.vcs().times_the_checkout_was_pulled(), 0);
}

#[tokio::test]
async fn a_checkout_with_nothing_uncommitted_is_left_to_fleets_own_catch_up() {
    let home = TempDir::new();
    let fleet = held_by(&home, HeldBack { uncommitted: Vec::new(), ..dirty() }).await;

    let (code, _) = refused_with(fleet.catch_up_checkout(ask()).await);

    assert_eq!(code, "fleet.checkout_not_catchable");
    assert_eq!(fleet.vcs().times_the_checkout_was_pulled(), 0);
}

#[tokio::test]
async fn a_repository_this_fleet_does_not_serve_is_refused() {
    let home = TempDir::new();
    let fleet = held_by(&home, dirty()).await;

    let answer = fleet
        .catch_up_checkout(CatchUpCheckout { manifest_id: ManifestId::carried("01NOSUCHMANIFEST") })
        .await;

    match answer {
        Err(Refusal::Unacceptable(why)) => assert_eq!(why.code, "fleet.repository_not_served"),
        other => panic!("{other:?}"),
    }
    assert_eq!(fleet.vcs().times_the_checkout_was_pulled(), 0);
}

// ------------------------------------------------------- what an uncommitted file holds

fn asking_for(path: &str) -> CheckoutDiffAsk {
    CheckoutDiffAsk { manifest_id: ManifestId::carried("01FIXTUREMANIFEST"), path: path.to_string() }
}

#[tokio::test]
async fn a_path_the_hold_lists_answers_with_its_diff() {
    let home = TempDir::new();
    let fleet = held_by(&home, dirty()).await;
    fleet.vcs().uncommitted_change_is(Ok(String::from("diff --git a/armada.yml b/armada.yml\n+auto_merge: checks-pass\n")));

    let read = fleet.checkout_diff(asking_for("armada.yml")).await.expect("a listed path is read");

    assert!(read.diff.contains("+auto_merge: checks-pass"));
    assert_eq!(fleet.vcs().uncommitted_changes_read(), ["armada.yml"]);
}

#[tokio::test]
async fn a_change_past_the_line_bound_is_answered_cut_to_whole_lines() {
    let home = TempDir::new();
    let fleet = held_by(&home, dirty()).await;
    let long: String = (0..2500).map(|n| format!("+line {n}\n")).collect();
    fleet.vcs().uncommitted_change_is(Ok(long));

    let read = fleet.checkout_diff(asking_for("armada.yml")).await.expect("read");

    assert!(read.cut);
    assert_eq!(read.diff.lines().count(), 2000);
    assert!(read.diff.ends_with("+line 1999\n"));
}

/// A generated file is a few lines of megabytes each: the line bound alone would send them all.
#[tokio::test]
async fn a_change_past_the_byte_bound_is_answered_cut_without_splitting_a_character() {
    let home = TempDir::new();
    let fleet = held_by(&home, dirty()).await;
    let kept = String::from("+short\n");
    fleet.vcs().uncommitted_change_is(Ok(format!("{kept}+{}\n", "\u{00e9}".repeat(200_000))));

    let read = fleet.checkout_diff(asking_for("armada.yml")).await.expect("read");

    assert!(read.cut);
    assert_eq!(read.diff, kept, "the one line past the bound is left off whole");

    fleet.vcs().uncommitted_change_is(Ok(format!("+{}", "\u{00e9}".repeat(200_000))));
    let read = fleet.checkout_diff(asking_for("armada.yml")).await.expect("read");
    assert!(read.cut);
    assert!(read.diff.len() <= 256 * 1024 && read.diff.starts_with("+\u{00e9}"));
}

#[tokio::test]
async fn a_change_at_the_bounds_is_answered_whole() {
    let home = TempDir::new();
    let fleet = held_by(&home, dirty()).await;
    let exact: String = (0..2000).map(|n| format!("+line {n}\n")).collect();
    fleet.vcs().uncommitted_change_is(Ok(exact.clone()));

    let read = fleet.checkout_diff(asking_for("armada.yml")).await.expect("read");

    assert!(!read.cut);
    assert_eq!(read.diff, exact);
}

/// Nothing outside the list is read, whatever the path says.
#[tokio::test]
async fn a_path_the_hold_does_not_list_is_refused_and_git_is_not_asked() {
    let home = TempDir::new();
    let fleet = held_by(&home, dirty()).await;

    for path in ["src/other.rs", "../outside", "/etc/passwd", ""] {
        match fleet.checkout_diff(asking_for(path)).await {
            Err(Refusal::IllegalMove(why)) => assert_eq!(why.code, "fleet.checkout_path_not_held", "{path:?}"),
            other => panic!("{path:?}: {other:?}"),
        }
    }
    assert!(fleet.vcs().uncommitted_changes_read().is_empty());
}

#[tokio::test]
async fn nothing_is_read_where_no_hold_is_kept() {
    let home = TempDir::new();
    let fleet = a_fleet_over(&home, MANIFEST);

    match fleet.checkout_diff(asking_for("armada.yml")).await {
        Err(Refusal::IllegalMove(why)) => assert_eq!(why.code, "fleet.checkout_not_held"),
        other => panic!("{other:?}"),
    }
    assert!(fleet.vcs().uncommitted_changes_read().is_empty());
}

#[tokio::test]
async fn a_diff_git_would_not_print_is_a_fault_with_git_words() {
    let home = TempDir::new();
    let fleet = held_by(&home, dirty()).await;
    fleet.vcs().uncommitted_change_is(Err(String::from("fatal: bad revision 'HEAD'")));

    match fleet.checkout_diff(asking_for("armada.yml")).await {
        Err(Refusal::Fault(why)) => {
            assert_eq!(why.code, "fleet.checkout_diff_unread");
            assert!(why.message.contains("bad revision"));
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn a_diff_of_a_repository_this_fleet_does_not_serve_is_refused() {
    let home = TempDir::new();
    let fleet = held_by(&home, dirty()).await;
    let ask = CheckoutDiffAsk { manifest_id: ManifestId::carried("01NOSUCHMANIFEST"), path: String::from("armada.yml") };

    match fleet.checkout_diff(ask).await {
        Err(Refusal::Unacceptable(why)) => assert_eq!(why.code, "fleet.repository_not_served"),
        other => panic!("{other:?}"),
    }
}
