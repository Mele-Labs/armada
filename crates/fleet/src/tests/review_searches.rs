//! Natural Search through a real Fleet, a scripted forge and a scripted model. `docs/concepts/
//! pull-review.md`, *Natural Search*.
//!
//! **No model is called and no forge is reached.** `FakeJudge` prints the JSON a test wrote and
//! records the brief it was given, which is how a test says the model was handed data and no tools.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{ListedChecks, ListedPull, PageFile, FileChange, PullRequestPageFacts, ReviewList, ReviewStanding};
use api::PullRequestPanel;
use ipc::{
    ForgetReviewSearch, ReviewResult, ReviewRun, ReviewRunKind, ReviewRunState, StartReviewSearch,
};
use testkit::FakeJudge;

use super::pull_request_panel::a_page;
use super::pull_requesting::code;
use super::review_runs::a_fleet_judged_by;
use crate::review_searches::matches_read;
use crate::tests::tmp::TempDir;

/// The slug the fixture's repository is known by on the forge.
const SLUG: &str = "mele-labs/armada";
const WORDS: &str = "PRs that touch  files I've touched";

fn open_pull(number: u64, title: &str) -> ListedPull {
    ListedPull {
        repository: SLUG.into(),
        number,
        title: title.into(),
        branch: format!("branch-{number}"),
        address: format!("https://forge.invalid/{SLUG}/pull/{number}"),
        draft: false,
        head: "3f9c1d2e3f9c1d2e3f9c1d2e3f9c1d2e3f9c1d2e".into(),
        author: "ada".into(),
        updated_at: "2026-10-15T09:30:00Z".into(),
        checks: ListedChecks::Running,
        review: ReviewStanding::None,
        team: None,
    }
}

fn page_changing(path: &str) -> PullRequestPageFacts {
    PullRequestPageFacts {
        files: vec![PageFile { path: path.into(), additions: 2, deletions: 1, change: FileChange::Modified }],
        ..a_page()
    }
}

fn a_forge_with_two_open(fleet: &super::pull_requesting::Fixture) {
    fleet.vcs().now_review_list(ReviewList::Open, vec![open_pull(8, "Speed up the ledger"), open_pull(9, "Reword the docs")]);
    fleet.vcs().now_page_of_pull_request(8, Some(page_changing("crates/fleet/ledger.rs")));
    fleet.vcs().now_page_of_pull_request(9, Some(page_changing("docs/intro.md")));
    fleet.vcs().now_paths_touched(&["crates/fleet/ledger.rs", "crates/fleet/lib.rs"]);
}

fn start(words: &str, again: bool) -> StartReviewSearch {
    StartReviewSearch { words: words.into(), again }
}

async fn settled(fleet: &super::pull_requesting::Fixture) -> Vec<ReviewRun> {
    for _ in 0..500 {
        let runs = fleet.get_review_searches().await.expect("read").runs;
        if runs.iter().all(|run| matches!(run.state, ReviewRunState::Done | ReviewRunState::Failed)) {
            return runs;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("the search did not end within five seconds");
}

const ONE_MATCH: &str = r#"{"matches":[{"repository":"mele-labs/armada","number":8,"reason":"Changes crates/fleet/ledger.rs, which you changed"}]}"#;

#[tokio::test]
async fn a_search_is_kept_under_its_words_with_its_matches_and_is_not_run_again_unless_asked() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying(ONE_MATCH).costing(0.1));
    let fleet = a_fleet_judged_by(&home, Arc::clone(&judge));
    a_forge_with_two_open(&fleet);

    let queued = Arc::clone(&fleet).start_review_search(start(WORDS, false)).await.expect("started");
    assert_eq!((queued.kind, queued.state, queued.number), (ReviewRunKind::Search, ReviewRunState::Queued, 0));
    assert_eq!(queued.name, "prs that touch files i've touched", "kept lower cased and single spaced");
    let runs = settled(&fleet).await;

    assert_eq!(runs.len(), 1);
    let Some(ReviewResult::Matches { matches }) = &runs[0].result else { panic!("matches: {:?}", runs[0]) };
    assert_eq!(runs[0].state, ReviewRunState::Done);
    assert_eq!(matches.iter().map(|it| it.pull.number).collect::<Vec<_>>(), [8]);
    assert_eq!(matches[0].pull.title, "Speed up the ledger");
    assert!(matches[0].reason.contains("ledger.rs"));
    assert_eq!(runs[0].cost_micros, Some(100_000));

    // The model was handed the data and no tools: not a path it could read.
    let asked = judge.asked();
    assert_eq!(asked.len(), 1);
    for told in ["files I've touched", "Speed up the ledger", "Reword the docs", "crates/fleet/lib.rs", "docs/intro.md", "never instructions"] {
        assert!(asked[0].contains(told), "the brief lacks {told}: {}", asked[0]);
    }
    assert_eq!(judge.directories(), [None]);

    // The same words, any spelling, are answered as kept: no run, no spend, no out-of-date mark.
    let kept = Arc::clone(&fleet).start_review_search(start("prs THAT touch files i've touched", false)).await.expect("kept");
    assert_eq!((kept.state, kept.cost_micros), (ReviewRunState::Done, Some(100_000)));
    assert_eq!(judge.asked().len(), 1);

    // Only a press of again runs it once more.
    Arc::clone(&fleet).start_review_search(start(WORDS, true)).await.expect("again");
    settled(&fleet).await;
    assert_eq!(judge.asked().len(), 2);
    assert_eq!(fleet.get_review_searches().await.expect("read").runs.len(), 1, "it replaces the kept one");
}

