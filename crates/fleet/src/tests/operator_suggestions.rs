//! What the Operator would do with a note, as the board lists it and one press settles it: listed
//! oldest first and apart from any Job's own, Do it doing it and Skip leaving it, each in `done`, and the
//! board told on both. A Job's own suggestions settle on its list.

use std::sync::Arc;
use std::time::Duration;

use api::{Next, Refusal};
use ipc::operator_board::{AnswerOperatorSuggestion, OperatorBoard};
use ipc::Event;
use store::KeptSuggestion;

use super::backlog::manifest;
use super::operator_notes::{filing, leave, read, TEXT};
use super::operator_sources::Answers;
use super::session_host::{eventually, Rig};
use crate::operator::{Acting, Arrival, Arrived, FleetActing, Ruling, Source, Way};

const SIDEBAR: &str = "The sidebar should remember its width";

fn code(refused: Refusal) -> (u16, String) {
    (refused.status(), refused.error().code.clone())
}

fn press(id: &str, choice: u32) -> AnswerOperatorSuggestion {
    AnswerOperatorSuggestion { id: id.to_string(), choice }
}

async fn standing(rig: &Rig) -> Vec<KeptSuggestion> {
    rig.fleet.store().lock().await.operator_suggestions_of("").expect("read")
}

/// Notes left one after another, each ruled by the next of `rulings` and kept as a suggestion of no Job.
async fn suggested(rulings: &[&str], notes: &[(&str, &str)]) -> Rig {
    let rig = filing().await;
    assert!(rig.fleet.operator_seated_with(Answers::with(rulings)));
    rig.fleet.sources_handed().await;
    for (at, (id, text)) in notes.iter().enumerate() {
        leave(&rig, id, "open", None, text);
        rig.fleet.sources_handed().await;
        eventually(|| async { standing(&rig).await.len() == at + 1 }).await;
    }
    rig
}

const FILE: &str = "item: 1\nway: 1\nbecause: no open issue covers it";
// Once the note is filed, the issue it became is open and near the next, so a link is offered first.
const FILE_BESIDE_A_LINK: &str = "item: 1\nway: 2\nbecause: no open issue covers it";

async fn board(rig: &Rig) -> OperatorBoard {
    rig.fleet.operator_board_wire().await.expect("read")
}

#[tokio::test]
async fn the_board_lists_the_suggestions_no_job_holds_oldest_first_with_do_it_and_skip() {
    let rig = suggested(&[FILE, FILE], &[("n-1", TEXT), ("n-2", SIDEBAR)]).await;
    let kept = standing(&rig).await;

    // A Job's own suggestion is on its list, not the board's.
    let job = KeptSuggestion { arrival: "note:elsewhere".into(), session_id: "01JOB".into(), n: 0, raised_at: "2026-11-05T09:00:00.000Z".into(), ..kept[0].clone() };
    rig.fleet.store().lock().await.keep_operator_suggestion(&job).expect("kept");

    let listed = board(&rig).await.suggestions;
    assert_eq!(listed.iter().map(|one| one.id.clone()).collect::<Vec<_>>(), [format!("suggestion:{}", kept[0].n), format!("suggestion:{}", kept[1].n)], "oldest first");
    assert_eq!(listed[0].text, format!("The Operator would file your note \"{TEXT}\" as a new issue: no open issue covers it"));
    assert_eq!(listed[1].text, format!("The Operator would file your note \"{SIDEBAR}\" as a new issue: no open issue covers it"));
    assert_eq!(listed[0].at, kept[0].raised_at);
    for one in &listed {
        assert_eq!(one.options.iter().map(|option| option.label.as_str()).collect::<Vec<_>>(), ["Do it", "Skip"]);
    }
}

#[tokio::test]
async fn do_it_files_the_note_and_the_board_drops_it_and_says_what_was_filed() {
    let rig = suggested(&[FILE], &[("n-1", TEXT)]).await;
    let id = format!("suggestion:{}", standing(&rig).await[0].n);
    assert_eq!(board(&rig).await.suggestions.len(), 1);

    let after = Arc::clone(&rig.fleet).suggestion_answered(press(&id, 0)).await.expect("his press");

    assert!(after.suggestions.is_empty(), "{after:?}");
    let issue = rig.fleet.open_issues(&manifest(&rig)).await.expect("read").into_iter().find(|one| one.title.as_written() == TEXT).expect("filed");
    assert_eq!(after.done.iter().map(|one| one.outcome.clone()).collect::<Vec<_>>(), [format!("Filed: {}", issue.reference)], "the Operator's own row for it is not a second one");
    assert_eq!(read(&rig, "n-1").status, "done");
    let log = rig.fleet.store().lock().await.operator_rulings(10).expect("read");
    assert_eq!(log.iter().filter(|row| row.by == "human").map(|row| (row.outcome.as_str(), row.issue.as_deref())).collect::<Vec<_>>(), [("file", Some(issue.reference.as_str()))]);

    let again = Arc::clone(&rig.fleet).suggestion_answered(press(&id, 0)).await.expect_err("settled");
    assert_eq!(code(again), code(rig.fleet.settled_already()), "a second press is a 409");
    let other_way = Arc::clone(&rig.fleet).suggestion_answered(press(&id, 1)).await.expect_err("settled");
    assert_eq!(other_way.status(), 409);
}

