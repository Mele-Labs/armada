//! The claim, written before the code: **on the armada repository's Autopilot, the bugs agents
//! reported are offered to the picker beside issues, and a pick of one starts a Job briefed from the
//! report that holds it.** And **Autopilot pauses at a weekly usage reading, never at none.**
//!
//! The call is the fake. The reports box and the backlog box are independent: reports on with the
//! backlog off picks reports and never asks the tracker.

use std::sync::Arc;

use api::{Autopilots, HostedSessions, Sessions as _};
use config::Manifest;
use ipc::{AutopilotCap, AutopilotSettings, AutopilotState, AutopilotWork, CloseSession, PlanWindow, SessionId, SessionTag, TagKind};
use store::{Holder, KeptBugReport};
use testkit::{FakeJudge, FakeTracker, FakeTrackers};

use super::session_host::{eventually, rig_fitted, Rig};

const ARMADA: &str = "version: 1\nid: armada\n";
const NOTHING: &str = "issue: none\nworkflow: none\nsure: no\nbecause: nothing fits";
const SURE: &str = "issue: R-AAAAAA\nworkflow: bug\nsure: yes\nbecause: plainly covered";
const UNSURE: &str = "issue: R-AAAAAA\nworkflow: bug\nsure: no\nbecause: may need a decision";
const FAR: &str = "2099-01-01T00:00:00.000Z";
const LONG_AGO: &str = "2000-01-01T00:00:00.000Z";

/// A rig answering `answer` to the picker, its tracker listing one issue, and workflow `bug`. `armada`
/// is whether it serves the armada repository.
fn rig_picking(answer: &str, armada: bool) -> (Rig, Arc<FakeJudge>, Arc<FakeTracker>) {
    let judge = Arc::new(FakeJudge::saying(answer));
    let tracker = Arc::new(FakeTracker::listing(vec![FakeTracker::issue(5, "Fix the reader")]));
    let (held, listing) = (Arc::clone(&judge), Arc::clone(&tracker));
    let rig = rig_fitted(move |fittings| {
        fittings.judge = held;
        fittings.trackers = Arc::new(FakeTrackers::empty().tracker(listing));
        fittings.starting().workflows = [super::daemon::workflow_named("bug")].into_iter().map(|one| (one.id().clone(), one)).collect();
        if armada {
            fittings.starting().manifest = Manifest::parse(std::path::Path::new("armada.yml"), ARMADA).expect("loads");
        }
    });
    (rig, judge, tracker)
}

fn a_report(id: &str, title: &str) -> KeptBugReport {
    KeptBugReport {
        id: id.to_string(),
        session_id: "reporter".to_string(),
        job_title: String::new(),
        title: title.to_string(),
        what: "It broke.".to_string(),
        steps: vec!["Press go".to_string(), "Wait".to_string()],
        errors: vec!["boom".to_string()],
        logs: Vec::new(),
        at: "2026-11-03T08:00:00.000Z".to_string(),
        kept: None,
        fixed: None,
    }
}

async fn keep(rig: &Rig, report: KeptBugReport) {
    rig.fleet.store().lock().await.keep_bug_report(&report).expect("kept");
}

fn settings(backlog: bool, reports: bool) -> AutopilotSettings {
    AutopilotSettings {
        instruction: "Fix what is open".into(),
        at_once: 3,
        until: None,
        spend_cap_micros: None,
        session_cap: None,
        weekly_cap_percent: None,
        on_autopilot: AutopilotWork { backlog, running: false, reports },
        decisions: Default::default(),
    }
}

async fn start(rig: &Rig, backlog: bool, reports: bool) {
    rig.fleet.start_autopilot(rig.manifest.clone(), settings(backlog, reports)).await.expect("started");
}

/// What the picker was asked, whole, or none where it was not asked.
fn question(judge: &FakeJudge) -> String {
    judge.asked().into_iter().find(|one| one.contains("You choose the next issue")).unwrap_or_default()
}

fn tagging(report: &KeptBugReport) -> SessionTag {
    SessionTag { kind: TagKind::Report, id: report.id.clone(), title: report.title.clone(), url: None }
}

// ---- What is offered ----

