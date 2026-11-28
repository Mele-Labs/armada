//! The Operator as one hosted session: started while its switch is on, in a folder of its own with no
//! repository, ended with the switch, and carried on to a successor that is still the Operator.

use super::session_host::{eventually, rig_placing, Rig};

pub(super) async fn switched(rig: &Rig, on: bool) {
    let save = ipc::SaveSettings { changes: std::collections::BTreeMap::from([("features.operator".to_string(), Some(ipc::SettingValue::Bool(on)))]) };
    api::Settings::save_settings(rig.fleet.as_ref(), save).await.expect("saved");
}

async fn saved(rig: &Rig, name: &str, word: &str) {
    rig.fleet
        .save_preference_now(ipc::SavePreference { name: name.into(), value: false, text: Some(word.into()) })
        .await
        .unwrap_or_else(|why| panic!("{name} {word}: {why:?}"));
}

/// The live Operator's session, in a rig whose one repository is its one project's.
pub(super) async fn live(rig: &Rig) -> Option<String> {
    let all = rig.fleet.operator_sessions().await;
    assert!(all.len() <= 1, "one project, one Operator: {all:?}");
    all.into_iter().next().map(|(_, id)| id)
}

#[tokio::test]
async fn the_operators_session_starts_with_its_switch_in_a_folder_of_its_own_and_ends_with_it() {
    let rig = rig_placing(None);
    rig.fleet.operator_session_kept().await;
    assert!(live(&rig).await.is_none(), "off: none is started");

    switched(&rig, true).await;
    rig.fleet.operator_session_kept().await;
    let id = live(&rig).await.expect("on: one is started");
    rig.fleet.operator_session_kept().await;
    assert_eq!(live(&rig).await.as_deref(), Some(id.as_str()), "a second tick starts no second");

    let kept = rig.fleet.store().lock().await.session(&id).expect("read").expect("a row");
    assert!(kept.manifest_id.is_none(), "no repository: {kept:?}");
    let hosting = rig.fleet.store().lock().await.hosting(&id).expect("read").expect("hosted");
    assert!(hosting.operator && hosting.lease_slot.is_none() && hosting.manifest_id.is_empty(), "{hosting:?}");
    let project = hosting.project.clone().expect("the Operator is a project's");
    assert!(kept.cwd.ends_with(&format!("operator/session/{project}")), "a folder of its own under the project's id: {}", kept.cwd);

    eventually(|| async { rig.stand_in.starts().iter().any(|one| one.session == id) }).await;
    let start = rig.stand_in.starts().into_iter().find(|one| one.session == id).expect("its process");
    assert!(start.operator.is_some(), "its process is told the Operator's rules: {start:?}");

    let record = api::Sessions::list_sessions(rig.fleet.as_ref(), None, None, None).await.expect("listed").sessions.into_iter().find(|one| one.id.as_str() == id).expect("on the ledger");
    assert!(record.operator, "the record says so");

    switched(&rig, false).await;
    rig.fleet.operator_session_kept().await;
    assert!(live(&rig).await.is_none(), "turned off: it ends");
}

/// **The Operator is not a Job**, so its start note carries how the owner takes things in, from his
/// Settings, and neither the Jobs' Logbook nor a Job's own acts, `all_done` and `waiting_for`.
#[tokio::test]
async fn the_operators_start_note_carries_how_he_takes_things_in_and_no_jobs_acts() {
    let rig = rig_placing(None);
    saved(&rig, "take_in", "look").await;
    saved(&rig, "answer_with", "buttons").await;
    switched(&rig, true).await;
    rig.fleet.operator_session_kept().await;
    let id = live(&rig).await.expect("started");
    eventually(|| async { rig.stand_in.starts().iter().any(|one| one.session == id) }).await;

    let start = rig.stand_in.starts().into_iter().find(|one| one.session == id).expect("its process");
    let told = start.note.unwrap_or_default();
    assert!(told.contains("lead every answer and result with a picture through show_sketch"), "{told}");
    assert!(told.contains("ask every question with two to four options, your pick first, a cost on each"), "{told}");
    for word in ["all_done", "waiting_for", "AskUserQuestion", "How the owner works with Jobs", "The Logbook"] {
        assert!(!told.contains(word), "{word}: {told}");
    }
}

