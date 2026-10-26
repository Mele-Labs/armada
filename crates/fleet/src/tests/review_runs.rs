//! An agent's read-only turn about a pull request, through a real Fleet, a scripted forge and a
//! scripted model. `docs/concepts/pull-review.md`, *Agent runs*.
//!
//! **No model is called and no forge is reached.** `FakeJudge` renders a shell that prints the
//! JSON a test wrote, so the real runner, the real stdin and the real reader are exercised, and
//! `FakeVcs` stands in for the pool and the forge and records every bay it leased and gave back.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{
    slot_path, PullRequestDiff, PullRequestFacts, PullRequestStanding, ReviewList, ListedChecks,
    ListedPull, ReviewStanding,
};
use api::{Next, PullRequestPanel, Refusal};
use ipc::{
    Event, ManifestId, ReviewResult, ReviewRun, ReviewRunKind, ReviewRunState, ReviewToolKind, ReviewToolScope,
    ReviewToolSource, SaveReviewTool, StartReviewRun,
};
use testkit::{Delivering, FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct};

use super::pull_request_panel::a_page;
use super::pull_requesting::{a_session_holding_pull_request_12, code, manifest};
use crate::daemon::Fleet;
use crate::review_runs::answer_read;
use crate::tests::daemon::{fittings, one, workflow_named};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// What `a_page` says the head is.
const HEAD: &str = "3f9c1d2e3f9c1d2e3f9c1d2e3f9c1d2e3f9c1d2e";
const NEXT_HEAD: &str = "9a8b7c6d9a8b7c6d9a8b7c6d9a8b7c6d9a8b7c6d";

const DIFF: &str = "diff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n@@ -1,2 +1,2 @@\n-let x = 1;\n+let x = 2;\n";

const FINDINGS: &str = r#"{"findings":[{"title":"Overflow","body":"`x` can wrap.","severity":"high","at":{"path":"a.rs","side":"right","start":1,"end":2}}]}"#;

pub(super) fn a_fleet_judged_by(home: &TempDir, judge: Arc<FakeJudge>) -> Arc<Fixture> {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&[]));
    fittings.starting().workflows = one(workflow_named("code_review"));
    fittings.starting().manifest = config::Manifest::parse(
        std::path::Path::new("armada.yml"),
        "version: 1\nid: 01FIXTUREMANIFEST\nbase: main\n",
    )
    .expect("parses");
    fittings.vcs = FakeVcs::new().delivering(Delivering {
        pull_request_diff: Some(PullRequestDiff {
            files: vec!["a.rs".into()],
            patch: DIFF.into(),
        }),
        ..Delivering::default()
    });
    fittings.judge = judge;
    let fleet = Fleet::assembled(fittings);
    fleet.vcs().now_pull_request(Some(PullRequestFacts {
        standing: PullRequestStanding::Open,
        branch: "fleet/pr-acts".into(),
        base: "main".into(),
        auto_merge: false,
        title: "Take a draft out".into(),
        url: "https://forge.invalid/armada/pull/12".into(),
        body: Some("Run `just try`, then look at the page.".into()),
        closes: Vec::new(),
    }));
    fleet.vcs().now_pull_request_page(Some(a_page()));
    fleet.vcs().reading_pull_requests_at(HEAD);
    Arc::new(fleet)
}

fn start(kind: ReviewRunKind, name: &str, again: bool) -> StartReviewRun {
    StartReviewRun {
        number: 12,
        kind,
        name: name.into(),
        again,
        place: None,
    }
}

