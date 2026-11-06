//! A model switched off stays off for every Job: what names it goes back to the Job model, what
//! is on it moves when its turn is over, and a process that outlives the switch asks in vain.

use std::collections::BTreeMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use api::{HostedSessions, Settings};
use ipc::{SaveSettings, SessionId, SessionMode, SessionTurn, SettingValue, TuneSession};

use super::session_host::{denied, eventually, reason, rig_placing, Rig};

async fn tuned_to(rig: &Rig, id: &SessionId, model: &str) -> ipc::SessionRecord {
    rig.fleet
        .tune_session(TuneSession {
            session_id: id.clone(),
            model: Some(model.into()),
            effort: None,
            mode: SessionMode::Auto,
        })
        .await
        .expect("tuned")
}

/// `another-model` off, and what ships naming it moved in the same save, as Kit's switch moves it.
async fn another_model_switched_off(rig: &Rig) {
    let word = || Some(SettingValue::Text("a-model".into()));
    let roster = Some(SettingValue::List(vec![SettingValue::Text("a-model".into())]));
    let changes = BTreeMap::from([
        ("models.roster".to_string(), roster),
        ("models.retro".to_string(), word()),
        ("models.secondOpinion".to_string(), word()),
        ("plan.downshiftTo".to_string(), word()),
    ]);
    rig.fleet.save_settings(SaveSettings { changes }).await.expect("saved");
}

/// What the report saw: a Job handed on from one on Fable ran on Fable with the switch off.
#[tokio::test]
async fn a_job_naming_a_model_switched_off_starts_on_the_job_model_and_says_so() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    tuned_to(&rig, &id, "a-model-switched-off").await;

    rig.send(&id, "first").await;
    let started = &rig.stand_in.starts()[0];
    assert_eq!(started.model.as_deref(), Some("a-model"));
    assert_eq!(started.models, ["a-model", "another-model"], "the process is held to the models on");
    let hosted = Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("served").session.hosted.expect("hosted");
    assert_eq!(hosted.model, None, "its row names what it runs on");
}

/// Off moves a Job on the model as its model picker would: an idle one now, one working when its
/// turn is over. A Job on another model is left running.
#[tokio::test]
async fn switching_a_model_off_moves_the_jobs_on_it_when_their_turn_is_over() {
    let rig = rig_placing(None);
    let idle = rig.start().await;
    tuned_to(&rig, &idle, "another-model").await;
    rig.send(&idle, "first").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "ok");
    eventually(|| async { rig.turn(&idle).await == SessionTurn::Idle }).await;

    let working = rig.start().await;
    tuned_to(&rig, &working, "another-model").await;
    rig.send(&working, "first").await;
    rig.stand_in.init(1);

    let other = rig.start().await;
    rig.send(&other, "first").await;
    rig.stand_in.init(2);
    rig.stand_in.finishes(2, "ok");
    eventually(|| async { rig.turn(&other).await == SessionTurn::Idle }).await;

    another_model_switched_off(&rig).await;

    assert!(rig.stand_in.process(0).ended.load(Ordering::SeqCst), "an idle process on it is ended");
    assert!(!rig.stand_in.process(1).ended.load(Ordering::SeqCst), "a turn is not cut short");
    assert!(!rig.stand_in.process(2).ended.load(Ordering::SeqCst), "a Job on another model is left");
    rig.stand_in.finishes(1, "ok");
    eventually(|| async { rig.stand_in.process(1).ended.load(Ordering::SeqCst) }).await;

    rig.send(&idle, "second").await;
    let next = rig.stand_in.starts().pop().expect("started");
    assert_eq!(next.session, idle.as_str());
    assert_eq!(next.model.as_deref(), Some("a-model"));
    assert_eq!(next.models, ["a-model"]);
}

/// A process keeps the allowlist it started with, so a subagent asked for on a model switched off
/// since is refused here. One on a model on, or naming none, runs.
#[tokio::test]
async fn a_subagent_asked_for_on_a_model_switched_off_is_refused() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    another_model_switched_off(&rig).await;

    let answer = rig.gate(&id, "Agent", r#"{"prompt":"go","model":"another-model"}"#).await;
    assert!(denied(&answer), "{answer:?}");
    assert!(reason(&answer).contains("another-model") && reason(&answer).contains("a-model"), "{answer:?}");

    assert!(!denied(&rig.gate(&id, "Agent", r#"{"prompt":"go","model":"a-model"}"#).await));
    assert!(!denied(&rig.gate(&id, "Agent", r#"{"prompt":"go"}"#).await));
}
