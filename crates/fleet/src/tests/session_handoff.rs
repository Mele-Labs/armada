//! The claim, written before the code: **a Job never starts its successor on its own; the owner
//! presses Carry on on Fleet's handoff card, and then a fresh Job, not a copy of the conversation,
//! takes the old one's place.** The Job sends its brief with `handoff_brief` when the Context rule
//! asks, and that raises one card with its context, the limit and three figures to be reminded at.
//! One press answers it, and the Job is told nothing. The new Job is started first and told the
//! handoff and the brief; the old Job's ledger is handed to it with `release_session_holds` and its
//! target, and only then is the old Job ended. A hand that fails at any step leaves the old Job live
//! as it was and exactly one Job live, records `handoff_failed` with the reason, and the owner's Retry
//! runs it again. `docs/concepts/session/a-handed-off-job.md`.

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Sessions};
use ipc::{
    AnswerWaiting, AskingToRun, AttachmentState, DismissWaiting, HandoffBrief, HandoffPick, ReleaseSessionHolds, RetryHandoff,
    SessionId, SessionState, StartSession, WaitingSource, HANDOFF_STAY, HANDOFF_YES,
};
use store::{AttachmentState as Kept, Holder, KeptAttachment};

use super::agent_starts::{kind, rig, Rig};
use super::session_host::eventually;
use crate::session_host::replacing::FAULT;
use crate::session_host::Start;

const BRIEF: &str = "The fix is on branch fix/reader. The lint run is open.";

fn asking(ask: &str) -> AskingToRun {
    ipc::decode("an ask", ask.as_bytes()).expect("an ask")
}

/// Job `id` sends `brief` with `handoff_brief`, as its agent does once the Context rule told it to.
async fn brief_sent(rig: &Rig, id: &SessionId, brief: &str) -> Result<ipc::SessionRecord, api::Refusal> {
    rig.fleet.handoff_brief(None, HandoffBrief { brief: brief.into(), session_id: Some(id.clone()) }).await
}

/// The handoff card waiting on `id` now, as his Needs you lists it.
async fn card_of(rig: &Rig, id: &SessionId) -> ipc::WaitingItem {
    let cards: Vec<ipc::WaitingItem> = rig.record(id).await.waiting_for.into_iter().filter(|one| one.source == WaitingSource::Handoff).collect();
    assert_eq!(cards.len(), 1, "one card: {cards:?}");
    cards.into_iter().next().unwrap()
}

/// The owner presses `label` on the handoff card waiting on `id`.
async fn pressed(rig: &Rig, id: &SessionId, label: &str) -> ipc::SessionRecord {
    let card = card_of(rig, id).await;
    let choice = card.options.iter().position(|one| one.label == label).unwrap_or_else(|| panic!("{label} on {card:?}"));
    let said = AnswerWaiting { session_id: id.clone(), item_id: card.id, choice: Some(choice as u32), text: None, mode: None, fixed: None };
    Arc::clone(&rig.fleet).answer_waiting(said).await.expect("answered")
}

/// What the Job's process has been sent, every line.
fn told_to(rig: &Rig, id: &SessionId) -> Vec<String> {
    rig.stand_in.process_of(id.as_str()).map_or_else(Vec::new, |one| one.sent.lock().unwrap().clone())
}

/// The Context rule asks Job `id` for its brief, it sends [`BRIEF`], and the owner presses `label` on
/// the card. A Carry on waits for the hand to settle, and fails rather than hangs where it never does.
pub(super) async fn asked_and_answered(rig: &Rig, id: &SessionId, label: &str) {
    let process = rig.stand_in.process_of(id.as_str()).expect("the job's process");
    assert!(!process.ended.load(Ordering::SeqCst), "the brief comes from a live process");
    rig.fleet.handoff_asked(id.as_str()).await;
    brief_sent(rig, id, BRIEF).await.expect("the card is raised");
    pressed(rig, id, label).await;
    if label == HANDOFF_YES {
        eventually(|| async { kind(&rig.record(id).await, "handoff_starting").is_some() }).await;
        eventually(|| async { standing(&rig.record(id).await, "handoff_starting").is_empty() }).await;
    }
}

async fn holding_a_pull_request(rig: &Rig, id: &SessionId) {
    let now = String::from("2026-10-18T09:00:00Z");
    rig.fleet
        .store()
        .lock()
        .await
        .attach(
            &KeptAttachment {
                holder: Holder::session(id.as_str()),
                kind: String::from("pr"),
                manifest_id: rig.manifest.as_str().to_string(),
                target: String::from("2430"),
                state: Kept::Standing,
                detail: Default::default(),
                since: now.clone(),
                changed_at: now,
            },
            false,
        )
        .unwrap();
}

async fn others(rig: &Rig, old: &SessionId) -> Vec<ipc::SessionRecord> {
    Arc::clone(&rig.fleet)
        .list_sessions(None, None, None)
        .await
        .expect("listed")
        .sessions
        .into_iter()
        .filter(|one| one.id != *old)
        .collect()
}

fn standing(record: &ipc::SessionRecord, kind: &str) -> Vec<String> {
    record
        .attachments
        .iter()
        .filter(|one| one.kind == kind && one.state == AttachmentState::Standing)
        .map(|one| one.target.clone())
        .collect()
}

