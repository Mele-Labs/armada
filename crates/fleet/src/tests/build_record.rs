//! A Session's build record: the rules between what Fleet observed and what its row shows, then the
//! same through a Fleet. `docs/concepts/session.md`, *The build record*.

use std::sync::Arc;

use api::{HostedSessions, Sessions};
use ipc::{
    AttachmentNamed, AttachmentReport, AttachmentState as Wire, BuildPhase, BuildPieceKind, BuildStopId, ReportedTask, ReportedTaskStatus, SessionBuild, SessionFact, SessionId,
    SessionOrigin, SessionReport, ShowWindow,
};
use store::{AttachmentState, Holder, KeptAttachment, KeptBuild, KeptTask, RoadTimes};

use super::timeline::{kept, kinds, read};
use crate::build_record::{current_pull, derived, landed_since, reached, taken, usual_road, Observed, Read};
use crate::tests::tmp::TempDir;

const T: &str = "2026-10-14T09:";

fn at(minute: u32) -> String {
    format!("{T}{minute:02}:00.000Z")
}

fn task(id: &str, status: ReportedTaskStatus) -> ReportedTask {
    ReportedTask { id: id.to_string(), text: format!("Task {id}"), status, by: None }
}

fn list(statuses: &[ReportedTaskStatus]) -> Observed {
    Observed::Tasks(statuses.iter().enumerate().map(|(place, status)| task(&(place + 1).to_string(), *status)).collect())
}

/// What the rows hold after each of these happened, in order.
fn after(steps: &[(u32, Observed)]) -> (KeptBuild, Vec<KeptTask>) {
    let (mut build, mut tasks) = (None, Vec::new());
    for (minute, heard) in steps {
        let took = taken(build, tasks, heard, &at(*minute));
        (build, tasks) = (Some(took.build), took.tasks);
    }
    (build.expect("something happened"), tasks)
}

fn record(build: &KeptBuild, tasks: &[KeptTask], road: &RoadTimes, forecast: &[BuildStopId]) -> SessionBuild {
    derived(&Read { build, tasks, road, pull_since: None, merged_fallback: None, forecast, workers: Vec::new(), stuck: false })
}

fn pieces(build: &SessionBuild) -> Vec<BuildPieceKind> {
    build.pieces.iter().map(|one| one.piece).collect()
}

use ReportedTaskStatus::{Completed as Done, InProgress as Doing, Pending as Todo};
use BuildPieceKind::{Ghost, Hollow, Scaffold, Solid};

/// **Solid is a completion with an edit seen while it was worked; hollow is one with none; scaffold is
/// the first task not done, once edits have landed since the last completion; the rest are ghosts.**
#[test]
fn a_task_ticked_with_an_edit_seen_is_solid_and_one_ticked_with_none_is_hollow() {
    // The list at 00, an edit at 05, the first ticked at 10: solid. An edit at 12 raises scaffold round the second.
    let (build, tasks) = after(&[
        (0, list(&[Todo, Todo, Todo])),
        (5, Observed::Edited),
        (10, list(&[Done, Doing, Todo])),
    ]);
    assert_eq!(pieces(&record(&build, &tasks, &RoadTimes::default(), &[])), [Solid, Ghost, Ghost], "no edit since the last tick");
    let (build, tasks) = after(&[
        (0, list(&[Todo, Todo, Todo])),
        (5, Observed::Edited),
        (10, list(&[Done, Doing, Todo])),
        (12, Observed::Edited),
    ]);
    assert_eq!(pieces(&record(&build, &tasks, &RoadTimes::default(), &[])), [Solid, Scaffold, Ghost]);

    // The second ticked after the edit at 12 is solid, the third ticked with none since is hollow.
    let (build, tasks) = after(&[
        (0, list(&[Todo, Todo, Todo])),
        (5, Observed::Edited),
        (10, list(&[Done, Doing, Todo])),
        (12, Observed::Edited),
        (20, list(&[Done, Done, Doing])),
        (30, list(&[Done, Done, Done])),
    ]);
    assert_eq!(pieces(&record(&build, &tasks, &RoadTimes::default(), &[])), [Solid, Solid, Hollow]);
}