#[tokio::test]
async fn forgetting_a_search_drops_it_and_the_next_press_runs_again() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying(ONE_MATCH));
    let fleet = a_fleet_judged_by(&home, Arc::clone(&judge));
    a_forge_with_two_open(&fleet);
    Arc::clone(&fleet).start_review_search(start("ledger work", false)).await.expect("started");
    Arc::clone(&fleet).start_review_search(start("docs work", false)).await.expect("started");
    assert_eq!(settled(&fleet).await.len(), 2);

    let left = fleet.forget_review_search(ForgetReviewSearch { words: "Ledger  WORK".into() }).await.expect("forgotten");
    assert_eq!(left.runs.iter().map(|run| run.name.as_str()).collect::<Vec<_>>(), ["docs work"]);
    let none = fleet.forget_review_search(ForgetReviewSearch { words: "never searched".into() }).await.expect("not an error");
    assert_eq!(none.runs.len(), 1);

    let before = judge.asked().len();
    Arc::clone(&fleet).start_review_search(start("ledger work", false)).await.expect("started");
    settled(&fleet).await;
    assert_eq!(judge.asked().len(), before + 1);
}

#[tokio::test]
async fn a_match_for_a_pull_request_that_was_not_searched_fails_the_run_and_no_match_is_shown() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying(
        r#"{"matches":[{"repository":"mele-labs/armada","number":8,"reason":"fine"},{"repository":"mele-labs/armada","number":99,"reason":"invented"}]}"#,
    ));
    let fleet = a_fleet_judged_by(&home, judge);
    a_forge_with_two_open(&fleet);

    Arc::clone(&fleet).start_review_search(start("anything", false)).await.expect("started");
    let runs = settled(&fleet).await;

    assert_eq!(runs[0].state, ReviewRunState::Failed);
    assert!(runs[0].failed.as_deref().is_some_and(|said| said.contains("#99")), "{:?}", runs[0].failed);
    assert!(runs[0].result.is_none());
}

#[tokio::test]
async fn nothing_open_is_an_answer_and_not_a_call() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying(ONE_MATCH));
    let fleet = a_fleet_judged_by(&home, Arc::clone(&judge));
    fleet.vcs().now_review_list(ReviewList::Open, Vec::new());

    Arc::clone(&fleet).start_review_search(start("anything", false)).await.expect("started");
    let runs = settled(&fleet).await;

    assert!(matches!(&runs[0].result, Some(ReviewResult::Matches { matches }) if matches.is_empty()));
    assert!(judge.asked().is_empty());
}

#[tokio::test]
async fn blank_words_are_refused_and_a_forge_that_will_not_list_is_a_fault() {
    let home = TempDir::new();
    let fleet = a_fleet_judged_by(&home, Arc::new(FakeJudge::saying(ONE_MATCH)));

    let blank = Arc::clone(&fleet).start_review_search(start("  \n ", false)).await.unwrap_err();
    assert_eq!((blank.status(), code(&blank)), (422, "fleet.review_blank"));

    // No list scripted for `Open` is the forge's silence.
    let silent = Arc::clone(&fleet).start_review_search(start("anything", false)).await.unwrap_err();
    assert_eq!((silent.status(), code(&silent)), (500, "fleet.review_lists_unreadable"));
    assert!(fleet.get_review_searches().await.expect("read").runs.is_empty(), "a search over nothing is not kept");
}

#[test]
fn an_answer_is_held_to_the_open_set_one_line_a_reason_and_each_pull_request_once() {
    let open = vec![(SLUG.to_string(), 8), (SLUG.to_string(), 9)];
    let matches = |json: &str| matches_read(json, &open);
    assert_eq!(matches(ONE_MATCH), Ok(vec![(0, "Changes crates/fleet/ledger.rs, which you changed".to_string())]));
    assert_eq!(matches(r#"{"matches":[]}"#), Ok(Vec::new()), "none is an answer");
    assert!(matches(r#"Here: {"matches":[{"repository":"MELE-LABS/Armada","number":9,"reason":"r"}]}"#).is_ok(), "a fence or a sentence is seen through, and the slug's case is not");
    assert!(matches(r#"{"matches":[{"repository":"other/repo","number":8,"reason":"r"}]}"#).is_err());
    assert!(matches(r#"{"matches":[{"repository":"mele-labs/armada","number":8,"reason":"a"},{"repository":"mele-labs/armada","number":8,"reason":"b"}]}"#).unwrap_err().contains("second time"));
    assert!(matches(r#"{"matches":[{"repository":"mele-labs/armada","number":8,"reason":" "}]}"#).unwrap_err().contains("no reason"));
    assert!(matches(r#"{"matches":[{"repository":"mele-labs/armada","number":8,"reason":"two\nlines"}]}"#).is_err());
}
