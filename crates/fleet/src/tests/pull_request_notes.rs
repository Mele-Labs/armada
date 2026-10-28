//! A note the owner left on a pull request's line, resolved by the Job it was sent to and by no
//! other. `docs/concepts/pull-review.md`.

use api::{PullRequestPanel, Sessions};
use ipc::{
    NoteSide, NoteStatus, PullRequestNote, ResolvePullRequestNote, SavePullRequestNote, SendPullRequestNotes, SessionId,
};

use super::session_host::{rig_placing, Rig};

const OTHERS: &str = "0b0b0b0b-0000-0000-0000-000000000000";

fn draft(body: &str) -> SavePullRequestNote {
    SavePullRequestNote {
        id: None,
        path: Some("crates/fleet/src/lib.rs".to_string()),
        line: Some(12),
        side: Some(NoteSide::Right),
        body: body.to_string(),
    }
}

fn resolve(id: &SessionId, note_id: &str) -> ResolvePullRequestNote {
    ResolvePullRequestNote { note_id: note_id.to_string(), session_id: Some(id.clone()) }
}

async fn saved(rig: &Rig, body: &str) -> PullRequestNote {
    rig.fleet.save_pull_request_note(rig.manifest.clone(), 7, draft(body)).await.unwrap()
}

async fn sent(rig: &Rig, note: &PullRequestNote, to: &str) {
    let send = SendPullRequestNotes {
        note_ids: vec![note.id.clone()],
        session_id: SessionId::carried(to),
        title: "Fix the rail".to_string(),
    };
    rig.fleet.send_pull_request_notes(rig.manifest.clone(), 7, send).await.unwrap();
}

#[tokio::test]
async fn the_job_a_note_was_sent_to_resolves_it() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let note = saved(&rig, "  Too tight  ").await;
    assert_eq!((note.body.as_str(), note.status), ("Too tight", NoteStatus::Open));
    sent(&rig, &note, id.as_str()).await;

    let resolved = rig.fleet.resolve_pull_request_note(None, resolve(&id, &note.id)).await.unwrap();

    assert_eq!(resolved.status, NoteStatus::Done);
    let listed = rig.fleet.list_pull_request_notes(rig.manifest.clone(), 7).await.unwrap();
    assert_eq!(listed.notes[0].status, NoteStatus::Done, "kept done");
}

#[tokio::test]
async fn another_job_is_refused_and_the_note_stays_open() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let note = saved(&rig, "Too tight").await;
    sent(&rig, &note, OTHERS).await;

    let refused = rig.fleet.resolve_pull_request_note(None, resolve(&id, &note.id)).await.unwrap_err();

    assert_eq!(refused.error().code, "fleet.pull_request_note_refused");
    assert!(refused.error().message.contains("not sent to this job"), "{refused:?}");
    let listed = rig.fleet.list_pull_request_notes(rig.manifest.clone(), 7).await.unwrap();
    assert_eq!(listed.notes[0].status, NoteStatus::Open);
}

#[tokio::test]
async fn an_unsent_unknown_or_already_resolved_note_is_refused() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let unsent = saved(&rig, "Never sent").await;
    let refused = rig.fleet.resolve_pull_request_note(None, resolve(&id, &unsent.id)).await.unwrap_err();
    assert!(refused.error().message.contains("not sent to this job"), "{refused:?}");

    let refused = rig.fleet.resolve_pull_request_note(None, resolve(&id, "no-such-note")).await.unwrap_err();
    assert!(refused.error().message.contains("no note"), "{refused:?}");

    let once = saved(&rig, "Done once").await;
    sent(&rig, &once, id.as_str()).await;
    rig.fleet.resolve_pull_request_note(None, resolve(&id, &once.id)).await.unwrap();
    let again = rig.fleet.resolve_pull_request_note(None, resolve(&id, &once.id)).await.unwrap_err();
    assert!(again.error().message.contains("already resolved"), "{again:?}");
}

#[tokio::test]
async fn a_blank_note_is_refused_and_an_edit_waits_on_the_note_being_open_and_unsent() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let blank = rig.fleet.save_pull_request_note(rig.manifest.clone(), 7, draft("   ")).await.unwrap_err();
    assert_eq!(blank.error().code, "fleet.pull_request_note_invalid");

    let note = saved(&rig, "First").await;
    let edit = |body: &str| SavePullRequestNote { id: Some(note.id.clone()), path: None, line: None, side: None, body: body.to_string() };
    let edited = rig.fleet.save_pull_request_note(rig.manifest.clone(), 7, edit("Second")).await.unwrap();
    assert_eq!((edited.id.as_str(), edited.body.as_str()), (note.id.as_str(), "Second"));

    sent(&rig, &note, id.as_str()).await;
    let refused = rig.fleet.save_pull_request_note(rig.manifest.clone(), 7, edit("Third")).await.unwrap_err();
    assert_eq!(refused.error().code, "fleet.pull_request_note_invalid");

    let other = saved(&rig, "Removable").await;
    let left = rig.fleet.remove_pull_request_note(rig.manifest.clone(), 7, other.id).await.unwrap();
    assert_eq!(left.notes.len(), 1);
}
