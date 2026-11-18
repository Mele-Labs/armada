//! The pull requests waiting on a person, as five lists through a real Fleet and a scripted
//! forge. **No forge is called**: `FakeVcs` answers each list as it was told to and records what
//! it was asked, which is how a test says one call a list named every repository.

use adapter_traits::{ListedChecks, ListedPull, ReviewList, ReviewStanding};
use api::PullRequestPanel;
use ipc::{PullReviewChecks, PullReviewList, PullRequestReviewState, MOST_PER_LIST};

use super::pull_requesting::{a_fleet, code, manifest, Fixture};
use crate::tests::tmp::TempDir;

/// The slug the fixture's repository is known by on the forge.
pub(super) const SLUG: &str = "mele-labs/armada";

pub(super) fn pull(number: u64, repository: &str) -> ListedPull {
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
        my_latest: None,
        asked_at: None,
    }
}

pub(super) fn signed_in_as(fleet: &Fixture, login: &str) {
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
    let lists = fleet.get_review_lists(None).await.expect("read");
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
    let lists = fleet.get_review_lists(None).await.expect("read");
    assert!(lists.silent.is_empty(), "lists read one after another: {:?}", lists.silent);
    assert_eq!([&lists.requested, &lists.team, &lists.assigned, &lists.mine].map(Vec::len), [1; 4]);
}

#[tokio::test]
async fn one_call_a_list_names_the_login_and_every_served_repository() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    signed_in_as(&fleet, "nick");
    fleet.vcs().now_review_list(ReviewList::Mine, Vec::new());
    fleet.get_review_lists(None).await.expect("read");
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
    let lists = fleet.get_review_lists(None).await.expect("read");
    assert_eq!(lists.silent, [PullReviewList::Team, PullReviewList::Assigned]);
    assert_eq!((lists.requested.len(), lists.mine.len()), (1, 1));
    assert!(lists.team.is_empty() && lists.assigned.is_empty());
}

#[tokio::test]
async fn four_silent_lists_are_a_fault() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    signed_in_as(&fleet, "nick");
    let refused = fleet.get_review_lists(None).await.unwrap_err();
    assert_eq!(refused.status(), 500);
    assert_eq!(code(&refused), "fleet.review_lists_unreadable");
}

#[tokio::test]
async fn nobody_known_to_ask_as_is_a_fault_and_the_forge_is_not_asked() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    fleet.vcs().now_review_list(ReviewList::Mine, vec![pull(2, SLUG)]);
    let refused = fleet.get_review_lists(None).await.unwrap_err();
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
    let lists = fleet.get_review_lists(None).await.expect("read");
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
    let lists = fleet.get_review_lists(None).await.expect("read");
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
    let lists = fleet.get_review_lists(None).await.expect("read");
    assert_eq!(lists.mine.len(), MOST_PER_LIST);
    assert_eq!(lists.mine[0].number, 1, "the forge's order is kept");
}

#[tokio::test]
async fn the_forges_time_he_was_asked_rides_the_row() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    signed_in_as(&fleet, "mira");
    let mut asked = pull(41, SLUG);
    asked.asked_at = Some("2026-11-14T09:00:00Z".into());
    fleet.vcs().now_review_list(ReviewList::Requested, vec![asked]);
    for list in [ReviewList::Team, ReviewList::Assigned, ReviewList::Mine] {
        fleet.vcs().now_review_list(list, Vec::new());
    }
    let lists = fleet.get_review_lists(None).await.expect("read");
    assert_eq!(lists.requested[0].asked_at.as_deref(), Some("2026-11-14T09:00:00Z"));
}

#[tokio::test]
async fn where_the_forge_gave_no_time_a_request_by_name_is_dated_from_when_fleet_told_him() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    signed_in_as(&fleet, "mira");
    let mut dated = pull(13, SLUG);
    dated.asked_at = Some("2026-11-01T09:00:00Z".into());
    fleet.vcs().now_review_list(ReviewList::Requested, vec![pull(12, SLUG), dated]);
    fleet.vcs().now_review_list(ReviewList::Team, vec![pull(14, SLUG)]);
    for list in [ReviewList::Assigned, ReviewList::Mine] {
        fleet.vcs().now_review_list(list, Vec::new());
    }
    fleet.review_requests_looked().await;
    let told = {
        let store = fleet.store().lock().await;
        let kept = store.review_requests_told().expect("read");
        kept.into_iter().find(|it| it.number == 12).expect("told of #12").told_at
    };

    let lists = fleet.get_review_lists(None).await.expect("read");
    let asked: Vec<(u64, Option<&str>)> = lists.requested.iter().map(|row| (row.number, row.asked_at.as_deref())).collect();
    assert_eq!(asked, [(12, Some(told.as_str())), (13, Some("2026-11-01T09:00:00Z"))], "the forge's own time is kept");
    assert_eq!(lists.team[0].asked_at, None, "Fleet tells only of a request by name");
}

