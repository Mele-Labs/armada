//! A note on a screen goes to every live Job on the screen's features, and the first claim takes it; unclaimed, it goes
//! to the Job that last wrote the screen's files; with no Job live on the feature, Fleet starts one with his words.
//! `docs/concepts/feature-catalog.md`, *A note on a screen*.

use std::sync::Arc;
use std::time::Duration;

use api::{Features, HostedSessions, Sessions};
use ipc::{ClaimNote, NoteElement, NotePlace, NoteState, SendNotesToFeatures, SessionId, SessionNote, SessionRow, StartSession};

use super::agent_starts::{rig, Rig};
use super::feature_library::{a_project, held_slot, seeded, SOMEWHERE, VOTING};

const NOTE: &str = "mfz1-abc";

/// A Job on `project` called `title`, live and holding a slot, and the stand-in process it runs in.
async fn a_job_called(rig: &Rig, project: &str, title: &str) -> (SessionId, usize) {
    let start = StartSession {
        project: Some(project.to_string()),
        manifest_id: rig.manifest.clone(),
        title: Some(title.into()),
        model: None,
        effort: None,
        mode: None,
        root: None,
        fork: None,
        request: None,
        after: None,
        held: false,
        project_rows: Vec::new(),
        on_your_words: None,
        workflow: None,
    };
    let id = rig.fleet.start_session(start).await.expect("started").id;
    rig.send(&id, "go on").await;
    let process = rig.stand_in.starts().len() - 1;
    rig.stand_in.init(process);
    held_slot(rig, &id, &format!("slot-{process}")).await;
    (id, process)
}

/// The Job wrote `path` under the voting feature at `at`, as Fleet files an observed write.
async fn wrote(rig: &Rig, project: &str, id: &SessionId, path: &str, at: &str) {
    let mut store = rig.fleet.store().lock().await;
    let voting = store.features_in(project).expect("read").into_iter().find(|feature| feature.slug == VOTING).expect("the feature").id;
    store.keep_feature_write(&voting, id.as_str(), path, at).expect("kept");
}

fn sent(project: &str, rig: &Rig) -> SendNotesToFeatures {
    let note = SessionNote {
        said: "The rank chip clips".to_string(),
        address: "armada://native/Jobs".to_string(),
        note_id: Some(NOTE.to_string()),
        place: NotePlace { selector: "button \"Rank\"".into(), element: NoteElement { tag: "button".into(), text: "Rank".into(), label: None }, location: "/".into(), ..NotePlace::default() },
        state: NoteState::default(),
        crop: None,
        job: Vec::new(),
    };
    SendNotesToFeatures { manifest_id: rig.manifest.clone(), project: Some(project.to_string()), paths: vec![SOMEWHERE.to_string()], words: "The rank chip clips".into(), notes: vec![note] }
}

fn claim(id: &SessionId) -> ClaimNote {
    ClaimNote { note_id: NOTE.to_string(), session_id: Some(id.clone()) }
}

/// Fleet's words in the Job's thread that hold `part`.
async fn told(rig: &Rig, id: &SessionId, part: &str) -> bool {
    rig.rows(id).await.iter().any(|row| matches!(row, SessionRow::Message { text, .. } if text.contains(part)))
}

/// Two live Jobs on the voting feature.
async fn two_on_voting(rig: &Rig) -> (String, (SessionId, usize), (SessionId, usize)) {
    let project = a_project(rig).await;
    rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");
    let rail = a_job_called(rig, &project, "Fix the rail").await;
    let ranks = a_job_called(rig, &project, "Add rank votes").await;
    wrote(rig, &project, &rail.0, &format!("{SOMEWHERE}/rail.rs"), "2026-11-27T09:00:00.000Z").await;
    wrote(rig, &project, &ranks.0, "crates/pages/voting/ranks.rs", "2026-11-27T10:00:00.000Z").await;
    (project, rail, ranks)
}

