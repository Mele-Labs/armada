//! The Session model's step down through a real Fleet: the weekly window fed as sessions report it,
//! the owner's four commands, and what `models.session` is afterwards. `docs/concepts/machine.md`, *Quota*.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Next, Queries, Refusal, Sessions, Settings, Subscription};
use ipc::{
    Event, ModelDownshift, SaveSettings, SessionFact, SessionId, SessionReport, SessionUsage, SettingValue,
};
use store::settings_file;
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::settings::Reread;
use crate::tests::daemon::fittings;
use crate::tests::session_host::{eventually, rig_placing};
use crate::tests::settings_json::settings_file_in;
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const WEEK: &str = "2026-10-17T09:00:00.000Z";
const NEXT_WEEK: &str = "2026-10-24T09:00:00.000Z";

fn fleet_in(home: &TempDir) -> Fixture {
    Fleet::assembled(fittings(home, FakeWorkProduct::changed(&[])))
}

fn report(session: &str, fact: SessionFact) -> SessionReport {
    SessionReport { harness: "a_harness".into(), session_id: SessionId::carried(session), fact }
}

/// A session reports the weekly window at `percent`, resetting at `resets_at`.
async fn feed(fleet: &Fixture, percent: u32, resets_at: &str) {
    let started = SessionFact::Started {
        cwd: "/nowhere".into(),
        title: None,
        origin: ipc::SessionOrigin::Terminal,
        mod_version: None,
    };
    fleet.report_session(report("s1", started)).await.unwrap();
    let plan = vec![ipc::PlanWindow {
        kind: "seven_day".into(),
        percent_used: percent,
        resets_at: Some(ipc::Instant::carried(resets_at)),
    }];
    let measured = SessionFact::Measured { usage: SessionUsage::default(), plan, context_breakdown: None };
    fleet.report_session(report("s1", measured)).await.unwrap();
}

async fn state(fleet: &Fixture) -> Option<ModelDownshift> {
    fleet.get_model_downshift().await.unwrap().downshift
}

fn changing(value: Option<&str>) -> SaveSettings {
    SaveSettings {
        changes: BTreeMap::from([("models.session".to_string(), value.map(|one| SettingValue::Text(one.into())))]),
    }
}

fn changing_default(value: Option<&str>) -> SaveSettings {
    SaveSettings {
        changes: BTreeMap::from([("models.default".to_string(), value.map(|one| SettingValue::Text(one.into())))]),
    }
}

/// What settings.json holds for `models.session`.
fn file_session(home: &TempDir) -> Option<String> {
    let file = settings_file::read(&settings_file_in(home)).ok().flatten()?;
    match file.get("models.session") {
        Some(settings_file::SettingValue::Text(text)) => Some(text.clone()),
        _ => None,
    }
}

fn offered(percent: u32) -> ModelDownshift {
    ModelDownshift::Offered { percent, was: "a-model".into(), to: "another-model".into() }
}

fn shifted(until: &str) -> ModelDownshift {
    ModelDownshift::Shifted { was: "a-model".into(), to: "another-model".into(), until: ipc::Instant::carried(until) }
}

fn refused_as_conflict<T: std::fmt::Debug>(answer: Result<T, Refusal>) {
    match answer {
        Err(Refusal::IllegalMove(_)) => {}
        other => panic!("a 409 was expected, not {other:?}"),
    }
}

async fn next_state(watching: &mut Subscription) -> Option<ModelDownshift> {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match watching.next().await {
                Some(Next::Send(delivered)) => {
                    if let Event::ModelDownshiftChanged(state) = delivered.event {
                        return state.downshift;
                    }
                }
                other => panic!("the stream ended or dropped: {other:?}"),
            }
        }
    })
    .await
    .expect("a model_downshift.changed arrives")
}

#[tokio::test]
async fn the_line_offers_once_for_a_window_and_the_percent_follows_while_it_stands() {
    let home = TempDir::new();
    let fleet = fleet_in(&home);
    feed(&fleet, 40, WEEK).await;
    feed(&fleet, 79, WEEK).await;
    assert_eq!(state(&fleet).await, None, "below the line nothing is offered");

    let mut watching = fleet.events().subscribe();
    feed(&fleet, 80, WEEK).await;
    assert_eq!(state(&fleet).await, Some(offered(80)));
    assert_eq!(next_state(&mut watching).await, Some(offered(80)), "and said on the stream");

    feed(&fleet, 86, WEEK).await;
    assert_eq!(state(&fleet).await, Some(offered(86)));

    fleet.settle_downshift().await.unwrap();
    assert_eq!(next_state(&mut watching).await, Some(offered(86)), "the percent moved once more");
    assert_eq!(next_state(&mut watching).await, None, "settled");
    feed(&fleet, 95, WEEK).await;
    assert_eq!(state(&fleet).await, None, "a settled window is not offered again");
    assert_eq!(fleet.session_model(), "a-model", "settling touches no setting");
}

#[tokio::test]
async fn nothing_is_offered_where_the_default_is_already_the_lower_model() {
    let home = TempDir::new();
    let fleet = fleet_in(&home);
    fleet.save_settings(changing(Some("another-model"))).await.unwrap();
    feed(&fleet, 97, WEEK).await;
    assert_eq!(state(&fleet).await, None);
}

