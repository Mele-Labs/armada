//! The Operator's roles and its call, as Fleet reads them: every role ships Read-only, a switch saved
//! with two positions or the old words carries across, **the retired Answer role stays Read-only
//! whatever is saved**, and the call is kept started only while the Operator is on.

use std::sync::Arc;
use std::time::Duration;

use ipc::SessionId;

use super::session_host::{rig_placing, Rig};

async fn switched(rig: &Rig, key: &str, on: bool) {
    let value = match key {
        "features.operator" => ipc::SettingValue::Bool(on),
        _ => ipc::SettingValue::Text(if on { "decide" } else { "read_only" }.to_string()),
    };
    let save = ipc::SaveSettings { changes: std::collections::BTreeMap::from([(key.to_string(), Some(value))]) };
    api::Settings::save_settings(rig.fleet.as_ref(), save).await.expect("saved");
}

/// The Operator turned on, which ships off, and its Dispatch role as `on` says.
pub(super) async fn dispatch(rig: &Rig, on: bool) {
    switched(rig, "features.operator", true).await;
    switched(rig, "features.operatorDispatch", on).await;
    switched(rig, "features.operatorDelegate", on).await;
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

    switched(&rig, "features.operator", true).await;
    rig.fleet.operator_kept_in_step().await;
    assert_eq!(judge.models().len(), 1);
    assert!(call.is_waiting().await, "on: one waits");

    switched(&rig, "features.operator", false).await;
    rig.fleet.operator_kept_in_step().await;
    assert!(!call.is_waiting().await, "turned off live: the one waiting is stopped");
}

#[tokio::test]
async fn every_role_ships_read_only_and_a_switch_saved_with_two_positions_or_the_old_words_carries_across() {
    use ipc::operator_positions::{OperatorPosition as Position, OperatorRole as Role};
    let rig = rig_placing(None);
    switched(&rig, "features.operator", true).await;
    for role in [Role::Dispatch, Role::Delegate, Role::Answer, Role::File] {
        assert_eq!(rig.fleet.operator_position(role, "a repository"), Position::ReadOnly, "{role:?}");
    }

    // Saved before a role had three positions; Dispatch's switch held Delegate too. A saved Answer is
    // not read: that role is retired, and questions and cards go to the person.
    let saved = [("features.operatorDispatch", true), ("features.operatorAnswer", true), ("features.operatorFile", true)];
    let save = ipc::SaveSettings {
        changes: saved.into_iter().map(|(key, on)| (key.to_string(), Some(ipc::SettingValue::Bool(on)))).collect(),
    };
    api::Settings::save_settings(rig.fleet.as_ref(), save).await.expect("a saved switch is still read");

    for (role, position) in [(Role::Dispatch, Position::Decide), (Role::Delegate, Position::Decide), (Role::Answer, Position::ReadOnly), (Role::File, Position::Decide)] {
        assert_eq!(rig.fleet.operator_position(role, "a repository"), position, "{role:?}");
    }
    switched(&rig, "features.operatorDelegate", false).await;
    assert_eq!(rig.fleet.operator_position(Role::Delegate, "a repository"), Position::ReadOnly, "its own value wins once he sets one");

    // Saved as the words of the three positions that came first, and any other word.
    for (word, position) in [("on", Position::Decide), ("off", Position::ReadOnly), ("suggest", Position::Suggest), ("decide", Position::Decide), ("read_only", Position::ReadOnly)] {
        let save = ipc::SaveSettings { changes: std::collections::BTreeMap::from([("features.operatorFile".to_string(), Some(ipc::SettingValue::Text(word.to_string())))]) };
        api::Settings::save_settings(rig.fleet.as_ref(), save).await.expect("an old word is still saved");
        assert_eq!(rig.fleet.operator_position(Role::File, "a repository"), position, "{word}");
    }
}

#[tokio::test]
async fn a_jobs_own_events_reach_no_call_and_its_first_write_waits_for_nothing_with_the_operator_on() {
    use super::operator_sources::Answers;
    use super::session_host::denied;
    use super::session_queue::close;

    let rig = rig_placing(None);
    dispatch(&rig, true).await;
    let call = Answers::with(&[]);
    assert!(rig.fleet.operator_seated_with(call.clone()));

    let first = asked(&rig, "Keep the terminal scrollback when the pane redraws").await;
    let second = asked(&rig, "Keep the terminal scrollback when the pane resizes").await;
    for id in [&first, &second] {
        let write = rig.gate(id, "Write", &format!(r#"{{"file_path":"{}/panes/terminal.tsx"}}"#, rig.root)).await;
        assert!(!denied(&write), "its first write is not held for a ruling: {write:?}");
    }
    close(&rig, &first).await;
    close(&rig, &second).await;
    rig.fleet.sources_handed().await;

    assert!(call.asked().is_empty(), "a Job's start, write and end hand the Operator nothing: {:?}", call.asked());
    assert!(rig.fleet.store().lock().await.operator_rulings(10).expect("read").is_empty(), "and it rules on none of them");
}
