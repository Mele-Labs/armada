//! Saved issue queries and the query writer, through a real Fleet, a scripted tracker and a
//! scripted model. **No model is called and no tracker is reached**: `FakeJudge` prints the JSON a
//! test wrote and records the brief it was given, and `FakeTracker` records the queries it was asked.

use std::sync::Arc;

use adapter_traits::FromOutside;
use api::{Issues, Refusal};
use ipc::{IssueQueries, ManifestId, SaveIssueQuery, SearchIssues, WireError, WriteIssueQuery};
use testkit::{FakeHarness, FakeJudge, FakeTracker, FakeTrackers, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

type Hosted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const WORDS: &str = "what I should look at this week";
const WRITTEN: &str = r#"{"query":"state = open assignee = me","row_fields":["Labels"]}"#;

fn manifest(fleet: &Hosted) -> ManifestId {
    let served = fleet.repositories().first().expect("a served repository");
    ManifestId::carried(served.manifest().id().as_str())
}

fn a_fleet(home: &TempDir, tracker: &Arc<FakeTracker>, judge: &Arc<FakeJudge>) -> Hosted {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&[]));
    fittings.trackers = Arc::new(FakeTrackers::empty().tracker(Arc::clone(tracker)));
    fittings.judge = Arc::clone(judge) as _;
    Fleet::assembled(fittings)
}

/// A fleet whose Manifest names a tracker provider this build has no word for, so the tracker
/// refuses every call and says why.
fn a_fleet_with_an_unreadable_tracker(home: &TempDir, judge: &Arc<FakeJudge>) -> Hosted {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&[]));
    fittings.trackers = Arc::new(adapters::TrackersByProvider);
    fittings.judge = Arc::clone(judge) as _;
    fittings.starting().manifest = config::Manifest::parse(
        std::path::Path::new("armada.yml"),
        "version: 1\nid: 01FIXTUREMANIFEST\ntracker:\n  provider: nowhere\n",
    )
    .expect("loads");
    Fleet::assembled(fittings)
}

fn write(words: &str) -> WriteIssueQuery {
    WriteIssueQuery { words: words.to_string() }
}

fn save(id: Option<&str>, name: &str, query: &str, row_fields: &[&str]) -> SaveIssueQuery {
    SaveIssueQuery {
        id: id.map(String::from),
        name: name.to_string(),
        words: WORDS.to_string(),
        query: query.to_string(),
        row_fields: row_fields.iter().map(|one| one.to_string()).collect(),
    }
}

fn error_of(refusal: Refusal) -> (&'static str, WireError) {
    match refusal {
        Refusal::Unacceptable(error) => ("unacceptable", error),
        Refusal::Fault(error) => ("fault", error),
        Refusal::NotFound(error) => ("not found", error),
        other => panic!("not one of these: {other:?}"),
    }
}

async fn saved_names(fleet: &Hosted) -> Vec<String> {
    let listed = fleet.list_issue_queries(manifest(fleet)).await.expect("listed");
    listed.saved.into_iter().map(|one| one.name).collect()
}

// ------------------------------------------------------------------ the writer

#[tokio::test]
async fn a_written_query_is_run_before_it_is_answered_and_nothing_is_kept() {
    let home = TempDir::new();
    let tracker = Arc::new(FakeTracker::empty());
    tracker.finds(vec![FakeTracker::issue(7, "Found by the query")]);
    let judge = Arc::new(FakeJudge::saying(WRITTEN).costing(0.1));
    let fleet = a_fleet(&home, &tracker, &judge);

    let written = fleet.write_issue_query(manifest(&fleet), write(WORDS)).await.expect("written");

    assert_eq!(tracker.searches(), ["state = open assignee = me"], "the tracker ran what the model wrote");
    assert_eq!(written.query, "state = open assignee = me");
    assert_eq!(written.row_fields, ["Labels"]);
    assert_eq!(written.issues.iter().map(|one| one.reference.as_str()).collect::<Vec<_>>(), ["#7"]);
    assert_eq!(written.cost_micros, Some(100_000));
    assert_eq!(judge.asked().len(), 1);
    assert!(saved_names(&fleet).await.is_empty(), "nothing is kept until he saves");
}

