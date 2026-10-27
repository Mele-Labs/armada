//! The claim, written before the code: **text the person should OK waits on him as a draft.** It is
//! a `draft:<row>` item under its title; approving, fixing or sending it back reaches the session as
//! his message, with the exact text to use or the note, and settles the row so it waits no more.
//! `crates/ipc/operations/propose_draft.toml`.

use std::sync::Arc;

use api::{HostedSessions, Refusal, Sessions};
use ipc::{
    AnswerWaiting, DismissWaiting, DraftHow, ProposeDraft, SessionRow, SessionVoice, ShowWindow, WaitingActKind,
    WaitingSource,
};

use super::session_host::{rig_placing, Rig};

const TITLE: &str = "Description for #41";
const TEXT: &str = "Reads the draft row back as an item.";

fn proposed(id: &ipc::SessionId) -> ProposeDraft {
    ProposeDraft {
        title: format!("  {TITLE} "),
        text: format!("{TEXT}\n"),
        before: Some("Old description.".into()),
        session_id: Some(id.clone()),
    }
}

fn the_draft(record: &ipc::SessionRecord) -> &ipc::WaitingItem {
    record
        .waiting_for
        .iter()
        .find(|one| one.source == WaitingSource::Draft)
        .unwrap_or_else(|| panic!("no draft waits: {:?}", record.waiting_for))
}

