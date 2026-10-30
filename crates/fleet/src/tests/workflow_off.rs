//! The claim, written before the code: **one workflow can be switched off, for the machine or for one
//! repository, beside the switch for all of them. An off workflow is not offered to the proposer or
//! to Autopilot's picker, cannot be given to a Job, and a Job already on it runs as it does with the
//! main switch off; switching it back on picks the rail up where the facts put it.**
//! `crates/config/settings.toml`, *Workflows switched off*.
//!
//! Beside `session_rail`, which sits near the line count the gate reports, and whose tests of the
//! main switch these follow. What a held Job takes as its default is in `start_card`.

use std::collections::BTreeMap;
use std::sync::Arc;

use api::{Autopilots, HostedSessions, Refusal, Sessions};
use ipc::{
    AutopilotSettings, BeginSession, DeliveryChoice, JudgeSession, SaveSettings, SessionFact, SessionReport, SetSessionWorkflow,
    SettingValue, SettingsList,
};
use testkit::{FakeBeads, FakeBeadsRemote, FakeJudge, FakeTrackers};

use super::session_host::{eventually, rig_fitted, Rig};
use super::session_rail::{approved_plan, edit, on_the_rail, record, told, workflow, REQUEST};

/// The `workflows.off` setting as Bridge saves it: the machine's list, and one list per repository.
pub(super) fn off(machine: &[&str], repositories: &[(&str, &[&str])]) -> SaveSettings {
    let list = |names: &[&str]| SettingValue::List(names.iter().map(|one| SettingValue::Text((*one).into())).collect());
    let value = SettingValue::Map(BTreeMap::from([
        ("machine".to_string(), list(machine)),
        ("repositories".to_string(), SettingValue::Map(repositories.iter().map(|(manifest, names)| (manifest.to_string(), list(names))).collect())),
    ]));
    SaveSettings { changes: BTreeMap::from([("workflows.off".to_string(), Some(value))]) }
}

/// What `workflows.off` held, taken back out.
pub(super) fn back_on() -> SaveSettings {
    SaveSettings { changes: BTreeMap::from([("workflows.off".to_string(), None)]) }
}

/// The main switch, `features.workflows`.
fn main_switch(on: bool) -> SaveSettings {
    SaveSettings { changes: BTreeMap::from([("features.workflows".to_string(), Some(SettingValue::Bool(on)))]) }
}

pub(super) async fn saved(rig: &Rig, save: SaveSettings) -> SettingsList {
    api::Settings::save_settings(rig.fleet.as_ref(), save).await.expect("saved")
}

/// The repository the rig serves.
fn here(rig: &Rig) -> String {
    rig.manifest.as_str().to_string()
}

fn two_workflows(fittings: &mut crate::daemon::Fittings<testkit::FakeHarness, testkit::FakeVcs, testkit::FakeWorkProduct>) {
    fittings.starting().workflows =
        ["bug", "feature"].into_iter().map(super::daemon::workflow_named).map(|one| (one.id().clone(), one)).collect();
}

#[tokio::test]
async fn a_workflow_is_on_only_where_the_main_switch_is_on_and_neither_list_names_it() {
    let rig = rig_fitted(|_| {});
    let on = |manifest: &str, workflow: &str| rig.fleet.workflow_on(manifest, workflow);
    assert!(on("one", "bug") && on("two", "bug"), "nothing is off to begin with");

    saved(&rig, off(&["bug"], &[])).await;
    assert!(!on("one", "bug") && !on("two", "bug"), "off for the machine is off in every repository");
    assert!(on("one", "feature"), "and only that workflow");

    saved(&rig, off(&[], &[("one", &["feature"])])).await;
    assert!(!on("one", "feature"), "off for that repository");
    assert!(on("two", "feature"), "and on for another");
    assert!(on("one", "bug"), "the machine's list is empty now");

    saved(&rig, back_on()).await;
    assert!(on("one", "feature") && on("one", "bug"), "removed, they are on again");
    saved(&rig, main_switch(false)).await;
    assert!(!on("one", "bug") && !on("two", "feature"), "the main switch off is every workflow off");
    saved(&rig, off(&[], &[("one", &["bug"])])).await;
    assert!(!on("two", "bug"), "and a list does not switch it back on");
}