#[tokio::test]
async fn do_it_on_a_go_ahead_to_start_a_job_kept_before_new_jobs_started_by_themselves_starts_it_and_done_names_the_job() {
    let rig = suggested(&[FILE], &[("n-1", TEXT)]).await;
    rig.agents_start("at_once");
    let filed = standing(&rig).await.remove(0);
    rig.fleet.store().lock().await.settle_operator_suggestion(filed.n, false, "2026-11-12T08:00:00.000Z").expect("settled");
    let old = KeptSuggestion { n: 0, way: "new_job".into(), what: None, text: format!("The Operator would start a Job on your note \"{TEXT}\": no Job is near it"), ..filed };
    rig.fleet.store().lock().await.keep_operator_suggestion(&old).expect("kept");
    let id = format!("suggestion:{}", standing(&rig).await[0].n);

    let after = Arc::clone(&rig.fleet).suggestion_answered(press(&id, 0)).await.expect("his press");

    assert_eq!(after.done.iter().map(|one| one.outcome.as_str()).collect::<Vec<_>>(), [format!("Started: {TEXT}").as_str()]);
    assert!(read(&rig, "n-1").sent.is_some(), "the note went to the Job");
}

#[tokio::test]
async fn skip_leaves_the_note_and_done_says_skipped() {
    let rig = suggested(&[FILE], &[("n-1", TEXT)]).await;
    let id = format!("suggestion:{}", standing(&rig).await[0].n);

    let after = Arc::clone(&rig.fleet).suggestion_answered(press(&id, 1)).await.expect("his press");

    assert!(after.suggestions.is_empty());
    assert_eq!(after.done.iter().map(|one| (one.id.as_str(), one.outcome.as_str(), one.title.as_str())).collect::<Vec<_>>(), [(standing_arrival(&rig).as_str(), "Skipped", TEXT)]);
    assert!(!rig.fleet.open_issues(&manifest(&rig)).await.expect("read").iter().any(|one| one.title.as_written() == TEXT), "nothing is filed");
    assert_eq!(read(&rig, "n-1").status, "open");
    let log = rig.fleet.store().lock().await.operator_rulings(10).expect("read");
    assert_eq!(log.iter().filter(|row| row.by == "human").map(|row| row.outcome.as_str()).collect::<Vec<_>>(), ["skipped"]);
}

fn standing_arrival(rig: &Rig) -> String {
    format!("note:{}:n-1", manifest(rig).as_str())
}

#[tokio::test]
async fn an_unknown_id_a_jobs_own_suggestion_and_a_choice_that_is_neither_are_refused_and_nothing_is_settled() {
    let rig = suggested(&[FILE], &[("n-1", TEXT)]).await;
    let kept = standing(&rig).await.remove(0);
    let job = KeptSuggestion { arrival: "note:elsewhere".into(), session_id: "01JOB".into(), n: 0, ..kept.clone() };
    rig.fleet.store().lock().await.keep_operator_suggestion(&job).expect("kept");
    let jobs = rig.fleet.store().lock().await.operator_suggestions_of("01JOB").expect("read").remove(0);

    for id in ["suggestion:9999".to_string(), "suggestion:".to_string(), "nonsense".to_string(), format!("suggestion:{}", jobs.n)] {
        let refused = Arc::clone(&rig.fleet).suggestion_answered(press(&id, 0)).await.expect_err("no such suggestion");
        assert_eq!(code(refused), (404, "fleet.operator_suggestion_unknown".to_string()), "{id}");
    }
    for choice in [2, 7] {
        let refused = Arc::clone(&rig.fleet).suggestion_answered(press(&format!("suggestion:{}", kept.n), choice)).await.expect_err("neither");
        assert_eq!(code(refused).1, "fleet.session_waiting_empty", "{choice}");
    }
    assert_eq!(standing(&rig).await.len(), 1);
    assert!(rig.fleet.store().lock().await.operator_suggestion(jobs.n).expect("read").is_some(), "the Job's own stands");
}