#[tokio::test]
async fn an_open_report_is_offered_kept_or_not_and_a_fixed_or_held_one_is_not() {
    let (rig, judge, _) = rig_picking(NOTHING, true);
    let (open, kept, fixed) = (a_report("01R00000000000000000AAAAAA", "Open one"), a_report("01R00000000000000000BBBBBB", "Kept one"), a_report("01R00000000000000000CCCCCC", "Fixed one"));
    let (held, ended, other) = (a_report("01R00000000000000000DDDDDD", "Held one"), a_report("01R00000000000000000EEEEEE", "Ended one"), a_report("01R00000000000000000FFFFFF", "Other one"));
    let fixed = KeptBugReport { fixed: Some(store::KeptBugFix { at: "2026-11-02T00:00:00Z".into(), pull_request: 3, title: "Fix".into(), url: "u".into() }), ..fixed };
    let kept = KeptBugReport { kept: Some("2026-11-03T08:30:00.000Z".into()), ..kept };
    for one in [&open, &kept, &fixed, &held, &ended, &other] {
        keep(&rig, one.clone()).await;
    }
    let manifest = rig.manifest.as_str().to_string();
    let live = rig.start().await;
    rig.fleet.reports_named(live.as_str(), &manifest, &[tagging(&held)]).await;
    let gone = rig.start().await;
    rig.fleet.reports_named(gone.as_str(), &manifest, &[tagging(&ended)]).await;
    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: gone.clone() }).await.expect("ended");
    start(&rig, false, true).await;

    rig.fleet.autopilot_pass().await.expect("passed");

    let asked = question(&judge);
    for offered in ["R-AAAAAA Open one", "R-BBBBBB Kept one", "R-EEEEEE Ended one", "R-FFFFFF Other one"] {
        assert!(asked.contains(offered), "{offered} is offered: {asked}");
    }
    for left in ["R-CCCCCC", "R-DDDDDD"] {
        assert!(!asked.contains(left), "{left} is not offered: {asked}");
    }
    assert!(asked.contains("labels: reported bug") && asked.contains("It broke."), "a report is told from an issue, and read: {asked}");
}

#[tokio::test]
async fn with_reports_off_or_on_another_repository_none_is_offered() {
    for (armada, reports) in [(true, false), (false, true)] {
        let (rig, judge, _) = rig_picking(NOTHING, armada);
        keep(&rig, a_report("01R00000000000000000AAAAAA", "Open one")).await;
        start(&rig, true, reports).await;

        rig.fleet.autopilot_pass().await.expect("passed");

        let asked = question(&judge);
        assert!(asked.contains("Fix the reader"), "the backlog is still offered: {asked}");
        assert!(!asked.contains("R-AAAAAA") && !asked.contains("reported bug"), "armada {armada} reports {reports}: {asked}");
    }
}

#[tokio::test]
async fn with_the_backlog_off_and_reports_on_reports_are_picked_and_the_tracker_is_not_asked() {
    let (rig, judge, tracker) = rig_picking(NOTHING, true);
    keep(&rig, a_report("01R00000000000000000AAAAAA", "Open one")).await;
    start(&rig, false, true).await;

    rig.fleet.autopilot_pass().await.expect("passed");

    let asked = question(&judge);
    assert!(asked.contains("R-AAAAAA Open one") && !asked.contains("Fix the reader"), "{asked}");
    assert_eq!(tracker.listings(), 0, "the backlog box is off");
}

#[tokio::test]
async fn with_both_boxes_on_issues_and_reports_are_offered_together() {
    let (rig, judge, tracker) = rig_picking(NOTHING, true);
    keep(&rig, a_report("01R00000000000000000AAAAAA", "Open one")).await;
    start(&rig, true, true).await;

    rig.fleet.autopilot_pass().await.expect("passed");

    let asked = question(&judge);
    assert!(asked.contains("R-AAAAAA Open one") && asked.contains("Fix the reader"), "{asked}");
    assert_eq!(tracker.listings(), 1);
}

// ---- What a pick starts ----

/// The turn sent to the Job's process holds `text`. The lines are the process's own, so a newline is
/// the two characters `\n`.
async fn brief_holds(rig: &Rig, text: &str) {
    eventually(|| async { rig.stand_in.process(0).sent.lock().unwrap().iter().any(|line| line.contains(text)) }).await;
}

const BRIEF: &str = "Open one\\n\\nIt broke.\\n\\nSteps\\n1. Press go\\n2. Wait\\n\\nErrors\\nboom";

