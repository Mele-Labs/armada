//! The claim, written before the code: **with two Macs paired and a Job running in a shared project on one, the
//! Operator on the other starts an overlapping Job with that work in what the Job is told first, and a Log line
//! naming the Mac.** It tells and holds nothing. Rows from a Mac asleep more than 2 hours are left out and the Log
//! says so. A plan meeting the other Mac's plan carries it as `other_macs`, which never goes back out.
//! `armada-hu76.2`.

use std::sync::Arc;

use api::{HostedSessions, OtherMacs};
use ipc::{
    DispatchJob, DispatchMessage, DispatchThen, OtherMacSessions, SessionId, SessionPlan, SessionPlanGroup, SessionRecord, StartSession,
};
use store::{MacState, PairedMac};

use super::document_answers::plan_recorded;
use super::operator_sources::{rulings, switched, Answers};
use super::session_host::{eventually, rig_placing, Rig};
use super::share::{job_in, project, shared};
use crate::operator::keep_the_raises;

const WORK: &str = "mac-work";
const THEIR_JOB: &str = "01WORKJOB";
const THEIRS: &str = "Move the store tables";
/// A path the other Mac's plan works, and this Mac's `plan_recorded` names.
const PLACE: &str = "crates/store";
const BRIEF: &str = "Add a column to crates/store/src/macs.rs for the colour.";
const ALSO: &str = "Keep the old rows.";

async fn seated(rig: &Rig) {
    let call = Answers::with(&["item: 1\nway: 1\nbecause: nothing else covers it"]);
    assert!(rig.fleet.operator_seated_with(call));
    switched(rig, "features.operator", true).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
}

/// Mac `WORK`, named Work, paired and asleep since `asleep_since` where given.
async fn paired(rig: &Rig, asleep_since: Option<i64>) {
    let mac = PairedMac {
        id: WORK.into(),
        name: "Work".into(),
        colour: "teal".into(),
        address: "https://work.example".into(),
        public_key: Some(vec![1]),
        build: None,
        paired_at: 1,
        state: MacState::Linked,
        back_code: None,
        last_heard_at: None,
        asleep_since,
        why: asleep_since.map(|_| "Work stopped answering".into()),
        wire: None,
        built_at: None,
        source: None,
    };
    rig.fleet.store().lock().await.keep_mac(&mac).expect("paired");
}

/// Work's live Job in this repository, its plan working `PLACE`, shaped as this Mac's `template`.
fn theirs(template: &SessionRecord) -> SessionRecord {
    let mut record = template.clone();
    record.id = SessionId::carried(THEIR_JOB);
    record.title = Some(THEIRS.into());
    record.plan = Some(SessionPlan {
        version: 1,
        summary: "The store keeps the Mac rows.".into(),
        recorded_at: template.started_at.clone(),
        approved: true,
        approved_at: None,
        set_aside_at: None,
        scope: vec![PLACE.into()],
        groups: vec![SessionPlanGroup {
            id: "g1".into(),
            agent: "rust-engineer".into(),
            title: "Store".into(),
            gives: None,
            paths: vec![PLACE.into()],
            tasks: Vec::new(),
            issue: None,
            worked_at: None,
            lands_in: Default::default(),
            own_job: false,
            gate: Default::default(),
            gate_closed: None,
            gate_checks: None,
        }],
        changes: None,
        sketch: None,
        held_by: Vec::new(),
        held: None,
        other_macs: Vec::new(),
    });
    record
}

/// Work's Job held here, as its Gateway hands it in, and this Mac's own Job in a shared project.
async fn held_from_work(rig: &Rig) -> SessionId {
    let shop = project(rig, "Shop").await;
    shared(rig, &shop, true).await;
    let mine = job_in(rig, Some(&shop)).await;
    let template = Arc::clone(&rig.fleet).get_session(mine.clone()).await.expect("the Job").session;
    rig.fleet.hold_other_mac_sessions(OtherMacSessions { mac: WORK.into(), sessions: vec![theirs(&template)] }).await.expect("held");
    mine
}

fn the_dispatch(rig: &Rig) -> DispatchJob {
    let start = StartSession {
        project: None,
        manifest_id: rig.manifest.clone(),
        title: None,
        model: None,
        effort: None,
        mode: None,
        root: None,
        fork: None,
        request: Some(BRIEF.to_string()),
        after: None,
        held: false,
        project_rows: Vec::new(),
        on_your_words: None,
    };
    let then = vec![DispatchThen::Message(DispatchMessage { text: ALSO.into(), attachments: Vec::new(), mentions: Vec::new(), sketches: Vec::new() })];
    DispatchJob { start, kind: None, then }
}