async fn board_told(watching: &mut api::Subscription) -> Event {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match watching.next().await {
                Some(Next::Send(delivered)) if delivered.event.kind() == "operator.board_changed" => return delivered.event,
                Some(_) => continue,
                None => panic!("the stream ended"),
            }
        }
    })
    .await
    .expect("the board's change is told")
}

#[tokio::test]
async fn the_board_is_told_when_a_suggestion_is_added_and_when_one_is_settled() {
    let rig = filing().await;
    let mut watching = rig.fleet.events().subscribe();
    let arrival = Arrival {
        id: format!("note:{}:n-1", manifest(&rig).as_str()),
        arrived: Arrived::Note,
        source: Source::Person,
        session_id: String::new(),
        manifest_id: manifest(&rig).as_str().to_string(),
        subject: TEXT.to_string(),
        words: TEXT.to_string(),
        paths: Vec::new(),
        issues: Vec::new(),
        ways: Vec::new(),
        open: Vec::new(),
    };

    FleetActing::on(&rig.fleet).suggest(&arrival, &Ruling::alone(Way::File, "no open issue covers it")).await.expect("carried out");

    assert!(matches!(board_told(&mut watching).await, Event::OperatorBoardChanged(_)));
    eventually(|| async { standing(&rig).await.len() == 1 }).await;
    let id = format!("suggestion:{}", standing(&rig).await[0].n);

    // Skip does nothing else, so what is told next is the settling.
    let mut watching = rig.fleet.events().subscribe();
    Arc::clone(&rig.fleet).suggestion_answered(press(&id, 1)).await.expect("his press");
    assert!(matches!(board_told(&mut watching).await, Event::OperatorBoardChanged(_)));
}

/// A note left after the first, ruled by the next of the rulings seated, and waited on until the Log has its row.
async fn left_again(rig: &Rig, id: &str, text: &str) -> store::KeptRuling {
    leave(rig, id, "open", None, text);
    rig.fleet.sources_handed().await;
    let arrival = format!("note:{}:{id}", manifest(rig).as_str());
    eventually(|| async { rig.fleet.store().lock().await.operator_rulings(20).expect("read").iter().any(|row| row.arrival == arrival) }).await;
    rig.fleet.store().lock().await.operator_rulings(20).expect("read").into_iter().find(|row| row.arrival == arrival).expect("logged")
}

fn noted_as_a_repeat(row: &store::KeptRuling, of: &str) {
    assert_eq!(row.outcome, "noted", "{}", row.because);
    assert!(row.because.contains(of), "{}", row.because);
}

#[tokio::test]
async fn a_repeat_of_a_suggestion_still_standing_raises_nothing_and_is_noted_as_its_repeat() {
    let rig = suggested(&[FILE, FILE], &[("n-1", TEXT)]).await;

    let row = left_again(&rig, "n-2", &format!("  {}  ", TEXT.to_uppercase())).await;

    noted_as_a_repeat(&row, &standing_arrival(&rig));
    assert_eq!(standing(&rig).await.len(), 1, "no second suggestion");
    assert!(board(&rig).await.suggestions.len() == 1);
}

#[tokio::test]
async fn a_repeat_after_skip_raises_nothing() {
    let rig = suggested(&[FILE, FILE], &[("n-1", TEXT)]).await;
    let id = format!("suggestion:{}", standing(&rig).await[0].n);
    Arc::clone(&rig.fleet).suggestion_answered(press(&id, 1)).await.expect("his press");

    let row = left_again(&rig, "n-2", TEXT).await;

    noted_as_a_repeat(&row, &standing_arrival(&rig));
    assert!(standing(&rig).await.is_empty());
}

#[tokio::test]
async fn a_repeat_after_do_it_raises_nothing() {
    let rig = suggested(&[FILE, FILE_BESIDE_A_LINK], &[("n-1", TEXT)]).await;
    let id = format!("suggestion:{}", standing(&rig).await[0].n);
    Arc::clone(&rig.fleet).suggestion_answered(press(&id, 0)).await.expect("his press");

    let row = left_again(&rig, "n-2", TEXT).await;

    noted_as_a_repeat(&row, &standing_arrival(&rig));
    assert!(standing(&rig).await.is_empty());
}

#[tokio::test]
async fn a_different_subject_still_raises() {
    let rig = suggested(&[FILE, FILE], &[("n-1", TEXT), ("n-2", SIDEBAR)]).await;

    assert_eq!(standing(&rig).await.len(), 2);
}