fn live(all: &[ipc::SessionRecord]) -> usize {
    all.iter().filter(|one| one.state == SessionState::Live).count()
}

async fn all_of(rig: &Rig) -> Vec<ipc::SessionRecord> {
    Arc::clone(&rig.fleet).list_sessions(None, None, None).await.expect("listed").sessions
}

/// Break one step of the next hand, on this test's thread.
fn breaking(at: Option<&'static str>) {
    FAULT.with(|one| one.set(at));
}

/// The first process the new Job `id` starts, with nobody writing to it.
async fn start_of(rig: &Rig, id: &SessionId) -> Start {
    eventually(|| async { rig.stand_in.starts().iter().any(|one| one.session == id.as_str()) }).await;
    rig.stand_in.starts().into_iter().find(|one| one.session == id.as_str()).unwrap()
}

#[tokio::test]
async fn the_owners_yes_starts_a_fresh_job_with_the_brief_hands_it_the_ledger_and_ends_the_old_one() {
    let rig = rig(None);
    let (old, _) = rig.an_agent_that_has_spoken().await;
    holding_a_pull_request(&rig, &old).await;
    // Auto land chosen on the Job (armada-di91: a hand lost it, and a green pull request sat unmerged).
    {
        let mut store = rig.fleet.store().lock().await;
        let mut hosting = store.hosting(old.as_str()).unwrap().expect("hosted");
        hosting.auto_land = true;
        store.keep_hosting(&hosting).unwrap();
        // What the Job froze at its start, which the layers no longer say.
        let frozen = store::KeptValue { key: "pr_mode".into(), value: Some("draft".into()), layer: "project".into() };
        store.keep_job_values(old.as_str(), &[frozen]).unwrap();
    }

    let before = told_to(&rig, &old).len();
    asked_and_answered(&rig, &old, HANDOFF_YES).await;

    assert_eq!(told_to(&rig, &old).len(), before, "the old Job is told nothing: its process is ended");
    assert_eq!(live(&all_of(&rig).await), 1, "exactly one live job");
    let made = others(&rig, &old).await;
    assert_eq!(made.len(), 1, "one successor");
    let new = &made[0];
    assert_eq!(new.state, SessionState::Live);
    let old_record = rig.record(&old).await;
    assert_eq!(old_record.title, new.title, "the title carries");
    assert_eq!(kind(new, "replaces").map(|one| one.target.as_str()), Some(old.as_str()));
    assert_eq!(kind(&old_record, "replaced_by").map(|one| one.target.as_str()), Some(new.id.as_str()));
    assert_eq!(standing(new, "pr"), vec!["2430"], "the ledger moved to the new job");
    assert!(standing(&old_record, "pr").is_empty(), "and the old job keeps none");
    assert!(kind(new, "forked_from").is_none(), "not a fork: the conversation is not carried");
    assert!(new.auto_land, "Auto land carries: to the owner it is the same Job");
    let carried = rig.fleet.store().lock().await.job_values(new.id.as_str()).unwrap();
    assert_eq!(
        carried.iter().find(|one| one.key == "pr_mode").map(|one| (one.value.as_deref(), one.layer.as_str())),
        Some((Some("draft"), "project")),
        "the values it froze carry, and are not frozen again from the layers as they stand now"
    );
    assert_eq!(old_record.state, SessionState::Ended, "the old job is ended last");
    assert!(standing(&old_record, "handoff_starting").is_empty());
    assert!(kind(&old_record, "handoff_failed").is_none());

    // Nobody writes: it starts on its own, told the handoff and the brief, and copies nothing.
    let start = start_of(&rig, &new.id).await;
    assert!(start.forking.is_none() && start.fork_cut.is_none(), "no fork of the old thread");
    let told = start.note.expect("a note");
    assert!(told.contains("You are a new job"), "{told}");
    assert!(told.contains("The fix is on branch fix/reader. The lint run is open."), "the brief: {told}");
    assert!(told.contains("Handoff from The agent") && told.contains("Pull request #2430"), "the composed context: {told}");
    assert!(told.contains("Agent: Fixed."), "the last of the thread: {told}");
    let first = rig.stand_in.process_of(new.id.as_str()).unwrap().sent.lock().unwrap().join("\n");
    assert!(first.contains("Carry on with the work of the job you took the place of"), "its first turn: {first}");
}

#[tokio::test]
async fn staying_starts_nothing_changes_nothing_and_sends_the_agent_nothing() {
    let rig = rig(None);
    let (old, _) = rig.an_agent_that_has_spoken().await;
    holding_a_pull_request(&rig, &old).await;
    let before = told_to(&rig, &old).len();

    asked_and_answered(&rig, &old, HANDOFF_STAY).await;

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(told_to(&rig, &old).len(), before, "no word to the agent, so no prose follows");
    assert!(others(&rig, &old).await.is_empty());
    let old_record = rig.record(&old).await;
    assert_eq!(old_record.state, SessionState::Live);
    assert_eq!(standing(&old_record, "pr"), vec!["2430"]);
    assert!(kind(&old_record, "replaced_by").is_none());
}