/// The Job holds the report, and a pull request it holds merging marks it fixed.
async fn holds_and_is_fixed_by_its_merge(rig: &Rig, job: &str) {
    let manifest = rig.manifest.as_str().to_string();
    let rows = rig.fleet.store().lock().await.attachments_at("report", "01R00000000000000000AAAAAA", Some(&manifest)).unwrap();
    assert_eq!(rows.iter().map(|one| one.holder.clone()).collect::<Vec<_>>(), [Holder::session(job)], "the Job holds the report");

    let at = rig.fleet.now().as_str().to_string();
    let pr = store::KeptAttachment {
        holder: Holder::session(job),
        kind: "pr".into(),
        manifest_id: manifest.clone(),
        target: "9".into(),
        state: store::AttachmentState::Standing,
        detail: Default::default(),
        since: at.clone(),
        changed_at: at,
    };
    rig.fleet.store().lock().await.attach(&pr, false).expect("attached");
    let served = rig.fleet.served_named(Some(&rig.manifest)).expect("served");
    rig.fleet.reports_fixed_by_merge(&served, 9, "Fix the thing", "https://forge.invalid/pull/9", "2026-11-03T10:00:00.000Z").await;

    let fixed = rig.fleet.list_bug_reports().await.expect("listed").reports.remove(0).fixed.expect("marked fixed");
    assert_eq!(fixed.pull_request, 9);
}

#[tokio::test]
async fn a_sure_report_starts_a_job_briefed_from_it_that_holds_it() {
    let (rig, _, _) = rig_picking(SURE, true);
    keep(&rig, a_report("01R00000000000000000AAAAAA", "Open one")).await;
    start(&rig, false, true).await;

    assert_eq!(rig.fleet.autopilot_pass().await.expect("passed"), 1);

    let pick = rig.fleet.get_autopilot(rig.manifest.clone()).await.expect("read").picks.remove(0);
    assert_eq!((pick.issue_url.as_str(), pick.issue_reference.as_str(), pick.issue_title.as_str()), ("report:01R00000000000000000AAAAAA", "R-AAAAAA", "Open one"));
    let job = pick.session_id.expect("a Job started").as_str().to_string();
    brief_holds(&rig, &format!("{BRIEF}\\n\\nAutopilot picked this issue: plainly covered")).await;
    assert!(!rig.stand_in.process(0).sent.lock().unwrap().iter().any(|line| line.contains("Logs")), "a part with nothing in it is left out");
    holds_and_is_fixed_by_its_merge(&rig, &job).await;
}

#[tokio::test]
async fn a_set_aside_report_a_person_dispatches_is_briefed_and_held_the_same_way() {
    let (rig, _, _) = rig_picking(UNSURE, true);
    keep(&rig, a_report("01R00000000000000000AAAAAA", "Open one")).await;
    start(&rig, false, true).await;
    assert_eq!(rig.fleet.autopilot_pass().await.expect("passed"), 0);
    let aside = rig.fleet.get_autopilot(rig.manifest.clone()).await.expect("read").picks.remove(0);
    assert_eq!(aside.issue_url, "report:01R00000000000000000AAAAAA");

    let dispatched = Arc::clone(&rig.fleet).dispatch_set_aside(rig.manifest.clone(), aside.pick_id).await.expect("dispatched");

    let job = dispatched.picks[0].session_id.clone().expect("a Job started").as_str().to_string();
    brief_holds(&rig, BRIEF).await;
    holds_and_is_fixed_by_its_merge(&rig, &job).await;
}

#[tokio::test]
async fn a_report_dropped_since_it_was_set_aside_starts_a_job_from_the_title_and_holds_nothing() {
    let (rig, _, _) = rig_picking(UNSURE, true);
    keep(&rig, a_report("01R00000000000000000AAAAAA", "Open one")).await;
    start(&rig, false, true).await;
    rig.fleet.autopilot_pass().await.expect("passed");
    let aside = rig.fleet.get_autopilot(rig.manifest.clone()).await.expect("read").picks.remove(0);
    rig.fleet.store().lock().await.drop_bug_report("01R00000000000000000AAAAAA").expect("dropped");

    let dispatched = Arc::clone(&rig.fleet).dispatch_set_aside(rig.manifest.clone(), aside.pick_id).await.expect("dispatched");

    assert!(dispatched.picks[0].session_id.is_some(), "a Job started");
    brief_holds(&rig, "Open one").await;
    let manifest = rig.manifest.as_str().to_string();
    assert!(rig.fleet.store().lock().await.attachments_at("report", "01R00000000000000000AAAAAA", Some(&manifest)).unwrap().is_empty());
}

// ---- The weekly cap ----

fn week(percent: u32, resets_at: &str) -> Vec<PlanWindow> {
    vec![PlanWindow { kind: "seven_day".into(), percent_used: percent, resets_at: Some(ipc::Instant::carried(resets_at)) }]
}

fn read_week(rig: &Rig, windows: Vec<PlanWindow>) {
    rig.fleet.hosts().plan_read(windows, "2026-11-03T08:00:00.000Z");
}

fn capped_at(percent: u32) -> AutopilotSettings {
    AutopilotSettings { weekly_cap_percent: Some(percent), ..settings(false, false) }
}