#[test]
fn tasks_ticked_together_share_the_window_and_a_list_first_written_ticked_is_hollow() {
    let (build, tasks) = after(&[
        (0, list(&[Todo, Todo])),
        (5, Observed::Edited),
        (10, list(&[Done, Done])),
    ]);
    assert_eq!(pieces(&record(&build, &tasks, &RoadTimes::default(), &[])), [Solid, Solid]);

    // Edits before the list was written are not in a window that opens at the list's first write.
    let (build, tasks) = after(&[(0, Observed::Edited), (5, list(&[Done, Todo]))]);
    assert_eq!(pieces(&record(&build, &tasks, &RoadTimes::default(), &[])), [Hollow, Ghost]);
}

#[test]
fn a_reordered_list_keeps_what_was_seen_of_each_and_an_untick_takes_the_tick_back() {
    let reorder = Observed::Tasks(vec![task("2", Todo), task("1", Done)]);
    let (build, tasks) = after(&[
        (0, list(&[Todo, Todo])),
        (5, Observed::Edited),
        (10, list(&[Done, Todo])),
        (15, reorder),
    ]);
    assert_eq!(tasks[1].completed_at, Some(at(10)), "the first task ticked at 10 still was");
    assert_eq!(pieces(&record(&build, &tasks, &RoadTimes::default(), &[])), [Ghost, Solid]);

    let (_, tasks) = after(&[(0, list(&[Done])), (5, list(&[Doing]))]);
    assert_eq!((tasks[0].completed_at.clone(), tasks[0].edited), (None, false));
}

#[test]
fn with_no_list_there_are_no_pieces_and_a_cleared_list_clears_them() {
    let (build, tasks) = after(&[(0, Observed::Edited)]);
    assert!(record(&build, &tasks, &RoadTimes::default(), &[]).pieces.is_empty(), "progress is never invented");
    let (build, tasks) = after(&[(0, list(&[Todo])), (5, Observed::Tasks(Vec::new()))]);
    assert!(record(&build, &tasks, &RoadTimes::default(), &[]).pieces.is_empty());
    assert_eq!(build.tasks_at, None);
}

fn road(walked: Option<u32>, pushed: Option<u32>, opened: Option<u32>, merged: Option<u32>) -> RoadTimes {
    let text = |minute: Option<u32>| minute.map(at);
    RoadTimes { walked: text(walked), pushed: text(pushed), opened: text(opened), checked: None, queued: None, merged: text(merged) }
}

/// **Scope has no edit; plan has a list or a left plan mode and no edit since; implement has edits; road has a stop after the
/// first edit; landed has the pull request merged.**
#[test]
fn the_phase_follows_the_edits_the_plan_and_the_road() {
    let phase = |steps: &[(u32, Observed)], road: &RoadTimes| {
        let (build, tasks) = after(steps);
        record(&build, &tasks, road, &[]).phase
    };
    let none = RoadTimes::default();
    assert_eq!(phase(&[(0, Observed::Worked)], &none), BuildPhase::Scope);
    assert_eq!(phase(&[(0, Observed::Worked), (1, list(&[Todo]))], &none), BuildPhase::Plan);
    assert_eq!(phase(&[(0, Observed::Worked), (1, Observed::PlanLeft)], &none), BuildPhase::Plan);
    assert_eq!(phase(&[(0, Observed::Worked), (1, Observed::PlanLeft), (2, Observed::Edited)], &none), BuildPhase::Implement);
    assert_eq!(phase(&[(0, Observed::Edited)], &none), BuildPhase::Implement, "edits with no plan");
    let edited = [(0, Observed::PlanLeft), (2, Observed::Edited)];
    assert_eq!(phase(&edited, &road(None, Some(3), None, None)), BuildPhase::Road);
    assert_eq!(phase(&edited, &road(None, Some(1), None, None)), BuildPhase::Implement, "a push before the first edit is not on this road");
    assert_eq!(phase(&edited, &road(Some(3), None, Some(4), Some(9))), BuildPhase::Landed);
    assert_eq!(phase(&[(0, Observed::Worked)], &none), BuildPhase::Scope, "no edit and no stop is still scope");
}

