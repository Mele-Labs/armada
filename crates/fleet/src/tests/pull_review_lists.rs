//! The pull requests waiting on a person, as four lists through a real Fleet and a scripted
//! forge. **No forge is called**: `FakeVcs` answers each list as it was told to and records what
//! it was asked, which is how a test says one call a list named every repository.

use adapter_traits::{ListedChecks, ListedPull, ReviewList, ReviewStanding};
use api::PullRequestPanel;
use ipc::{PullReviewChecks, PullReviewList, PullRequestReviewState, MOST_PER_LIST};

use super::pull_requesting::{a_fleet, code, manifest, Fixture};
use crate::tests::tmp::TempDir;

/// The slug the fixture's repository is known by on the forge.
const SLUG: &str = "mele-labs/armada";

fn pull(number: u64, repository: &str) -> ListedPull {
    ListedPull {
        repository: repository.into(),
        number,
        title: format!("Pull request {number}"),
        branch: format!("branch-{number}"),
        address: format!("https://forge.invalid/{repository}/pull/{number}"),
        draft: false,
        head: "3f9c1d2e3f9c1d2e3f9c1d2e3f9c1d2e3f9c1d2e".into(),
        author: "ada".into(),
        updated_at: "2026-10-15T09:30:00Z".into(),
        checks: ListedChecks::Running,
        review: ReviewStanding::None,
        team: None,
    }
}

fn signed_in_as(fleet: &Fixture, login: &str) {
    fleet.vcs().main_ci.login_is(Some(login));
}

#[tokio::test]
async fn each_list_is_the_forges_rows_in_the_wires_words() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    signed_in_as(&fleet, "NickMele");
    let mut failing = pull(7, SLUG);
    failing.checks = ListedChecks::Failed;
    failing.review = ReviewStanding::ChangesRequested;
    failing.draft = true;
    fleet.vcs().now_review_list(ReviewList::Requested, vec![failing, pull(8, SLUG)]);
    fleet.vcs().now_review_list(ReviewList::Team, Vec::new());
    fleet.vcs().now_review_list(ReviewList::Assigned, vec![pull(9, SLUG)]);
    fleet.vcs().now_review_list(ReviewList::Mine, vec![pull(7, SLUG)]);
    let lists = fleet.get_review_lists().await.expect("read");
    assert_eq!(lists.me, "NickMele", "the login as the forge wrote it");
    assert!(lists.silent.is_empty() && !lists.fetched_at.is_empty());
    let numbers = |rows: &[ipc::PullReviewRow]| rows.iter().map(|it| it.number).collect::<Vec<_>>();
    assert_eq!(numbers(&lists.requested), [7, 8]);
    assert!(lists.team.is_empty());
    assert_eq!(numbers(&lists.assigned), [9]);
    assert_eq!(numbers(&lists.mine), [7], "a pull request on two lists is a row on each");
    let row = &lists.requested[0];
    assert_eq!(row.manifest_id, id);
    assert_eq!(row.repository, SLUG);
    assert_eq!((row.checks, row.review, row.draft), (PullReviewChecks::Failed, PullRequestReviewState::ChangesRequested, true));
    assert_eq!(row.head.len(), 40);
}

#[tokio::test]
async fn the_four_lists_are_read_at_once() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    signed_in_as(&fleet, "NickMele");
    for list in [ReviewList::Requested, ReviewList::Team, ReviewList::Assigned, ReviewList::Mine] {
        fleet.vcs().now_review_list(list, vec![pull(7, SLUG)]);
    }
    // Each read answers only while the other three are being read.
    fleet.vcs().review_lists_answer_together(4);
    let lists = fleet.get_review_lists().await.expect("read");
    assert!(lists.silent.is_empty(), "lists read one after another: {:?}", lists.silent);
    assert_eq!([&lists.requested, &lists.team, &lists.assigned, &lists.mine].map(Vec::len), [1; 4]);
}