/// Wait until nothing is going for pull request 12, then say what there is.
async fn settled(fleet: &Fixture, id: &ManifestId) -> Vec<ReviewRun> {
    for _ in 0..500 {
        let runs = fleet.get_review_runs(id.clone(), 12).await.expect("read").runs;
        if runs.iter().all(|run| matches!(run.state, ReviewRunState::Done | ReviewRunState::Failed)) {
            return runs;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("the run did not end within five seconds");
}

fn findings_of(run: &ReviewRun) -> usize {
    match &run.result {
        Some(ReviewResult::Findings { findings }) => findings.len(),
        other => panic!("findings were expected, not {other:?}"),
    }
}

#[tokio::test]
async fn a_tool_run_keeps_its_findings_for_the_head_it_read_and_gives_the_bay_back() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying(FINDINGS).costing(0.25));
    let fleet = a_fleet_judged_by(&home, Arc::clone(&judge));
    let (id, root) = manifest(&fleet);
    let mut watching = fleet.events().subscribe();

    let queued = Arc::clone(&fleet).start_review_run(id.clone(), start(ReviewRunKind::Tool, "risks", false)).await.expect("started");
    assert_eq!((queued.state, queued.head.as_str(), queued.number), (ReviewRunState::Queued, HEAD, 12));
    let runs = settled(&fleet, &id).await;

    assert_eq!(runs.len(), 1);
    let run = &runs[0];
    assert_eq!((run.state, run.head.as_str(), run.name.as_str()), (ReviewRunState::Done, HEAD, "risks"));
    assert_eq!(findings_of(run), 1);
    assert_eq!(run.cost_micros, Some(250_000), "what the vendor billed is kept with the run");
    assert!(run.finished_at.is_some() && run.failed.is_none());

    // The model was told the definition, the pull request and the diff, and could read only the bay.
    let asked = judge.asked();
    assert_eq!(asked.len(), 1);
    for told in ["Take a draft out", "Run `just try`", HEAD, "let x = 2;", "a.rs", "findings"] {
        assert!(asked[0].contains(told), "the brief lacks {told}: {}", asked[0]);
    }
    assert_eq!(judge.models(), ["a-model"], "models.review");
    assert_eq!(judge.directories(), [Some(slot_path(&root, 1))]);

    // The bay was leased for this pull request and given back to the one that took it.
    let leased = fleet.vcs().review_bays_leased();
    assert_eq!(leased.len(), 1);
    assert_eq!(leased[0].0, 12);
    assert_eq!(fleet.vcs().review_bays_released(), [(1, leased[0].1.clone())]);
    assert!(fleet.vcs().slot_holders(&root).iter().all(Option::is_none));

    // Three small events and nothing between: queued, running, done.
    let mut states = Vec::new();
    while states.last() != Some(&ReviewRunState::Done) {
        match tokio::time::timeout(Duration::from_secs(5), watching.next()).await.expect("an event") {
            Some(Next::Send(delivered)) => {
                if let Event::ReviewRunChanged { run } = delivered.event {
                    states.push(run.state);
                }
            }
            other => panic!("the stream ended: {other:?}"),
        }
    }
    assert_eq!(states, [ReviewRunState::Queued, ReviewRunState::Running, ReviewRunState::Done]);
}

#[tokio::test]
async fn a_kept_result_answers_with_no_run_and_a_new_head_leaves_it_served_with_the_commit_it_read() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying(FINDINGS));
    let fleet = a_fleet_judged_by(&home, Arc::clone(&judge));
    let (id, _) = manifest(&fleet);

    Arc::clone(&fleet).start_review_run(id.clone(), start(ReviewRunKind::Tool, "risks", false)).await.expect("started");
    settled(&fleet, &id).await;

    let again = Arc::clone(&fleet).start_review_run(id.clone(), start(ReviewRunKind::Tool, "risks", false)).await.expect("answered");
    assert_eq!((again.state, again.head.as_str()), (ReviewRunState::Done, HEAD));
    assert_eq!(findings_of(&again), 1);
    assert_eq!(judge.asked().len(), 1, "the kept one was answered with no run");

    // The branch moves. The old answer is served beside the commit it read, and still runs nothing.
    let mut page = a_page();
    page.head = Some(NEXT_HEAD.into());
    fleet.vcs().now_pull_request_page(Some(page));
    fleet.vcs().reading_pull_requests_at(NEXT_HEAD);
    let stale = Arc::clone(&fleet).start_review_run(id.clone(), start(ReviewRunKind::Tool, "risks", false)).await.expect("answered");
    assert_eq!((stale.state, stale.head.as_str()), (ReviewRunState::Done, HEAD));
    assert_eq!(judge.asked().len(), 1);

    // Run again is the one way to spend on it, and it keeps both heads.
    let forced = Arc::clone(&fleet).start_review_run(id.clone(), start(ReviewRunKind::Tool, "risks", true)).await.expect("started");
    assert_eq!((forced.state, forced.head.as_str()), (ReviewRunState::Queued, NEXT_HEAD));
    let runs = settled(&fleet, &id).await;
    assert_eq!(judge.asked().len(), 2);
    let heads: Vec<&str> = runs.iter().map(|run| run.head.as_str()).collect();
    assert_eq!(heads.len(), 2);
    assert!(heads.contains(&HEAD) && heads.contains(&NEXT_HEAD), "{heads:?}");
}

