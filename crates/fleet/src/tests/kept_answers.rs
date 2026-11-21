//! The claim, written before the code: **an answered ask leaves his list at once, and his answer stays for its thread
//! card**, kept apart from what the Job is told: the item it answered, its question, how he answered, his own words
//! and what it was asked on. Takes take his Ask for changes as well as a pick, and the Job reads his words.
//! `.claude/decisions/2026-11-19-one-ask-one-item-one-place.md`.

use std::sync::Arc;

use api::HostedSessions;
use ipc::{
    AddPart, AnswerHow, AnswerPart, AnswerWaiting, AnsweredAsk, DismissWaiting, PartHow, PartState, SessionId, SetWaitingFor, WaitingAct,
    WaitingActKind, WaitingInput, WaitingOption,
};

use super::session_host::{rig_placing, Rig};

const TAKE_A: &str = r#"{"nodes":[{"id":"m","kind":"box","x":0,"y":0,"title":"Left"}],"edges":[]}"#;
const TAKE_B: &str = r#"{"nodes":[{"id":"m","kind":"box","x":0,"y":0,"title":"Right"}],"edges":[]}"#;

fn takes(id: &SessionId) -> AddPart {
    let body = format!(
        r#"{{"session_id":"{}","title":"Catalog","part":{{"id":"layout","kind":"takes","question":"Which layout reads better?","takes":[{{"id":"a","title":"Left","scene":{TAKE_A}}},{{"id":"b","title":"Right","scene":{TAKE_B}}}]}}}}"#,
        id.as_str()
    );
    ipc::decode("a part", body.as_bytes()).unwrap()
}

async fn record(rig: &Rig, id: &SessionId) -> ipc::SessionRecord {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session
}

/// His answers as `(item, question, how, words, pick)`.
fn said(answers: &[AnsweredAsk]) -> Vec<(&str, &str, AnswerHow, Option<&str>, Option<&str>)> {
    answers.iter().map(|one| (one.item.as_str(), one.question.as_str(), one.how, one.words.as_deref(), one.pick.as_deref())).collect()
}

fn told(rows: &[ipc::SessionRow]) -> Vec<String> {
    rows.iter()
        .filter_map(|row| match row {
            ipc::SessionRow::Message { from: ipc::SessionVoice::You, text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn takes_sent_back_with_his_words_pick_none_and_the_job_reads_what_to_change() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.add_part(None, takes(&id)).await.unwrap();
    let layout = rig.fleet.get_job_artifacts(id.clone()).await.unwrap()[0].parts[0].clone();

    let back = AnswerPart {
        session_id: id.clone(),
        artifact: "sketch:Catalog".into(),
        part: "layout".into(),
        drawn: layout.drawn.clone(),
        how: PartHow::SentBack,
        take: None,
        words: Some("Neither: put the price first.".into()),
        notes: Default::default(),
        extras: Default::default(),
    };
    let artifact = Arc::clone(&rig.fleet).answer_part(back).await.expect("takes take an Ask for changes");
    assert_eq!(artifact.parts[0].state, PartState::SentBack);
    let after = record(&rig, &id).await;
    assert!(after.waiting_for.iter().all(|one| one.id != "sketch:Catalog"), "answered, it leaves his list at once");
    assert!(told(&rig.rows(&id).await).iter().any(|one| one.contains("\"layout\"") && one.contains("Neither: put the price first.")));

    let answer = after.answers.last().expect("his answer is kept");
    assert_eq!(
        said(&after.answers),
        [("sketch:Catalog", "Which layout reads better?", AnswerHow::AskedForChanges, Some("Neither: put the price first."), None)]
    );
    assert_eq!((answer.artifact.as_deref(), answer.part.as_deref()), (Some("sketch:Catalog"), Some("layout")), "the card opens what it asked on");
}

#[tokio::test]
async fn his_pick_and_his_dismiss_of_a_jobs_items_are_kept_apart_from_what_the_job_is_told() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let option = |label: &str| WaitingOption { label: label.into(), description: None, picture: None };
    let item = |key: &str, text: &str| WaitingInput {
        id: key.into(),
        text: text.into(),
        act: Some(WaitingAct { kind: WaitingActKind::Answer, target: key.into(), on: None }),
        options: vec![option("Rows"), option("Cards")],
        approves: false,
        steps: Vec::new(),
        artifact: None,
    };
    let items = vec![item("layout", "Rows or cards for the list?"), item("colour", "Which accent colour?")];
    rig.fleet.set_waiting(None, SetWaitingFor { items, session_id: Some(id.clone()) }).await.unwrap();

    let pick = AnswerWaiting { session_id: id.clone(), item_id: "layout".into(), choice: Some(1), text: Some("Wider gaps.".into()), mode: None, fixed: None, extras: Default::default() };
    let after = Arc::clone(&rig.fleet).answer_waiting(pick).await.unwrap();
    assert!(after.waiting_for.iter().all(|one| one.id != "layout"), "answered, it leaves his list at once");
    assert_eq!(said(&after.answers), [("layout", "Rows or cards for the list?", AnswerHow::Picked, Some("Wider gaps."), Some("Cards"))]);

    let away = DismissWaiting { session_id: id.clone(), item_id: "colour".into(), reason: Some("Not now.".into()), misread: false };
    let after = Arc::clone(&rig.fleet).dismiss_waiting(away).await.unwrap();
    assert_eq!(said(&after.answers)[1], ("colour", "Which accent colour?", AnswerHow::Dismissed, Some("Not now."), None));
    assert!(after.attachments.iter().all(|one| one.kind != "answer"), "served typed, not as ledger rows");
    assert!(told(&rig.rows(&id).await).iter().any(|one| one.contains("Cards")), "the Job is told as before");
}