/// A change to how he takes things in reaches the running Operator, in its own words, as it reaches a Job.
#[tokio::test]
async fn a_saved_change_to_how_he_takes_things_in_is_told_to_the_running_operator() {
    let rig = rig_placing(None);
    switched(&rig, true).await;
    rig.fleet.operator_session_kept().await;
    let id = live(&rig).await.expect("started");
    eventually(|| async { rig.stand_in.process_of(&id).is_some() }).await;

    let before = api::Settings::get_preferences(rig.fleet.as_ref()).await.expect("read");
    saved(&rig, "answer_with", "talk").await;
    let now = api::Settings::get_preferences(rig.fleet.as_ref()).await.expect("read");
    api::Settings::preferences_saved(std::sync::Arc::clone(&rig.fleet), before, now).await;

    let sent = rig.stand_in.process_of(&id).expect("running").sent.lock().unwrap().join("\n");
    assert!(sent.contains("he answers by voice"), "{sent}");
    assert!(!sent.contains("How the owner works with Jobs"), "a Job's note is not the Operator's: {sent}");
}

#[tokio::test]
async fn a_hand_on_keeps_the_successor_the_operator() {
    let rig = rig_placing(None);
    switched(&rig, true).await;
    rig.fleet.operator_session_kept().await;
    let old = live(&rig).await.expect("started");
    eventually(|| async { rig.stand_in.starts().iter().any(|one| one.session == old) }).await;

    rig.fleet.handed_on(&old, None).await;
    eventually(|| async { live(&rig).await.is_some_and(|now| now != old) }).await;
    let new = live(&rig).await.expect("a successor");
    let hosting = rig.fleet.store().lock().await.hosting(&new).expect("read").expect("hosted");
    assert!(hosting.operator && hosting.manifest_id.is_empty(), "{hosting:?}");
    assert_eq!(rig.fleet.job_now(&old).await, new, "the hand is one Job to him");
}

#[tokio::test]
async fn an_act_only_the_operator_may_make_is_refused_to_any_other_caller() {
    let rig = rig_placing(None);
    let refused = rig.fleet.operator_called(None).expect_err("no session placed");
    assert!(format!("{refused:?}").contains("fleet.not_the_operator"), "{refused:?}");
}

/// **A picked effort is the Operator's own setting**, so a session that ends and starts again runs on it; the
/// harness's own word names none.
#[tokio::test]
async fn the_operators_session_starts_on_its_saved_effort_and_again_after_a_restart() {
    let rig = rig_placing(None);
    switched(&rig, true).await;
    rig.fleet.operator_session_kept().await;
    let first = live(&rig).await.expect("started");
    eventually(|| async { rig.stand_in.starts().iter().any(|one| one.session == first) }).await;
    let start = rig.stand_in.starts().into_iter().find(|one| one.session == first).expect("its process");
    assert_eq!(start.effort, None, "none saved: the harness picks");

    let save = ipc::SaveSettings { changes: std::collections::BTreeMap::from([("effort.operator".to_string(), Some(ipc::SettingValue::Text("high".into())))]) };
    api::Settings::save_settings(rig.fleet.as_ref(), save).await.expect("saved");
    switched(&rig, false).await;
    rig.fleet.operator_session_kept().await;
    switched(&rig, true).await;
    rig.fleet.operator_session_kept().await;
    let second = live(&rig).await.expect("started again");
    assert_ne!(first, second);
    eventually(|| async { rig.stand_in.starts().iter().any(|one| one.session == second) }).await;
    let start = rig.stand_in.starts().into_iter().find(|one| one.session == second).expect("its process");
    assert_eq!(start.effort.as_deref(), Some("high"), "the restart runs on the effort saved");
}
