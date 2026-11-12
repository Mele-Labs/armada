//! Editing a message the person sent. Since 10 Nov 2026. **A stand-in process, as `session_host`
//! has**: what is read is the thread's row and what was written to the agent's input.

use std::sync::Arc;

use api::{HostedSessions, Sessions};
use ipc::{
    EditSessionMessage, SessionFact, SessionId, SessionOrigin, SessionReport, SessionRow, SessionVoice,
};

use super::session_host::{eventually, rig_placing, Rig};
use super::session_pause::{holding_slot_one, pause, resume};

fn code(refusal: api::Refusal) -> String {
    match refusal {
        api::Refusal::IllegalMove(why) | api::Refusal::Unacceptable(why) => why.code,
        other => panic!("not a refusal with a code: {other:?}"),
    }
}

fn edit(id: &SessionId, row: &str, text: &str) -> EditSessionMessage {
    EditSessionMessage { session_id: id.clone(), row_id: row.into(), text: text.into() }
}

/// The row id and the marks of the person's message whose words are `words`.
async fn his(rig: &Rig, id: &SessionId, words: &str) -> (String, Option<ipc::Instant>) {
    rig.rows(id)
        .await
        .into_iter()
        .find_map(|row| match row {
            SessionRow::Message { id, from: SessionVoice::You, text, edited_at, .. } if text == words => Some((id, edited_at)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no message of his reads {words:?}"))
}

/// Everything written to the `index`th process the stand-in started.
fn sent_to(rig: &Rig, index: usize) -> String {
    rig.stand_in.process(index).sent.lock().unwrap().join("\n")
}

#[tokio::test]
async fn a_message_already_written_is_corrected_and_the_agent_is_told_what_changed() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "rename the helper").await;
    let (row, _) = his(&rig, &id, "rename the helper").await;
    assert!(sent_to(&rig, 0).contains("rename the helper"), "it was written to the agent");

    Arc::clone(&rig.fleet).edit_session_message(edit(&id, &row, "  rename the helper to tidy  ")).await.expect("edited");

    let rows = rig.rows(&id).await;
    let kept = rows.iter().find(|one| one.id() == row).expect("the row stays");
    match kept {
        SessionRow::Message { text, edited_at, from, .. } => {
            assert_eq!(text, "rename the helper to tidy", "the new words, trimmed");
            assert!(edited_at.is_some(), "marked edited");
            assert_eq!(*from, SessionVoice::You);
        }
        other => panic!("not a message: {other:?}"),
    }
    let his_rows = rows.iter().filter(|one| matches!(one, SessionRow::Message { from: SessionVoice::You, .. })).count();
    assert_eq!(his_rows, 1, "no second message of his");
    assert!(
        !rows.iter().any(|one| matches!(one, SessionRow::Message { text, .. } if text.contains("edited a message"))),
        "the turn the agent is sent is no row of the thread"
    );
    eventually(|| async { sent_to(&rig, 0).contains("Act on the new version where it changes anything.") }).await;
    let said = sent_to(&rig, 0);
    assert!(said.contains("The owner edited a message they sent earlier."), "{said}");
    // The line is JSON, so a newline reads as its two characters.
    assert!(said.contains(r"Before:\n> rename the helper\n\nNow:\n> rename the helper to tidy\n\nAct on"), "{said}");
}

#[tokio::test]
async fn words_the_same_as_the_rows_are_no_change_and_send_nothing() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "keep it").await;
    let (row, _) = his(&rig, &id, "keep it").await;
    let before = sent_to(&rig, 0);

    Arc::clone(&rig.fleet).edit_session_message(edit(&id, &row, " keep it ")).await.expect("a no-op that succeeds");

    assert_eq!(his(&rig, &id, "keep it").await.1, None, "not marked");
    assert_eq!(sent_to(&rig, 0), before, "nothing was sent");
}

#[tokio::test]
async fn a_turn_kept_for_a_paused_jobs_resume_changes_in_place_and_the_agent_reads_only_the_new_words() {
    let rig = rig_placing(None);
    let id = holding_slot_one(&rig).await;
    rig.fleet.pause_session(pause(&id)).await.expect("paused");
    let starts = rig.stand_in.starts().len();
    rig.send(&id, "while paused").await;
    let (row, _) = his(&rig, &id, "while paused").await;

    Arc::clone(&rig.fleet).edit_session_message(edit(&id, &row, "while paused, but better")).await.expect("edited");

    let (_, marked) = his(&rig, &id, "while paused, but better").await;
    assert_eq!(marked, None, "changed in place, so not marked");
    assert_eq!(rig.fleet.store().lock().await.held_later_turns(id.as_str()).unwrap().len(), 1, "still one turn kept");
    assert_eq!(rig.stand_in.starts().len(), starts, "nothing woke the job");

    Arc::clone(&rig.fleet).resume_session(resume(&id)).await.expect("resumed");
    eventually(|| async { rig.stand_in.starts().len() == starts + 1 }).await;
    eventually(|| async { sent_to(&rig, starts).contains("while paused, but better") }).await;
    let said = sent_to(&rig, starts);
    assert!(!said.contains("edited a message"), "no correction: {said}");
    assert_eq!(said.matches("while paused").count(), 1, "the old words were not sent: {said}");
}