#[tokio::test]
async fn an_answer_that_is_not_json_is_a_failed_run_with_its_reason_and_not_a_kept_one() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying("It looks fine to me."));
    let fleet = a_fleet_judged_by(&home, Arc::clone(&judge));
    let (id, root) = manifest(&fleet);

    Arc::clone(&fleet).start_review_run(id.clone(), start(ReviewRunKind::Tool, "risks", false)).await.expect("started");
    let runs = settled(&fleet, &id).await;
    assert_eq!(runs[0].state, ReviewRunState::Failed);
    assert!(runs[0].result.is_none());
    let said = runs[0].failed.clone().expect("a reason");
    assert!(said.contains("JSON"), "{said}");
    assert_eq!(fleet.vcs().review_bays_released().len(), 1, "a failed run gives its bay back too");
    assert!(fleet.vcs().slot_holders(&root).iter().all(Option::is_none));

    // A failure is not a kept result: pressing again runs.
    Arc::clone(&fleet).start_review_run(id.clone(), start(ReviewRunKind::Tool, "risks", false)).await.expect("started");
    settled(&fleet, &id).await;
    assert_eq!(judge.asked().len(), 2);
}

#[tokio::test]
async fn a_call_that_cannot_be_made_is_a_failed_run_naming_why() {
    let home = TempDir::new();
    let fleet = a_fleet_judged_by(&home, Arc::new(FakeJudge::that_fails("the network")));
    let (id, _) = manifest(&fleet);
    Arc::clone(&fleet).start_review_run(id.clone(), start(ReviewRunKind::Explanation, "why", false)).await.expect("started");
    let runs = settled(&fleet, &id).await;
    assert_eq!(runs[0].state, ReviewRunState::Failed);
    assert!(runs[0].failed.as_deref().unwrap_or_default().contains("did not answer"));
}

#[tokio::test]
async fn a_pull_request_that_moved_while_the_run_waited_is_not_read_at_the_wrong_commit() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying(FINDINGS));
    let fleet = a_fleet_judged_by(&home, Arc::clone(&judge));
    let (id, _) = manifest(&fleet);
    fleet.vcs().reading_pull_requests_at(NEXT_HEAD);
    Arc::clone(&fleet).start_review_run(id.clone(), start(ReviewRunKind::Tool, "risks", false)).await.expect("started");
    let runs = settled(&fleet, &id).await;
    assert_eq!(runs[0].state, ReviewRunState::Failed);
    assert!(runs[0].failed.as_deref().unwrap_or_default().contains("moved"));
    assert!(judge.asked().is_empty(), "nothing was asked at the wrong commit");
    assert_eq!(fleet.vcs().review_bays_released().len(), 1);
}

#[tokio::test]
async fn a_forge_that_will_not_give_the_head_starts_nothing() {
    let home = TempDir::new();
    let fleet = a_fleet_judged_by(&home, Arc::new(FakeJudge::saying(FINDINGS)));
    let (id, _) = manifest(&fleet);
    fleet.vcs().now_pull_request_page(None);
    let refused = Arc::clone(&fleet).start_review_run(id, start(ReviewRunKind::Tool, "risks", false)).await.unwrap_err();
    assert_eq!(code(&refused), "fleet.review_pull_request_unreadable");
    assert!(fleet.vcs().review_bays_leased().is_empty());
}

