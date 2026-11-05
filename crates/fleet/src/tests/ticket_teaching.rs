//! A tracker's own status taught as one Ticket state from the menu on a ticket: written to the Ticket
//! Kit keeps for the repository's Workspace, taken off any other state, put in the copy of every live
//! Job there, and read again at once. `crate::piece_saving::teaching`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use adapter_traits::{FromOutside, IssueField, ReadIssue};
use api::HostedSessions;
use core_model::{Lifecycle, LifecycleId, Observed, Standing, StateId};
use ipc::{PieceScope, SessionId, TeachTicketWord};
use store::{AttachmentState, PieceKind};
use testkit::{FakeTracker, FakeTrackers};

use super::lifecycle_keeping::holds;
use super::projects::a_second_repository;
use super::session_host::{rig_fitted, rig_placing, Rig};
use crate::tests::tmp::TempDir;

const WORD: &str = "Ready for QA";

fn plant(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("a folder")).expect("made");
    std::fs::write(path, text).expect("written");
}

fn kit(rig: &Rig, folder: &str) -> PathBuf {
    PathBuf::from(&rig.fleet.host().kit_home).join(folder)
}

/// A Workspace in Kit, `my-store`, holding the rig's repository.
fn in_a_workspace(rig: &Rig) {
    let text = format!("version: 1\nworkspace_id: my-store\nname: My store\nrepositories:\n  - {}\n", rig.manifest.as_str());
    plant(&kit(rig, "workspaces").join("my-store.yml"), &text);
}

fn teaching(rig: &Rig, state: &str) -> TeachTicketWord {
    TeachTicketWord { manifest_id: rig.manifest.clone(), word: WORD.to_string(), state: state.to_string() }
}

fn state(id: &str) -> StateId {
    StateId::new(id).expect("a state id")
}

fn ticket_in(lifecycles: &[Lifecycle]) -> Lifecycle {
    lifecycles.iter().find(|one| one.id().as_str() == "ticket").expect("a ticket").clone()
}

fn ticket_of(rig: &Rig) -> Lifecycle {
    let repository = rig.manifest.to_domain();
    let applying = rig.fleet.lifecycles_applying_to(&repository).expect("read");
    applying.get(&LifecycleId::new("ticket").expect("an id")).expect("a ticket").clone()
}

async fn ticket_of_job(rig: &Rig, job: &SessionId, manifest: &str) -> Lifecycle {
    let store = rig.fleet.store().lock().await;
    ticket_in(&rig.fleet.lifecycles_of_job(&store, job.as_str(), manifest))
}

/// `(key, from, to)` of every move a Job's tickets made.
async fn ticket_moves(rig: &Rig, job: &SessionId) -> Vec<(String, String, String)> {
    let store = rig.fleet.store().lock().await;
    let kept = store.lifecycle_moves_of(job.as_str()).expect("reads");
    kept.into_iter().filter(|one| one.lifecycle == "ticket").map(|one| (one.key, one.from, one.to)).collect()
}

#[tokio::test]
async fn teaching_a_status_writes_it_on_the_state_in_the_ticket_kit_keeps_for_the_workspace() {
    let rig = rig_placing(None);
    in_a_workspace(&rig);
    assert!(matches!(ticket_of(&rig).standing_of(&Observed::Tracker(WORD)), Standing::Unmapped(_)), "unmapped before");

    let taught = rig.fleet.ticket_word_taught(teaching(&rig, "in_review")).await.expect("taught");

    assert_eq!(taught.saved.scope, PieceScope::Within { id: "my-store".to_string() }, "the Workspace's, not the repository's");
    assert!(taught.saved.file.starts_with(&kit(&rig, "lifecycles").display().to_string()), "{}", taught.saved.file);
    assert!(!taught.saved.replaced, "Kit held no Ticket for the Workspace");
    assert_eq!(ticket_of(&rig).standing_of(&Observed::Tracker(WORD)), Standing::In(state("in_review")));
    assert_eq!(
        ticket_of(&rig).standing_of(&Observed::Tracker("closed")),
        Standing::In(state("closed")),
        "the words the Ticket already read carry over"
    );
}

#[tokio::test]
async fn teaching_a_status_takes_it_off_the_state_that_listed_it() {
    let rig = rig_placing(None);
    in_a_workspace(&rig);
    let file = kit(&rig, "lifecycles").join("tickets.yml");
    plant(
        &file,
        "version: 1\nlifecycle_id: ticket\nname: Ticket\nscope: my-store\nstates:\n  \
         - { id: in_progress, label: In progress, read: { tracker: [Doing, Ready for QA] } }\n  \
         - { id: in_review, label: In review, read: { tracker: [Review] } }\n  \
         - { id: closed, label: Closed, read: { tracker: [Done] } }\nends: [closed]\n",
    );

    let taught = rig.fleet.ticket_word_taught(teaching(&rig, "in_review")).await.expect("taught");

    assert!(taught.saved.replaced, "Kit's own file for the Workspace is the one edited");
    assert_eq!(taught.saved.file, file.display().to_string());
    let ticket = ticket_of(&rig);
    assert_eq!(ticket.standing_of(&Observed::Tracker(WORD)), Standing::In(state("in_review")));
    assert_eq!(ticket.standing_of(&Observed::Tracker("Doing")), Standing::In(state("in_progress")), "the rest of the state stays");
    assert_eq!(ticket.standing_of(&Observed::Tracker("Review")), Standing::In(state("in_review")));
}

