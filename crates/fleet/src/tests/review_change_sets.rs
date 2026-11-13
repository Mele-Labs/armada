//! A pull request read as its change sets: the sets name the diff's own hunks, the overview and a
//! practice check stand on the sets kept at the head, and practices are read from the docs once an
//! area and kind and read again when a doc moves. `docs/concepts/pull-review.md`, *Change sets*.

use std::path::Path;
use std::sync::Arc;

use adapter_traits::{slot_path, ComparedPatches, PatchedFile, PullRequestDiff};
use api::PullRequestPanel;
use ipc::{
    ChangeSetKind, HunkRef, ManifestId, OverviewSeverity, PracticeSource, PracticeVerdict, ReviewResult, ReviewRun,
    ReviewRunKind, ReviewRunState, StartReviewRun,
};
use testkit::FakeJudge;

use super::pull_requesting::{code, manifest};
use super::review_runs::{a_fleet_reading, settled, Fixture, HEAD};
use crate::review_change_sets::{change_sets_read, overview_read, practices_read};
use crate::tests::tmp::TempDir;

const DIFF: &str = "diff --git a/db/0007_orders.sql b/db/0007_orders.sql\nnew file mode 100644\n--- /dev/null\n\
+++ b/db/0007_orders.sql\n@@ -0,0 +1,3 @@\n+CREATE TABLE orders (\n+  id INTEGER PRIMARY KEY\n+);\n\
diff --git a/src/orders.rs b/src/orders.rs\n--- a/src/orders.rs\n+++ b/src/orders.rs\n@@ -1,2 +1,3 @@\n fn list() {\n\
+    read_orders();\n }\n@@ -10 +11,2 @@\n--- old\n+new\n+more\n";

const SETS: &str = r#"{"sets":[
  {"title":"Add the orders table","kind":"migration","purpose":"Orders are kept in a table of their own.","hunks":["h1"]},
  {"title":"List orders","kind":"logic","purpose":"The list reads the new table.","hunks":["h3","h2"]}]}"#;

const OVERVIEW: &str = r#"{"findings":[
  {"severity":"bug","set":"set-2","title":"Reads before it exists","reason":"The list reads orders before the table is made, so run the migration first.","at":{"path":"src/orders.rs","side":"right","start":2,"end":2}},
  {"severity":"blocker","set":"set-1","title":"No way back","reason":"The migration has no down step, so add one that drops the table.","at":{"path":"db/0007_orders.sql","side":"right","start":1,"end":3}}]}"#;

const PRACTICES_READ: &str = r#"{"items":[
  {"item":"Every migration has a down step","source":"docs/migrations.md","verdict":"missed","note":"Only an up step."},
  {"item":"A table name is plural","source":"docs/migrations.md","verdict":"met","note":"orders"}]}"#;

const PRACTICES_CHECKED: &str = r#"{"checks":[
  {"item":2,"verdict":"met","note":"orders"},
  {"item":1,"verdict":"not_applicable","note":"Nothing to undo yet."}]}"#;

fn the_diff() -> PullRequestDiff {
    PullRequestDiff { files: vec!["db/0007_orders.sql".into(), "src/orders.rs".into()], patch: DIFF.into() }
}

fn start(kind: ReviewRunKind, name: &str, again: bool) -> StartReviewRun {
    StartReviewRun { number: 12, kind, name: name.into(), again, place: None, finding: None, since: None }
}

fn hunk(path: &str, old: (u64, u64), new: (u64, u64)) -> HunkRef {
    HunkRef { path: path.into(), old_start: old.0, old_lines: old.1, new_start: new.0, new_lines: new.1 }
}

/// The run of `kind`, once nothing is going.
async fn run_of(fleet: &Fixture, id: &ManifestId, kind: ReviewRunKind) -> ReviewRun {
    let runs = settled(fleet, id).await;
    runs.into_iter().find(|run| run.kind == kind).expect("a run of the kind")
}

/// The doc the practices are read from, in the bay.
fn bay_holding_the_doc(root: &str) {
    let docs = Path::new(&slot_path(root, 1)).join("docs");
    std::fs::create_dir_all(&docs).expect("a bay");
    std::fs::write(docs.join("migrations.md"), "# Migrations\n\nEvery migration has a down step.\n").expect("written");
}

