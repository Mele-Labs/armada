//! The claim, written before the code: **a draft knows the pull request whose description it
//! replaces.** Fleet links it where its `before` is the body of a pull request the Job holds,
//! whitespace aside, or where the Job names the pull request, and refuses a named one the Job does
//! not hold. The link is served on the Job's record and on the pull request's page, and goes when
//! the draft is settled, however it is. `crates/ipc/operations/propose_draft.toml`.

use std::sync::Arc;

use api::{HostedSessions, PullRequestPanel, Refusal, Sessions};
use ipc::{AnswerWaiting, ManifestId, ProposeDraft, PullRequestDraft, SessionId, SessionRow, WaitingSource, WithdrawDraft};
use store::{AttachmentState, Holder, KeptAttachment};

use super::session_host::{rig_placing, Rig};

const BODY: &str = "Takes a draft out.\n\n- one\n- two\n";
const TEXT: &str = "Takes a draft out, and says why.";

/// The Job's repository, as its record names it.
async fn repository(rig: &Rig, id: &SessionId) -> String {
    rig.fleet.store().lock().await.session(id.as_str()).unwrap().unwrap().manifest_id.expect("a repository")
}

/// The Job holds pull request 41 of its repository, whose body Fleet last read as `BODY`.
async fn holding_41(rig: &Rig, id: &SessionId) -> String {
    let manifest = repository(rig, id).await;
    let now = rig.fleet.instant().as_str().to_string();
    let pull = KeptAttachment {
        holder: Holder::session(id.as_str()),
        kind: "pr".into(),
        manifest_id: manifest.clone(),
        target: "41".into(),
        state: AttachmentState::Standing,
        detail: [("state".to_string(), "open".to_string()), ("body".to_string(), BODY.to_string())].into(),
        since: now.clone(),
        changed_at: now,
    };
    rig.fleet.store().lock().await.attach(&pull, false).unwrap();
    manifest
}

fn proposed(id: &SessionId, before: Option<&str>, pull_request: Option<u64>) -> ProposeDraft {
    ProposeDraft {
        title: "Description for #41".into(),
        text: TEXT.into(),
        before: before.map(str::to_string),
        pull_request,
        session_id: Some(id.clone()),
    }
}

fn row_of(record: &ipc::SessionRecord) -> String {
    let item = record.waiting_for.iter().find(|one| one.source == WaitingSource::Draft).expect("a draft waits");
    item.id.strip_prefix("draft:").unwrap().to_string()
}

fn on_41(manifest: &str, row: &str, before: Option<&str>, revision: u32) -> PullRequestDraft {
    PullRequestDraft {
        manifest_id: ManifestId::carried(manifest),
        number: 41,
        draft: row.to_string(),
        text: TEXT.into(),
        before: before.map(str::to_string),
        revision,
    }
}

#[tokio::test]
async fn a_draft_whose_before_is_a_held_pull_requests_body_whitespace_aside_is_linked_to_it() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let manifest = holding_41(&rig, &id).await;
    let before = "  Takes a draft out.\n- one\n\n- two";

    let record = rig.fleet.propose_draft(None, proposed(&id, Some(before), None)).await.unwrap();

    let row = row_of(&record);
    assert_eq!(record.pull_request_drafts, [on_41(&manifest, &row, Some(before.trim()), 1)]);
    assert!(record.attachments.iter().all(|one| one.kind != "pr_draft"), "served typed, not as a row: {:?}", record.attachments);
}

#[tokio::test]
async fn a_draft_whose_before_is_no_held_body_is_linked_to_nothing() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    holding_41(&rig, &id).await;

    let record = rig.fleet.propose_draft(None, proposed(&id, Some("Another description."), None)).await.unwrap();

    row_of(&record);
    assert!(record.pull_request_drafts.is_empty(), "{:?}", record.pull_request_drafts);
}

#[tokio::test]
async fn a_draft_naming_a_pull_request_the_job_holds_is_linked_to_it_with_no_before() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let manifest = holding_41(&rig, &id).await;

    let record = rig.fleet.propose_draft(None, proposed(&id, None, Some(41))).await.unwrap();

    assert_eq!(record.pull_request_drafts, [on_41(&manifest, &row_of(&record), None, 1)]);
}

#[tokio::test]
async fn a_draft_naming_a_pull_request_the_job_does_not_hold_is_refused_and_nothing_is_kept() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    holding_41(&rig, &id).await;

    let Err(Refusal::Unacceptable(why)) = rig.fleet.propose_draft(None, proposed(&id, Some(BODY), Some(42))).await else {
        panic!("a pull request the Job does not hold is a 422")
    };

    assert_eq!(why.code, "fleet.draft_pull_request_not_held");
    assert!(why.message.contains("#42"), "the reason names it: {}", why.message);
    let rows = rig.rows(&id).await;
    assert!(!rows.iter().any(|row| matches!(row, SessionRow::Draft { .. })), "nothing kept: {rows:?}");
}

#[tokio::test]
async fn approving_the_draft_drops_its_link() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    holding_41(&rig, &id).await;
    let record = rig.fleet.propose_draft(None, proposed(&id, None, Some(41))).await.unwrap();

    let said = AnswerWaiting {
        session_id: id.clone(),
        item_id: format!("draft:{}", row_of(&record)),
        choice: Some(0),
        text: None,
        mode: None,
        fixed: None,
    };
    let after = Arc::clone(&rig.fleet).answer_waiting(said).await.unwrap();

    assert!(after.pull_request_drafts.is_empty(), "{:?}", after.pull_request_drafts);
}

#[tokio::test]
async fn a_draft_sent_back_drops_its_link_and_the_next_one_is_its_second_revision() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let manifest = holding_41(&rig, &id).await;
    let first = rig.fleet.propose_draft(None, proposed(&id, Some(BODY), None)).await.unwrap();

    let note = AnswerWaiting {
        session_id: id.clone(),
        item_id: format!("draft:{}", row_of(&first)),
        choice: None,
        text: Some("Say what it fixes.".into()),
        mode: None,
        fixed: None,
    };
    let after = Arc::clone(&rig.fleet).answer_waiting(note).await.unwrap();
    assert!(after.pull_request_drafts.is_empty(), "{:?}", after.pull_request_drafts);

    let second = rig.fleet.propose_draft(None, proposed(&id, Some(BODY), None)).await.unwrap();
    assert_eq!(second.pull_request_drafts, [on_41(&manifest, &row_of(&second), Some(BODY.trim()), 2)]);
}

#[tokio::test]
async fn the_pull_requests_page_carries_the_draft_until_it_is_withdrawn() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let manifest = holding_41(&rig, &id).await;
    rig.fleet.vcs().now_pull_request_page(Some(super::pull_request_panel::a_page()));
    let record = rig.fleet.propose_draft(None, proposed(&id, None, Some(41))).await.unwrap();
    let row = row_of(&record);

    let page = rig.fleet.get_pull_request_detail(ManifestId::carried(&manifest), 41).await.unwrap();
    assert_eq!(page.draft, Some(on_41(&manifest, &row, None, 1)));

    let withdraw = WithdrawDraft { draft: format!("draft:{row}"), session_id: Some(id.clone()) };
    rig.fleet.withdraw_draft(None, withdraw).await.unwrap();
    let page = rig.fleet.get_pull_request_detail(ManifestId::carried(&manifest), 41).await.unwrap();
    assert_eq!(page.draft, None);
}
