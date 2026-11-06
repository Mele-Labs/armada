//! A note the person left on Bridge, as it reaches the Operator: once, and only if it came after the
//! first listing, open and sent to no Job by the time it has waited for him to send it. One he sends
//! to a Job is his row in the Log, and then the Operator's to move or leave. **An unsent one is only
//! ever proposed**: its ruling, but a raise, waits
//! on one press whatever the role stands at, and nothing is filed, started or told until Do it. Do it
//! marks the note sent to the Job it went to, or done where it was filed or linked.

use std::path::PathBuf;
use std::sync::Arc;

use ipc::annotations::{note_read, NoteRead};
use ipc::SessionId;
use testkit::{FakeBeads, FakeBeadsRemote};

use super::backlog::{machine, manifest};
use super::operator_sources::{on, rulings, switched, Answers};
use super::session_host::{eventually, Rig};

pub(super) const TEXT: &str = "The terminal scrollback is lost on redraw";

/// A rig over beads, so a note may be filed, with the Operator on and File on, and a note handed on
/// the tick it is read.
pub(super) async fn filing() -> Rig {
    let copy = Arc::new(FakeBeads::on(&FakeBeadsRemote::new()));
    let rig = machine("north", &copy).await;
    on(&rig).await;
    switched(&rig, "features.operatorFile", true).await;
    // An unsent note is handed at once, where a test does not mean to watch it.
    rig.fleet.operator_seat().note_wait_is(std::time::Duration::ZERO);
    rig
}

fn notes_dir(rig: &Rig) -> PathBuf {
    PathBuf::from(&rig.root).join(".armada").join("preview").join(".armada").join("annotations")
}