#[tokio::test]
async fn a_query_the_tracker_refuses_fails_the_write_with_its_words_and_costs_the_one_call() {
    let home = TempDir::new();
    let tracker = Arc::new(FakeTracker::empty());
    tracker.will_not_list("Error in the JQL Query: unknown field 'banana'");
    let judge = Arc::new(FakeJudge::saying(WRITTEN));
    let fleet = a_fleet(&home, &tracker, &judge);

    let refused = fleet.write_issue_query(manifest(&fleet), write(WORDS)).await.expect_err("refused");

    let (kind, error) = error_of(refused);
    assert_eq!((kind, error.code.as_str()), ("unacceptable", "fleet.issue_query_refused"));
    assert!(error.message.contains("unknown field 'banana'"), "{}", error.message);
    assert_eq!(judge.asked().len(), 1, "the one call was spent");
    assert_eq!(tracker.searches(), ["state = open assignee = me"]);
}

#[tokio::test]
async fn a_row_field_the_guide_does_not_list_fails_the_write_before_the_tracker_is_asked() {
    let home = TempDir::new();
    let tracker = Arc::new(FakeTracker::empty());
    let judge = Arc::new(FakeJudge::saying(r#"{"query":"state = open","row_fields":["Labels","Mood"]}"#));
    let fleet = a_fleet(&home, &tracker, &judge);

    let (kind, error) =
        error_of(fleet.write_issue_query(manifest(&fleet), write(WORDS)).await.expect_err("refused"));

    assert_eq!((kind, error.code.as_str()), ("fault", "fleet.issue_query_not_written"));
    assert!(error.message.contains("`Mood` is not a field this tracker fills: Labels"), "{}", error.message);
    assert!(tracker.searches().is_empty());
}

#[tokio::test]
async fn an_answer_that_is_not_a_query_fails_the_write_in_its_own_words() {
    let home = TempDir::new();
    let tracker = Arc::new(FakeTracker::empty());
    for answer in ["I could not decide.", r#"{"query":"  ","row_fields":[]}"#] {
        let judge = Arc::new(FakeJudge::saying(answer));
        let fleet = a_fleet(&home, &tracker, &judge);
        let (kind, error) =
            error_of(fleet.write_issue_query(manifest(&fleet), write(WORDS)).await.expect_err("refused"));
        assert_eq!((kind, error.code.as_str()), ("fault", "fleet.issue_query_not_written"), "{answer}");
        assert!(error.message.contains("the answer was refused"), "{}", error.message);
    }
    assert!(tracker.searches().is_empty());
}

#[tokio::test]
async fn the_model_is_given_the_guide_and_his_words_and_no_ticket_text_and_no_tools() {
    let home = TempDir::new();
    let mut ticket = FakeTracker::issue(7, "SECRETTITLE of a ticket");
    ticket.excerpt = FromOutside::verbatim("SECRETBODY of a ticket");
    let tracker = Arc::new(FakeTracker::listing(vec![ticket.clone()]));
    tracker.finds(vec![ticket]);
    let judge = Arc::new(FakeJudge::saying(WRITTEN));
    let fleet = a_fleet(&home, &tracker, &judge);

    fleet.write_issue_query(manifest(&fleet), write(WORDS)).await.expect("written");

    let asked = judge.asked();
    assert_eq!(asked.len(), 1);
    let brief = &asked[0];
    let guide = FakeTracker::guide();
    for given in [WORDS, guide.language.as_str(), guide.guide.as_str(), guide.open.as_str(), guide.assigned_to_me.as_str(), "Labels"] {
        assert!(brief.contains(given), "the brief lacks {given:?}: {brief}");
    }
    assert!(!brief.contains("SECRETTITLE") && !brief.contains("SECRETBODY"), "ticket text reached the model");
    assert_eq!(judge.models(), [fleet.review_model()], "models.review, as Natural Search uses");
    assert_eq!(judge.directories(), [None], "a call that can read nothing");
}

#[tokio::test]
async fn blank_words_cost_no_call_and_a_tracker_that_cannot_be_read_costs_none_either() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying(WRITTEN));
    let fleet = a_fleet(&home, &Arc::new(FakeTracker::empty()), &judge);
    let (kind, error) = error_of(fleet.write_issue_query(manifest(&fleet), write("  \n ")).await.expect_err("blank"));
    assert_eq!((kind, error.code.as_str()), ("unacceptable", "fleet.issue_query_blank"));

    let other_home = TempDir::new();
    let unreadable = a_fleet_with_an_unreadable_tracker(&other_home, &judge);
    let (kind, error) =
        error_of(unreadable.write_issue_query(manifest(&unreadable), write(WORDS)).await.expect_err("refused"));
    assert_eq!((kind, error.code.as_str()), ("fault", "fleet.issues_unreadable"));
    assert!(error.message.contains("nowhere"), "{}", error.message);
    assert!(judge.asked().is_empty());
}