#[tokio::test]
async fn a_new_job_that_does_not_start_leaves_the_old_one_as_it_was_and_records_the_failure() {
    let rig = rig(None);
    let (old, _) = rig.an_agent_that_has_spoken().await;
    holding_a_pull_request(&rig, &old).await;
    breaking(Some("start"));

    asked_and_answered(&rig, &old, HANDOFF_YES).await;
    breaking(None);

    assert!(others(&rig, &old).await.is_empty(), "no successor");
    let old_record = rig.record(&old).await;
    assert_eq!(old_record.state, SessionState::Live);
    assert_eq!(standing(&old_record, "pr"), vec!["2430"], "nothing moved");
    assert!(kind(&old_record, "replaced_by").is_none());
    assert_eq!(standing(&old_record, "handoff_failed").len(), 1, "the failure is a fact on the ledger");
    let reason = kind(&old_record, "handoff_failed").unwrap().detail.get("reason").cloned().unwrap();
    assert!(reason.starts_with("new job did not start"), "a bare fact, lowercase: {reason}");
    assert!(standing(&old_record, "handoff_starting").is_empty());
    assert!(old_record.hosted.as_ref().unwrap().asked.is_none(), "no second call is raised");
}

#[tokio::test]
async fn a_failure_after_the_new_job_is_up_leaves_exactly_one_live_job_and_the_ledger_where_it_was() {
    let rig = rig(None);
    let (old, _) = rig.an_agent_that_has_spoken().await;
    holding_a_pull_request(&rig, &old).await;
    breaking(Some("close"));

    asked_and_answered(&rig, &old, HANDOFF_YES).await;
    breaking(None);

    assert_eq!(live(&all_of(&rig).await), 1, "one live job: the old one");
    let old_record = rig.record(&old).await;
    assert_eq!(old_record.state, SessionState::Live);
    assert_eq!(standing(&old_record, "pr"), vec!["2430"], "the ledger came back");
    let made = others(&rig, &old).await;
    assert_eq!(made.len(), 1, "the new job was started");
    assert_eq!(made[0].state, SessionState::Ended, "and closed");
    assert!(standing(&made[0], "pr").is_empty());
    let spent = |record: &ipc::SessionRecord, link: &str| {
        record.attachments.iter().any(|one| one.kind == link && one.state == AttachmentState::Spent)
    };
    assert!(!spent(&old_record, "replaced_by") && !spent(&made[0], "replaces"), "no link stands");
    assert_eq!(standing(&old_record, "handoff_failed").len(), 1);
}

#[tokio::test]
async fn retry_after_a_failure_succeeds_and_gives_back_the_failed_row() {
    let rig = rig(None);
    let (old, _) = rig.an_agent_that_has_spoken().await;
    holding_a_pull_request(&rig, &old).await;
    breaking(Some("close"));
    asked_and_answered(&rig, &old, HANDOFF_YES).await;

    // Retry while it still fails: a fresh row replaces the old one, and one job is live.
    let again = Arc::clone(&rig.fleet).retry_handoff(RetryHandoff { session_id: old.clone() }).await;
    assert!(again.is_err(), "still failing");
    assert_eq!(standing(&rig.record(&old).await, "handoff_failed").len(), 1, "a fresh row replaces it");
    assert_eq!(live(&all_of(&rig).await), 1);

    breaking(None);
    let new = Arc::clone(&rig.fleet).retry_handoff(RetryHandoff { session_id: old.clone() }).await.expect("retried");
    assert_eq!(new.state, SessionState::Live);
    assert_ne!(new.id, old);
    let old_record = rig.record(&old).await;
    assert_eq!(old_record.state, SessionState::Ended);
    assert!(standing(&old_record, "handoff_failed").is_empty(), "the failed row is given back");
    assert_eq!(standing(&rig.record(&new.id).await, "pr"), vec!["2430"]);
    assert_eq!(kind(&old_record, "replaced_by").map(|one| one.target.as_str()), Some(new.id.as_str()));
    assert_eq!(live(&all_of(&rig).await), 1, "one live job: the new one");

    // The brief the card carried survived the failures and reached the retried job.
    let told = start_of(&rig, &new.id).await.note.expect("a note");
    assert!(told.contains("The fix is on branch fix/reader."), "{told}");
}

#[tokio::test]
async fn retry_where_no_hand_off_failed_is_refused() {
    let rig = rig(None);
    let (old, _) = rig.an_agent_that_has_spoken().await;

    let refused = Arc::clone(&rig.fleet).retry_handoff(RetryHandoff { session_id: old.clone() }).await;

    assert!(matches!(refused, Err(api::Refusal::IllegalMove(_))), "{refused:?}");
    assert!(others(&rig, &old).await.is_empty());
}