/// **With no edit ever observed, a stop is the road, since a pull request, a push or a walk is evidence of work; a stop before a
/// first edit that is known still belongs to other work.**
#[test]
fn a_stop_is_the_road_when_no_edit_was_ever_observed() {
    let phase = |steps: &[(u32, Observed)], road: &RoadTimes| {
        let (build, tasks) = after(steps);
        record(&build, &tasks, road, &[]).phase
    };
    assert_eq!(phase(&[(0, Observed::Worked)], &road(None, Some(3), None, None)), BuildPhase::Road, "a push, no edit heard");
    assert_eq!(phase(&[(0, Observed::Worked)], &road(None, None, Some(3), None)), BuildPhase::Road, "a pull request, no edit heard");
    assert_eq!(phase(&[(5, Observed::Edited)], &road(None, Some(3), None, None)), BuildPhase::Implement, "a stop before a known first edit");
}

/// **A commit or a push proves code changed, so it sets the first edit where none was heard; a read or a command does not, and an edit
/// already heard is not moved.**
#[test]
fn a_commit_counts_as_the_first_edit_and_a_read_does_not() {
    let (build, _) = after(&[(0, Observed::Worked), (1, Observed::Worked)]);
    assert_eq!(build.first_edit_at, None, "reads and commands are not edits");
    let (build, _) = after(&[(0, Observed::Worked), (4, Observed::Committed)]);
    assert_eq!((build.first_edit_at, build.last_edit_at), (Some(at(4)), None), "a commit is the first edit and no later edit");
    let (build, _) = after(&[(0, Observed::Edited), (4, Observed::Committed)]);
    assert_eq!((build.first_edit_at, build.last_edit_at), (Some(at(0)), Some(at(0))), "an edit heard stands");
    let (build, tasks) = after(&[(0, list(&[Todo])), (4, Observed::Committed)]);
    assert_eq!(record(&build, &tasks, &RoadTimes::default(), &[]).phase, BuildPhase::Implement, "work after the plan is implementing");
    let (build, tasks) = after(&[(0, Observed::Committed), (4, list(&[Todo]))]);
    assert_eq!(record(&build, &tasks, &RoadTimes::default(), &[]).phase, BuildPhase::Plan, "a plan written after the commit");
}

/// **A commit does not start the build that follows a landing, as a read does not.**
#[test]
fn a_commit_after_a_landing_does_not_begin_work() {
    assert!(!Observed::Committed.begins_work());
    assert!(!Observed::Worked.begins_work());
    assert!(Observed::Edited.begins_work());
}

fn work(edited: Option<(u32, u32)>, commit: Option<u32>, push: Option<u32>) -> store::WorkTimes {
    store::WorkTimes {
        edited: edited.map(|(first, last)| (at(first), at(last))),
        first_commit: commit.map(at),
        first_push: push.map(at),
    }
}