#[tokio::test]
async fn a_name_nobody_defined_and_a_blank_question_are_refused_before_anything_is_spent() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying(FINDINGS));
    let fleet = a_fleet_judged_by(&home, Arc::clone(&judge));
    let (id, _) = manifest(&fleet);
    let unknown = Arc::clone(&fleet).start_review_run(id.clone(), start(ReviewRunKind::Tool, "no-such-tool", false)).await.unwrap_err();
    assert_eq!(code(&unknown), "fleet.review_unknown_tool");
    // An explanation is not a tool: the kind has to be the definition's.
    let wrong = Arc::clone(&fleet).start_review_run(id.clone(), start(ReviewRunKind::Tool, "why", false)).await.unwrap_err();
    assert_eq!(code(&wrong), "fleet.review_unknown_tool");
    let blank = Arc::clone(&fleet).start_review_run(id, start(ReviewRunKind::Ask, "   ", false)).await.unwrap_err();
    assert_eq!(code(&blank), "fleet.review_blank");
    assert!(judge.asked().is_empty() && fleet.vcs().review_bays_leased().is_empty());
}

#[tokio::test]
async fn a_question_is_kept_under_its_words_and_answered_in_text() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying(r#"{"body":"It swaps a constant."}"#));
    let fleet = a_fleet_judged_by(&home, Arc::clone(&judge));
    let (id, _) = manifest(&fleet);
    Arc::clone(&fleet).start_review_run(id.clone(), start(ReviewRunKind::Ask, "What does   it DO?", false)).await.expect("started");
    let runs = settled(&fleet, &id).await;
    assert_eq!(runs[0].name, "what does it do?");
    assert_eq!(runs[0].result, Some(ReviewResult::Text { body: "It swaps a constant.".into() }));
    assert!(judge.asked()[0].contains("What does it DO?"), "the question as it was asked, its spaces tidied");
    // The same words in other case are the same question, kept.
    let kept = Arc::clone(&fleet).start_review_run(id, start(ReviewRunKind::Ask, "what does it do?", false)).await.expect("answered");
    assert_eq!(kept.state, ReviewRunState::Done);
    assert_eq!(judge.asked().len(), 1);
}

#[tokio::test]
async fn a_run_waits_queued_while_every_bay_is_held_and_goes_when_one_is_free() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying(FINDINGS));
    let fleet = a_fleet_judged_by(&home, Arc::clone(&judge));
    let (id, root) = manifest(&fleet);
    for slot in 1..=8 {
        fleet.vcs().hold_slot(&root, slot, "somebody");
    }
    let started = Arc::clone(&fleet).start_review_run(id.clone(), start(ReviewRunKind::Tool, "risks", false)).await.expect("started");
    assert_eq!(started.state, ReviewRunState::Queued);
    tokio::time::sleep(Duration::from_millis(1_300)).await;
    let waiting = fleet.get_review_runs(id.clone(), 12).await.expect("read").runs;
    assert_eq!(waiting[0].state, ReviewRunState::Queued, "nothing is read while no bay is free");
    assert!(judge.asked().is_empty());
    fleet.vcs().free_slot(&root, 3);
    let runs = settled(&fleet, &id).await;
    assert_eq!(runs[0].state, ReviewRunState::Done);
    assert_eq!(fleet.vcs().review_bays_released()[0].0, 3);
}

#[tokio::test]
async fn a_run_the_last_process_left_going_ends_failed_and_its_bay_goes_back() {
    let home = TempDir::new();
    let fleet = a_fleet_judged_by(&home, Arc::new(FakeJudge::saying(FINDINGS)));
    let (id, root) = manifest(&fleet);
    fleet.vcs().hold_slot(&root, 2, "review-left-behind");
    let left = store::KeptReviewRun {
        manifest_id: id.as_str().into(),
        number: 12,
        head: HEAD.into(),
        kind: "tool".into(),
        name: "risks".into(),
        label: "risks".into(),
        state: "running".into(),
        started_at: "2026-10-16T09:00:00Z".into(),
        finished_at: None,
        failed: None,
        result: None,
        cost_micros: None,
        slot: Some(2),
        holder: Some("review-left-behind".into()),
    };
    fleet.store().lock().await.keep_review_run(&left).expect("kept");
    let runs = fleet.get_review_runs(id, 12).await.expect("read").runs;
    assert_eq!(runs[0].state, ReviewRunState::Failed);
    assert!(runs[0].failed.as_deref().unwrap_or_default().contains("restarted"));
    assert_eq!(fleet.vcs().review_bays_released(), [(2, "review-left-behind".to_string())]);
}