#[tokio::test]
async fn change_sets_name_the_diffs_own_hunks_in_the_order_they_run() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying(SETS));
    let fleet = a_fleet_reading(&home, Arc::clone(&judge), the_diff());
    let (id, _) = manifest(&fleet);

    let queued = Arc::clone(&fleet).start_review_run(id.clone(), start(ReviewRunKind::ChangeSets, "", false)).await.expect("started");
    assert_eq!((queued.state, queued.name.as_str()), (ReviewRunState::Queued, "change-sets"));
    let run = run_of(&fleet, &id, ReviewRunKind::ChangeSets).await;

    let Some(ReviewResult::ChangeSets { sets }) = &run.result else { panic!("change sets, not {:?} ({:?})", run.result, run.failed) };
    let named: Vec<(&str, ChangeSetKind, &str)> = sets.iter().map(|set| (set.id.as_str(), set.kind, set.area.as_str())).collect();
    assert_eq!(named, [("set-1", ChangeSetKind::Migration, "db"), ("set-2", ChangeSetKind::Logic, "src")]);
    assert_eq!(sets[0].hunks, [hunk("db/0007_orders.sql", (0, 0), (1, 3))]);
    assert_eq!(sets[1].hunks, [hunk("src/orders.rs", (10, 1), (11, 2)), hunk("src/orders.rs", (1, 2), (1, 3))], "the agent's order");
    let asked = judge.asked();
    for told in ["h1 `db/0007_orders.sql` @@ -0,0 +1,3 @@", "h3 `src/orders.rs` @@ -10,1 +11,2 @@", "\"migration\""] {
        assert!(asked[0].contains(told), "the brief lacks {told}: {}", asked[0]);
    }
}