/// **A row with no first edit is filled from what the timeline kept: the earliest edit, commit or push is the first edit, and the
/// last edit entry the last; a row with a first edit, or nothing kept, is not touched.**
#[test]
fn the_timeline_fills_a_row_that_has_no_first_edit() {
    let (made, _) = after(&[(30, Observed::Worked)]);
    let seeded = crate::build_record::seeded(Some(made.clone()), &work(Some((5, 20)), Some(25), Some(26))).expect("a row");
    assert_eq!((seeded.first_edit_at, seeded.last_edit_at), (Some(at(5)), Some(at(20))));
    assert_eq!(seeded.begun_at, at(5), "the build began with the work, not when Fleet first heard it");

    let by_commit = crate::build_record::seeded(Some(made.clone()), &work(None, Some(25), Some(26))).expect("a row");
    assert_eq!((by_commit.first_edit_at, by_commit.last_edit_at), (Some(at(25)), None), "a commit stands for the first edit only");

    assert_eq!(crate::build_record::seeded(Some(made.clone()), &work(None, None, None)), Some(made.clone()), "nothing kept");
    assert_eq!(crate::build_record::seeded(None, &work(None, None, None)), None, "no row is made from nothing");
    let new = crate::build_record::seeded(None, &work(Some((5, 20)), None, None)).expect("a row from the kept work");
    assert_eq!((new.begun_at.clone(), new.first_edit_at.clone()), (at(5), Some(at(5))));

    let (heard, _) = after(&[(30, Observed::Edited)]);
    assert_eq!(crate::build_record::seeded(Some(heard.clone()), &work(Some((5, 20)), None, None)), Some(heard), "an edit heard stands");
}

#[test]
fn the_road_is_the_stops_reached_in_order_then_the_ones_usual_for_the_repository() {
    let (build, tasks) = after(&[(0, Observed::Edited)]);
    let usual = [BuildStopId::Walk, BuildStopId::Pr, BuildStopId::Checks, BuildStopId::AutoMerge];
    let read = record(&build, &tasks, &road(Some(5), None, Some(3), None), &usual);
    let ids: Vec<_> = read.stops.iter().map(|one| (one.id, one.at.is_some())).collect();
    assert_eq!(
        ids,
        [(BuildStopId::Pr, true), (BuildStopId::Walk, true), (BuildStopId::Checks, false), (BuildStopId::AutoMerge, false)],
        "reached by time, then the unreached usual ones with no time"
    );
    assert_eq!(read.forecast, usual);
    assert_eq!(read.landed_at, None);

    // A pull request exists before anything Fleet read of it.
    let seen = RoadTimes { checked: Some(at(7)), ..RoadTimes::default() };
    assert_eq!(reached(&seen, None)[0], (BuildStopId::Pr, at(7)));
    assert_eq!(reached(&seen, Some(&at(6)))[0], (BuildStopId::Pr, at(6)));
}

/// **The forecast is the most common whole sequence of stops among a repository's recent landed Sessions; a tie goes to the
/// newest; a Session that left no road takes no part.**
#[test]
fn the_forecast_is_the_most_common_sequence_and_a_tie_goes_to_the_newest() {
    use BuildStopId::{AutoMerge, Checks, Pr, Pushed, Walk};
    assert_eq!(usual_road(&[]), Vec::new(), "no history, no forecast");
    assert_eq!(usual_road(&[Vec::new(), Vec::new()]), Vec::new());
    let newest_first = [vec![Pr, Checks], vec![Walk, Pr, Checks, AutoMerge], vec![Pr, Checks], vec![Walk, Pushed, Pr]];
    assert_eq!(usual_road(&newest_first), [Pr, Checks], "two of four");
    let tied = [vec![Walk, Pr], Vec::new(), vec![Pr, Checks], vec![Pr, Checks], vec![Walk, Pr]];
    assert_eq!(usual_road(&tied), [Walk, Pr], "two and two: the newest wins");
}

fn pull(state: AttachmentState, merged: Option<&str>, since: &str, changed: &str) -> KeptAttachment {
    KeptAttachment {
        holder: Holder::session("s1"),
        kind: "pr".to_string(),
        manifest_id: "armada".to_string(),
        target: "12".to_string(),
        state,
        detail: merged.map(|state| ("state".to_string(), state.to_string())).into_iter().collect(),
        since: since.to_string(),
        changed_at: changed.to_string(),
    }
}