/// A hand a Fleet left partway, as the one that stopped left it: the new Job made and handed the
/// ledger, the old one live, and `handoff_starting` standing. `named` writes the new Job's id on the
/// mark, as this Fleet does; without it is an older Fleet's mark.
async fn left_partway(rig: &Rig, named: bool) -> (SessionId, SessionId) {
    let (old, _) = rig.an_agent_that_has_spoken().await;
    holding_a_pull_request(rig, &old).await;
    let since = rig.fleet.now().as_str().to_string();
    let new = rig
        .fleet
        .start_session(StartSession {
            project: None,
            manifest_id: rig.manifest.clone(),
            title: Some("The agent".into()),
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
    rig.fleet
        .release_session_holds(ReleaseSessionHolds { session_id: old.clone(), to_session_id: Some(new.clone()), rows: None })
        .await
        .expect("handed");
    let detail = match named {
        true => [(String::from("new"), new.as_str().to_string())].into(),
        false => Default::default(),
    };
    rig.fleet
        .store()
        .lock()
        .await
        .attach(
            &KeptAttachment {
                holder: Holder::session(old.as_str()),
                kind: String::from("handoff_starting"),
                manifest_id: rig.manifest.as_str().to_string(),
                target: old.as_str().to_string(),
                state: Kept::Standing,
                detail,
                since: since.clone(),
                changed_at: since,
            },
            false,
        )
        .unwrap();
    assert_eq!(live(&all_of(rig).await), 2, "broken as the owner found it");
    (old, new)
}

async fn rolled_back_at_boot(named: bool) {
    let rig = rig(None);
    let (old, new) = left_partway(&rig, named).await;

    assert_eq!(rig.fleet.handoffs_healed().await, 1);

    assert_eq!(live(&all_of(&rig).await), 1, "exactly one live job");
    let old_record = rig.record(&old).await;
    assert_eq!(old_record.state, SessionState::Live, "the old job stands");
    assert_eq!(rig.record(&new).await.state, SessionState::Ended, "the new one is closed");
    assert_eq!(standing(&old_record, "pr"), vec!["2430"], "the ledger came back");
    assert!(standing(&old_record, "handoff_starting").is_empty());
    let reason = kind(&old_record, "handoff_failed").and_then(|one| one.detail.get("reason").cloned());
    assert_eq!(reason.as_deref(), Some("fleet stopped during the hand off"));
    assert_eq!(standing(&old_record, "handoff_failed").len(), 1, "for its Retry");
}

#[tokio::test]
async fn a_hand_left_standing_at_boot_is_rolled_back_to_one_live_job_with_handoff_failed() {
    rolled_back_at_boot(true).await;
}

#[tokio::test]
async fn a_hand_an_older_fleet_left_standing_at_boot_finds_its_new_job_and_is_rolled_back() {
    rolled_back_at_boot(false).await;
}

// ---- The card, its answers, and the Large context setting ----

/// One main-thread turn of the Job holding `context` tokens, at `minute` past eight, read by the rules.
async fn turn_heard(rig: &Rig, id: &SessionId, minute: u32, context: u64) {
    // The telling waits for a quiet moment: the Job's turn has ended. A turn an answered card holds open does not end here.
    for _ in 0..20 {
        if !matches!(rig.fleet.hosts().of(id.as_str()).state().turn, ipc::SessionTurn::Working { .. }) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    let turn = store::KeptTurnUsage {
        message_id: format!("m{minute}"),
        at: format!("2026-10-18T{:02}:{:02}:00.000Z", 8 + minute / 60, minute % 60),
        model: "model-a".into(),
        input_tokens: 0,
        cache_creation_tokens: 0,
        cache_creation_1h_tokens: 0,
        cache_read_tokens: context,
        output_tokens: 1_000,
        tools: vec!["Edit".into()],
        subagent: false,
        bash_use: None,
    };
    rig.fleet.store().lock().await.keep_turn_usage(id.as_str(), &turn).unwrap();
    rig.fleet.usage_rules_read(&[id.as_str().to_string()]).await;
}

/// What Fleet said in the Job's thread that starts with `opening`.
async fn fleet_said(rig: &Rig, id: &SessionId, opening: &str) -> Vec<String> {
    rig.rows(id)
        .await
        .into_iter()
        .filter_map(|row| match row {
            ipc::SessionRow::Message { text, from: ipc::SessionVoice::Session { .. }, .. } if text.starts_with(opening) => Some(text),
            _ => None,
        })
        .collect()
}

async fn handoff_set(rig: &Rig, value: &str) {
    let changes = [(String::from("plan.contextHandoff"), Some(ipc::SettingValue::Text(value.into())))].into();
    api::Settings::save_settings(rig.fleet.as_ref(), ipc::SaveSettings { changes }).await.expect("saved");
}

/// The Job's call standing on `id`, once one is raised whose question is `question`.
async fn asked_card(rig: &Rig, id: &SessionId, question: &str) -> ipc::CallInFlight {
    let found = std::sync::Mutex::new(None);
    eventually(|| async {
        let asked = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session.hosted.unwrap().asked;
        let here = asked.filter(|one| one.questions.iter().any(|asked| asked.question == question));
        let held = here.is_some();
        *found.lock().unwrap() = here;
        held
    })
    .await;
    found.into_inner().unwrap().unwrap()
}

/// The Context rule tells `id`, over the line at 312k, and its turn ends.
async fn told_at_312k(rig: &Rig, id: &SessionId) {
    turn_heard(rig, id, 0, 312_000).await;
    rig.stand_in.finishes(0, "Sent.");
}

#[tokio::test]
async fn the_trip_tells_the_job_to_write_only_a_brief_and_send_it_with_handoff_brief() {
    let rig = rig(None);
    let (old, _) = rig.an_agent_that_has_spoken().await;

    turn_heard(&rig, &old, 0, 312_000).await;

    let told = fleet_said(&rig, &old, "Context is").await;
    assert_eq!(told.len(), 1, "told once: {told:?}");
    assert!(told[0].starts_with("Context is 312k tokens, past the 300k limit."), "the figure and the limit: {}", told[0]);
    assert!(told[0].contains("send it with handoff_brief"), "{}", told[0]);
    assert!(told[0].contains("Ask him nothing and start nothing yourself"), "{}", told[0]);
    assert!(!told[0].contains(HANDOFF_YES), "no options to ask with: {}", told[0]);
    assert!(rig.record(&old).await.hosted.unwrap().asked.is_none(), "no card is asked for");
}

#[tokio::test]
async fn a_brief_raises_one_card_with_the_figure_the_limit_and_the_ladder() {
    let rig = rig(None);
    let (old, _) = rig.an_agent_that_has_spoken().await;
    told_at_312k(&rig, &old).await;

    let record = brief_sent(&rig, &old, &format!("  {BRIEF}\n")).await.expect("the card");

    let rows: Vec<ipc::SessionRow> = rig.rows(&old).await.into_iter().filter(|row| matches!(row, ipc::SessionRow::Handoff { .. })).collect();
    let [ipc::SessionRow::Handoff { id, tokens, limit, brief, remind_at, answer, .. }] = rows.as_slice() else { panic!("one card: {rows:?}") };
    assert_eq!((*tokens, *limit), (312_000, 300_000));
    assert_eq!(brief, BRIEF, "trimmed");
    assert_eq!(remind_at, &[400_000, 450_000, 500_000]);
    assert!(answer.is_none());
    let card = card_of(&rig, &old).await;
    assert_eq!(card.id, format!("handoff:{id}"));
    assert_eq!(card.text, "Carry on in a new Job?");
    assert_eq!(card.act.as_ref().map(|act| act.kind), Some(ipc::WaitingActKind::Answer));
    let labels: Vec<&str> = card.options.iter().map(|one| one.label.as_str()).collect();
    assert_eq!(labels, ["Carry on in a new Job", "Stay here", "Don't ask again", "Remind at 400k", "Remind at 450k", "Remind at 500k"]);
    assert_eq!(record.waiting_for.iter().filter(|one| one.source == WaitingSource::Handoff).count(), 1);
    assert!(record.hosted.unwrap().asked.is_none(), "no question card");

    let again = brief_sent(&rig, &old, BRIEF).await;
    assert!(again.is_err(), "one brief for one telling: {again:?}");
}

#[tokio::test]
async fn handoff_brief_is_refused_when_fleet_did_not_ask_and_when_it_is_empty() {
    let rig = rig(None);
    let (old, _) = rig.an_agent_that_has_spoken().await;

    let refused = brief_sent(&rig, &old, BRIEF).await;
    assert!(format!("{refused:?}").contains("fleet.handoff_not_asked"), "{refused:?}");
    assert!(rig.rows(&old).await.iter().all(|row| !matches!(row, ipc::SessionRow::Handoff { .. })), "nothing kept");

    told_at_312k(&rig, &old).await;
    let empty = brief_sent(&rig, &old, "  \n ").await;
    assert!(format!("{empty:?}").contains("fleet.handoff_brief_empty"), "{empty:?}");
    brief_sent(&rig, &old, BRIEF).await.expect("an empty brief spends nothing");
}

#[tokio::test]
async fn carry_on_on_the_card_starts_a_successor_carrying_the_brief() {
    let rig = rig(None);
    let (old, _) = rig.an_agent_that_has_spoken().await;
    holding_a_pull_request(&rig, &old).await;
    told_at_312k(&rig, &old).await;
    brief_sent(&rig, &old, BRIEF).await.expect("the card");

    pressed(&rig, &old, HANDOFF_YES).await;

    eventually(|| async { rig.record(&old).await.state == SessionState::Ended }).await;
    eventually(|| async { standing(&rig.record(&old).await, "handoff_starting").is_empty() }).await;
    let made = others(&rig, &old).await;
    assert_eq!(made.len(), 1, "one successor");
    assert_eq!(live(&all_of(&rig).await), 1, "exactly one live job");
    assert_eq!(standing(&made[0], "pr"), vec!["2430"], "the ledger moved");
    let told = start_of(&rig, &made[0].id).await.note.expect("a note");
    assert!(told.contains(BRIEF), "the card's brief: {told}");
    let answered = rig.rows(&old).await.into_iter().find_map(|row| match row {
        ipc::SessionRow::Handoff { answer, .. } => answer,
        _ => None,
    });
    assert_eq!(answered, Some(HandoffPick::CarryOn));
}

#[tokio::test]
async fn dont_ask_again_is_kept_and_the_next_trip_tells_nothing() {
    let rig = rig(None);
    let (old, _) = rig.an_agent_that_has_spoken().await;
    told_at_312k(&rig, &old).await;
    brief_sent(&rig, &old, BRIEF).await.expect("the card");
    let before = told_to(&rig, &old).len();

    let record = pressed(&rig, &old, ipc::HANDOFF_QUIET).await;

    assert!(kind(&record, "handoff_quiet").is_some(), "the owner's answer is a fact on the Job's ledger");
    assert!(record.waiting_for.iter().all(|one| one.source != WaitingSource::Handoff), "the card is answered");
    assert!(others(&rig, &old).await.is_empty(), "nothing started");
    assert_eq!(told_to(&rig, &old).len(), before, "the agent is sent nothing");
    // Under the line, then over it again: his answer keeps it quiet.
    turn_heard(&rig, &old, 1, 80_000).await;
    turn_heard(&rig, &old, 45, 340_000).await;
    turn_heard(&rig, &old, 46, 700_000).await;
    assert_eq!(fleet_said(&rig, &old, "Context is").await.len(), 1, "never told again");

    // Another Job is told as before: the answer was this Job's alone.
    let other = rig
        .fleet
        .start_session(StartSession {
            project: None,
            manifest_id: rig.manifest.clone(),
            title: Some("Another".into()),
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
    turn_heard(&rig, &other, 0, 312_000).await;
    assert_eq!(fleet_said(&rig, &other, "Context is").await.len(), 1);
}

#[tokio::test]
async fn remind_at_keeps_the_figure_in_one_press_and_the_job_is_told_again_past_it() {
    let rig = rig(None);
    let (old, _) = rig.an_agent_that_has_spoken().await;
    told_at_312k(&rig, &old).await;
    brief_sent(&rig, &old, BRIEF).await.expect("the card");
    let before = told_to(&rig, &old).len();

    let record = pressed(&rig, &old, "Remind at 450k").await;

    assert_eq!(standing(&record, "handoff_remind_at"), vec!["450000"], "the figure is kept on the ledger");
    assert!(record.waiting_for.iter().all(|one| one.source != WaitingSource::Handoff), "no second question");
    assert!(record.hosted.as_ref().unwrap().asked.is_none(), "no question card either");
    assert_eq!(told_to(&rig, &old).len(), before, "the agent is sent nothing");

    turn_heard(&rig, &old, 1, 440_000).await;
    assert_eq!(fleet_said(&rig, &old, "Context is").await.len(), 1, "under the figure: not told");
    turn_heard(&rig, &old, 2, 462_000).await;
    let told = fleet_said(&rig, &old, "Context is").await;
    assert_eq!(told.len(), 2, "past it: told again {told:?}");
    assert!(told[1].starts_with("Context is 462k tokens."));
    assert!(standing(&rig.record(&old).await, "handoff_remind_at").is_empty(), "the reminder is spent");
    rig.stand_in.finishes(0, "Sent.");
    brief_sent(&rig, &old, BRIEF).await.expect("the telling asks for a brief again");
    let card = card_of(&rig, &old).await;
    let labels: Vec<&str> = card.options[3..].iter().map(|one| one.label.as_str()).collect();
    assert_eq!(labels, ["Remind at 500k", "Remind at 550k", "Remind at 600k"]);
}

#[tokio::test]
async fn dismissing_the_card_is_stay() {
    let rig = rig(None);
    let (old, _) = rig.an_agent_that_has_spoken().await;
    told_at_312k(&rig, &old).await;
    brief_sent(&rig, &old, BRIEF).await.expect("the card");
    let before = told_to(&rig, &old).len();
    let card = card_of(&rig, &old).await;

    let record = Arc::clone(&rig.fleet)
        .dismiss_waiting(DismissWaiting { session_id: old.clone(), item_id: card.id, reason: None, misread: false })
        .await
        .expect("dismissed");

    assert!(record.waiting_for.iter().all(|one| one.source != WaitingSource::Handoff), "{:?}", record.waiting_for);
    assert!(kind(&record, "handoff_quiet").is_none() && kind(&record, "handoff_remind_at").is_none(), "nothing kept");
    assert_eq!(record.state, SessionState::Live);
    assert!(others(&rig, &old).await.is_empty(), "nothing started");
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(told_to(&rig, &old).len(), before, "the agent is sent nothing");
    let answered = rig.rows(&old).await.into_iter().find_map(|row| match row {
        ipc::SessionRow::Handoff { answer, .. } => answer,
        _ => None,
    });
    assert_eq!(answered, Some(HandoffPick::Stay));
}

#[tokio::test]
async fn set_to_never_no_job_is_told() {
    let rig = rig(None);
    let (old, _) = rig.an_agent_that_has_spoken().await;
    handoff_set(&rig, "never").await;

    turn_heard(&rig, &old, 0, 312_000).await;

    assert!(fleet_said(&rig, &old, "Context is").await.is_empty());
    assert!(fleet_said(&rig, &old, "Reply with your brief").await.is_empty());
    assert!(others(&rig, &old).await.is_empty());
}

#[tokio::test]
async fn set_to_carry_on_automatically_the_job_is_asked_its_brief_and_handed_on_once_with_no_card() {
    let rig = rig(None);
    let (old, _) = rig.an_agent_that_has_spoken().await;
    holding_a_pull_request(&rig, &old).await;
    handoff_set(&rig, "carry on automatically").await;

    turn_heard(&rig, &old, 0, 312_000).await;

    assert!(fleet_said(&rig, &old, "Context is").await.is_empty(), "the Job is not told to ask");
    eventually(|| async { !fleet_said(&rig, &old, "Reply with your brief").await.is_empty() }).await;
    assert!(rig.record(&old).await.hosted.unwrap().asked.is_none(), "no card");
    rig.stand_in.finishes(0, "The fix is on branch fix/reader. The lint run is open.");

    eventually(|| async { rig.record(&old).await.state == SessionState::Ended }).await;
    eventually(|| async { standing(&rig.record(&old).await, "handoff_starting").is_empty() }).await;
    let made = others(&rig, &old).await;
    assert_eq!(made.len(), 1, "one successor");
    assert_eq!(live(&all_of(&rig).await), 1, "exactly one live job");
    assert_eq!(standing(&made[0], "pr"), vec!["2430"], "the ledger moved");
    let told = start_of(&rig, &made[0].id).await.note.expect("a note");
    assert!(told.contains("The fix is on branch fix/reader. The lint run is open."), "the brief it replied: {told}");

    // The old Job's later turns start nothing more.
    turn_heard(&rig, &old, 45, 340_000).await;
    turn_heard(&rig, &old, 46, 360_000).await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(others(&rig, &old).await.len(), 1, "handed on once");
    assert_eq!(live(&all_of(&rig).await), 1);
}

const KEPT_ASK: &str = r#"{"tool_name":"AskUserQuestion","tool_use_id":"toolu_k","input":{"questions":[
  {"question":"Carry on in a new Job?","header":"Context","multiSelect":false,
   "options":[{"label":"Carry on in a new Job","description":"The fix is on branch fix/reader. The lint run is open."},
              {"label":"Stay here","description":""},
              {"label":"Don't ask again","description":""}]}]}}"#;

/// A question kept for a walk, as a Fleet that refused questions during one wrote it. Fleet no
/// longer writes the state, but a thread that holds one still reads, and answers, as it did.
async fn kept_for_a_walk(rig: &Rig, id: &SessionId) {
    kept_ask(rig, id, KEPT_ASK, "Carry on in a new Job?").await;
}

async fn kept_ask(rig: &Rig, id: &SessionId, call: &'static str, question: &str) {
    let (fleet, session) = (Arc::clone(&rig.fleet), id.as_str().to_string());
    let held = tokio::spawn(async move { fleet.session_permission(&session, asking(call)).await });
    let card = asked_card(rig, id, question).await;
    held.abort();
    let row = ipc::SessionRow::Ask {
        id: format!("ask-{}-{}", card.call, id.as_str()),
        at: rig.fleet.instant(),
        ask: card,
        state: ipc::SessionAskState::Kept,
        via: None,
    };
    rig.fleet.row_put(id.as_str(), row).await;
}

const OTHER_ASK: &str = r#"{"tool_name":"AskUserQuestion","tool_use_id":"toolu_o","input":{"questions":[
  {"question":"Which night?","header":"Night","multiSelect":false,
   "options":[{"label":"Friday","description":""},{"label":"Thursday","description":""}]}]}}"#;