#[tokio::test]
async fn one_call_a_list_names_the_login_and_every_served_repository() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    signed_in_as(&fleet, "nick");
    fleet.vcs().now_review_list(ReviewList::Mine, Vec::new());
    fleet.get_review_lists().await.expect("read");
    let asked = fleet.vcs().review_lists_asked();
    let lists: Vec<ReviewList> = asked.iter().map(|(list, ..)| *list).collect();
    assert_eq!(lists.len(), 4, "{lists:?}");
    for list in [ReviewList::Requested, ReviewList::Team, ReviewList::Assigned, ReviewList::Mine] {
        assert!(lists.contains(&list), "{list:?} was not asked: {lists:?}");
    }
    assert!(asked.iter().all(|(_, login, repositories)| login == "nick" && repositories == &[SLUG.to_string()]));
}

#[tokio::test]
async fn a_list_the_forge_would_not_answer_is_empty_and_named_while_the_others_stand() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    signed_in_as(&fleet, "nick");
    fleet.vcs().now_review_list(ReviewList::Requested, vec![pull(1, SLUG)]);
    fleet.vcs().now_review_list(ReviewList::Mine, vec![pull(2, SLUG)]);
    let lists = fleet.get_review_lists().await.expect("read");
    assert_eq!(lists.silent, [PullReviewList::Team, PullReviewList::Assigned]);
    assert_eq!((lists.requested.len(), lists.mine.len()), (1, 1));
    assert!(lists.team.is_empty() && lists.assigned.is_empty());
}

#[tokio::test]
async fn four_silent_lists_are_a_fault() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    signed_in_as(&fleet, "nick");
    let refused = fleet.get_review_lists().await.unwrap_err();
    assert_eq!(refused.status(), 500);
    assert_eq!(code(&refused), "fleet.review_lists_unreadable");
}

#[tokio::test]
async fn nobody_known_to_ask_as_is_a_fault_and_the_forge_is_not_asked() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    fleet.vcs().now_review_list(ReviewList::Mine, vec![pull(2, SLUG)]);
    let refused = fleet.get_review_lists().await.unwrap_err();
    assert_eq!(refused.status(), 500);
    assert_eq!(code(&refused), "fleet.review_lists_unreadable");
    assert!(fleet.vcs().review_lists_asked().is_empty());
}

#[tokio::test]
async fn a_row_of_a_repository_fleet_does_not_serve_is_dropped() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    signed_in_as(&fleet, "nick");
    fleet.vcs().now_review_list(
        ReviewList::Requested,
        vec![pull(1, "stranger/thing"), pull(2, "Mele-Labs/Armada")],
    );
    for list in [ReviewList::Team, ReviewList::Assigned, ReviewList::Mine] {
        fleet.vcs().now_review_list(list, Vec::new());
    }
    let lists = fleet.get_review_lists().await.expect("read");
    assert_eq!(lists.requested.len(), 1);
    assert_eq!(lists.requested[0].number, 2, "the forge's case is not the slug's");
}

#[tokio::test]
async fn the_team_list_carries_the_team_a_row_was_asked_of() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    signed_in_as(&fleet, "nick");
    let mut asked = pull(5, SLUG);
    asked.team = Some("Platform".into());
    fleet.vcs().now_review_list(ReviewList::Team, vec![asked]);
    for list in [ReviewList::Requested, ReviewList::Assigned, ReviewList::Mine] {
        fleet.vcs().now_review_list(list, Vec::new());
    }
    let lists = fleet.get_review_lists().await.expect("read");
    assert_eq!(lists.team[0].team.as_deref(), Some("Platform"));
}

#[tokio::test]
async fn a_list_keeps_the_most_the_wire_allows() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    signed_in_as(&fleet, "nick");
    let many: Vec<ListedPull> = (1..=MOST_PER_LIST as u64 + 10).map(|n| pull(n, SLUG)).collect();
    fleet.vcs().now_review_list(ReviewList::Mine, many);
    for list in [ReviewList::Requested, ReviewList::Team, ReviewList::Assigned] {
        fleet.vcs().now_review_list(list, Vec::new());
    }
    let lists = fleet.get_review_lists().await.expect("read");
    assert_eq!(lists.mine.len(), MOST_PER_LIST);
    assert_eq!(lists.mine[0].number, 1, "the forge's order is kept");
}