// The definitions.

fn write(path: std::path::PathBuf, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

#[tokio::test]
async fn kit_answers_over_the_repository_and_says_what_it_hides_and_a_bad_file_is_named() {
    let home = TempDir::new();
    let fleet = a_fleet_judged_by(&home, Arc::new(FakeJudge::saying(FINDINGS)));
    let (id, root) = manifest(&fleet);
    let kit = home.path().join("kit").join("review-tools");
    write(kit.join("risks.md"), "---\nkind: tool\nname: Risks\n---\nKit's risks.\n");
    write(kit.join("mine.md"), "---\nkind: tool\nname: Mine\n---\nKit's own.\n");
    let repository = std::path::Path::new(&root).join(".armada").join("review-tools");
    write(repository.join("risks.md"), "---\nkind: tool\nname: Risks\n---\nThe repository's risks.\n");
    write(repository.join("broken.md"), "no front matter\n");

    let listed = fleet.list_review_tools(id).await.expect("listed");
    let find = |id: &str| listed.tools.iter().find(|tool| tool.id == id).unwrap_or_else(|| panic!("no {id}"));
    assert_eq!(
        (find("risks").source, find("risks").replaces, find("risks").prompt.as_str()),
        (ReviewToolSource::Kit, Some(ReviewToolSource::Repository), "Kit's risks.")
    );
    assert_eq!(find("risks").hides, Some(ReviewToolSource::Repository));
    assert_eq!((find("mine").source, find("mine").hides), (ReviewToolSource::Kit, None));
    assert_eq!(find("how-to-test").source, ReviewToolSource::Armada);
    assert_eq!(listed.left_out.len(), 1);
    assert!(listed.left_out[0].file.ends_with("broken.md") && listed.left_out[0].source == ReviewToolSource::Repository);
}

#[tokio::test]
async fn the_run_asks_with_the_definition_that_wins() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying(FINDINGS));
    let fleet = a_fleet_judged_by(&home, Arc::clone(&judge));
    let (id, root) = manifest(&fleet);
    let repository = std::path::Path::new(&root).join(".armada").join("review-tools");
    write(repository.join("risks.md"), "---\nkind: tool\nname: Risks\n---\nLook only at refunds.\n");
    Arc::clone(&fleet).start_review_run(id.clone(), start(ReviewRunKind::Tool, "risks", false)).await.expect("started");
    settled(&fleet, &id).await;
    assert!(judge.asked()[0].contains("Look only at refunds."));
}

fn save(scope: ReviewToolScope, name: &str, prompt: &str, overwrite: bool) -> SaveReviewTool {
    SaveReviewTool {
        scope,
        kind: ReviewToolKind::Tool,
        name: name.into(),
        prompt: prompt.into(),
        overwrite,
    }
}

