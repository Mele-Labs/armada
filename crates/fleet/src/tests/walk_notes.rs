//! A Job's walk notes: what a person pointed at while walking its served mock.
//!
//! **The claim is that a note outlives the worktree and reaches the next
//! Drone once.** Each case stands a Job at a person's gate through a real
//! dispatch, the way `tests::sending_back` does, captures onto it, and reads
//! what `get_job` serves and what the waiting note says.

use std::path::Path;
use std::sync::Arc;

use api::{Commands, Queries, Refusal};
use core_model::JobId;
use testkit::FakeWorkProduct;

use crate::tests::reviewing::{a_fleet_reviewing_the_first_step, at_the_gate, Fixture};
use crate::tests::tmp::TempDir;

fn code(refusal: &Refusal) -> &str {
    match refusal {
        Refusal::NoSuchJob(e)
        | Refusal::IllegalMove(e)
        | Refusal::Unacceptable(e)
        | Refusal::Fault(e) => &e.code,
    }
}

fn a_walk_note(said: &str, staged: Option<&Path>) -> ipc::CaptureWalkNote {
    ipc::CaptureWalkNote {
        said: said.to_string(),
        capture: ipc::StudioCapture {
            component: None,
            owners: Vec::new(),
            selector: "main > button.save".to_string(),
            element: ipc::CaptureElement {
                tag: "button".to_string(),
                text: "Save".to_string(),
                label: None,
            },
            screen: None,
            layer: None,
            location: "/settings".to_string(),
            bounds: ipc::CaptureBounds {
                x: 10,
                y: 20,
                width: 30,
                height: 12,
            },
            window: ipc::CaptureWindow {
                width: 1440,
                height: 900,
            },
            styles: Default::default(),
            markup: "<button class=\"save\">Save</button>".to_string(),
            source: None,
            frame: None,
            served: Some(ipc::CaptureServed {
                run: "01RUN".to_string(),
                name: "mock".to_string(),
                address: "http://127.0.0.1:5173".to_string(),
            }),
        },
        frame: staged.map(|path| ipc::StagedFrame {
            staged_path: path.to_string_lossy().to_string(),
            width: 60,
            height: 24,
        }),
    }
}