fn sent(rows: &[SessionRow]) -> Vec<String> {
    rows.iter()
        .filter_map(|row| match row {
            SessionRow::Message { from: SessionVoice::You, text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn settled(rows: &[SessionRow]) -> Option<ipc::DraftSettled> {
    rows.iter().find_map(|row| match row {
        SessionRow::Draft { settled, .. } => settled.clone(),
        _ => None,
    })
}

async fn drafting(rig: &Rig) -> (ipc::SessionId, String) {
    let id = rig.start().await;
    let record = rig.fleet.propose_draft(None, proposed(&id)).await.unwrap();
    let item = the_draft(&record).id.clone();
    (id, item)
}

fn answer(id: &ipc::SessionId, item: &str) -> AnswerWaiting {
    AnswerWaiting { session_id: id.clone(), item_id: item.to_string(), choice: None, text: None, mode: None, fixed: None }
}

#[tokio::test]
async fn a_draft_with_no_title_or_no_text_is_refused_and_nothing_is_kept() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    for (title, text, named) in [("  ", TEXT, "title"), (TITLE, "\n ", "text")] {
        let blank = ProposeDraft { title: title.into(), text: text.into(), before: None, session_id: Some(id.clone()) };
        let Err(Refusal::Unacceptable(why)) = rig.fleet.propose_draft(None, blank).await else {
            panic!("a draft with a blank {named} is a 422")
        };
        assert_eq!(why.code, "fleet.draft_empty");
        assert!(why.message.contains(named), "the reason names the {named}: {}", why.message);
    }
    let rows = rig.rows(&id).await;
    assert!(!rows.iter().any(|row| matches!(row, SessionRow::Draft { .. })), "nothing kept: {rows:?}");
}

#[tokio::test]
async fn a_proposed_draft_waits_under_its_title_and_its_row_keeps_what_it_replaces() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let record = rig.fleet.propose_draft(None, proposed(&id)).await.unwrap();

    let item = the_draft(&record);
    assert!(item.id.starts_with("draft:"), "{}", item.id);
    assert_eq!(item.text, TITLE, "the receipt finds the item by its trimmed title");
    assert_eq!(item.options.iter().map(|one| one.label.as_str()).collect::<Vec<_>>(), ["Approve"]);
    let act = item.act.as_ref().expect("an act, so Needs you answers it in place");
    assert_eq!((act.kind, act.target.as_str()), (WaitingActKind::Answer, item.id.as_str()));

    let rows = rig.rows(&id).await;
    let row = item.id.strip_prefix("draft:").unwrap();
    assert!(
        rows.iter().any(|one| matches!(one,
            SessionRow::Draft { id, title, text, before: Some(before), settled: None, .. }
                if id == row && title == TITLE && text == TEXT && before == "Old description.")),
        "the thread keeps the draft: {rows:?}"
    );
}

#[tokio::test]
async fn approving_a_draft_tells_the_session_to_apply_it_as_written_and_it_waits_no_more() {
    let rig = rig_placing(None);
    let (id, item) = drafting(&rig).await;
    let mut said = answer(&id, &item);
    said.choice = Some(0);
    let after = Arc::clone(&rig.fleet).answer_waiting(said).await.unwrap();
    assert!(after.waiting_for.iter().all(|one| one.source != WaitingSource::Draft), "settled: {:?}", after.waiting_for);

    let rows = rig.rows(&id).await;
    assert_eq!(sent(&rows).pop().unwrap(), format!("Approved: {TITLE}\nApply it as written."));
    let settled = settled(&rows).expect("the row says how it was settled");
    assert_eq!((settled.how, settled.words), (DraftHow::Approved, None));
}

#[tokio::test]
async fn fixing_a_draft_sends_the_exact_text_to_use_and_keeps_it_on_the_row() {
    let rig = rig_placing(None);
    let (id, item) = drafting(&rig).await;
    let fixed = "Reads the draft row back as a waiting item.\n\n- one\n- two";
    let mut said = answer(&id, &item);
    said.fixed = Some(fixed.into());
    let after = Arc::clone(&rig.fleet).answer_waiting(said).await.unwrap();
    assert!(after.waiting_for.iter().all(|one| one.source != WaitingSource::Draft));

    let rows = rig.rows(&id).await;
    assert_eq!(
        sent(&rows).pop().unwrap(),
        format!("Approved with my fixes: {TITLE}\nUse exactly this text, as written:\n\n{fixed}")
    );
    let settled = settled(&rows).expect("settled");
    assert_eq!((settled.how, settled.words.as_deref()), (DraftHow::Fixed, Some(fixed)));
}

#[tokio::test]
async fn a_note_sends_a_draft_back_to_be_rewritten_and_proposed_again() {
    let rig = rig_placing(None);
    let (id, item) = drafting(&rig).await;
    let mut blank = answer(&id, &item);
    blank.fixed = Some("  ".into());
    assert!(Arc::clone(&rig.fleet).answer_waiting(blank).await.is_err(), "a blank fix with nothing else");

    let mut said = answer(&id, &item);
    said.text = Some("Shorter, and say why.".into());
    Arc::clone(&rig.fleet).answer_waiting(said).await.unwrap();

    let rows = rig.rows(&id).await;
    assert_eq!(
        sent(&rows).pop().unwrap(),
        format!("Sent back: {TITLE}\nShorter, and say why.\nRewrite it and propose it again with propose_draft.")
    );
    let settled = settled(&rows).expect("settled");
    assert_eq!((settled.how, settled.words.as_deref()), (DraftHow::SentBack, Some("Shorter, and say why.")));
}

fn dismissed(id: &ipc::SessionId, item: &str, reason: Option<&str>) -> DismissWaiting {
    DismissWaiting { session_id: id.clone(), item_id: item.to_string(), reason: reason.map(str::to_string), misread: false }
}

#[tokio::test]
async fn dismissing_a_draft_withdraws_it_and_tells_the_session_why() {
    let rig = rig_placing(None);
    let (id, item) = drafting(&rig).await;
    let after = Arc::clone(&rig.fleet)
        .dismiss_waiting(dismissed(&id, &item, Some(" The pull request was closed. ")))
        .await
        .unwrap();
    assert!(after.waiting_for.iter().all(|one| one.source != WaitingSource::Draft), "withdrawn: {:?}", after.waiting_for);

    let rows = rig.rows(&id).await;
    assert_eq!(
        sent(&rows).pop().unwrap(),
        format!("Withdrawn: {TITLE}\nDo not propose it again.\nThe pull request was closed.")
    );
    let settled = settled(&rows).expect("the row says how it was settled");
    assert_eq!((settled.how, settled.words.as_deref()), (DraftHow::Withdrawn, Some("The pull request was closed.")));
}

#[tokio::test]
async fn dismissing_a_draft_with_no_reason_sends_no_trailing_line() {
    let rig = rig_placing(None);
    let (id, item) = drafting(&rig).await;
    Arc::clone(&rig.fleet).dismiss_waiting(dismissed(&id, &item, Some("  "))).await.unwrap();

    let rows = rig.rows(&id).await;
    assert_eq!(sent(&rows).pop().unwrap(), format!("Withdrawn: {TITLE}\nDo not propose it again."));
    let settled = settled(&rows).expect("settled");
    assert_eq!((settled.how, settled.words), (DraftHow::Withdrawn, None));
}

#[tokio::test]
async fn fixed_text_on_an_item_that_is_not_a_draft_is_refused_and_nothing_is_sent() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let record = rig
        .fleet
        .show_window(None, ShowWindow { url: "http://localhost:5173/".into(), title: Some("Findings".into()), session_id: Some(id.clone()), variations: Vec::new(), ..Default::default() })
        .await
        .unwrap();
    let walk = record.waiting_for.iter().find(|one| one.source == WaitingSource::Walk).expect("the walk waits").id.clone();
    let before = sent(&rig.rows(&id).await).len();

    let mut said = answer(&id, &walk);
    said.fixed = Some("my text".into());
    let Err(Refusal::Unacceptable(why)) = Arc::clone(&rig.fleet).answer_waiting(said).await else {
        panic!("fixed text is for a draft only")
    };
    assert_eq!(why.code, "fleet.fixed_not_a_draft");
    assert_eq!(sent(&rig.rows(&id).await).len(), before, "nothing sent");
}