#[tokio::test]
async fn every_kept_question_stays_open_after_a_later_one_is_kept_and_the_owner_types() {
    let rig = rig(None);
    let (id, _) = rig.an_agent_that_has_spoken().await;
    kept_for_a_walk(&rig, &id).await;
    kept_ask(&rig, &id, OTHER_ASK, "Which night?").await;
    rig.send(&id, "There is no question waiting for me").await;

    let open = crate::session_host::handoff_answers::lapsed_question(&rig.rows(&id).await).expect("both stay open");
    let asked: Vec<&str> = open.iter().map(|one| one.question.as_str()).collect();
    assert_eq!(asked, ["Carry on in a new Job?", "Which night?"]);
}

/// How many lines the parent's process was sent that contain `text`.
fn parent_told(rig: &Rig, parent: &SessionId, text: &str) -> usize {
    rig.stand_in.process_of(parent.as_str()).map_or(0, |one| one.sent.lock().unwrap().iter().filter(|line| line.contains(text)).count())
}

/// Wait for `check`, naming `what` where it never comes true.
async fn until<F: std::future::Future<Output = bool>>(what: &str, mut check: impl FnMut() -> F) {
    for _ in 0..500 {
        if check().await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("{what}: not within five seconds");
}

/// The links `start_another_session` writes for `group`: `started_by` on the child, `started_to` on the parent.
async fn linked_under(rig: &Rig, parent: &SessionId, child: &SessionId, group: &str) {
    let now = String::from("2026-08-26T08:00:00Z");
    let mut store = rig.fleet.store().lock().await;
    for (holder, kind, target) in [(child, "started_by", parent), (parent, "started_to", child)] {
        let link = KeptAttachment {
            holder: Holder::session(holder.as_str()),
            kind: kind.to_string(),
            manifest_id: rig.manifest.as_str().to_string(),
            target: target.as_str().to_string(),
            state: Kept::Spent,
            detail: [(String::from("group"), group.to_string())].into(),
            since: now.clone(),
            changed_at: now.clone(),
        };
        store.attach(&link, false).unwrap();
    }
}

/// The claim of armada-3903: **a Job handed on for context takes over the original's links.** The Job
/// that started the original is told it was handed on and to which Job, follows the successor under
/// the same plan group, and hears the successor's merge and its end. 30 Oct 2026: a parent heard only
/// that its child ended, started a duplicate, and watched the successor's pull request itself.
#[tokio::test]
async fn a_linked_job_handed_on_passes_its_link_and_the_parent_follows_the_successor() {
    let rig = rig(None);
    // The child asks from process 0, as every hand here does; its parent is started after it.
    let (child, _) = rig.an_agent_that_has_spoken().await;
    crate::linked_following::keep_the_linked(Arc::clone(&rig.fleet));
    let parent = rig
        .fleet
        .start_session(StartSession {
            project: None,
            manifest_id: rig.manifest.clone(),
            title: Some("The planner".into()),
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
    linked_under(&rig, &parent, &child, "g1").await;

    asked_and_answered(&rig, &child, HANDOFF_YES).await;

    let old = rig.record(&child).await;
    assert_eq!(old.state, SessionState::Ended);
    let new = SessionId::carried(&kind(&old, "replaced_by").expect("a successor").target);
    // Closing the old Job gives back the hand's mark, so the hand's last steps may still be running.
    until("the successor names the parent", || async { kind(&rig.record(&new).await, "started_by").is_some() }).await;
    let by = kind(&rig.record(&new).await, "started_by").cloned().unwrap();
    assert_eq!((by.target.as_str(), by.detail.get("group").map(String::as_str)), (parent.as_str(), Some("g1")));
    let to = rig
        .record(&parent)
        .await
        .attachments
        .into_iter()
        .find(|one| one.kind == "started_to" && one.state != AttachmentState::GivenBack)
        .expect("the parent names a child");
    assert_eq!(
        (to.target.as_str(), to.detail.get("group").map(String::as_str)),
        (new.as_str(), Some("g1")),
        "the successor is the group's newest link, which the group's row draws"
    );
    // **One child, not two**: the link to the old Job is given back, so every list of what the parent
    // started names the successor alone.
    until("the old link given back", || async {
        rig.record(&parent).await.attachments.iter().any(|one| one.kind == "started_to" && one.target == child.as_str() && one.state == AttachmentState::GivenBack)
    })
    .await;
    let links: Vec<(String, AttachmentState)> = rig
        .record(&parent)
        .await
        .attachments
        .iter()
        .filter(|one| one.kind == "started_to")
        .map(|one| (one.target.clone(), one.state))
        .collect();
    assert!(links.contains(&(child.as_str().to_string(), AttachmentState::GivenBack)), "{links:?}");
    assert_eq!(crate::started_jobs::children_in(&rig.record(&parent).await.attachments), vec![new.as_str().to_string()]);

    let handed = format!("was handed on to a new Job ({}) for context. Plan group: g1.", new.as_str());
    until("told it was handed on", || async { parent_told(&rig, &parent, &handed) == 1 }).await;
    let row_of = |rows: Vec<(String, ipc::LinkedEvent)>, id: &SessionId| rows.into_iter().filter(|(one, _)| one == id.as_str()).map(|(_, event)| event).collect::<Vec<_>>();
    assert!(!row_of(rig.linked_rows(&parent, 1).await, &child).contains(&ipc::LinkedEvent::Done), "not told as a bare end");
    // The row names the Job it took the place of, so a reader draws the one row that started it.
    let replaced: Vec<Option<SessionId>> = rig
        .rows(&parent)
        .await
        .into_iter()
        .filter_map(|row| match row {
            ipc::SessionRow::Linked { session, replaces, .. } if session.id == new => Some(replaces),
            _ => None,
        })
        .collect();
    assert_eq!(replaced, vec![Some(child.clone())]);

    let merged = [("state".to_string(), "merged".to_string())].into();
    rig.report(&new, ipc::SessionFact::Attached { attachment: ipc::AttachmentReport { kind: "pr".into(), target: "3084".into(), detail: merged } }).await;
    rig.report(&new, ipc::SessionFact::Settled { attachment: ipc::AttachmentNamed { kind: "pr".into(), target: "3084".into() }, state: AttachmentState::Spent }).await;
    until("told the successor's merge", || async { parent_told(&rig, &parent, "had its pull request #3084 merged") == 1 }).await;

    Arc::clone(&rig.fleet).close_session(ipc::CloseSession { session_id: new.clone() }).await.expect("closed");
    let ended = format!("({}) ended.", new.as_str());
    until("told the successor's end", || async { parent_told(&rig, &parent, &ended) == 1 }).await;
    let events = row_of(rig.linked_rows(&parent, 3).await, &new);
    assert!(events.contains(&ipc::LinkedEvent::Merged) && events.contains(&ipc::LinkedEvent::Done), "{events:?}");
}