/// A finished run of `kind` on pull request `number`, started at `started`.
async fn ran(fleet: &Fixture, id: &ipc::ManifestId, number: u64, kind: &str, started: &str) {
    let run = store::KeptReviewRun {
        manifest_id: id.as_str().into(),
        number,
        head: "3f9c1d2e3f9c1d2e3f9c1d2e3f9c1d2e3f9c1d2e".into(),
        kind: kind.into(),
        name: "risks".into(),
        label: "risks".into(),
        state: "done".into(),
        started_at: started.into(),
        finished_at: Some(started.into()),
        failed: None,
        result: None,
        cost_micros: None,
        slot: None,
        holder: None,
    };
    fleet.store().lock().await.keep_review_run(&run).expect("kept");
}

#[tokio::test]
async fn a_row_he_ran_a_review_tool_on_says_so_and_one_he_only_asked_about_does_not() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    signed_in_as(&fleet, "nick");
    ran(&fleet, &id, 7, "tool", "2026-11-16T09:00:00Z").await;
    ran(&fleet, &id, 8, "ask", "2026-11-16T10:00:00Z").await;
    fleet.vcs().now_review_list(ReviewList::Requested, vec![pull(7, SLUG), pull(8, SLUG)]);
    fleet.vcs().now_review_list(ReviewList::Mine, vec![pull(7, SLUG)]);
    for list in [ReviewList::Team, ReviewList::Assigned] {
        fleet.vcs().now_review_list(list, Vec::new());
    }
    let lists = fleet.get_review_lists(None).await.expect("read");
    let flagged: Vec<(u64, bool)> = lists.requested.iter().map(|row| (row.number, row.reviewing)).collect();
    assert_eq!(flagged, [(7, true), (8, false)]);
    assert!(lists.mine[0].reviewing, "on a row of every list");
    assert!(lists.reviewing.is_empty(), "a pull request a list names is not listed again: {:?}", lists.reviewing);
}

#[tokio::test]
async fn an_open_pull_request_he_ran_a_tool_on_that_no_list_names_is_listed_and_a_closed_one_is_not() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    signed_in_as(&fleet, "nick");
    ran(&fleet, &id, 7, "tool", "2026-11-16T12:00:00Z").await;
    ran(&fleet, &id, 20, "tool", "2026-11-16T09:00:00Z").await;
    ran(&fleet, &id, 21, "overview", "2026-11-16T11:00:00Z").await;
    ran(&fleet, &id, 22, "change_sets", "2026-11-16T10:00:00Z").await;
    ran(&fleet, &id, 23, "fix", "2026-11-16T13:00:00Z").await;
    fleet.vcs().now_review_list(ReviewList::Requested, vec![pull(7, SLUG)]);
    for list in [ReviewList::Team, ReviewList::Assigned, ReviewList::Mine] {
        fleet.vcs().now_review_list(list, Vec::new());
    }
    // #22 has closed, so the forge leaves it out of a read by number.
    fleet.vcs().now_open_by_number(vec![pull(20, SLUG), pull(7, SLUG), pull(21, SLUG), pull(23, SLUG)]);
    let lists = fleet.get_review_lists(None).await.expect("read");
    let listed: Vec<(u64, bool)> = lists.reviewing.iter().map(|row| (row.number, row.reviewing)).collect();
    assert_eq!(listed, [(21, true), (20, true)], "newest run first, and a fix is not a review tool");
    assert_eq!(lists.reviewing[0].manifest_id, id);
    assert!(lists.silent.is_empty());
    let asked: Vec<Vec<(String, u64)>> = fleet
        .vcs()
        .numbered_reads_asked()
        .into_iter()
        .map(|read| read.into_iter().map(|it| (it.repository, it.number)).collect())
        .collect();
    let slug = SLUG.to_string();
    assert_eq!(asked, [vec![(slug.clone(), 7), (slug.clone(), 21), (slug.clone(), 22), (slug, 20)]], "one call");
}

#[tokio::test]
async fn a_read_by_number_the_forge_would_not_answer_leaves_the_fifth_list_empty_and_named() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    signed_in_as(&fleet, "nick");
    ran(&fleet, &id, 20, "tool", "2026-11-16T09:00:00Z").await;
    fleet.vcs().now_review_list(ReviewList::Requested, vec![pull(7, SLUG)]);
    for list in [ReviewList::Team, ReviewList::Assigned, ReviewList::Mine] {
        fleet.vcs().now_review_list(list, Vec::new());
    }
    let lists = fleet.get_review_lists(None).await.expect("read");
    assert_eq!(lists.silent, [PullReviewList::Reviewing]);
    assert!(lists.reviewing.is_empty());
    assert_eq!(lists.requested.len(), 1, "the four stand");
}