/// A Session that goes on to a second piece of work is not shown as the first one landed.
#[test]
fn a_build_that_landed_ends_with_the_work_after_it_and_its_pull_request_is_not_the_next_builds() {
    let merged = pull(AttachmentState::Spent, Some("merged"), &at(2), &at(30));
    assert!(landed_since(std::slice::from_ref(&merged), &at(1)));
    assert!(!landed_since(std::slice::from_ref(&merged), &at(31)), "merged before this build began");
    assert!(current_pull(std::slice::from_ref(&merged), &at(31)).is_none());
    assert!(current_pull(std::slice::from_ref(&merged), &at(10)).is_some());
    let open = pull(AttachmentState::Standing, Some("open"), &at(40), &at(40));
    assert_eq!(current_pull(&[merged, open], &at(31)).map(|one| one.since.as_str()), Some(at(40).as_str()));
    let closed = pull(AttachmentState::GivenBack, Some("closed"), &at(5), &at(6));
    assert!(current_pull(&[closed], &at(0)).is_none());
    // Spent by a `gh pr merge` the forge has not confirmed reads open, and is not a landing.
    let unconfirmed = pull(AttachmentState::Spent, Some("open"), &at(2), &at(30));
    assert!(!landed_since(&[unconfirmed], &at(1)));
}

async fn report(fleet: &super::timeline::Fixture, fact: SessionFact) -> ipc::SessionRecord {
    fleet
        .report_session(SessionReport { harness: "a-harness".to_string(), session_id: SessionId::carried("t1"), fact })
        .await
        .expect("a report is kept")
}

async fn apart() {
    tokio::time::sleep(std::time::Duration::from_millis(4)).await;
}

/// Through a Fleet: the list, the edits, a commit, a push and a walk, read back off the row, and the
/// commit, the push and the walk are on the timeline under their own kinds.
#[tokio::test]
async fn a_terminal_sessions_build_is_kept_and_read_back_off_its_row() {
    let home = TempDir::new();
    let (fleet, _) = kept(&home);
    let started = SessionFact::Started {
        cwd: "/work".to_string(),
        title: Some("Build record".to_string()),
        origin: SessionOrigin::Terminal,
        mod_version: None,
    };
    assert!(report(&fleet, started).await.build.is_none(), "a Session that has done nothing has no build");

    let tasks = |statuses: &[ReportedTaskStatus]| SessionFact::Tasks {
        tasks: statuses.iter().enumerate().map(|(place, status)| task(&(place + 1).to_string(), *status)).collect(),
    };
    report(&fleet, tasks(&[Todo, Todo])).await;
    apart().await;
    report(&fleet, SessionFact::Touched { paths: vec!["src/a.rs".to_string()], by: None, act: None }).await;
    apart().await;
    let row = report(&fleet, tasks(&[Done, Doing])).await;
    let build = row.build.expect("a build");
    assert_eq!(build.phase, BuildPhase::Implement);
    assert_eq!(pieces(&build), [Solid, Ghost]);
    assert!(build.quiet_since.is_some());

    apart().await;
    report(&fleet, SessionFact::Touched { paths: vec!["src/b.rs".to_string()], by: None, act: None }).await;
    apart().await;
    report(&fleet, SessionFact::Committed { branch: Some("armada/topic".to_string()) }).await;
    apart().await;
    report(&fleet, SessionFact::Pushed { branch: Some("armada/topic".to_string()) }).await;
    apart().await;
    fleet
        .show_window(
            None,
            ShowWindow {
                url: "http://localhost:5173/one".to_string(),
                title: Some("The ring".to_string()),
                session_id: Some(SessionId::carried("t1")),
                variations: Vec::new(),
                ..Default::default()
            },
        )
        .await
        .expect("a walk is shown");
    report(&fleet, SessionFact::Read { path: "src/a.rs".to_string() }).await;

    // The stops are entered on the timeline by a writer of their own, which publishes the row again.
    let events = read(&fleet).await;
    let row = Arc::clone(&fleet).get_session(SessionId::carried("t1")).await.expect("the row").session;
    let build = row.build.expect("a build");
    assert_eq!(pieces(&build), [Solid, Scaffold], "edits since the first tick raise scaffold round the next task");
    assert_eq!(build.phase, BuildPhase::Road);
    assert_eq!(build.stops.iter().map(|one| one.id).collect::<Vec<_>>(), [BuildStopId::Pushed, BuildStopId::Walk]);
    let wire = kinds(&events);
    for expected in ["session.committed", "session.pushed", "session.walked"] {
        assert!(wire.contains(&expected.to_string()), "{expected} missing from {wire:?}");
    }
    let pushed = events.iter().find(|one| one.kind.as_wire() == "session.pushed").expect("a push");
    assert_eq!(pushed.detail.as_deref(), Some("armada/topic"));
    let walked = events.iter().find(|one| one.kind.as_wire() == "session.walked").expect("a walk");
    assert_eq!(walked.detail.as_deref(), Some("The ring"));
}