#[tokio::test]
async fn a_save_writes_one_file_in_the_scope_named_and_refuses_what_it_would_clobber() {
    let home = TempDir::new();
    let fleet = a_fleet_judged_by(&home, Arc::new(FakeJudge::saying(FINDINGS)));
    let (id, root) = manifest(&fleet);

    let kit = fleet.save_review_tool(id.clone(), save(ReviewToolScope::Kit, "Test gaps", "Find untested branches.", false)).await.expect("saved");
    let saved = kit.tools.iter().find(|tool| tool.id == "test-gaps").expect("listed after the save");
    assert_eq!((saved.source, saved.kind), (ReviewToolSource::Kit, ReviewToolKind::Tool));
    assert!(home.path().join("kit/review-tools/test-gaps.md").is_file());

    fleet.save_review_tool(id.clone(), save(ReviewToolScope::Repository, "Test gaps", "The repository's.", false)).await.expect("saved");
    assert!(std::path::Path::new(&root).join(".armada/review-tools/test-gaps.md").is_file());
    let listed = fleet.list_review_tools(id.clone()).await.expect("listed");
    let winner = listed.tools.iter().find(|tool| tool.id == "test-gaps").unwrap();
    assert_eq!((winner.source, winner.replaces), (ReviewToolSource::Kit, Some(ReviewToolSource::Repository)));

    let exists = fleet.save_review_tool(id.clone(), save(ReviewToolScope::Kit, "test gaps", "Other.", false)).await.unwrap_err();
    assert_eq!(code(&exists), "fleet.review_tool_exists");
    let replaced = fleet.save_review_tool(id.clone(), save(ReviewToolScope::Kit, "test gaps", "Other.", true)).await.expect("overwritten");
    assert!(replaced.tools.iter().all(|tool| tool.id != "test-gaps" || tool.prompt == "Other."));
    assert!(std::fs::read_to_string(home.path().join("kit/review-tools/test-gaps.md")).unwrap().contains("Other."));

    for (name, prompt, wanted) in [
        ("  ", "x", "fleet.review_tool_blank"),
        ("Name", "  ", "fleet.review_tool_blank"),
        ("!!!", "x", "fleet.review_tool_not_a_name"),
    ] {
        let refused = fleet.save_review_tool(id.clone(), save(ReviewToolScope::Kit, name, prompt, false)).await.unwrap_err();
        assert_eq!(code(&refused), wanted, "{name:?}");
        assert!(matches!(refused, Refusal::Unacceptable(_)));
    }
}

// What an answer must be to be kept.

fn checkout_with(files: &[&str]) -> TempDir {
    let dir = TempDir::new();
    for file in files {
        write(dir.path().join(file), "docs");
    }
    dir
}

fn changed() -> Vec<String> {
    vec!["a.rs".to_string(), "b/c.rs".to_string()]
}

fn read(shape: &str, answer: &str, dir: &TempDir, description: Option<&str>) -> Result<ReviewResult, String> {
    answer_read(shape, answer, &changed(), dir.path(), description, None)
}