/// The Job a dispatch started, once the Operator has ruled it a new Job.
async fn started(rig: &Rig, known: &[&SessionId]) -> String {
    Arc::clone(&rig.fleet).dispatch_job(the_dispatch(rig)).await.expect("dispatched");
    for _ in 0..500 {
        let all = {
            let store = rig.fleet.store().lock().await;
            store.find_sessions(&store::SessionSearch { manifest_id: None, state: None, text: None }).expect("read")
        };
        if let Some(id) = all.into_iter().map(|one| one.id).find(|id| !known.iter().any(|known| known.as_str() == id)) {
            return id;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    panic!("no Job started");
}

async fn thread(rig: &Rig, job: &str) -> String {
    rig.fleet.store().lock().await.session_rows(job).expect("rows").join("\n")
}

#[tokio::test]
async fn a_dispatch_near_another_macs_shared_job_is_told_of_it_after_his_words_and_the_log_names_the_mac() {
    let rig = rig_placing(None);
    paired(&rig, None).await;
    let mine = held_from_work(&rig).await;
    seated(&rig).await;

    let job = started(&rig, &[&mine]).await;

    eventually(|| async { thread(&rig, &job).await.contains(&format!("On Work: Job \\\"{THEIRS}\\\" ({THEIR_JOB}), sharing paths")) }).await;
    let said = thread(&rig, &job).await;
    assert!(said.find(ALSO) < said.find("under way on another Mac"), "his words stay first: {said}");
    assert!(said.contains("Nothing here waits on it"), "it tells and holds nothing");
    eventually(|| async { rulings(&rig).await.iter().any(|row| row.session_id == job && row.because.contains(&format!("\"{THEIRS}\" on Work is near"))) }).await;
}

#[tokio::test]
async fn a_mac_asleep_past_two_hours_is_left_out_and_the_log_says_which_and_for_how_long() {
    let rig = rig_placing(None);
    paired(&rig, Some(rig.fleet.unix_seconds() - 3 * 60 * 60 - 60)).await;
    let mine = held_from_work(&rig).await;
    seated(&rig).await;

    let job = started(&rig, &[&mine]).await;

    eventually(|| async { rulings(&rig).await.iter().any(|row| row.session_id == job && row.because.contains("Left out Work's Jobs: asleep for 3 hours.")) }).await;
    eventually(|| async { thread(&rig, &job).await.contains(ALSO) }).await;
    assert!(!thread(&rig, &job).await.contains("another Mac"), "nothing of the asleep Mac is told");
}

#[tokio::test]
async fn a_mac_asleep_under_two_hours_still_counts() {
    let rig = rig_placing(None);
    paired(&rig, Some(rig.fleet.unix_seconds() - 60 * 60)).await;
    let mine = held_from_work(&rig).await;
    seated(&rig).await;

    let job = started(&rig, &[&mine]).await;

    eventually(|| async { thread(&rig, &job).await.contains("On Work: Job") }).await;
}

#[tokio::test]
async fn with_the_operator_off_nothing_is_told_or_logged() {
    let rig = rig_placing(None);
    paired(&rig, None).await;
    let mine = held_from_work(&rig).await;

    let job = started(&rig, &[&mine]).await;

    eventually(|| async { thread(&rig, &job).await.contains(ALSO) }).await;
    assert!(!thread(&rig, &job).await.contains("another Mac"));
    assert!(rulings(&rig).await.iter().all(|row| row.session_id != job));
}

#[tokio::test]
async fn a_plan_meeting_another_macs_plan_carries_it_and_never_sends_it_back_out() {
    let rig = rig_placing(None);
    paired(&rig, None).await;
    let mine = held_from_work(&rig).await;

    plan_recorded(&rig, &mine).await;

    let plan = Arc::clone(&rig.fleet).get_session(mine.clone()).await.expect("the Job").session.plan.expect("a plan");
    let met: Vec<(&str, &str, &str, Vec<&str>)> =
        plan.other_macs.iter().map(|one| (one.mac.as_str(), one.session_id.as_str(), one.title.as_str(), one.paths.iter().map(String::as_str).collect())).collect();
    assert_eq!(met, [(WORK, THEIR_JOB, THEIRS, vec![PLACE])]);
    let sent = rig.fleet.share_sessions().await.expect("shared").sessions;
    assert!(sent.iter().any(|one| one.id == mine && one.plan.is_some()), "this Mac's Job and its plan go out");
    assert!(sent.iter().filter_map(|one| one.plan.as_ref()).all(|plan| plan.other_macs.is_empty()), "it names Jobs as this Mac sees them");
}

#[tokio::test]
async fn a_plan_meets_nothing_from_a_mac_asleep_past_two_hours() {
    let rig = rig_placing(None);
    paired(&rig, Some(rig.fleet.unix_seconds() - 3 * 60 * 60)).await;
    let mine = held_from_work(&rig).await;

    plan_recorded(&rig, &mine).await;

    let plan = Arc::clone(&rig.fleet).get_session(mine).await.expect("the Job").session.plan.expect("a plan");
    assert!(plan.other_macs.is_empty(), "{:?}", plan.other_macs);
}