#[tokio::test]
async fn teaching_a_state_the_ticket_lacks_or_a_blank_word_writes_nothing() {
    let rig = rig_placing(None);
    in_a_workspace(&rig);

    let unknown = rig.fleet.ticket_word_taught(teaching(&rig, "shipped")).await.expect_err("refused");
    let mut blank = teaching(&rig, "in_review");
    blank.word = "  ".to_string();
    let empty = rig.fleet.ticket_word_taught(blank).await.expect_err("refused");

    for (refusal, says) in [(unknown, "no state `shipped`"), (empty, "status is blank")] {
        match refusal {
            api::Refusal::Unacceptable(error) => {
                assert_eq!(error.code, "fleet.piece_unfit");
                assert!(error.message.contains(says), "{}", error.message);
            }
            other => panic!("not a 422: {other:?}"),
        }
    }
    let written = std::fs::read_dir(kit(&rig, "lifecycles")).map(Iterator::count).unwrap_or(0);
    assert_eq!(written, 0, "nothing was written");
}

#[tokio::test]
async fn a_live_jobs_copy_takes_the_word_and_its_unmapped_ticket_reads_in_review_at_once() {
    let rig = rig_placing(None);
    in_a_workspace(&rig);
    let job = rig.start().await;
    let at = rig.fleet.now().as_str().to_string();
    rig.fleet.store().lock().await.take_on_lifecycle_instance(job.as_str(), "ticket", "ARM-41", &[], &at).expect("taken on");
    rig.fleet.lifecycle_seen(job.as_str(), "ticket", "ARM-41", Observed::Tracker(WORD)).await.expect("read");

    let taught = rig.fleet.ticket_word_taught(teaching(&rig, "in_review")).await.expect("taught");

    assert_eq!(taught.jobs, [job.clone()]);
    let copy = rig.fleet.store().lock().await.job_copy_of(job.as_str()).expect("reads").expect("a copy");
    assert!(
        copy.pieces.iter().any(|piece| piece.kind == PieceKind::Lifecycle && piece.piece_id == "ticket" && piece.text.contains(WORD)),
        "the Job's own copy holds the word: {:?}",
        copy.pieces
    );
    let ticket = ticket_of_job(&rig, &job, rig.manifest.as_str()).await;
    assert_eq!(ticket.standing_of(&Observed::Tracker(WORD)), Standing::In(state("in_review")));
    assert_eq!(
        ticket_moves(&rig, &job).await.last().cloned(),
        Some(("ARM-41".to_string(), format!("unmapped:{WORD}"), "in:in_review".to_string())),
        "the ticket moved without waiting for the tracker's next read"
    );
}

#[tokio::test]
async fn a_job_in_a_repository_outside_the_workspace_keeps_its_ticket_as_it_was() {
    let rig = rig_placing(None);
    let home = TempDir::new();
    let second = a_second_repository(&rig.fleet, &home);
    in_a_workspace(&rig);
    let outside = Arc::clone(&rig.fleet)
        .start_session(ipc::StartSession {
            manifest_id: second.clone(),
            title: None,
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
        })
        .await
        .expect("started")
        .id;

    let taught = rig.fleet.ticket_word_taught(teaching(&rig, "in_review")).await.expect("taught");

    assert!(!taught.jobs.contains(&outside), "{:?}", taught.jobs);
    let copy = rig.fleet.store().lock().await.job_copy_of(outside.as_str()).expect("reads").unwrap_or_default();
    assert!(copy.pieces.iter().all(|piece| piece.piece_id != "ticket"), "{:?}", copy.pieces);
    let ticket = ticket_of_job(&rig, &outside, second.as_str()).await;
    assert!(matches!(ticket.standing_of(&Observed::Tracker(WORD)), Standing::Unmapped(_)));
}

#[tokio::test]
async fn the_issue_row_carries_the_status_in_the_trackers_own_words() {
    let tracker = Arc::new(FakeTracker::empty());
    let mut read = ReadIssue::bare("ARM-41".into(), FromOutside::verbatim("t"), FromOutside::verbatim("u"), false);
    read.fields.push(IssueField { name: "Status".into(), values: vec![FromOutside::verbatim(WORD)] });
    tracker.now_knows(read);
    let held_tracker = Arc::clone(&tracker);
    let rig = rig_fitted(move |fittings| fittings.trackers = Arc::new(FakeTrackers::empty().tracker(held_tracker)));
    let job = rig.start().await;
    holds(&rig, &job, "issue", "ARM-41", AttachmentState::Standing, &[]).await;

    rig.fleet.issue_ledger_kept_current(&rig.manifest, &rig.root).await;

    let rows = rig.fleet.store().lock().await.attachments_at("issue", "ARM-41", Some(rig.manifest.as_str())).expect("reads");
    let row = rows.iter().find(|row| row.holder.id == job.as_str()).expect("the Job's row");
    assert_eq!(row.detail.get("status").map(String::as_str), Some(WORD));
    assert_eq!(row.detail.get("state").map(String::as_str), Some("open"), "the open or closed it already said stands beside it");
}