#[tokio::test]
async fn a_new_window_withdraws_an_offer_and_judges_the_reading_afresh() {
    let home = TempDir::new();
    let fleet = fleet_in(&home);
    feed(&fleet, 85, WEEK).await;
    assert_eq!(state(&fleet).await, Some(offered(85)));
    feed(&fleet, 10, NEXT_WEEK).await;
    assert_eq!(state(&fleet).await, None);
    feed(&fleet, 81, NEXT_WEEK).await;
    assert_eq!(state(&fleet).await, Some(offered(81)), "the new window is offered on its own");
}

#[tokio::test]
async fn accepting_writes_the_saved_word_and_restoring_puts_it_back() {
    let home = TempDir::new();
    let fleet = fleet_in(&home);
    fleet.save_settings(changing(Some("a-model"))).await.unwrap();
    feed(&fleet, 82, WEEK).await;

    let answered = fleet.accept_downshift().await.unwrap();
    assert_eq!(answered.downshift, Some(shifted(WEEK)));
    assert_eq!(file_session(&home).as_deref(), Some("another-model"));
    assert_eq!(fleet.session_model(), "another-model");
    assert_eq!(fleet.list_models(false).await.unwrap().session, "another-model", "the picker follows");

    let answered = fleet.restore_downshift().await.unwrap();
    assert_eq!(answered.downshift, None);
    assert_eq!(file_session(&home).as_deref(), Some("a-model"), "the word the file held");
    assert_eq!(fleet.session_model(), "a-model");
}

#[tokio::test]
async fn restoring_a_default_that_was_not_saved_removes_the_key_rather_than_pinning_it() {
    let home = TempDir::new();
    let fleet = fleet_in(&home);
    feed(&fleet, 90, WEEK).await;
    fleet.accept_downshift().await.unwrap();
    assert_eq!(file_session(&home).as_deref(), Some("another-model"));

    fleet.restore_downshift().await.unwrap();
    assert_eq!(file_session(&home), None, "the key is gone, so what ships is back in force");
    assert_eq!(fleet.session_model(), "a-model");
}

#[tokio::test]
async fn the_window_rolling_over_while_shifted_offers_the_way_back_and_keep_leaves_the_lower_model() {
    let home = TempDir::new();
    let fleet = fleet_in(&home);
    feed(&fleet, 88, WEEK).await;
    fleet.accept_downshift().await.unwrap();
    feed(&fleet, 92, WEEK).await;
    assert_eq!(state(&fleet).await, Some(shifted(WEEK)), "the same window changes nothing");

    feed(&fleet, 3, NEXT_WEEK).await;
    assert_eq!(
        state(&fleet).await,
        Some(ModelDownshift::Reset { was: "a-model".into(), to: "another-model".into() })
    );
    assert_eq!(fleet.session_model(), "another-model", "still lowered until the owner says");

    let answered = fleet.keep_downshift().await.unwrap();
    assert_eq!(answered.downshift, None);
    assert_eq!(file_session(&home).as_deref(), Some("another-model"));
    feed(&fleet, 90, NEXT_WEEK).await;
    assert_eq!(state(&fleet).await, None, "default already is the lower model");
}

#[tokio::test]
async fn restoring_after_the_reset_puts_the_default_back() {
    let home = TempDir::new();
    let fleet = fleet_in(&home);
    feed(&fleet, 88, WEEK).await;
    fleet.accept_downshift().await.unwrap();
    feed(&fleet, 3, NEXT_WEEK).await;
    fleet.restore_downshift().await.unwrap();
    assert_eq!(state(&fleet).await, None);
    assert_eq!(fleet.session_model(), "a-model");
}

#[tokio::test]
async fn a_command_in_the_wrong_phase_is_a_conflict_and_changes_nothing() {
    let home = TempDir::new();
    let fleet = fleet_in(&home);
    refused_as_conflict(fleet.accept_downshift().await);
    refused_as_conflict(fleet.restore_downshift().await);
    refused_as_conflict(fleet.keep_downshift().await);
    refused_as_conflict(fleet.settle_downshift().await);

    feed(&fleet, 85, WEEK).await;
    refused_as_conflict(fleet.restore_downshift().await);
    refused_as_conflict(fleet.keep_downshift().await);
    fleet.accept_downshift().await.unwrap();
    refused_as_conflict(fleet.accept_downshift().await);
    refused_as_conflict(fleet.settle_downshift().await);
    refused_as_conflict(fleet.keep_downshift().await);
    assert_eq!(state(&fleet).await, Some(shifted(WEEK)));
}

#[tokio::test]
async fn a_refused_write_keeps_the_offer_and_the_default() {
    let home = TempDir::new();
    let fleet = fleet_in(&home);
    feed(&fleet, 85, WEEK).await;
    // A directory where the file goes: the rename over it fails, as a full disk or a lock would.
    std::fs::create_dir(settings_file_in(&home)).unwrap();
    assert!(matches!(fleet.accept_downshift().await, Err(Refusal::Fault(_))));
    assert_eq!(state(&fleet).await, Some(offered(85)));
    assert_eq!(fleet.session_model(), "a-model");
}

