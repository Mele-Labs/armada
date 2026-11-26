//! The Operator as one hosted session: started while its switch is on, in a folder of its own with no
//! repository, ended with the switch, and carried on to a successor that is still the Operator.

use super::session_host::{eventually, rig_placing, Rig};

pub(super) async fn switched(rig: &Rig, on: bool) {
    let save = ipc::SaveSettings { changes: std::collections::BTreeMap::from([("features.operator".to_string(), Some(ipc::SettingValue::Bool(on)))]) };
    api::Settings::save_settings(rig.fleet.as_ref(), save).await.expect("saved");
}

pub(super) async fn live(rig: &Rig) -> Option<String> {
    rig.fleet.operator_session_id().await
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
    assert!(kept.cwd.ends_with("operator/session"), "its own folder: {}", kept.cwd);
    let hosting = rig.fleet.store().lock().await.hosting(&id).expect("read").expect("hosted");
    assert!(hosting.operator && hosting.lease_slot.is_none() && hosting.manifest_id.is_empty(), "{hosting:?}");

    eventually(|| async { rig.stand_in.starts().iter().any(|one| one.session == id) }).await;
    let start = rig.stand_in.starts().into_iter().find(|one| one.session == id).expect("its process");
    assert!(start.operator, "its process is told the Operator's rules: {start:?}");

    let record = api::Sessions::list_sessions(rig.fleet.as_ref(), None, None, None).await.expect("listed").sessions.into_iter().find(|one| one.id.as_str() == id).expect("on the ledger");
    assert!(record.operator, "the record says so");

    switched(&rig, false).await;
    rig.fleet.operator_session_kept().await;
    assert!(live(&rig).await.is_none(), "turned off: it ends");
}

/// **The Operator is not a Job**, so its start note carries neither the Jobs' Logbook nor how the
/// owner works with Jobs, whose lines name `all_done` and `waiting_for`.
#[tokio::test]
async fn the_operators_start_note_carries_no_jobs_logbook_or_ways() {
    let rig = rig_placing(None);
    switched(&rig, true).await;
    rig.fleet.operator_session_kept().await;
    let id = live(&rig).await.expect("started");
    eventually(|| async { rig.stand_in.starts().iter().any(|one| one.session == id) }).await;

    let start = rig.stand_in.starts().into_iter().find(|one| one.session == id).expect("its process");
    let told = start.note.unwrap_or_default();
    assert!(!told.contains(&rig.fleet.ways_note()), "{told}");
    assert!(!told.contains("How the owner works with Jobs"), "{told}");
    assert!(!told.contains("The Logbook"), "{told}");
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