#[test]
fn a_change_set_answer_names_only_hunks_it_was_told_and_each_once() {
    let hunks = [hunk("a.rs", (1, 1), (1, 1)), hunk("b/c.rs", (4, 2), (4, 3)), hunk("b/d.rs", (1, 1), (1, 1))];
    let set = |hunks: &str, purpose: &str| {
        format!(r#"{{"sets":[{{"title":"T","kind":"logic","purpose":"{purpose}","hunks":[{hunks}]}}]}}"#)
    };
    let refused = |answer: String| change_sets_read(&answer, &hunks).unwrap_err();
    assert!(refused(set(r#""h9""#, "One thing.")).contains("h9"), "an id it was not told");
    assert!(refused(set(r#""h1","h1""#, "One thing.")).contains("twice"));
    assert!(refused(set(r#""h1""#, "One thing. And another.")).contains("one sentence"));
    assert!(refused(set("", "One thing.")).contains("no hunks"));

    // A hunk the agent put in no set is not lost: it lands in a last set of its own.
    let Ok(ReviewResult::ChangeSets { sets }) = change_sets_read(&set(r#""h2""#, "One thing."), &hunks) else { panic!("read") };
    assert_eq!(sets.len(), 2);
    assert_eq!((sets[0].area.as_str(), sets[1].id.as_str(), sets[1].kind), ("b", "set-2", ChangeSetKind::Other));
    assert_eq!(sets[1].hunks, [hunks[0].clone(), hunks[2].clone()]);
    assert_eq!(sets[1].area, "", "a.rs and b/d.rs share only the root");
}

#[tokio::test]
async fn the_overview_needs_the_sets_at_the_head_and_files_each_finding_under_one() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::answering_in_turn(&[SETS, OVERVIEW]));
    let fleet = a_fleet_reading(&home, Arc::clone(&judge), the_diff());
    let (id, _) = manifest(&fleet);

    let refused = Arc::clone(&fleet).start_review_run(id.clone(), start(ReviewRunKind::Overview, "", false)).await.unwrap_err();
    assert_eq!((refused.status(), code(&refused)), (409, "fleet.review_change_sets_needed"));
    assert!(judge.asked().is_empty(), "nothing was spent");

    Arc::clone(&fleet).start_review_run(id.clone(), start(ReviewRunKind::ChangeSets, "", false)).await.expect("started");
    run_of(&fleet, &id, ReviewRunKind::ChangeSets).await;
    Arc::clone(&fleet).start_review_run(id.clone(), start(ReviewRunKind::Overview, "", false)).await.expect("started");
    let run = run_of(&fleet, &id, ReviewRunKind::Overview).await;

    assert_eq!((run.name.as_str(), run.head.as_str()), ("overview", HEAD));
    let Some(ReviewResult::Overview { findings }) = &run.result else { panic!("an overview, not {:?} ({:?})", run.result, run.failed) };
    let read: Vec<(OverviewSeverity, &str)> = findings.iter().map(|found| (found.severity, found.set.as_str())).collect();
    assert_eq!(read, [(OverviewSeverity::Blocker, "set-1"), (OverviewSeverity::Bug, "set-2")], "blockers first");
    let asked = &judge.asked()[1];
    for told in ["set-1 (migration) Add the orders table", "What could go wrong that the change does not handle", "\"blocker\""] {
        assert!(asked.contains(told), "the brief lacks {told}: {asked}");
    }
}

#[test]
fn an_overview_finding_is_in_a_set_it_was_told_with_one_sentence_of_reason() {
    let changed = ["a.rs".to_string()];
    let finding = |set: &str, reason: &str| {
        format!(
            r#"{{"findings":[{{"severity":"risk","set":"{set}","title":"T","reason":"{reason}","at":{{"path":"a.rs","side":"right","start":1,"end":1}}}}]}}"#
        )
    };
    let sets = ["set-1".to_string()];
    assert!(overview_read(&finding("set-7", "Wraps, so check it."), &sets, &changed).unwrap_err().contains("set-7"));
    assert!(overview_read(&finding("set-1", "It wraps. Check it."), &sets, &changed).unwrap_err().contains("one sentence"));
    assert!(overview_read(&finding("set-1", "Wraps, so check it."), &sets, &changed).is_ok());
}

#[tokio::test]
async fn practices_are_read_once_an_area_and_kind_and_read_again_when_a_doc_moves() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::answering_in_turn(&[SETS, PRACTICES_READ, PRACTICES_CHECKED, PRACTICES_READ]));
    let fleet = a_fleet_reading(&home, Arc::clone(&judge), the_diff());
    let (id, root) = manifest(&fleet);
    bay_holding_the_doc(&root);
    fleet.vcs().blobs_at_base(&[("docs/migrations.md", "b1")]);

    let refused = Arc::clone(&fleet).start_review_run(id.clone(), start(ReviewRunKind::Practices, "set-1", false)).await.unwrap_err();
    assert_eq!((refused.status(), code(&refused)), (409, "fleet.review_change_sets_needed"));
    Arc::clone(&fleet).start_review_run(id.clone(), start(ReviewRunKind::ChangeSets, "", false)).await.expect("started");
    run_of(&fleet, &id, ReviewRunKind::ChangeSets).await;
    let refused = Arc::clone(&fleet).start_review_run(id.clone(), start(ReviewRunKind::Practices, "set-9", false)).await.unwrap_err();
    assert_eq!((refused.status(), code(&refused)), (422, "fleet.review_change_set_unknown"));

    // The first read finds the doc, reads it into items, and checks the set against them.
    Arc::clone(&fleet).start_review_run(id.clone(), start(ReviewRunKind::Practices, "set-1", false)).await.expect("started");
    let first = practices_of(&run_of(&fleet, &id, ReviewRunKind::Practices).await);
    assert_eq!((first.set.as_str(), first.kind, first.area.as_str(), first.reused), ("set-1", ChangeSetKind::Migration, "db", false));
    assert_eq!(first.sources, [PracticeSource { path: "docs/migrations.md".into(), blob: Some("b1".into()) }]);
    assert_eq!(first.verdicts, [PracticeVerdict::Missed, PracticeVerdict::Met]);
    assert!(judge.asked()[1].contains("Find the repository's own docs"), "{}", judge.asked()[1]);

    // The next run for the area and kind reads no doc: it checks against what was kept.
    Arc::clone(&fleet).start_review_run(id.clone(), start(ReviewRunKind::Practices, "set-1", true)).await.expect("started");
    let second = practices_of(&run_of(&fleet, &id, ReviewRunKind::Practices).await);
    assert_eq!((second.reused, second.read_at.as_str()), (true, first.read_at.as_str()));
    assert_eq!(second.verdicts, [PracticeVerdict::NotApplicable, PracticeVerdict::Met], "in the catalogue's order");
    let asked = &judge.asked()[2];
    assert!(asked.contains("1. Every migration has a down step (from `docs/migrations.md`)"), "{asked}");
    assert!(!asked.contains("Find the repository's own docs"), "{asked}");

    // A doc that moved at the base is read again.
    fleet.vcs().blobs_at_base(&[("docs/migrations.md", "b2")]);
    Arc::clone(&fleet).start_review_run(id.clone(), start(ReviewRunKind::Practices, "set-1", true)).await.expect("started");
    let third = practices_of(&run_of(&fleet, &id, ReviewRunKind::Practices).await);
    assert_eq!((third.reused, third.sources[0].blob.as_deref()), (false, Some("b2")));
    assert_eq!(judge.asked().len(), 4);
}

#[test]
fn a_practice_names_a_doc_in_the_checkout_and_a_check_covers_every_item_once() {
    let dir = TempDir::new();
    std::fs::create_dir_all(dir.path().join("docs")).expect("docs");
    std::fs::write(dir.path().join("docs/migrations.md"), "x").expect("written");
    let read = |answer: &str, items: usize| practices_read(answer, items, dir.path());
    let missing = read(r#"{"items":[{"item":"I","source":"docs/none.md","verdict":"met","note":"n"}]}"#, 0).unwrap_err();
    assert!(missing.contains("docs/none.md"), "{missing}");
    let short = read(r#"{"checks":[{"item":1,"verdict":"met","note":"n"}]}"#, 2).unwrap_err();
    assert!(short.contains("item 2"), "{short}");
    let twice = read(r#"{"checks":[{"item":1,"verdict":"met","note":"n"},{"item":1,"verdict":"missed","note":"n"}]}"#, 1).unwrap_err();
    assert!(twice.contains("twice"), "{twice}");
    assert!(read(r#"{"items":[]}"#, 0).is_ok(), "no doc states a practice: an answer");
}

#[tokio::test]
async fn what_changed_writes_one_paragraph_kept_for_the_range_it_read() {
    const SINCE: &str = "1a2b3c4d1a2b3c4d1a2b3c4d1a2b3c4d1a2b3c4d";
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying(r#"{"body":"The list now reads orders from their own table."}"#));
    let fleet = a_fleet_reading(&home, Arc::clone(&judge), the_diff());
    let (id, _) = manifest(&fleet);
    fleet.vcs().now_pull_request_compare(
        SINCE,
        HEAD,
        ComparedPatches::Patches(vec![PatchedFile { path: "src/orders.rs".into(), patch: Some("@@ -1 +1 @@\n-a\n+b".into()), blob: None }]),
    );
    let opened = StartReviewRun { since: Some(SINCE.into()), ..start(ReviewRunKind::Explanation, "what-changed", false) };

    Arc::clone(&fleet).start_review_run(id.clone(), opened.clone()).await.expect("started");
    let run = run_of(&fleet, &id, ReviewRunKind::Explanation).await;
    assert_eq!(run.result, Some(ReviewResult::Text { body: "The list now reads orders from their own table.".into() }));

    // Opened again at the same head, it is answered by what was kept.
    let again = Arc::clone(&fleet).start_review_run(id.clone(), opened).await.expect("answered");
    assert_eq!((again.state, again.since.as_deref()), (ReviewRunState::Done, Some(SINCE)));
    assert_eq!(judge.asked().len(), 1);
    assert!(crate::review_runs::answer_read("paragraph", r#"{"body":"One.\n\nTwo."}"#, &[], Path::new("/"), None, None).is_err());
}

/// What a practices result says, flattened for asserting.
struct Practised {
    set: String,
    kind: ChangeSetKind,
    area: String,
    reused: bool,
    read_at: String,
    sources: Vec<PracticeSource>,
    verdicts: Vec<PracticeVerdict>,
}

fn practices_of(run: &ReviewRun) -> Practised {
    match &run.result {
        Some(ReviewResult::Practices { set, kind, area, read_at, reused, sources, checks }) => Practised {
            set: set.clone(),
            kind: *kind,
            area: area.clone(),
            reused: *reused,
            read_at: read_at.clone(),
            sources: sources.clone(),
            verdicts: checks.iter().map(|check| check.verdict).collect(),
        },
        other => panic!("practices, not {other:?} ({:?})", run.failed),
    }
}