#[tokio::test]
async fn a_turn_held_behind_a_proposal_changes_in_place_and_is_sent_once() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "first").await;
    let runtime = rig.fleet.hosts().of(id.as_str());
    runtime.state().proposal.draining = true;
    rig.send_unsettled(&id, "second").await;
    let (row, _) = his(&rig, &id, "second").await;
    assert_eq!(runtime.state().proposal.held.len(), 1, "held");

    Arc::clone(&rig.fleet).edit_session_message(edit(&id, &row, "second, reworded")).await.expect("edited");

    assert_eq!(his(&rig, &id, "second, reworded").await.1, None, "not marked");
    assert_eq!(runtime.state().proposal.held.len(), 1, "still the one turn");
    assert!(!sent_to(&rig, 0).contains("second"), "nothing of it was sent yet");
    rig.fleet.held_sent(id.as_str()).await;
    let said = sent_to(&rig, 0);
    assert!(said.contains("second, reworded"), "{said}");
    assert_eq!(said.matches("second").count(), 1, "the old words were not sent: {said}");
    assert!(!said.contains("edited a message"), "{said}");
}

#[tokio::test]
async fn a_correction_waits_behind_the_turns_held_for_a_proposal() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "first").await;
    let (row, _) = his(&rig, &id, "first").await;
    let runtime = rig.fleet.hosts().of(id.as_str());
    runtime.state().proposal.draining = true;

    Arc::clone(&rig.fleet).edit_session_message(edit(&id, &row, "first, again")).await.expect("edited");

    assert!(his(&rig, &id, "first, again").await.1.is_some(), "marked: the agent has the first");
    assert_eq!(runtime.state().proposal.held.len(), 1, "queued behind the others");
    assert!(!sent_to(&rig, 0).contains("edited a message"));
    rig.fleet.held_sent(id.as_str()).await;
    assert!(sent_to(&rig, 0).contains("The owner edited a message they sent earlier."));
}

#[tokio::test]
async fn an_edit_is_refused_for_what_it_is_not_allowed_to_change() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "do it").await;
    let (row, _) = his(&rig, &id, "do it").await;
    let said = rig.fleet.instant();
    rig.fleet
        .row_put(
            id.as_str(),
            SessionRow::Message {
                id: "agent-row".into(),
                at: said,
                from: SessionVoice::Agent,
                text: "done".into(),
                files: Vec::new(),
                tags: Vec::new(),
                notice: None,
                queued: false,
                edited_at: None,
                sketches: Vec::new(),
            },
        )
        .await;
    let edit_of = |row: &str, text: &str| edit(&id, row, text);
    let refuse = |why: api::Refusal| code(why);

    let unknown = Arc::clone(&rig.fleet).edit_session_message(edit_of("no-such-row", "x")).await.unwrap_err();
    assert_eq!(refuse(unknown), "fleet.session_row_unknown");
    let not_a_message = rig.rows(&id).await.iter().find(|one| !matches!(one, SessionRow::Message { .. })).map(|one| one.id().to_string());
    if let Some(other) = not_a_message {
        let refused = Arc::clone(&rig.fleet).edit_session_message(edit_of(&other, "x")).await.unwrap_err();
        assert_eq!(refuse(refused), "fleet.session_row_unknown", "a row that is no message");
    }
    let agents = Arc::clone(&rig.fleet).edit_session_message(edit_of("agent-row", "x")).await.unwrap_err();
    assert_eq!(refuse(agents), "fleet.session_row_not_his");
    let empty = Arc::clone(&rig.fleet).edit_session_message(edit_of(&row, "  \n ")).await.unwrap_err();
    assert_eq!(refuse(empty), "fleet.session_message_empty");
    let nobody = Arc::clone(&rig.fleet).edit_session_message(edit(&SessionId::carried("zzzz"), &row, "x")).await.unwrap_err();
    assert_eq!(refuse(nobody), "fleet.no_such_session");
    assert_eq!(his(&rig, &id, "do it").await.1, None, "nothing changed");

    Arc::clone(&rig.fleet).close_session(ipc::CloseSession { session_id: id.clone() }).await.expect("closed");
    let ended = Arc::clone(&rig.fleet).edit_session_message(edit_of(&row, "x")).await.unwrap_err();
    assert_eq!(refuse(ended), "fleet.session_closed");
}

#[tokio::test]
async fn an_edit_in_a_terminal_job_is_refused() {
    let rig = rig_placing(None);
    const ID: &str = "bbbb-2222";
    rig.fleet
        .report_session(SessionReport {
            harness: adapters::HOSTED_HARNESS.into(),
            session_id: SessionId::carried(ID),
            fact: SessionFact::Started {
                cwd: rig.root.clone(),
                title: None,
                origin: SessionOrigin::Terminal,
                mod_version: None,
            },
        })
        .await
        .unwrap();

    let refused = Arc::clone(&rig.fleet).edit_session_message(edit(&SessionId::carried(ID), "r1", "x")).await.unwrap_err();

    assert_eq!(code(refused), "fleet.session_command_refused");
}

#[tokio::test]
async fn a_row_kept_before_edits_existed_still_reads_and_is_not_marked() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let old = r#"{"kind":"message","id":"old-1","at":"2026-11-01T10:00:00.000Z","from":{"kind":"you"},"text":"an old one"}"#;
    rig.fleet.store().lock().await.keep_session_row(id.as_str(), "old-1", old).expect("kept");

    let read = rig.fleet.rows_of(id.as_str()).await.expect("read");

    assert!(
        read.iter().any(|row| matches!(row, SessionRow::Message { id, text, edited_at: None, .. } if id == "old-1" && text == "an old one")),
        "{read:?}"
    );
}