#[tokio::test]
async fn a_note_goes_to_every_job_on_the_feature_and_the_first_claim_takes_it_and_the_others_are_told() {
    let rig = rig(None);
    let (project, (rail, rail_process), (ranks, ranks_process)) = two_on_voting(&rig).await;

    let routed = Arc::clone(&rig.fleet).send_notes_to_features(sent(&project, &rig)).await.expect("routed");
    assert_eq!(routed.features, [VOTING]);
    assert_eq!(routed.offered.iter().map(|job| job.title.as_str()).collect::<Vec<_>>(), ["Fix the rail", "Add rank votes"]);
    for process in [rail_process, ranks_process] {
        let read = rig.sent_to(process).join("\n");
        assert!(read.contains("The rank chip clips") && read.contains(&format!("call claim_note with note_id {NOTE}. The first claim takes it.")), "{read}");
    }
    assert!(rig.sent_to(rail_process).join("\n").contains("you and Add rank votes"), "the line names the other Job");

    let first = Arc::clone(&rig.fleet).claim_note(None, claim(&rail)).await.expect("the first claim takes it");
    assert_eq!((first.session_id.as_str(), first.title.as_str(), first.given), (rail.as_str(), "Fix the rail", false));
    let again = Arc::clone(&rig.fleet).claim_note(None, claim(&rail)).await.expect("its own claim again answers the same");
    assert_eq!(again.session_id, rail);

    let later = Arc::clone(&rig.fleet).claim_note(None, claim(&ranks)).await.expect_err("a later claim is refused");
    assert_eq!((later.status(), later.error().code.as_str()), (409, "fleet.note_taken"));
    assert!(later.error().message.contains("Fix the rail"), "{}", later.error().message);
    assert!(told(&rig, &ranks, &format!("Fix the rail claimed note {NOTE}, so it is theirs. Drop it.")).await, "the other Job is told who took it");
    assert!(!told(&rig, &rail, "Drop it.").await, "the Job that took it is not told to drop it");

    // Unclaimed no longer, it is given to nobody else when the wait is over.
    rig.fleet.notes_unclaimed_given(Duration::ZERO).await;
    assert_eq!(rig.fleet.store().lock().await.note_claim(NOTE).expect("read").expect("kept").claimed_by, rail.as_str());
}

#[tokio::test]
async fn a_note_nobody_claims_goes_to_the_job_that_last_wrote_the_screens_files() {
    let rig = rig(None);
    let (project, (rail, _), (ranks, _)) = two_on_voting(&rig).await;
    Arc::clone(&rig.fleet).send_notes_to_features(sent(&project, &rig)).await.expect("routed");

    rig.fleet.notes_unclaimed_given(Duration::from_secs(3600)).await;
    assert!(rig.fleet.store().lock().await.note_claim(NOTE).expect("read").expect("kept").claimed_by.is_empty(), "not before the wait is over");

    rig.fleet.notes_unclaimed_given(Duration::ZERO).await;
    let kept = rig.fleet.store().lock().await.note_claim(NOTE).expect("read").expect("kept");
    assert_eq!((kept.claimed_by.as_str(), kept.given), (ranks.as_str(), true), "Add rank votes wrote the screen's files last");
    assert!(told(&rig, &ranks, &format!("Nobody claimed note {NOTE}, so it is yours: you last wrote the files of the screen it was left on.")).await);
    assert!(told(&rig, &rail, &format!("Nobody claimed note {NOTE}, so Fleet gave it to Add rank votes. Drop it.")).await);

    let late = Arc::clone(&rig.fleet).claim_note(None, claim(&rail)).await.expect_err("taken");
    assert!(late.error().message.contains("Fleet gave it to Add rank votes"), "{}", late.error().message);
}

#[tokio::test]
async fn a_note_on_a_feature_no_job_is_on_starts_a_job_with_his_words_that_holds_it() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");
    let before = rig.stand_in.starts().len();

    let routed = Arc::clone(&rig.fleet).send_notes_to_features(sent(&project, &rig)).await.expect("routed");

    assert!(routed.offered.is_empty());
    let started = routed.started.expect("a Job was started");
    assert_eq!(started.title, "The rank chip clips", "titled with his words");
    assert_eq!(rig.record(&started.session_id).await.title.as_deref(), Some("The rank chip clips"));
    super::session_host::eventually(|| async { rig.stand_in.starts().len() > before }).await;
    assert!(rig.card_message(&started.session_id).await.is_none(), "started at once, with no start card held");
    let kept = rig.fleet.store().lock().await.note_claim(NOTE).expect("read").expect("kept");
    assert_eq!((kept.claimed_by.as_str(), kept.given), (started.session_id.as_str(), true));
}

#[tokio::test]
async fn paths_no_feature_owns_are_refused_so_bridge_offers_its_picker() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");
    let mut nowhere = sent(&project, &rig);
    nowhere.paths = vec!["crates/elsewhere".into()];
    let refused = Arc::clone(&rig.fleet).send_notes_to_features(nowhere).await.expect_err("no feature");
    assert_eq!((refused.status(), refused.error().code.as_str()), (422, "fleet.notes_no_feature"));
}