// ------------------------------------------------------------------ saving

#[tokio::test]
async fn a_save_runs_the_query_and_keeps_it_without_asking_the_model() {
    let home = TempDir::new();
    let tracker = Arc::new(FakeTracker::empty());
    let judge = Arc::new(FakeJudge::saying(WRITTEN));
    let fleet = a_fleet(&home, &tracker, &judge);

    let kept = fleet
        .save_issue_query(manifest(&fleet), save(None, "This week", "state = open assignee = me", &["Labels"]))
        .await
        .expect("saved");

    assert_eq!(tracker.searches(), ["state = open assignee = me"]);
    assert!(judge.asked().is_empty(), "saving makes no model call");
    assert_eq!((kept.name.as_str(), kept.words.as_str(), kept.row_fields.clone()), ("This week", WORDS, vec!["Labels".to_string()]));
    assert!(!kept.id.is_empty());
    assert_eq!(saved_names(&fleet).await, ["This week"]);
}

#[tokio::test]
async fn a_save_the_tracker_refuses_is_refused_with_its_words_and_keeps_nothing() {
    let home = TempDir::new();
    let tracker = Arc::new(FakeTracker::empty());
    tracker.will_not_list("Error in the JQL Query: expected a field");
    let fleet = a_fleet(&home, &tracker, &Arc::new(FakeJudge::saying(WRITTEN)));

    let (kind, error) = error_of(
        fleet.save_issue_query(manifest(&fleet), save(None, "Typo", "state =", &[])).await.expect_err("refused"),
    );

    assert_eq!((kind, error.code.as_str()), ("unacceptable", "fleet.issue_query_refused"));
    assert!(error.message.contains("expected a field"), "{}", error.message);
    tracker.answers_again();
    assert!(saved_names(&fleet).await.is_empty());
}

#[tokio::test]
async fn a_save_with_an_unknown_row_field_or_a_blank_name_or_query_is_refused() {
    let home = TempDir::new();
    let tracker = Arc::new(FakeTracker::empty());
    let fleet = a_fleet(&home, &tracker, &Arc::new(FakeJudge::saying(WRITTEN)));
    for (name, query, fields, code) in [
        ("Mood", "state = open", &["Mood"][..], "fleet.issue_query_refused"),
        ("  ", "state = open", &[][..], "fleet.issue_query_blank"),
        ("Empty", "  ", &[][..], "fleet.issue_query_blank"),
    ] {
        let (kind, error) = error_of(
            fleet.save_issue_query(manifest(&fleet), save(None, name, query, fields)).await.expect_err("refused"),
        );
        assert_eq!((kind, error.code.as_str()), ("unacceptable", code), "{name} {query}");
    }
    assert!(tracker.searches().is_empty(), "none of them got as far as the tracker");
}