/// A note as Bridge writes it, with `sent` where it went to a Job.
pub(super) fn leave(rig: &Rig, id: &str, status: &str, sent: Option<&str>, text: &str) {
    std::fs::create_dir_all(notes_dir(rig)).expect("made");
    let sent = sent
        .map(|job| format!(r#""sent": {{"at": "2026-11-05T09:00:00.000Z", "sessionId": "{job}", "title": "Elsewhere"}}, "#))
        .unwrap_or_default();
    let note = format!(
        r#"{{"id": "{id}", "status": "{status}", {sent}"text": "{text}", "element": {{"tag": "p"}}, "screen": "usage", "updatedAt": "2026-11-05T09:00:00.000Z"}}"#
    );
    std::fs::write(notes_dir(rig).join(format!("{id}.json")), note).expect("written");
}

pub(super) fn read(rig: &Rig, id: &str) -> NoteRead {
    note_read(&std::fs::read(notes_dir(rig).join(format!("{id}.json"))).expect("there")).expect("a note")
}

/// The suggestion kept for the panel, which has no Job to list it on.
async fn kept(rig: &Rig) -> store::KeptSuggestion {
    let found = rig.fleet.store().lock().await.operator_suggestions_of("").expect("read");
    assert_eq!(found.len(), 1, "{found:#?}");
    found.into_iter().next().expect("one")
}

/// Do it on a suggestion that has no Job. The panel Job owns the press, so it is made here at Fleet.
async fn do_it(rig: &Rig, suggestion: &store::KeptSuggestion) {
    let _ = Arc::clone(&rig.fleet).suggestion_chosen("", &format!("suggestion:{}", suggestion.n), Some(0)).await;
}

#[tokio::test]
async fn a_note_new_after_the_first_listing_arrives_once_and_a_done_and_an_earlier_one_do_not() {
    let rig = filing().await;
    let call = Answers::with(&["item: 1\nway: 3\nbecause: it is for him to decide"]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    leave(&rig, "n-before", "open", None, "An idea from before Fleet started");

    // The first listing since Fleet started only marks what is there.
    rig.fleet.sources_handed().await;
    leave(&rig, "n-new", "open", None, TEXT);
    leave(&rig, "n-done", "done", None, "A note already done");
    rig.fleet.sources_handed().await;

    eventually(|| async { !rulings(&rig).await.is_empty() }).await;
    let asked = call.asked();
    assert_eq!(asked.len(), 1, "{asked:#?}");
    for part in ["The person left a note, by the person's own hand: \"The terminal scrollback is lost on redraw\"", "Left on the usage screen", "1. file a new issue; 2. new job; 3. raise\n"] {
        assert!(asked[0].contains(part), "{part}:\n{}", asked[0]);
    }
    for left_out in ["from before Fleet started", "already done"] {
        assert!(!asked[0].contains(left_out), "{left_out}:\n{}", asked[0]);
    }
    let log = rulings(&rig).await;
    assert_eq!(log.len(), 1, "{log:#?}");
    assert_eq!(
        (log[0].arrival.as_str(), log[0].arrived.as_str(), log[0].outcome.as_str()),
        (format!("note:{}:n-new", manifest(&rig).as_str()).as_str(), "note", "raise")
    );

    // Listed again, it is not ruled twice.
    rig.fleet.sources_handed().await;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert_eq!(call.asked().len(), 1);
    assert_eq!(rulings(&rig).await.len(), 1);
}

#[tokio::test]
async fn a_note_ruled_filed_is_kept_as_a_suggestion_with_file_on_and_filed_and_marked_done_only_on_do_it() {
    let rig = filing().await;
    let call = Answers::with(&["item: 1\nway: 1\nbecause: no open issue covers it"]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    rig.fleet.sources_handed().await;
    leave(&rig, "n-1", "open", None, TEXT);
    rig.fleet.sources_handed().await;

    eventually(|| async { rig.fleet.store().lock().await.operator_suggestions_of("").expect("read").len() == 1 }).await;
    let suggestion = kept(&rig).await;
    assert_eq!(suggestion.text, format!("The Operator would file your note \"{TEXT}\" as a new issue: no open issue covers it"));
    let filed = || async { rig.fleet.open_issues(&manifest(&rig)).await.expect("read").iter().any(|one| one.title.as_written() == TEXT) };
    assert!(!filed().await, "nothing is filed until he presses");
    assert_eq!(read(&rig, "n-1").status, "open");

    do_it(&rig, &suggestion).await;

    eventually(|| async { filed().await }).await;
    eventually(|| async { read(&rig, "n-1").status == "done" }).await;
    assert_eq!(read(&rig, "n-1").sent, None, "filed, so it went to no Job");
}

#[tokio::test]
async fn a_note_ruled_a_new_job_starts_it_on_do_it_and_is_marked_sent_to_it() {
    let rig = filing().await;
    let call = Answers::with(&["item: 1\nway: 2\nbecause: no Job is near it"]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    rig.fleet.sources_handed().await;
    leave(&rig, "n-1", "open", None, TEXT);
    rig.fleet.sources_handed().await;

    eventually(|| async { rig.fleet.store().lock().await.operator_suggestions_of("").expect("read").len() == 1 }).await;
    let suggestion = kept(&rig).await;
    assert_eq!(suggestion.text, format!("The Operator would start a Job on your note \"{TEXT}\": no Job is near it"));
    assert_eq!(read(&rig, "n-1").sent, None, "no Job until he presses");

    do_it(&rig, &suggestion).await;

    eventually(|| async { read(&rig, "n-1").sent.is_some() }).await;
    let job = read(&rig, "n-1").sent.expect("sent").session_id;
    let session = rig.fleet.store().lock().await.session(&job).expect("read").expect("a Job");
    assert_eq!(session.title.as_deref(), Some(TEXT));
    assert_eq!(read(&rig, "n-1").status, "open", "the Job marks it done when it finishes");
}

#[tokio::test]
async fn a_note_ruled_to_a_live_job_tells_it_on_do_it_and_is_marked_sent_to_it() {
    let rig = filing().await;
    let near: SessionId = rig.start_titled(Some("Terminal scrollback loss")).await;
    let ruling = format!("item: 1\nway: 2\njob: {}\nshared: terminal, scrollback\nbecause: its work is on the same fault", near.as_str());
    let call = Answers::with(&[ruling.as_str()]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    rig.fleet.sources_handed().await;
    leave(&rig, "n-1", "open", None, TEXT);
    rig.fleet.sources_handed().await;

    eventually(|| async { rig.fleet.store().lock().await.operator_suggestions_of("").expect("read").len() == 1 }).await;
    let suggestion = kept(&rig).await;
    assert_eq!(
        suggestion.text,
        format!("The Operator would send your note \"{TEXT}\" to \"Terminal scrollback loss\": its work is on the same fault")
    );
    assert_eq!(read(&rig, "n-1").sent, None, "nobody is told until he presses");

    do_it(&rig, &suggestion).await;

    eventually(|| async { read(&rig, "n-1").sent.is_some() }).await;
    let sent = read(&rig, "n-1").sent.expect("sent");
    assert_eq!(sent.session_id, near.as_str());
}

#[tokio::test]
async fn with_file_off_a_note_ruled_filed_is_kept_in_the_log_and_proposes_nothing() {
    let rig = filing().await;
    switched(&rig, "features.operatorFile", false).await;
    let call = Answers::with(&["item: 1\nway: 1\nbecause: no open issue covers it"]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    rig.fleet.sources_handed().await;
    leave(&rig, "n-1", "open", None, TEXT);
    rig.fleet.sources_handed().await;

    eventually(|| async { !rulings(&rig).await.is_empty() }).await;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert!(rig.fleet.store().lock().await.operator_suggestions_of("").expect("read").is_empty());
    assert_eq!(read(&rig, "n-1").status, "open");
}

/// A role stood at `word`, one of the positions.
pub(super) async fn positioned(rig: &Rig, key: &str, word: &str) {
    let save = ipc::SaveSettings { changes: std::collections::BTreeMap::from([(key.to_string(), Some(ipc::SettingValue::Text(word.to_string())))]) };
    api::Settings::save_settings(rig.fleet.as_ref(), save).await.expect("saved");
}

async fn board_done(rig: &Rig) -> Vec<ipc::operator_board::OperatorBoardSettled> {
    rig.fleet.operator_board_wire().await.expect("read").done
}

const SCROLLBACK_JOB: &str = "Terminal scrollback loss";

/// A note first listed, left open and unsent, and watched for `wait` before it is handed on.
async fn watched(wait: std::time::Duration, answers: &[&str]) -> (Rig, Arc<Answers>) {
    let rig = filing().await;
    rig.fleet.operator_seat().note_wait_is(wait);
    let call = Answers::with(answers);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    rig.fleet.sources_handed().await;
    leave(&rig, "n-1", "open", None, TEXT);
    rig.fleet.sources_handed().await;
    (rig, call)
}

#[tokio::test]
async fn an_open_note_sent_to_no_job_is_not_handed_until_it_has_waited_and_then_is_handed_as_it_stands() {
    let (rig, call) = watched(std::time::Duration::from_millis(600), &["item: 1\nway: 3\nbecause: it is for him to decide"]).await;

    // Read again each tick, and not handed while it may still be sent.
    rig.fleet.sources_handed().await;
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    assert!(call.asked().is_empty());
    assert!(rulings(&rig).await.is_empty());

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    rig.fleet.sources_handed().await;
    eventually(|| async { !rulings(&rig).await.is_empty() }).await;
    let log = rulings(&rig).await;
    assert_eq!((log[0].arrival.as_str(), log[0].by.as_str()), (format!("note:{}:n-1", manifest(&rig).as_str()).as_str(), "operator"));
    assert_eq!(call.asked().len(), 1);
    assert!(call.asked()[0].contains("1. file a new issue; 2. new job; 3. raise\n"), "as it stands, with no word of a send");
}

#[tokio::test]
async fn an_open_note_that_is_done_before_it_has_waited_is_never_handed() {
    let (rig, call) = watched(std::time::Duration::from_millis(300), &[]).await;
    leave(&rig, "n-1", "done", None, TEXT);
    rig.fleet.sources_handed().await;
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    rig.fleet.sources_handed().await;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert!(call.asked().is_empty());
    assert!(rulings(&rig).await.is_empty());
}

#[tokio::test]
async fn a_note_sent_to_a_job_is_his_row_in_the_log_and_the_operator_may_leave_it_where_he_sent_it() {
    let (rig, call) = watched(std::time::Duration::from_secs(600), &["item: 1\nway: 2\nbecause: the Job it went to is the right one"]).await;
    let job: SessionId = rig.start_titled(Some(SCROLLBACK_JOB)).await;
    assert!(call.asked().is_empty());

    leave(&rig, "n-1", "open", Some(job.as_str()), TEXT);
    rig.fleet.sources_handed().await;

    eventually(|| async { rulings(&rig).await.len() == 2 }).await;
    let log = rulings(&rig).await;
    let note = format!("note:{}:n-1", manifest(&rig).as_str());
    let sent = log.iter().find(|row| row.outcome == "sent").expect("his row");
    assert_eq!((sent.arrival.as_str(), sent.by.as_str(), sent.other_session.as_deref()), (note.as_str(), "human", Some(job.as_str())));
    let left = log.iter().find(|row| row.outcome == "leave").expect("the Operator's");
    assert_eq!((left.arrival.clone(), left.by.as_str()), (format!("note-sent:{}:n-1", manifest(&rig).as_str()), "operator"));

    let asked = call.asked();
    assert_eq!(asked.len(), 1);
    for part in ["The person sent it to Job \"Elsewhere\" from Bridge.", "1. new job; 2. leave it with the Job he sent it to\n"] {
        assert!(asked[0].contains(part), "{part}:\n{}", asked[0]);
    }

    // Only his send is Done: leaving it is nothing that happened.
    let done = board_done(&rig).await;
    assert_eq!(done.len(), 1, "{done:#?}");
    assert_eq!((done[0].outcome.as_str(), done[0].by), (format!("Sent: to {SCROLLBACK_JOB}").as_str(), ipc::operator_board::OperatorDecider::Person));
    assert_eq!(read(&rig, "n-1").sent.expect("still sent").session_id, job.as_str());
}

/// A note sent to one Job while another live Job works on the same fault, and the call ruling it to that other.
async fn redirected() -> (Rig, SessionId, SessionId) {
    let rig = filing().await;
    let sent_to: SessionId = rig.start_titled(Some("Sidebar spacing")).await;
    let other: SessionId = rig.start_titled(Some(SCROLLBACK_JOB)).await;
    let ruling = format!("item: 1\nway: 1\njob: {}\nshared: terminal, scrollback\nbecause: its work is on the same fault", other.as_str());
    rig.fleet.operator_seat().note_wait_is(std::time::Duration::from_secs(600));
    assert!(rig.fleet.operator_seated_with(Answers::with(&[ruling.as_str()])));
    rig.fleet.sources_handed().await;
    leave(&rig, "n-1", "open", None, TEXT);
    rig.fleet.sources_handed().await;
    leave(&rig, "n-1", "open", Some(sent_to.as_str()), TEXT);
    (rig, sent_to, other)
}

#[tokio::test]
async fn a_redirect_of_a_note_he_sent_is_a_suggestion_at_suggest_and_moves_the_note_only_on_do_it() {
    let (rig, sent_to, other) = redirected().await;
    positioned(&rig, "features.operatorDelegate", "suggest").await;

    rig.fleet.sources_handed().await;

    eventually(|| async { rig.fleet.store().lock().await.operator_suggestions_of("").expect("read").len() == 1 }).await;
    let suggestion = kept(&rig).await;
    assert_eq!(suggestion.text, format!("The Operator would send your note \"{TEXT}\" to \"{SCROLLBACK_JOB}\": its work is on the same fault"));
    assert_eq!(read(&rig, "n-1").sent.expect("sent").session_id, sent_to.as_str(), "nobody is told until he presses");
    assert_eq!(board_done(&rig).await.iter().map(|one| one.outcome.as_str()).collect::<Vec<_>>(), [format!("Sent: to Sidebar spacing")]);

    do_it(&rig, &suggestion).await;

    eventually(|| async { read(&rig, "n-1").sent.is_some_and(|sent| sent.session_id == other.as_str()) }).await;
    eventually(|| async { board_done(&rig).await.len() == 2 }).await;
    let done = board_done(&rig).await;
    assert_eq!((done[0].outcome.as_str(), done[0].by), (format!("Handed over: to {SCROLLBACK_JOB}").as_str(), ipc::operator_board::OperatorDecider::Person));
}

#[tokio::test]
async fn a_redirect_of_a_note_he_sent_is_done_at_on_and_its_row_is_the_operators() {
    let (rig, _sent_to, other) = redirected().await;

    rig.fleet.sources_handed().await;

    eventually(|| async { read(&rig, "n-1").sent.is_some_and(|sent| sent.session_id == other.as_str()) }).await;
    assert!(rig.fleet.store().lock().await.operator_suggestions_of("").expect("read").is_empty());
    eventually(|| async { board_done(&rig).await.len() == 2 }).await;
    let done = board_done(&rig).await;
    assert_eq!((done[0].outcome.as_str(), done[0].by), (format!("Handed over: to {SCROLLBACK_JOB}").as_str(), ipc::operator_board::OperatorDecider::Operator));
}
