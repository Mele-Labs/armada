//! The claim, written before the code: **a note of a verdict may point at the words it is about, and a Job reads it
//! as `On "<quote>": <text>`; a note with no `on`, or a quote that is blank, reads as it always did. The newest
//! revision of an artifact he opened is kept, only moves forward, is served on `JobArtifact` as `seen`, and a revision
//! never kept is refused.**
//! `crates/ipc/operations/answer_items.toml`, `crates/ipc/operations/mark_artifact_seen.toml`.

use std::sync::Arc;

use api::{HostedSessions, Refusal, Sessions};
use ipc::{AnswerItems, ItemAnswer, ItemHow, ItemNote, ItemSettled, MarkArtifactSeen, NoteOn, SessionId, ShowSketch};

use super::page_actions::{a_job, page_holding, sent};
use super::page_sealed::rewritten;
use super::session_host::{rig_placing, Rig};
use crate::session_host::sketching;

const SCENE: &str = r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0,"title":"Bay"}],"edges":[]}"#;
const PAGE: &str = "<html><head><title>Flaky tests</title></head><body><h1>Flaky tests</h1><p>lease_clock drifts</p></body></html>";

fn note(text: &str, on: Option<(&str, &str)>) -> ItemNote {
    ItemNote { text: text.into(), on: on.map(|(quote, block)| NoteOn { quote: quote.into(), block: block.into() }) }
}

/// A sketch of `title` waits on him, and he sends it back with `notes`. What the Job was sent.
async fn sent_back_with(notes: Vec<ItemNote>) -> String {
    let rig = rig_placing(None);
    let (id, _at) = a_job(&rig).await;
    let body = format!(r#"{{"title":"Bay flow","session_id":"{}","scene":{SCENE}}}"#, id.as_str());
    rig.fleet.show_sketch(None, ipc::decode::<ShowSketch>("a sketch to show", body.as_bytes()).unwrap()).await.unwrap();
    let answer = ItemAnswer { session_id: id.clone(), item: "sketch:Bay flow".into(), how: ItemHow::SendBack, notes };
    let settled = Arc::clone(&rig.fleet).answer_items(AnswerItems { items: vec![answer] }, Vec::new()).await;
    assert_eq!(settled.outcomes[0].outcome, ItemSettled::Settled);
    sent(&rig.rows(&id).await).pop().expect("the Job was told")
}

#[tokio::test]
async fn an_anchored_note_reads_as_on_the_words_then_the_note() {
    let told = sent_back_with(vec![note("Too tall", Some(("the header", "header-1")))]).await;
    assert_eq!(told, sketching::sent_back("Bay flow", "1. On \"the header\": Too tall"));
}

#[tokio::test]
async fn a_plain_note_reads_as_it_always_did_and_numbers_beside_an_anchored_one() {
    let told = sent_back_with(vec![note("Bigger", None), note("  ", None), note("Align it", Some(("the list", "list-2")))]).await;
    assert_eq!(told, sketching::sent_back("Bay flow", "1. Bigger\n2. On \"the list\": Align it"));
}

#[tokio::test]
async fn a_quote_blank_after_trimming_reads_as_a_plain_note() {
    let told = sent_back_with(vec![note("Bigger", Some(("  ", "header-1")))]).await;
    assert_eq!(told, sketching::sent_back("Bay flow", "1. Bigger"));
}

#[test]
fn a_note_with_no_on_is_a_text_alone_on_the_wire() {
    let plain: ItemNote = ipc::decode("a note", br#"{"text":"Bigger"}"#).unwrap();
    assert_eq!(plain, note("Bigger", None));
    assert_eq!(ipc::encode(&plain).unwrap(), r#"{"text":"Bigger"}"#);
    let anchored: ItemNote = ipc::decode("a note", br#"{"text":"Too tall","on":{"quote":"the header","block":"header-1"}}"#).unwrap();
    assert_eq!(anchored, note("Too tall", Some(("the header", "header-1"))));
}

fn marking(id: &SessionId, path: &str, revision: u32) -> MarkArtifactSeen {
    MarkArtifactSeen { session_id: id.clone(), artifact: path.into(), revision }
}

async fn marked(rig: &Rig, mark: MarkArtifactSeen) -> Result<u32, Refusal> {
    rig.fleet.mark_artifact_seen(mark).await.map(|one| one.seen)
}

async fn seen_on(rig: &Rig, id: &SessionId, path: &str) -> Option<u32> {
    rig.fleet.get_job_artifacts(id.clone()).await.unwrap().into_iter().find(|one| one.id == path).unwrap().seen
}

#[tokio::test]
async fn seen_moves_forward_and_holds_on_an_older_revision_and_is_served_on_the_artifact() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    let path = page_holding(&rig, &id, at, PAGE).await;
    assert_eq!(seen_on(&rig, &id, &path).await, None, "he opened none");
    let before = sent(&rig.rows(&id).await).len();
    rewritten(&rig, &id, at, &path, &PAGE.replace("Flaky tests</h1>", "Flaky tests, again</h1>"), 2).await;
    rewritten(&rig, &id, at, &path, &PAGE.replace("Flaky tests</h1>", "Flaky tests, thrice</h1>"), 3).await;

    assert_eq!(marked(&rig, marking(&id, &path, 2)).await.unwrap(), 2);
    assert_eq!(seen_on(&rig, &id, &path).await, Some(2));
    assert_eq!(marked(&rig, marking(&id, &path, 3)).await.unwrap(), 3);
    assert_eq!(marked(&rig, marking(&id, &path, 1)).await.unwrap(), 3, "an older revision is answered with the current seen");
    assert_eq!(seen_on(&rig, &id, &path).await, Some(3));
    assert_eq!(sent(&rig.rows(&id).await).len(), before, "nothing is sent to the Job");
}

#[tokio::test]
async fn a_revision_never_kept_is_refused_and_so_is_an_artifact_not_held() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    let path = page_holding(&rig, &id, at, PAGE).await;

    for revision in [0, 2] {
        let refused = marked(&rig, marking(&id, &path, revision)).await.expect_err("refused");
        assert_eq!(refused.error().code, "fleet.artifact_revision_unknown");
    }
    assert_eq!(seen_on(&rig, &id, &path).await, None, "a refused mark keeps nothing");
    let refused = marked(&rig, marking(&id, "/nowhere.html", 1)).await.expect_err("refused");
    assert_eq!(refused.error().code, "fleet.artifact_not_held");
}