#[tokio::test]
async fn a_save_by_id_edits_that_query_in_place_and_an_unknown_id_is_a_404() {
    let home = TempDir::new();
    let tracker = Arc::new(FakeTracker::empty());
    let fleet = a_fleet(&home, &tracker, &Arc::new(FakeJudge::saying(WRITTEN)));
    let first = fleet.save_issue_query(manifest(&fleet), save(None, "First", "state = open", &[])).await.expect("saved");
    fleet.save_issue_query(manifest(&fleet), save(None, "Second", "assignee = me", &[])).await.expect("saved");

    let edited = fleet
        .save_issue_query(manifest(&fleet), save(Some(&first.id), "First, by hand", "state = open text ~ web", &["Labels"]))
        .await
        .expect("edited");

    assert_eq!(edited.id, first.id);
    let listed = fleet.list_issue_queries(manifest(&fleet)).await.expect("listed").saved;
    let shown: Vec<(&str, &str)> = listed.iter().map(|one| (one.name.as_str(), one.query.as_str())).collect();
    assert_eq!(shown, [("First, by hand", "state = open text ~ web"), ("Second", "assignee = me")]);
    assert_eq!(listed[0].row_fields, ["Labels"]);

    let (kind, error) = error_of(
        fleet.save_issue_query(manifest(&fleet), save(Some("nope"), "X", "state = open", &[])).await.expect_err("none"),
    );
    assert_eq!((kind, error.code.as_str()), ("not found", "fleet.no_such_issue_query"));
}

#[tokio::test]
async fn a_forgotten_query_is_gone_and_forgetting_it_again_is_a_404() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, &Arc::new(FakeTracker::empty()), &Arc::new(FakeJudge::saying(WRITTEN)));
    let first = fleet.save_issue_query(manifest(&fleet), save(None, "First", "state = open", &[])).await.expect("saved");
    fleet.save_issue_query(manifest(&fleet), save(None, "Second", "assignee = me", &[])).await.expect("saved");

    let left: IssueQueries = fleet.forget_issue_query(manifest(&fleet), first.id.clone()).await.expect("forgotten");

    assert_eq!(left.saved.iter().map(|one| one.name.as_str()).collect::<Vec<_>>(), ["Second"]);
    let (kind, error) = error_of(fleet.forget_issue_query(manifest(&fleet), first.id).await.expect_err("gone"));
    assert_eq!((kind, error.code.as_str()), ("not found", "fleet.no_such_issue_query"));
}

#[tokio::test]
async fn saved_queries_survive_a_restart_of_fleet() {
    let home = TempDir::new();
    let tracker = Arc::new(FakeTracker::empty());
    let judge = Arc::new(FakeJudge::saying(WRITTEN));
    {
        let fleet = a_fleet(&home, &tracker, &judge);
        fleet.save_issue_query(manifest(&fleet), save(None, "Kept", "state = open", &["Labels"])).await.expect("saved");
    }

    let again = a_fleet(&home, &tracker, &judge);

    let listed = again.list_issue_queries(manifest(&again)).await.expect("listed").saved;
    assert_eq!(listed.iter().map(|one| one.name.as_str()).collect::<Vec<_>>(), ["Kept"]);
    assert_eq!(listed[0].row_fields, ["Labels"]);
}

// ------------------------------------------------------------------ listing and running

#[tokio::test]
async fn the_guide_is_summarised_for_a_person_without_the_prose_a_model_reads() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, &Arc::new(FakeTracker::empty()), &Arc::new(FakeJudge::saying(WRITTEN)));

    let listed = fleet.list_issue_queries(manifest(&fleet)).await.expect("listed");

    let guide = listed.guide.expect("a guide");
    assert_eq!(listed.refused, None);
    assert_eq!(
        (guide.language.as_str(), guide.open.as_str(), guide.assigned_to_me.as_str(), guide.fields.clone()),
        ("Fake search", "state = open", "state = open assignee = me", vec!["Labels".to_string()])
    );
    let wire = ipc::encode(&guide).expect("plain data");
    assert!(!wire.contains(&FakeTracker::guide().guide), "{wire}");
}

#[tokio::test]
async fn an_unreadable_tracker_answers_its_refusal_with_no_guide_and_still_lists_what_was_saved() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying(WRITTEN));
    let fleet = a_fleet_with_an_unreadable_tracker(&home, &judge);
    // A query saved while the tracker was readable stays his to forget.
    fleet
        .store()
        .lock()
        .await
        .keep_issue_query(
            &store::KeptIssueQuery {
                id: "q1".into(),
                manifest_id: manifest(&fleet).as_str().into(),
                name: "Old".into(),
                words: String::new(),
                query: "state = open".into(),
                row_fields: Vec::new(),
            },
            "2026-10-23T10:00:00.000Z",
        )
        .expect("kept");

    let listed = fleet.list_issue_queries(manifest(&fleet)).await.expect("listed");

    assert_eq!(listed.guide, None);
    let refused = listed.refused.expect("a reason");
    assert!(refused.contains("nowhere"), "{refused}");
    assert_eq!(listed.saved.iter().map(|one| one.name.as_str()).collect::<Vec<_>>(), ["Old"]);
}