#[tokio::test]
async fn a_reading_at_the_weekly_cap_pauses_naming_it_and_one_below_does_not() {
    let (rig, _, _) = rig_picking(NOTHING, false);
    read_week(&rig, week(79, FAR));
    rig.fleet.start_autopilot(rig.manifest.clone(), capped_at(80)).await.expect("started");

    rig.fleet.autopilot_pass().await.expect("passed");
    let now = rig.fleet.get_autopilot(rig.manifest.clone()).await.expect("read");
    assert_eq!((now.state, now.capped_by, now.weekly_cap_percent), (AutopilotState::Running, None, Some(80)), "below it");

    read_week(&rig, week(80, FAR));
    rig.fleet.autopilot_pass().await.expect("passed");
    let now = rig.fleet.get_autopilot(rig.manifest.clone()).await.expect("read");
    assert_eq!((now.state, now.capped_by), (AutopilotState::Paused, Some(AutopilotCap::Weekly)), "at it");
}

#[tokio::test]
async fn no_reading_and_a_window_that_has_reset_never_count_as_the_cap_reached() {
    for windows in [Vec::new(), week(100, LONG_AGO), vec![PlanWindow { kind: "five_hour".into(), percent_used: 100, resets_at: None }]] {
        let (rig, _, _) = rig_picking(NOTHING, false);
        read_week(&rig, windows.clone());
        rig.fleet.start_autopilot(rig.manifest.clone(), capped_at(10)).await.expect("started past a cap it has no reading of");

        rig.fleet.autopilot_pass().await.expect("passed");

        let now = rig.fleet.get_autopilot(rig.manifest.clone()).await.expect("read");
        assert_eq!((now.state, now.capped_by), (AutopilotState::Running, None), "{windows:?}");
    }
}

#[tokio::test]
async fn a_start_past_the_weekly_cap_is_refused_and_so_is_an_update_to_one() {
    let (rig, _, _) = rig_picking(NOTHING, false);
    read_week(&rig, week(85, FAR));

    let refused = rig.fleet.start_autopilot(rig.manifest.clone(), capped_at(80)).await.unwrap_err();
    assert_eq!(refused.status(), 422, "{refused:?}");
    assert!(refused.error().message.contains("weekly usage is already past that"), "{refused:?}");

    rig.fleet.start_autopilot(rig.manifest.clone(), capped_at(90)).await.expect("under it");
    let refused = rig.fleet.update_autopilot(rig.manifest.clone(), capped_at(80)).await.unwrap_err();
    assert_eq!(refused.status(), 422, "{refused:?}");
}

#[tokio::test]
async fn resume_is_refused_while_the_weekly_cap_is_reached_and_raising_it_lets_it_through() {
    let (rig, _, _) = rig_picking(NOTHING, false);
    let job: SessionId = rig.start().await;
    read_week(&rig, week(70, FAR));
    rig.fleet.start_autopilot(rig.manifest.clone(), capped_at(80)).await.expect("started");
    read_week(&rig, week(82, FAR));
    rig.fleet.autopilot_pass().await.expect("passed");
    assert_eq!(rig.fleet.get_autopilot(rig.manifest.clone()).await.expect("read").capped_by, Some(AutopilotCap::Weekly));
    let still = rig.fleet.store().lock().await.session(job.as_str()).unwrap().expect("kept").state;
    assert_eq!(still, store::SessionState::Live, "a running Job is never stopped");

    let refused = rig.fleet.resume_autopilot(rig.manifest.clone()).await.unwrap_err();
    assert_eq!(refused.error().code, "fleet.autopilot_capped", "{refused:?}");
    assert!(refused.error().message.contains("the weekly usage cap is reached. Raise it or wait for the week to reset, then resume"), "{refused:?}");

    rig.fleet.update_autopilot(rig.manifest.clone(), capped_at(90)).await.expect("raised");
    let resumed = rig.fleet.resume_autopilot(rig.manifest.clone()).await.expect("resumed");
    assert_eq!((resumed.state, resumed.capped_by), (AutopilotState::Running, None));
}

#[tokio::test]
async fn a_proposal_names_no_weekly_cap() {
    let (rig, _, _) = rig_picking("instruction: fix what is open\nat_once: 1\nuntil: none\nspend: none\nsessions: none", false);
    let proposed = rig
        .fleet
        .propose_autopilot(rig.manifest.clone(), ipc::ProposeAutopilot { sentence: "fix what is open".into(), local_now: None })
        .await
        .expect("proposed");
    assert_eq!(proposed.settings.weekly_cap_percent, None);
    assert!(!proposed.settings.on_autopilot.reports);
}