/// A subagent writes into the Session's list, and its pieces carry its worker label from its
/// attachment, which they keep after it ends.
#[tokio::test]
async fn a_subagents_tasks_are_pieces_with_its_worker_label_kept_after_it_ends() {
    let home = TempDir::new();
    let (fleet, _) = kept(&home);
    let started = SessionFact::Started {
        cwd: "/work".to_string(),
        title: Some("Build record".to_string()),
        origin: SessionOrigin::Terminal,
        mod_version: None,
    };
    report(&fleet, started).await;
    let detail = [("type", "Explore"), ("description", "Map the claims")].map(|(k, v)| (k.to_string(), v.to_string()));
    let attachment = AttachmentReport { kind: "subagent".to_string(), target: "a1".to_string(), detail: detail.into() };
    report(&fleet, SessionFact::Attached { attachment }).await;

    let by = |id: &str, by: Option<&str>| ReportedTask { by: by.map(str::to_string), ..task(id, Todo) };
    let row = report(&fleet, SessionFact::Tasks { tasks: vec![by("1", None), by("2", Some("a1")), by("3", Some("gone"))] }).await;
    let build = row.build.expect("a build");
    assert_eq!(build.workers, ["Explore: Map the claims"]);
    let made: Vec<_> = build.pieces.iter().map(|one| one.by.as_deref()).collect();
    assert_eq!(made, [None, Some("Explore: Map the claims"), Some("gone")], "an id with no attachment stays the id");

    let over = SessionFact::Settled { attachment: AttachmentNamed { kind: "subagent".to_string(), target: "a1".to_string() }, state: Wire::Spent };
    report(&fleet, over).await;
    let done = ReportedTask { status: Done, ..by("2", Some("a1")) };
    let row = report(&fleet, SessionFact::Tasks { tasks: vec![by("1", None), done, by("3", Some("gone"))] }).await;
    let build = row.build.expect("a build");
    assert!(build.workers.is_empty());
    assert_eq!(build.pieces[1].by.as_deref(), Some("Explore: Map the claims"), "a finished subagent's piece stays its own");
}

fn working(build: &SessionBuild) -> Vec<bool> {
    build.pieces.iter().map(|one| one.working).collect()
}

/// The newest build a published row carried, drained from what is waiting.
async fn build_published(live: &mut api::Subscription) -> Option<SessionBuild> {
    let mut latest = None;
    while let Ok(Some(api::Next::Send(delivered))) = tokio::time::timeout(std::time::Duration::from_millis(100), live.next()).await {
        if let ipc::Event::SessionChanged(row) = delivered.event {
            latest = row.build.or(latest);
        }
    }
    latest
}

