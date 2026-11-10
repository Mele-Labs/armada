//! The Operator's call, as Fleet keeps it: started only while the Operator is on. **There are no levels**:
//! the switch is the one setting that turns it on, and every act it would take asks.

use std::sync::Arc;
use std::time::Duration;

use ipc::SessionId;

use super::session_host::{rig_placing, Rig};

async fn switched(rig: &Rig, on: bool) {
    let save = ipc::SaveSettings { changes: std::collections::BTreeMap::from([("features.operator".to_string(), Some(ipc::SettingValue::Bool(on)))]) };
    api::Settings::save_settings(rig.fleet.as_ref(), save).await.expect("saved");
}

/// The Operator turned on, which ships off.
pub(super) async fn operator_on(rig: &Rig) {
    switched(rig, true).await;
}

/// A Job asked `words`, its first message.
pub(super) async fn asked(rig: &Rig, words: &str) -> SessionId {
    let id = rig.start().await;
    rig.send(&id, words).await;
    id
}

#[tokio::test]
async fn the_call_is_kept_started_only_while_the_operator_is_on() {
    let rig = rig_placing(None);
    let judge = Arc::new(testkit::FakeJudge::saying("item: 1\nway: 1\njob: none\nshared: none\nbecause: nothing"));
    let call = Arc::new(crate::operator::KeptStarted::new(
        judge.clone(),
        || adapter_traits::Model::named("the-operator-model").ok(),
        adapter_traits::Environment::nothing(),
        crate::JudgeBudget::of(Duration::from_secs(10)),
    ));

    rig.fleet.operator_kept(Arc::clone(&call), Arc::new(crate::operator::Blind(call.clone()))).await;
    assert!(judge.models().is_empty() && !call.is_waiting().await, "off: no process is started");

    switched(&rig, true).await;
    rig.fleet.operator_kept_in_step().await;
    assert_eq!(judge.models().len(), 1);
    assert!(call.is_waiting().await, "on: one waits");

    switched(&rig, false).await;
    rig.fleet.operator_kept_in_step().await;
    assert!(!call.is_waiting().await, "turned off live: the one waiting is stopped");
}

#[tokio::test]
async fn a_jobs_own_events_reach_no_call_and_its_first_write_waits_for_nothing_with_the_operator_on() {
    use super::operator_sources::Answers;
    use super::session_queue::close;

    let rig = rig_placing(None);
    operator_on(&rig).await;
    let call = Answers::with(&[]);
    assert!(rig.fleet.operator_seated_with(call.clone()));

    let first = asked(&rig, "Keep the terminal scrollback when the pane redraws").await;
    let second = asked(&rig, "Keep the terminal scrollback when the pane resizes").await;
    for id in [&first, &second] {
        let write = rig.gate(id, "Write", &format!(r#"{{"file_path":"{}/panes/terminal.tsx"}}"#, rig.root)).await;
        // The first write still moves the Job into a slot, which is Fleet's own; no ruling is waited on.
        assert!(!format!("{write:?}").contains("Operator") && !format!("{write:?}").contains("ruling"), "its first write is not held for a ruling: {write:?}");
    }
    close(&rig, &first).await;
    close(&rig, &second).await;
    rig.fleet.sources_handed().await;

    assert!(call.asked().is_empty(), "a Job's start, write and end hand the Operator nothing: {:?}", call.asked());
    assert!(rig.fleet.store().lock().await.operator_rulings(10).expect("read").is_empty(), "and it rules on none of them");
}