#[tokio::test]
async fn a_search_runs_the_query_as_given_and_lists_what_the_tracker_found_with_its_holders() {
    let home = TempDir::new();
    let tracker = Arc::new(FakeTracker::empty());
    tracker.finds(vec![FakeTracker::issue(7, "Found"), FakeTracker::issue(8, "Found too")]);
    let judge = Arc::new(FakeJudge::saying(WRITTEN));
    let fleet = a_fleet(&home, &tracker, &judge);
    let issue = ipc::IssueReference::read("8").expect("an issue");
    crate::session_issues::issue_held(
        &mut *fleet.store().lock().await,
        &store::Holder::session("job-a"),
        manifest(&fleet).as_str(),
        &issue,
        &Default::default(),
        "2026-10-23T10:00:00.000Z",
    )
    .expect("held");

    let list = fleet
        .search_issues(manifest(&fleet), SearchIssues { query: "text ~ found".into() })
        .await
        .expect("searched");

    assert_eq!(tracker.searches(), ["text ~ found"]);
    let seen: Vec<(&str, Vec<&str>)> = list
        .issues
        .iter()
        .map(|one| (one.reference.as_str(), one.holders.iter().map(String::as_str).collect()))
        .collect();
    assert_eq!(seen, [("#7", vec![]), ("#8", vec!["job-a"])]);
    assert!(judge.asked().is_empty(), "running a query asks no model");

    tracker.will_not_list("not signed in");
    let (kind, error) = error_of(
        fleet.search_issues(manifest(&fleet), SearchIssues { query: "x".into() }).await.expect_err("refused"),
    );
    assert_eq!((kind, error.code.as_str()), ("unacceptable", "fleet.issue_query_refused"));
    assert!(error.message.contains("not signed in"), "{}", error.message);
}

#[tokio::test]
async fn a_search_that_could_not_run_the_tool_names_it_on_the_wire_beside_the_words() {
    let home = TempDir::new();
    let tracker = Arc::new(FakeTracker::empty());
    let fleet = a_fleet(&home, &tracker, &Arc::new(FakeJudge::saying(WRITTEN)));

    tracker.lacks("bd");
    let (kind, error) = error_of(
        fleet.search_issues(manifest(&fleet), SearchIssues { query: "x".into() }).await.expect_err("refused"),
    );

    assert_eq!((kind, error.code.as_str()), ("unacceptable", "fleet.issue_query_refused"));
    assert_eq!(error.fields.get("missing_tool"), Some(&ipc::WireValue::Str("bd".into())));
    assert!(error.message.contains("not installed"), "{}", error.message);

    tracker.answers_again();
    tracker.will_not_list("not signed in");
    let (_, other) = error_of(
        fleet.search_issues(manifest(&fleet), SearchIssues { query: "x".into() }).await.expect_err("refused"),
    );
    assert_eq!(other.fields.get("missing_tool"), None, "a refusal for another reason names no tool");
}

#[tokio::test]
async fn bd_is_installed_when_asked_and_nothing_else_is() {
    let home = TempDir::new();
    let trackers = Arc::new(FakeTrackers::empty());
    let mut fittings = fittings(&home, FakeWorkProduct::changed(&[]));
    fittings.trackers = Arc::clone(&trackers) as _;
    let fleet = Fleet::assembled(fittings);

    let done = fleet.install_tool(manifest(&fleet), ipc::InstallTool { tool: "bd".into() }).await.expect("installed");

    assert_eq!((done.tool.as_str(), done.done), ("bd", true));
    assert_eq!(trackers.installs(), ["bd"]);

    let (kind, error) = error_of(
        fleet.install_tool(manifest(&fleet), ipc::InstallTool { tool: "rm".into() }).await.expect_err("refused"),
    );
    assert_eq!((kind, error.code.as_str()), ("unacceptable", "fleet.tool_not_installable"));
    assert_eq!(trackers.installs(), ["bd"], "a tool Fleet does not know is not run");
}