#[tokio::test]
async fn a_shifted_default_survives_a_restart_and_an_empty_reading_does_not_clear_it() {
    let home = TempDir::new();
    {
        let fleet = fleet_in(&home);
        feed(&fleet, 85, WEEK).await;
        fleet.accept_downshift().await.unwrap();
    }
    let fleet = fleet_in(&home);
    assert_eq!(state(&fleet).await, Some(shifted(WEEK)), "no session has measured since the start");
    assert_eq!(fleet.session_model(), "another-model");

    feed(&fleet, 30, WEEK).await;
    assert_eq!(state(&fleet).await, Some(shifted(WEEK)));
    feed(&fleet, 2, NEXT_WEEK).await;
    assert!(matches!(state(&fleet).await, Some(ModelDownshift::Reset { .. })));
}

#[tokio::test]
async fn a_settled_window_is_remembered_across_a_restart() {
    let home = TempDir::new();
    {
        let fleet = fleet_in(&home);
        feed(&fleet, 85, WEEK).await;
        fleet.settle_downshift().await.unwrap();
    }
    let fleet = fleet_in(&home);
    feed(&fleet, 96, WEEK).await;
    assert_eq!(state(&fleet).await, None);
}

#[tokio::test]
async fn setting_the_default_by_hand_settles_an_offer_and_a_shift_but_an_unrelated_edit_does_not() {
    let home = TempDir::new();
    let fleet = fleet_in(&home);
    feed(&fleet, 85, WEEK).await;
    std::fs::write(settings_file_in(&home), r#"{"limits.dronesAtOnce": 3}"#).unwrap();
    assert!(matches!(fleet.settings_reread().await, Reread::Taken));
    assert_eq!(state(&fleet).await, Some(offered(85)), "another setting is not the default");

    std::fs::write(settings_file_in(&home), r#"{"models.session": "another-model"}"#).unwrap();
    assert!(matches!(fleet.settings_reread().await, Reread::Taken));
    assert_eq!(state(&fleet).await, None, "the owner chose");
    feed(&fleet, 93, WEEK).await;
    assert_eq!(state(&fleet).await, None, "and is not asked again in this window");

    feed(&fleet, 20, NEXT_WEEK).await;
    feed(&fleet, 85, NEXT_WEEK).await;
    assert_eq!(state(&fleet).await, None, "the default is the lower model now");
    fleet.save_settings(changing(Some("a-model"))).await.unwrap();
    feed(&fleet, 86, NEXT_WEEK).await;
    assert_eq!(state(&fleet).await, Some(offered(86)), "a default that is not the lower model is offered over");
}

#[tokio::test]
async fn a_hand_edit_away_from_the_lower_model_ends_a_shift() {
    let home = TempDir::new();
    let fleet = fleet_in(&home);
    feed(&fleet, 85, WEEK).await;
    fleet.accept_downshift().await.unwrap();
    std::fs::write(settings_file_in(&home), r#"{"models.session": "a-model", "limits.dronesAtOnce": 2}"#).unwrap();
    assert!(matches!(fleet.settings_reread().await, Reread::Taken));
    assert_eq!(state(&fleet).await, None);
}

#[tokio::test]
async fn the_offer_follows_the_session_model_and_not_the_drones() {
    let home = TempDir::new();
    let fleet = fleet_in(&home);
    // The Drone's default already is the lower model; the Session's is not.
    fleet.save_settings(changing_default(Some("another-model"))).await.unwrap();
    feed(&fleet, 80, WEEK).await;
    assert_eq!(state(&fleet).await, Some(offered(80)), "at the line, over the Session's model");

    // Editing the Drone's default by hand while offered is nobody's step down, so it settles nothing.
    fleet.save_settings(changing_default(Some("a-model"))).await.unwrap();
    assert_eq!(state(&fleet).await, Some(offered(80)));
    fleet.accept_downshift().await.unwrap();
    assert_eq!(file_session(&home).as_deref(), Some("another-model"));
}

#[tokio::test]
async fn a_session_started_with_no_model_runs_on_the_session_model_at_the_time_its_process_starts() {
    let rig = rig_placing(None);
    let first = rig.start().await;
    rig.send(&first, "hello").await;
    eventually(|| async { rig.stand_in.starts().len() == 1 }).await;
    assert_eq!(rig.stand_in.starts()[0].model.as_deref(), Some("a-model"));

    Arc::clone(&rig.fleet).save_settings(changing(Some("another-model"))).await.unwrap();
    let second = rig.start().await;
    rig.send(&second, "hello").await;
    eventually(|| async { rig.stand_in.starts().len() == 2 }).await;
    assert_eq!(rig.stand_in.starts()[1].model.as_deref(), Some("another-model"));
    assert_eq!(rig.stand_in.starts()[0].model.as_deref(), Some("a-model"), "a process running is not moved");
    assert_eq!(rig.fleet.default_model(), "a-model", "a Drone's default is not the Session's");
}