#[tokio::test]
async fn workflows_off_saved_in_settings_takes_effect_on_the_next_read_with_no_restart() {
    let rig = rig_fitted(|_| {});
    assert!(rig.fleet.workflow_on(&here(&rig), "bug"));

    let answered = saved(&rig, off(&[], &[(&here(&rig), &["bug"])])).await;
    let row = answered.settings.iter().find(|one| one.key == "workflows.off").expect("the row");
    assert!(!row.pending_restart, "nothing waits for a restart: {row:?}");
    assert!(row.saved.is_some(), "the value is kept: {row:?}");
    assert!(!rig.fleet.workflow_on(&here(&rig), "bug"), "the next read sees it");

    saved(&rig, back_on()).await;
    assert!(rig.fleet.workflow_on(&here(&rig), "bug"), "and the next sees it taken out");
}

#[tokio::test]
async fn the_proposer_is_shown_no_workflow_that_is_off_for_its_repository_and_every_one_for_another() {
    let brief = |save: SaveSettings| async move {
        let judge = Arc::new(FakeJudge::saying("workflow: feature\ntitle: Log reader drops a line"));
        let rig = rig_fitted(|fittings| {
            fittings.judge = Arc::clone(&judge) as _;
            two_workflows(fittings);
        });
        saved(&rig, save).await;
        let id = rig.start_requesting(REQUEST, |_| {}).await.id;
        eventually(|| async { record(&rig, &id).await.proposal.is_none_or(|one| !one.reading) }).await;
        judge.asked().remove(0)
    };
    let manifest = super::daemon::manifest().id().as_str().to_string();

    let here_off = brief(off(&[], &[(&manifest, &["bug"])])).await;
    assert!(here_off.contains("only_in_feature") && !here_off.contains("only_in_bug"), "{here_off}");
    let elsewhere_off = brief(off(&[], &[("another-repository", &["bug"])])).await;
    assert!(elsewhere_off.contains("only_in_feature") && elsewhere_off.contains("only_in_bug"), "{elsewhere_off}");
}

#[tokio::test]
async fn autopilots_picker_is_shown_no_workflow_that_is_off_for_its_repository_and_every_one_for_another() {
    let question = |save: SaveSettings| async move {
        let judge = Arc::new(FakeJudge::saying("nothing"));
        let copy = Arc::new(FakeBeads::on(&FakeBeadsRemote::new()));
        copy.create("The log reader drops a line");
        let planted = Arc::clone(&copy);
        let rig = rig_fitted(|fittings| {
            fittings.judge = Arc::clone(&judge) as _;
            fittings.trackers = Arc::new(FakeTrackers::empty().beads_named("beads", planted));
            fittings.starting().manifest =
                config::Manifest::parse(std::path::Path::new("armada.yml"), super::backlog::BEADS).expect("loads");
            two_workflows(fittings);
        });
        saved(&rig, save).await;
        let settings = AutopilotSettings { instruction: "Fix what is open".into(), at_once: 1, until: None, spend_cap_micros: None, session_cap: None };
        rig.fleet.start_autopilot(rig.manifest.clone(), settings).await.expect("started");
        let _ = rig.fleet.autopilot_pass().await;
        judge.asked().into_iter().find(|one| one.contains("The workflows this repository holds")).expect("the picker was asked")
    };
    let manifest = super::daemon::manifest().id().as_str().to_string();

    let here_off = question(off(&[], &[(&manifest, &["bug"])])).await;
    assert!(here_off.contains("only_in_feature") && !here_off.contains("only_in_bug"), "{here_off}");
    let elsewhere_off = question(off(&[], &[("another-repository", &["bug"])])).await;
    assert!(elsewhere_off.contains("only_in_feature") && elsewhere_off.contains("only_in_bug"), "{elsewhere_off}");
}