/// A task is working where the agent's tool has it in progress, whatever piece it is drawn as.
#[test]
fn a_task_the_tool_has_in_progress_is_working_and_a_completed_one_is_not() {
    let (build, tasks) = after(&[(0, list(&[Done, Doing, Todo])), (5, Observed::Edited)]);
    let record = record(&build, &tasks, &RoadTimes::default(), &[]);
    assert_eq!(working(&record), [false, true, false]);
    assert_eq!(pieces(&record), [Hollow, Scaffold, Ghost], "working is the tool's word, apart from the piece");
}

/// **With no plan the agent's task list is the Session's own progress**: it is kept as the agent wrote it, in
/// its order, and the row is published when it changes, so Progress follows live. A list heard again
/// unchanged is not a change.
#[tokio::test]
async fn a_session_with_no_plan_keeps_the_agents_tasks_and_publishes_each_change() {
    let home = TempDir::new();
    let (fleet, _) = kept(&home);
    let started = SessionFact::Started { cwd: "/work".to_string(), title: None, origin: SessionOrigin::Terminal, mod_version: None };
    report(&fleet, started).await;
    let mut live = fleet.events().subscribe();

    let tasks = |statuses: &[ReportedTaskStatus]| SessionFact::Tasks {
        tasks: statuses.iter().enumerate().map(|(place, status)| task(&(place + 1).to_string(), *status)).collect(),
    };
    let row = report(&fleet, tasks(&[Todo, Todo])).await;
    assert!(row.plan.is_none());
    assert_eq!(row.build.as_ref().map(working), Some(vec![false, false]), "two open tasks, in the order made");
    assert_eq!(build_published(&mut live).await.map(|one| one.pieces.len()), Some(2), "the row went out with them");

    report(&fleet, tasks(&[Doing, Todo])).await;
    let published = build_published(&mut live).await.expect("a task beginning is a change worth publishing");
    assert_eq!(working(&published), [true, false]);
    assert_eq!(pieces(&published), [Ghost, Ghost]);

    let stored = Arc::clone(&fleet).get_session(SessionId::carried("t1")).await.expect("the row").session;
    assert_eq!(stored.build.as_ref().map(working), Some(vec![true, false]), "kept, so a reader later has the same list");
}

/// The forecast reads each landed pull request over the stretch from its Session's previous landing, and takes the most common
/// whole road, the newest winning a tie.
#[test]
fn a_repositorys_forecast_is_read_off_its_recent_landed_sessions() {
    let dir = TempDir::new();
    let mut store = store::Store::open(&dir.path().join("armada.db")).expect("opens");
    let entry = |minute: u32, kind: &str, subject_kind: &str, id: &str| store::TimelineEntry {
        at: at(minute),
        kind: kind.to_string(),
        subject_kind: subject_kind.to_string(),
        subject_id: id.to_string(),
        subject_url: None,
        title: None,
        repo: Some("armada".to_string()),
        tone: "idle".to_string(),
        says: "x".to_string(),
        detail: None,
        items: Vec::new(),
        service: None,
        service_name: None,
        to_id: None,
        live: false,
        over: false,
    };
    for (session, number) in [("s1", "12"), ("s2", "13"), ("s3", "14")] {
        store
            .attach(
                &KeptAttachment {
                    holder: Holder::session(session),
                    kind: "pr".to_string(),
                    manifest_id: "armada".to_string(),
                    target: number.to_string(),
                    state: AttachmentState::Spent,
                    detail: Default::default(),
                    since: at(0),
                    changed_at: at(0),
                },
                false,
            )
            .expect("attached");
    }
    // s1 and s2 walked, opened, had checks and merged. s3, the newest, did not walk.
    for e in [
        entry(1, "session.walked", "session", "s1"),
        entry(2, "pr.opened", "pull_request", "12"),
        entry(3, "pr.checks", "pull_request", "12"),
        entry(10, "pr.merged", "pull_request", "12"),
        entry(11, "session.walked", "session", "s2"),
        entry(12, "pr.opened", "pull_request", "13"),
        entry(13, "pr.checks", "pull_request", "13"),
        entry(20, "pr.merged", "pull_request", "13"),
        entry(21, "pr.opened", "pull_request", "14"),
        entry(22, "pr.checks", "pull_request", "14"),
        entry(30, "pr.merged", "pull_request", "14"),
    ] {
        store.append_timeline(&e).expect("appended");
    }
    let usual = crate::build_keeping::forecast_of(&store, "armada").expect("reads");
    assert_eq!(usual, [BuildStopId::Walk, BuildStopId::Pr, BuildStopId::Checks], "two of three walked");
    assert!(crate::build_keeping::forecast_of(&store, "another").expect("reads").is_empty());
}