#[test]
fn an_answer_is_found_inside_a_fence_or_a_sentence_and_read_into_its_shape() {
    let dir = checkout_with(&[]);
    let fenced = format!("Here you go:\n```json\n{FINDINGS}\n```\n");
    assert!(matches!(read("findings", &fenced, &dir, None), Ok(ReviewResult::Findings { findings }) if findings.len() == 1));
    assert!(matches!(read("findings", r#"{"findings":[]}"#, &dir, None), Ok(ReviewResult::Findings { findings }) if findings.is_empty()), "none found is an answer");
    let said = read("findings", "no braces here", &dir, None).unwrap_err();
    assert!(said.contains("JSON"), "{said}");
}

#[test]
fn a_finding_must_sit_on_a_file_the_change_touches_and_on_a_real_range() {
    let dir = checkout_with(&[]);
    let at = |path: &str, start: u64, end: u64| {
        format!(r#"{{"findings":[{{"title":"t","body":"b","severity":"low","at":{{"path":"{path}","side":"left","start":{start},"end":{end}}}}}]}}"#)
    };
    assert!(read("findings", &at("a.rs", 3, 3), &dir, None).is_ok());
    let outside = read("findings", &at("elsewhere.rs", 1, 1), &dir, None).unwrap_err();
    assert!(outside.contains("elsewhere.rs") && outside.contains("does not change"), "{outside}");
    assert!(read("findings", &at("a.rs", 0, 1), &dir, None).is_err());
    assert!(read("findings", &at("a.rs", 5, 2), &dir, None).is_err());
    let unknown_severity = at("a.rs", 1, 1).replace("low", "dire");
    assert!(read("findings", &unknown_severity, &dir, None).is_err());
}

#[test]
fn how_it_works_needs_themes_whose_changes_name_a_changed_place() {
    let dir = checkout_with(&[]);
    let change = |path: &str| {
        format!(
            r#"{{"summary":"s","themes":[{{"title":"T","purpose":"p","changes":[{{"title":"c","why":"w","at":{{"path":"{path}","side":"right","start":1,"end":1}}}}]}}]}}"#
        )
    };
    assert!(matches!(read("themes", &change("b/c.rs"), &dir, None), Ok(ReviewResult::Themes { .. })));
    assert!(read("themes", &change("zzz.rs"), &dir, None).is_err());
    assert!(read("themes", r#"{"summary":"s","themes":[]}"#, &dir, None).is_err());
    assert!(read("themes", r#"{"summary":"s","themes":[{"title":"T","purpose":"p","changes":[]}]}"#, &dir, None).is_err());
}

#[test]
fn how_to_test_puts_the_description_first_and_names_only_docs_the_checkout_has() {
    let dir = checkout_with(&["crates/x/README.md"]);
    let steps = r#"{"steps":[
        {"action":"run the crate","look":"it passes","code":"cargo test","source":{"from":"doc","path":"crates/x/README.md"}},
        {"action":"open the page","look":"a draft is gone","source":{"from":"description"}}]}"#;
    match read("steps", steps, &dir, Some("Open the page.")).expect("kept") {
        ReviewResult::Steps { steps } => {
            assert_eq!(steps[0].action, "open the page", "the author's directions come first");
            assert_eq!(steps[1].code.as_deref(), Some("cargo test"));
        }
        other => panic!("{other:?}"),
    }
    let missing = steps.replace("crates/x/README.md", "crates/y/README.md");
    assert!(read("steps", &missing, &dir, Some("Open the page.")).unwrap_err().contains("not a file in the checkout"));
    let outside = steps.replace("crates/x/README.md", "../../etc/passwd");
    assert!(read("steps", &outside, &dir, Some("d")).is_err());
    let no_description = read("steps", steps, &dir, None).unwrap_err();
    assert!(no_description.contains("description"), "{no_description}");
}

#[test]
fn why_and_user_impact_and_text_are_held_to_their_shapes() {
    let dir = checkout_with(&[]);
    assert!(read("reason", r#"{"problem":"p","cost":"c","links":[{"label":"issue 9","address":"https://forge.invalid/9"}]}"#, &dir, None).is_ok());
    assert!(read("reason", r#"{"problem":"p","cost":"c","links":[{"label":"x","address":"javascript:alert(1)"}]}"#, &dir, None).is_err());
    assert!(read("reason", r#"{"problem":"","cost":"c"}"#, &dir, None).is_err());
    assert!(read("impact", r#"{"rows":[{"what":"w","before":"b","after":"a"}]}"#, &dir, None).is_ok());
    assert!(read("impact", r#"{"rows":[{"what":"w","before":"b"}]}"#, &dir, None).is_err());
    assert!(read("text", r#"{"body":"  "}"#, &dir, None).is_err());
    assert!(read("text", r#"{"body":"fine"}"#, &dir, None).is_ok());
}

// Who holds a pull request.

#[tokio::test]
async fn a_listed_pull_request_names_the_session_that_holds_it() {
    let home = TempDir::new();
    let fleet = a_fleet_judged_by(&home, Arc::new(FakeJudge::saying(FINDINGS)));
    fleet.vcs().main_ci.login_is(Some("NickMele"));
    let pull = |number: u64| ListedPull {
        repository: "mele-labs/armada".into(),
        number,
        title: format!("Pull request {number}"),
        branch: format!("branch-{number}"),
        address: format!("https://forge.invalid/mele-labs/armada/pull/{number}"),
        draft: false,
        head: HEAD.into(),
        author: "ada".into(),
        updated_at: "2026-10-15T09:30:00Z".into(),
        checks: ListedChecks::Running,
        review: ReviewStanding::None,
        team: None,
    };
    for list in [ReviewList::Requested, ReviewList::Team, ReviewList::Assigned] {
        fleet.vcs().now_review_list(list, Vec::new());
    }
    fleet.vcs().now_review_list(ReviewList::Mine, vec![pull(12), pull(13)]);
    a_session_holding_pull_request_12(&fleet, "s-held").await;
    let lists = fleet.get_review_lists().await.expect("read");
    let held = lists.mine.iter().find(|row| row.number == 12).unwrap().holder.clone();
    assert_eq!(held.map(|holder| (holder.id, holder.title)), Some(("s-held".to_string(), "fix the ledger".to_string())));
    assert!(lists.mine.iter().find(|row| row.number == 13).unwrap().holder.is_none(), "nobody holds 13");
}