async fn walked(home: &TempDir) -> (Arc<Fixture>, JobId) {
    let fleet = a_fleet_reviewing_the_first_step(home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job_id = at_the_gate(&fleet, home).await;
    (Arc::new(fleet), job_id)
}

async fn detail(fleet: &Fixture, job: &JobId) -> ipc::JobDetail {
    Queries::get_job(fleet, ipc::JobId::from(job))
        .await
        .expect("a Job that exists")
}

/// **Captured, kept on the Job, its frame under the machine directory.**
#[tokio::test]
async fn a_walk_note_is_kept_on_the_job_with_its_frame_outside_the_worktree() {
    let home = TempDir::new();
    let (fleet, job_id) = walked(&home).await;
    let staged = home.path().join("staged.png");
    std::fs::write(&staged, [0u8; 256]).expect("a frame to stage");

    let notes = fleet
        .capture_walk_note(
            ipc::JobId::from(&job_id),
            a_walk_note("this button is too small", Some(&staged)),
        )
        .await
        .expect("a person's capture");

    let [note] = notes.notes.as_slice() else {
        panic!("one note: {notes:?}");
    };
    assert_eq!(note.said, "this button is too small");
    assert_eq!(note.element, "button “Save”");
    assert_eq!(note.selector, "main > button.save");
    assert_eq!(note.location, "/settings");
    assert_eq!(note.served.as_ref().map(|s| s.name.as_str()), Some("mock"));
    assert!(!note.sent);
    let frame = note.frame.as_deref().expect("the frame Fleet kept");
    assert!(Path::new(frame).is_file(), "a file at {frame}");
    assert!(
        frame.starts_with(&fleet.host().walk_frames_dir),
        "kept under the machine directory: {frame}"
    );
    let job = fleet.load(&job_id).await.expect("the Job");
    let worktree = fleet.surviving_worktree(&job).expect("its worktree");
    assert!(
        !frame.starts_with(worktree.path()),
        "never inside the worktree, which is reclaimed: {frame}"
    );

    assert_eq!(
        detail(&fleet, &job_id).await.walk_notes,
        notes.notes,
        "`get_job` carries every note"
    );
}

/// A note that says nothing is refused, and nothing is kept.
#[tokio::test]
async fn a_blank_walk_note_is_refused() {
    let home = TempDir::new();
    let (fleet, job_id) = walked(&home).await;
    let refused = fleet
        .capture_walk_note(ipc::JobId::from(&job_id), a_walk_note("  \n ", None))
        .await
        .expect_err("a blank `said`");
    assert_eq!(code(&refused), "fleet.walk_note_blank");
    assert!(matches!(refused, Refusal::Unacceptable(_)));

    let heavy = home.path().join("heavy.png");
    std::fs::write(&heavy, vec![0u8; 4 * 1024 * 1024 + 1]).expect("a frame over the cap");
    let refused = fleet
        .capture_walk_note(ipc::JobId::from(&job_id), a_walk_note("said", Some(&heavy)))
        .await
        .expect_err("a frame over the cap");
    assert_eq!(code(&refused), "fleet.walk_frame_too_large");

    let missing = home.path().join("never-written.png");
    let refused = fleet
        .capture_walk_note(
            ipc::JobId::from(&job_id),
            a_walk_note("said", Some(&missing)),
        )
        .await
        .expect_err("a staged frame that is not there");
    assert_eq!(code(&refused), "fleet.walk_frame_unreadable");

    assert!(detail(&fleet, &job_id).await.walk_notes.is_empty());
}

/// **`with_walk_notes` hands every unsent note to the Drone and marks them
/// sent**; a sent note cannot then be removed, and a later send carries none
/// of them again.
#[tokio::test]
async fn changes_requested_with_walk_notes_carry_them_once_and_mark_them_sent() {
    let home = TempDir::new();
    let (fleet, job_id) = walked(&home).await;
    let wire = ipc::JobId::from(&job_id);
    for said in ["this button is too small", "the header is\ntoo tall"] {
        fleet
            .capture_walk_note(wire.clone(), a_walk_note(said, None))
            .await
            .expect("captured");
    }
    let removable = fleet
        .capture_walk_note(wire.clone(), a_walk_note("never mind", None))
        .await
        .expect("captured")
        .notes
        .pop()
        .expect("the newest");
    let left = fleet
        .remove_walk_note(
            wire.clone(),
            ipc::RemoveWalkNote {
                id: removable.id.clone(),
            },
        )
        .await
        .expect("an unsent note is removed");
    assert_eq!(left.notes.len(), 2);

    Commands::request_changes(
        Arc::clone(&fleet),
        wire.clone(),
        ipc::ChangesRequested {
            note: String::new(),
            with_walk_notes: true,
        },
    )
    .await
    .expect("a blank note carrying walk notes is taken");

    let sent_back = detail(&fleet, &job_id).await;
    let waiting = sent_back
        .redirect_waiting
        .as_ref()
        .map(|note| note.note.clone())
        .expect("the note waits for the next Drone");
    assert!(
        waiting.contains("What the person pointed at while walking the work"),
        "{waiting}"
    );
    assert!(waiting.contains("button “Save”"), "{waiting}");
    assert!(waiting.contains("`/settings`"), "{waiting}");
    assert!(waiting.contains("`main > button.save`"), "{waiting}");
    assert!(waiting.contains("this button is too small"), "{waiting}");
    assert!(waiting.contains("the header is\n   too tall"), "{waiting}");
    assert!(!waiting.contains("never mind"), "a removed note is gone");
    assert!(
        sent_back.walk_notes.iter().all(|note| note.sent),
        "{:?}",
        sent_back.walk_notes
    );

    let refused = fleet
        .remove_walk_note(
            wire.clone(),
            ipc::RemoveWalkNote {
                id: sent_back.walk_notes[0].id.clone(),
            },
        )
        .await
        .expect_err("a sent note stays");
    assert_eq!(code(&refused), "fleet.walk_note_sent");
    assert!(matches!(refused, Refusal::IllegalMove(_)));

    let unknown = fleet
        .remove_walk_note(
            wire,
            ipc::RemoveWalkNote {
                id: "01NOPE".into(),
            },
        )
        .await
        .expect_err("no such note");
    assert_eq!(code(&unknown), "fleet.no_such_walk_note");
}

/// **A blank note is still refused where there is nothing unsent to carry**,
/// with or without the flag.
#[tokio::test]
async fn a_blank_note_with_no_unsent_walk_notes_is_still_refused() {
    let home = TempDir::new();
    let (fleet, job_id) = walked(&home).await;
    let refused = Commands::request_changes(
        Arc::clone(&fleet),
        ipc::JobId::from(&job_id),
        ipc::ChangesRequested {
            note: "   ".to_string(),
            with_walk_notes: true,
        },
    )
    .await
    .expect_err("nothing to tell the Drone");
    assert!(matches!(refused, Refusal::Unacceptable(_)), "{refused:?}");
    assert!(detail(&fleet, &job_id).await.redirect_waiting.is_none());
}

/// A Job that has ended takes no walk notes.
#[tokio::test]
async fn an_ended_job_takes_no_walk_notes() {
    let home = TempDir::new();
    let (fleet, job_id) = walked(&home).await;
    fleet
        .as_ref()
        .kill_job(&job_id)
        .await
        .expect("killed at the gate");
    let refused = fleet
        .capture_walk_note(ipc::JobId::from(&job_id), a_walk_note("too small", None))
        .await
        .expect_err("an ended Job");
    assert_eq!(code(&refused), "fleet.walk_notes_job_ended");
    assert!(matches!(refused, Refusal::IllegalMove(_)));
}