fn timeline_entry(minute: u32, kind: &str) -> store::TimelineEntry {
    store::TimelineEntry {
        at: at(minute),
        kind: kind.to_string(),
        subject_kind: "session".to_string(),
        subject_id: "t1".to_string(),
        subject_url: None,
        title: None,
        repo: Some("armada".to_string()),
        tone: "idle".to_string(),
        says: "x".to_string(),
        detail: None,
        items: Vec::new(),
        service: None,
        service_name: None,
        to_id: None,
        live: false,
        over: false,
    }
}

/// **A Session whose edits were entered on the timeline before Fleet heard it reads as implementing, not scoping: the first thing
/// heard of it fills the row from the timeline, in a read of its own entries only.**
#[test]
fn a_session_heard_after_a_restart_is_filled_from_the_edits_the_timeline_kept() {
    let dir = TempDir::new();
    let mut store = store::Store::open(&dir.path().join("armada.db")).expect("opens");
    for e in [timeline_entry(2, "session.edited"), timeline_entry(9, "session.edited"), timeline_entry(11, "session.committed")] {
        store.append_timeline(&e).expect("appended");
    }
    let mut another = timeline_entry(1, "session.edited");
    another.subject_id = "t2".to_string();
    store.append_timeline(&another).expect("appended");

    assert!(crate::build_keeping::build_heard(&mut store, "t1", &Observed::Worked, &at(30)).expect("heard"));
    let build = store.session_build("t1").expect("reads").expect("a row");
    assert_eq!((build.first_edit_at, build.last_edit_at, build.begun_at), (Some(at(2)), Some(at(9)), at(2)));
    assert_eq!(build.last_tool_at, Some(at(30)));
    assert!(store.session_build("t2").expect("reads").is_none(), "another Session's entries are its own");

    // A row that already has a first edit is not read again.
    store.append_timeline(&timeline_entry(1, "session.edited")).expect("appended");
    crate::build_keeping::build_heard(&mut store, "t1", &Observed::Worked, &at(31)).expect("heard");
    assert_eq!(store.session_build("t1").expect("reads").expect("a row").first_edit_at, Some(at(2)));
}

/// **Work before a landing belongs to the build that landed**, so a Session that merged and went on is filled only from after.
#[test]
fn a_filled_row_reads_only_from_the_last_landing_on() {
    let dir = TempDir::new();
    let mut store = store::Store::open(&dir.path().join("armada.db")).expect("opens");
    store
        .attach(
            &KeptAttachment {
                holder: Holder::session("t1"),
                kind: "pr".to_string(),
                manifest_id: "armada".to_string(),
                target: "12".to_string(),
                state: AttachmentState::Spent,
                detail: Default::default(),
                since: at(0),
                changed_at: at(10),
            },
            false,
        )
        .expect("attached");
    store.append_timeline(&timeline_entry(5, "session.edited")).expect("appended");
    store.append_timeline(&timeline_entry(20, "session.edited")).expect("appended");

    crate::build_keeping::build_heard(&mut store, "t1", &Observed::Worked, &at(30)).expect("heard");
    let build = store.session_build("t1").expect("reads").expect("a row");
    assert_eq!((build.first_edit_at, build.last_edit_at), (Some(at(20)), Some(at(20))), "the edit before the merge is the last build's");
}