#[tokio::test]
async fn an_off_workflow_cannot_be_set_on_a_job_and_the_others_and_clearing_still_can() {
    let rig = rig_fitted(|fittings| {
        fittings.judge = Arc::new(FakeJudge::with_no_objection());
        fittings.starting().workflows = ["rail", "bug"]
            .into_iter()
            .map(|id| if id == "rail" { workflow() } else { super::daemon::workflow_named(id) })
            .map(|one| (one.id().clone(), one))
            .collect();
    });
    let id = rig.start().await;
    let set = |named: Option<&str>| {
        Arc::clone(&rig.fleet).set_session_workflow(SetSessionWorkflow { session_id: id.clone(), workflow: named.map(String::from) })
    };
    assert_eq!(set(Some("rail")).await.expect("set while on").workflow.as_deref(), Some("rail"));

    saved(&rig, off(&[], &[(&here(&rig), &["rail"])])).await;
    let refused = set(Some("rail")).await;
    assert!(matches!(&refused, Err(Refusal::Unacceptable(why)) if why.code == "fleet.session_workflows_off"), "{refused:?}");
    assert_eq!(set(Some("bug")).await.expect("another is still allowed").workflow.as_deref(), Some("bug"));
    saved(&rig, off(&["bug"], &[])).await;
    let refused = set(Some("bug")).await;
    assert!(matches!(&refused, Err(Refusal::Unacceptable(why)) if why.code == "fleet.session_workflows_off"), "off for the machine too: {refused:?}");
    assert_eq!(set(None).await.expect("cleared").workflow, None, "turning a workflow off is always allowed");
}

#[tokio::test]
async fn a_job_on_an_off_workflow_has_no_step_nudge_or_boundary_judge_and_switching_it_back_on_resumes_the_rail() {
    let judge = Arc::new(FakeJudge::with_no_objection());
    let rig = super::session_rail::ruled_on(Arc::clone(&judge), workflow());
    let id = on_the_rail(&rig).await;
    assert_eq!(record(&rig, &id).await.workflow_step, Some(0));
    let asked_before = judge.asked().len();
    saved(&rig, off(&[], &[(&here(&rig), &["rail"])])).await;

    rig.stand_in.says(0, vec![edit("crates/reader/src/lib.rs")]);
    approved_plan(&rig, &id).await;
    let harness = record(&rig, &id).await.harness;
    rig.fleet
        .report_session(SessionReport { harness, session_id: id.clone(), fact: SessionFact::Pushed { branch: Some("armada/reader".into()) } })
        .await
        .expect("kept");
    rig.fleet.timeline().settled().await;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let read = record(&rig, &id).await;
    assert_eq!(read.workflow.as_deref(), Some("rail"), "the Job keeps the workflow it has");
    assert_eq!(read.workflow_step, None, "no current step");
    assert_eq!(told(&rig, "edited files before"), 0, "no nudge");
    assert_eq!(judge.asked().len(), asked_before, "no boundary judge: {:?}", judge.asked());
    let tile = Arc::clone(&rig.fleet).judge_session(JudgeSession { session_id: id.clone() }).await.expect("asked");
    assert!(tile.judging, "the tile still reads");

    saved(&rig, back_on()).await;
    let resumed = record(&rig, &id).await;
    assert_eq!(resumed.workflow_step, Some(2), "switched on again, the facts place the Job at its hand-off");
}

#[tokio::test]
async fn a_job_on_an_off_workflow_is_told_neither_its_steps_nor_its_skips_with_its_first_turn() {
    let rig = super::session_rail::ruled_on(Arc::new(FakeJudge::with_no_objection()), workflow());
    crate::timeline::keep_timeline(Arc::clone(&rig.fleet));
    crate::rail_following::keep_the_rail(Arc::clone(&rig.fleet));
    let id = rig.start_held().await;
    Arc::clone(&rig.fleet)
        .set_session_workflow(SetSessionWorkflow { session_id: id.clone(), workflow: Some("rail".into()) })
        .await
        .expect("set");
    rig.send_unsettled(&id, REQUEST).await;
    Arc::clone(&rig.fleet)
        .skip_session_step(ipc::SkipSessionStep { session_id: id.clone(), step: 0, skipped: true })
        .await
        .expect("skipped while on");
    saved(&rig, off(&[], &[(&here(&rig), &["rail"])])).await;

    Arc::clone(&rig.fleet)
        .begin_session(BeginSession { session_id: id.clone(), delivery: Some(DeliveryChoice::OpenPr), message: None })
        .await
        .expect("begun");
    eventually(|| async { rig.stand_in.starts().len() == 1 && told(&rig, REQUEST) == 1 }).await;
    assert_eq!(told(&rig, "follows the Rail workflow"), 0, "not told its steps");
    assert_eq!(told(&rig, "skipped the"), 0, "and not told what it skipped");
}
