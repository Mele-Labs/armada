//! The claim, written before the code: **a picture and a tag he sends with an answer reach the Job and stay on
//! his row, whichever box he answered in: Ask for changes on a document, on a part of an artifact, an answer to
//! an item, or one batched item.** The sketch's own box is `sketch_answers.rs`.

use std::sync::Arc;

use api::{HostedSessions, Sessions};
use ipc::{AnswerPart, AnswerWaiting, DocumentHow, PartHow, SessionId, SessionRow, SessionVoice, ShowSketch};

use super::document_answers::{answer as document_answer, one_page};
use super::session_host::rig_placing;

const SCENE: &str = r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0,"title":"Bay"}],"edges":[]}"#;

/// What he attached and tagged with an answer, as the row on the thread keeps it.
fn extras() -> ipc::AnswerExtras {
    ipc::AnswerExtras {
        attachments: vec![ipc::SessionUpload { name: "shot.png".into(), media_type: "image/png".into(), data: "aGVsbG8=".into() }],
        mentions: vec![ipc::SessionTag { kind: ipc::TagKind::Branch, id: "armada/other".into(), title: "armada/other".into(), url: None }],
    }
}

/// His row, the last he wrote: the picture and the tag it keeps.
fn his_row(rows: &[SessionRow]) -> (Vec<ipc::SentFile>, Vec<ipc::SessionTag>) {
    rows.iter()
        .rev()
        .find_map(|row| match row {
            SessionRow::Message { from: SessionVoice::You, files, tags, .. } => Some((files.clone(), tags.clone())),
            _ => None,
        })
        .expect("his message is on the thread")
}

fn kept_with_the_answer(rows: &[SessionRow]) {
    let (files, tags) = his_row(rows);
    assert_eq!(files.iter().map(|one| one.name.as_str()).collect::<Vec<_>>(), ["shot.png"], "the picture rides with the answer");
    assert_eq!(tags.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), ["armada/other"], "the tag rides with the answer");
}

// Proves: Ask for changes on a document sends the picture and tag to the Job and keeps them on his row.
#[tokio::test]
async fn a_picture_and_a_tag_sent_with_a_documents_ask_for_changes_reach_the_job_and_stay_on_his_row() {
    let rig = rig_placing(None);
    let (id, _work, _file, path) = one_page(&rig).await;
    let mut said = document_answer(&id, &path, "1", DocumentHow::SentBack, Some("Match this picture."));
    said.extras = extras();
    Arc::clone(&rig.fleet).answer_document(said).await.unwrap();

    kept_with_the_answer(&rig.rows(&id).await);
}

fn waiting_on(id: &SessionId) -> ipc::SetWaitingFor {
    ipc::SetWaitingFor {
        session_id: Some(id.clone()),
        items: vec![ipc::WaitingInput {
            approves: false,
            id: "tests".into(),
            text: "Run the tests".into(),
            act: Some(ipc::WaitingAct { kind: ipc::WaitingActKind::Run, target: "cargo test".into(), on: None }),
            options: vec![ipc::WaitingOption { label: "Yes".into(), description: None, picture: None }],
            steps: Vec::new(),
            artifact: None,
        }],
    }
}

// Proves: words, a picture and a tag sent to an agent's waiting item reach the Job and stay on his row.
#[tokio::test]
async fn a_picture_and_a_tag_sent_with_words_on_a_waiting_item_reach_the_job_and_stay_on_his_row() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.waiting_for(None, waiting_on(&id)).await.unwrap();
    let said = AnswerWaiting {
        session_id: id.clone(),
        item_id: "tests".into(),
        choice: None,
        text: Some("Like this one.".into()),
        mode: None,
        fixed: None,
        extras: extras(),
    };
    Arc::clone(&rig.fleet).answer_waiting(said).await.unwrap();

    kept_with_the_answer(&rig.rows(&id).await);
}

fn drawn(id: &SessionId, title: &str) -> ShowSketch {
    ipc::decode("a sketch", format!(r#"{{"title":"{title}","asks":false,"session_id":"{}","scene":{SCENE}}}"#, id.as_str()).as_bytes()).unwrap()
}

fn words_part(id: &SessionId, title: &str, part: &str, text: &str) -> ipc::AddPart {
    ipc::decode(
        "a part",
        format!(r#"{{"session_id":"{}","title":"{title}","question":"Does this cache design hold?","part":{{"id":"{part}","kind":"words","text":"{text}"}}}}"#, id.as_str()).as_bytes(),
    )
    .unwrap()
}

// Proves: Ask for changes on a words part of an artifact sends the picture and tag to the Job and keeps them on his row.
#[tokio::test]
async fn a_picture_and_a_tag_sent_with_ask_for_changes_on_a_part_reach_the_job_and_stay_on_his_row() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.show_sketch(None, drawn(&id, "Cache design")).await.unwrap();
    rig.fleet.add_part(None, words_part(&id, "Cache design", "notes", "Evict by age.")).await.unwrap();
    let held = rig.fleet.get_job_artifacts(id.clone()).await.unwrap();
    let notes = held[0].parts.iter().find(|part| part.id == "notes").expect("the words part");
    let said = AnswerPart {
        session_id: id.clone(),
        artifact: "sketch:Cache design".into(),
        part: "notes".into(),
        drawn: notes.drawn.clone(),
        how: PartHow::SentBack,
        take: None,
        words: Some("Change it.".into()),
        notes: Default::default(),
        extras: extras(),
    };
    Arc::clone(&rig.fleet).answer_part(said).await.unwrap();

    kept_with_the_answer(&rig.rows(&id).await);
}

// Proves: an agent item that asks for his approval, sent back inside a batch, carries the picture and tag to the Job.
#[tokio::test]
async fn a_picture_and_a_tag_sent_with_a_batched_item_reach_the_job_and_stay_on_his_row() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let mut waiting = waiting_on(&id);
    waiting.items[0].approves = true;
    waiting.items[0].options.clear();
    waiting.items[0].act = None;
    // An item that asks is worded as his question.
    waiting.items[0].text = "Run the tests now?".into();
    rig.fleet.waiting_for(None, waiting).await.unwrap();
    let item = ipc::ItemAnswer {
        session_id: id.clone(),
        item: "tests".into(),
        how: ipc::ItemHow::SendBack,
        notes: vec![ipc::ItemNote { text: "Match this picture.".into(), on: None }],
        extras: extras(),
    };
    let settled = Arc::clone(&rig.fleet).answer_items(ipc::AnswerItems { items: vec![item] }, Vec::new()).await;
    assert_eq!(settled.outcomes[0].outcome, ipc::ItemSettled::Settled, "{:?}", settled.outcomes);

    kept_with_the_answer(&rig.rows(&id).await);
}
