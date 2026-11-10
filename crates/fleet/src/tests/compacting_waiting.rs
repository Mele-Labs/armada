//! A hosted Job waiting on the owner is sent `/compact` once its cache is nearly out, once a wait,
//! and only with the switch on. `docs/concepts/session/where-the-tokens-went.md`, *A Job compacted
//! while it waits*.

use std::time::Duration;

use ipc::SessionId;
use store::{KeptTurnUsage, KeptWaiting};

use super::session_host::{eventually, rig_fitted, Rig};
use super::tmp::TempDir;
use crate::session_host::compacting::{compacts, Waiting, COMPACT, COMPACT_AFTER, COMPACT_FROM};

fn waiting() -> Waiting {
    Waiting { on_the_owner: true, quiet: COMPACT_AFTER, context: COMPACT_FROM, compacted: false }
}

#[test]
fn a_waiting_job_compacts_only_when_on_old_big_and_not_compacted_yet() {
    assert!(compacts(true, waiting()));
    assert!(!compacts(false, waiting()), "off does nothing");
    assert!(!compacts(true, Waiting { on_the_owner: false, ..waiting() }), "a Job not waiting on the owner");
    assert!(!compacts(true, Waiting { quiet: COMPACT_AFTER - Duration::from_secs(1), ..waiting() }), "a recent turn");
    assert!(!compacts(true, Waiting { context: COMPACT_FROM - 1, ..waiting() }), "a small context");
    assert!(!compacts(true, Waiting { compacted: true, ..waiting() }), "once a wait");
}

/// A rig whose settings.json holds `compact`, and a Job whose turn has ended, waiting on the owner for
/// one item, its last main-thread turn an hour ago where `old` and now where not, with `context` tokens.
async fn job_waiting(compact: bool, old: bool, context: u64) -> (Rig, SessionId, TempDir) {
    let home = TempDir::new();
    let file = super::settings_json::settings_file_in(&home);
    std::fs::write(&file, format!(r#"{{"tokens.compactWhileWaiting": {compact}}}"#)).unwrap();
    let rig = rig_fitted(move |fittings| {
        fittings.settings =
            crate::settings::MachineSettings::at(file, super::settings_json::supplied(), config::settings::Env::none());
    });
    let id = rig.start().await;
    rig.send(&id, "go").await;
    rig.stand_in.init(0);
    // A reply that only reports, so the one thing waiting on the owner is the item below.
    rig.stand_in.finishes(0, "Pushed both fixes.");
    eventually(|| async { matches!(rig.turn(&id).await, ipc::SessionTurn::Idle) }).await;
    // The rig's clock starts at nine.
    let turn_at = if old { "2026-08-26T08:00:00.000Z".to_string() } else { rig.fleet.now().as_str().to_string() };
    let mut store = rig.fleet.store().lock().await;
    let item = KeptWaiting {
        approves: false,
        item_id: "copy".into(),
        text: "Pick the copy".into(),
        since: "2026-08-26T08:00:00.000Z".into(),
        act: None,
        options: Vec::new(),
        steps: Vec::new(),
    };
    store.replace_waiting_for(id.as_str(), &[item]).unwrap();
    let turn = KeptTurnUsage {
        message_id: "m1".into(),
        at: turn_at,
        model: "a-model".into(),
        input_tokens: 4,
        cache_creation_tokens: 1_000,
        cache_creation_1h_tokens: 0,
        cache_read_tokens: context - 1_004,
        output_tokens: 10,
        tools: Vec::new(),
        subagent: false,
        bash_use: None,
    };
    store.keep_turn_usage(id.as_str(), &turn).unwrap();
    drop(store);
    (rig, id, home)
}

/// The `/compact` lines the Job's process was sent.
fn compacts_sent(rig: &Rig, id: &SessionId) -> usize {
    let process = rig.stand_in.process_of(id.as_str()).expect("a process");
    let sent = process.sent.lock().unwrap();
    sent.iter().filter(|line| line.contains(&format!("\"{COMPACT}\""))).count()
}

#[tokio::test]
async fn a_job_waiting_past_most_of_its_cache_with_a_big_context_is_compacted_once_a_wait() {
    let (rig, id, _home) = job_waiting(true, true, 180_000).await;
    rig.fleet.sweep_quiet().await;
    assert_eq!(compacts_sent(&rig, &id), 1, "sent once");
    let kept = rig.fleet.store().lock().await.waiting_compactions(id.as_str()).unwrap();
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].context_before, 180_000);

    // The compaction's turn ends and the Job still waits: the same wait, so nothing more is sent.
    rig.stand_in.finishes(0, "Compacted.");
    eventually(|| async { matches!(rig.turn(&id).await, ipc::SessionTurn::Idle) }).await;
    rig.fleet.sweep_quiet().await;
    assert_eq!(compacts_sent(&rig, &id), 1, "once a wait");
    assert_eq!(rig.fleet.store().lock().await.waiting_compactions(id.as_str()).unwrap().len(), 1);
}

#[tokio::test]
async fn the_switch_off_compacts_nothing() {
    let (rig, id, _home) = job_waiting(false, true, 180_000).await;
    rig.fleet.sweep_quiet().await;
    assert_eq!(compacts_sent(&rig, &id), 0);
    assert!(rig.fleet.store().lock().await.waiting_compactions(id.as_str()).unwrap().is_empty());
}

#[tokio::test]
async fn a_small_context_or_a_recent_turn_is_not_compacted() {
    let (rig, id, _home) = job_waiting(true, true, COMPACT_FROM - 1).await;
    rig.fleet.sweep_quiet().await;
    assert_eq!(compacts_sent(&rig, &id), 0, "a small context");

    let (rig, id, _home) = job_waiting(true, false, 180_000).await;
    rig.fleet.sweep_quiet().await;
    assert_eq!(compacts_sent(&rig, &id), 0, "a recent turn");
}

#[tokio::test]
async fn a_job_waiting_on_nothing_is_not_compacted() {
    let (rig, id, _home) = job_waiting(true, true, 180_000).await;
    rig.fleet.store().lock().await.replace_waiting_for(id.as_str(), &[]).unwrap();
    rig.fleet.sweep_quiet().await;
    assert_eq!(compacts_sent(&rig, &id), 0);
}
