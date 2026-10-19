//! The claim, written before the code: **a Session records a plan, the person approves it or marks
//! changes on it, and Fleet reads how far it got from what it saw, never from what the agent says.**
//! `docs/concepts/session.md`, *The plan* and *Who touched which file*.

use std::sync::Arc;

use adapter_traits::{CallDetail, DroneEvent};
use api::{HostedSessions, Refusal, Sessions};
use ipc::{
    ApprovePlan, PlanGroupInput, PlanMark, PlanSketchInput, PlanTaskInput, PlanTaskState, RecordPlan,
    ReportedTask, ReportedTaskStatus, SendPlanChanges, SessionFact, SessionId, SessionRecord, SessionReport,
    SessionRow, TouchAct, WaitingSource,
};

use super::session_host::{eventually, rig_placing, Rig};
use crate::session_host::Heard;

fn group(id: Option<&str>, title: &str, agent: &str, paths: &[&str], tasks: &[(Option<&str>, &str)]) -> PlanGroupInput {
    PlanGroupInput {
        id: id.map(String::from),
        title: title.to_string(),
        agent: agent.to_string(),
        paths: paths.iter().map(|one| one.to_string()).collect(),
        tasks: tasks.iter().map(|(id, title)| PlanTaskInput { id: id.map(String::from), title: title.to_string() }).collect(),
    }
}

fn plan(session: &SessionId, groups: Vec<PlanGroupInput>) -> RecordPlan {
    RecordPlan { groups, sketch: None, walk: None, session_id: Some(session.clone()) }
}

/// Two groups: the store's and Fleet's.
fn first(session: &SessionId) -> RecordPlan {
    plan(
        session,
        vec![
            group(None, "Store", "rust-engineer", &["crates/store"], &[(None, "Add the tables"), (None, "Add the migration")]),
            group(None, "Fleet", "rust-engineer", &["crates/fleet"], &[(None, "Record it")]),
        ],
    )
}

fn scene(second: &str) -> String {
    format!(
        r#"{{"nodes":[{{"id":"a","kind":"box","x":0,"y":0,"title":"Store"}},{{"id":"b","kind":"box","x":300,"y":0,"title":"{second}"}}],"edges":[{{"id":"e1","from":"a","to":"b","label":"rows"}}]}}"#
    )
}

fn sketch(title: &str, scene: &str) -> PlanSketchInput {
    ipc::decode("a plan sketch", format!(r#"{{"title":"{title}","scene":{scene}}}"#).as_bytes()).unwrap()
}

async fn recorded(rig: &Rig, groups: RecordPlan) -> SessionRecord {
    rig.fleet.record_plan(None, groups).await.expect("recorded")
}

/// A hosted Session with a process running, so what Fleet tells it can be read off the process.
async fn running(rig: &Rig) -> SessionId {
    let id = rig.start().await;
    rig.send(&id, "plan it").await;
    rig.stand_in.init(0);
    id
}

fn told(rig: &Rig, containing: &str) -> usize {
    rig.stand_in.process(0).sent.lock().unwrap().iter().filter(|line| line.contains(containing)).count()
}

async fn read(rig: &Rig, id: &SessionId) -> SessionRecord {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session
}

#[tokio::test]
async fn a_plan_is_recorded_pending_and_recording_again_keeps_the_diff() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let record = recorded(&rig, first(&id)).await;

    let one = record.plan.expect("a plan");
    assert_eq!((one.version, one.approved), (1, false));
    assert!(one.changes.is_none(), "the first version changes nothing");
    assert_eq!(one.scope, ["crates/store", "crates/fleet"]);
    let ids: Vec<&str> = one.groups.iter().flat_map(|g| std::iter::once(g.id.as_str()).chain(g.tasks.iter().map(|t| t.id.as_str()))).collect();
    assert_eq!(ids, ["g1", "t1", "t2", "g2", "t3"]);
    assert!(one.groups.iter().flat_map(|g| &g.tasks).all(|task| task.state == PlanTaskState::Open));

    // Again: t1 renamed under its id, t3 moved into the store's group (matched by its text), the
    // Fleet group gone, a Docs group new.
    let again = plan(
        &id,
        vec![
            group(
                Some("g1"),
                "Store",
                "rust-engineer",
                &["crates/store"],
                &[(Some("t1"), "Add the plan tables"), (None, "Add the migration"), (None, "Record it")],
            ),
            group(None, "Docs", "general-purpose", &["docs"], &[(None, "Write the concept")]),
        ],
    );
    let two = recorded(&rig, again).await.plan.expect("a plan");
    assert_eq!((two.version, two.approved), (2, false), "a new version is pending");
    let changes = two.changes.expect("what changed");
    assert_eq!(changes.added, ["g3", "t4"]);
    assert_eq!(changes.renamed.iter().map(|one| (one.id.as_str(), one.from.as_str())).collect::<Vec<_>>(), [("t1", "Add the tables")]);
    assert_eq!(changes.moved.iter().map(|one| (one.id.as_str(), one.from.as_str())).collect::<Vec<_>>(), [("t3", "g2")]);
    assert_eq!(changes.removed.iter().map(|one| (one.group.as_str(), one.text.as_str())).collect::<Vec<_>>(), [("g2", "Fleet")]);

    let listed = rig.fleet.list_sessions(None, None, None).await.unwrap().sessions;
    assert_eq!(listed.iter().find(|one| one.id == id).unwrap().plan.as_ref().map(|one| one.version), Some(2), "kept, not only answered");
}

#[tokio::test]
async fn a_sketch_that_breaks_a_rule_refuses_the_plan_and_nothing_is_kept() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let mut bad = first(&id);
    bad.sketch = Some(sketch("Plan flow", &scene("Fleet").replace(r#""kind":"box","x":300"#, r#""kind":"oval","x":300"#)));
    let Err(Refusal::Unacceptable(refused)) = rig.fleet.record_plan(None, bad).await else {
        panic!("a plan with a bad sketch is refused");
    };
    assert_eq!(refused.code, "fleet.plan_refused");
    assert!(refused.message.contains("`sketch`") && refused.message.contains("oval"), "{}", refused.message);
    assert!(refused.message.contains("record_plan"), "it names the tool to call again: {}", refused.message);
    assert!(read(&rig, &id).await.plan.is_none());

    let mut good = first(&id);
    good.sketch = Some(sketch("Plan flow", &scene("Fleet")));
    let kept = recorded(&rig, good).await;
    assert_eq!(kept.plan.unwrap().sketch.map(|one| one.title), Some("Plan flow".to_string()));
    assert!(kept.attachments.iter().any(|one| one.target == "sketch:Plan flow"), "the scene is the ledger's artifact");
}

#[tokio::test]
async fn a_plan_with_a_blank_group_or_a_path_outside_the_repository_is_refused_with_every_reason() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let bad = plan(&id, vec![group(None, " ", "rust engineer", &["../etc"], &[])]);
    let Err(Refusal::Unacceptable(refused)) = rig.fleet.record_plan(None, bad).await else { panic!("refused") };
    for part in ["blank `title`", "`agent`", "not a path inside the repository", "no tasks"] {
        assert!(refused.message.contains(part), "{part}: {}", refused.message);
    }
}

#[tokio::test]
async fn approving_clears_the_call_and_tells_the_agent_once() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    let record = recorded(&rig, first(&id)).await;
    let [item] = &record.waiting_for[..] else { panic!("the plan is one call: {:?}", record.waiting_for) };
    assert_eq!((item.source, item.id.as_str()), (WaitingSource::Plan, "plan:1"));

    let stale = Arc::clone(&rig.fleet).approve_plan(ApprovePlan { session_id: id.clone(), version: 2 }).await;
    assert!(matches!(&stale, Err(Refusal::IllegalMove(why)) if why.code == "fleet.plan_version_stale"), "{stale:?}");
    assert_eq!(told(&rig, "approved plan"), 0);

    let approved = Arc::clone(&rig.fleet).approve_plan(ApprovePlan { session_id: id.clone(), version: 1 }).await.unwrap();
    let plan = approved.plan.expect("a plan");
    assert!(plan.approved && plan.approved_at.is_some());
    assert!(approved.waiting_for.iter().all(|one| one.source != WaitingSource::Plan), "{:?}", approved.waiting_for);
    eventually(|| async { told(&rig, "approved plan v1") == 1 }).await;

    Arc::clone(&rig.fleet).approve_plan(ApprovePlan { session_id: id.clone(), version: 1 }).await.unwrap();
    assert_eq!(told(&rig, "approved plan v1"), 1, "a second press tells nothing");
}

#[tokio::test]
async fn an_approval_tells_the_agent_its_task_list_is_the_plans_tasks_word_for_word() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    recorded(&rig, first(&id)).await;
    let marked = told(&rig, "marked changes");
    let note = PlanMark::Note { id: "n1".into(), group: "g1".into(), task: None, text: "keep it small".into() };

    // Marks without an approval are proposals: the plan is not yet what is built.
    let marks = SendPlanChanges { session_id: id.clone(), version: 1, marks: vec![note.clone()], approve: false };
    Arc::clone(&rig.fleet).send_plan_changes(marks).await.unwrap();
    eventually(|| async { told(&rig, "marked changes on plan v1") == marked + 1 }).await;
    assert_eq!(told(&rig, "TaskCreate"), 0);

    let approved = SendPlanChanges { session_id: id.clone(), version: 1, marks: vec![note], approve: true };
    Arc::clone(&rig.fleet).send_plan_changes(approved).await.unwrap();
    eventually(|| async { told(&rig, "approved plan v1 with changes") == 1 }).await;
    let once = |line: &&String| line.contains("approved plan v1 with changes");
    let said = rig.stand_in.process(0).sent.lock().unwrap().iter().find(once).cloned().unwrap();
    assert!(said.contains("Your task list is this plan: TaskCreate each task word for word, and TaskUpdate it as you work."), "{said}");
    for task in ["- Add the tables", "- Add the migration", "- Record it"] {
        assert!(said.contains(task), "{task} in {said}");
    }

    // The plain approval says it too: here, on a plan the agent recorded again.
    let again = plan(&id, vec![group(Some("g1"), "Store", "rust-engineer", &["crates/store"], &[(Some("t1"), "Add the tables, revised")])]);
    recorded(&rig, again).await;
    Arc::clone(&rig.fleet).approve_plan(ApprovePlan { session_id: id.clone(), version: 2 }).await.unwrap();
    eventually(|| async { told(&rig, "approved plan v2") == 1 }).await;
    let said = rig.stand_in.process(0).sent.lock().unwrap().iter().find(|line| line.contains("approved plan v2")).cloned().unwrap();
    assert!(said.contains("TaskCreate each task word for word"), "{said}");
    assert!(said.contains("- Add the tables, revised"), "{said}");
    assert!(!said.contains("- Record it"), "only the plan being built: {said}");
}

#[tokio::test]
async fn a_plan_recorded_again_is_pending_again_and_nothing_blocked_while_it_was() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    recorded(&rig, first(&id)).await;
    // Nothing blocks the agent's tools while the plan is pending.
    let edit = rig.gate(&id, "Edit", r#"{"file_path":"crates/store/a.rs"}"#).await;
    assert!(!matches!(edit, ipc::GateAnswer::Hold { ref held } if held.reason.contains("plan")), "{edit:?}");
    Arc::clone(&rig.fleet).approve_plan(ApprovePlan { session_id: id.clone(), version: 1 }).await.unwrap();
    let again = recorded(&rig, first(&id)).await;
    let plan = again.plan.unwrap();
    assert_eq!((plan.version, plan.approved), (2, false));
    assert!(again.waiting_for.iter().any(|one| one.id == "plan:2"));
}

#[tokio::test]
async fn his_changes_are_one_message_and_a_thread_row_and_the_plan_stays_as_recorded() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    recorded(&rig, first(&id)).await;
    let marks = vec![
        PlanMark::Rename { group: "g1".into(), task: None, from: "Store".into(), to: "Storage".into() },
        PlanMark::Remove { group: "g2".into(), task: Some("t3".into()) },
        PlanMark::AddTask { group: "g1".into(), id: "x1".into(), text: "Index it".into() },
        PlanMark::Note { id: "n1".into(), group: "g1".into(), task: Some("x1".into()), text: "after the tables".into() },
    ];
    let sent = SendPlanChanges { session_id: id.clone(), version: 1, marks, approve: false };
    let record = Arc::clone(&rig.fleet).send_plan_changes(sent).await.unwrap();

    let plan = record.plan.expect("a plan");
    assert_eq!((plan.version, plan.approved), (1, false), "approval is not implied");
    assert_eq!(plan.groups[0].title, "Store", "the plan changes when the agent records it again");
    assert_eq!(plan.groups[1].tasks.len(), 1);
    assert!(record.waiting_for.iter().any(|one| one.source == WaitingSource::Plan), "still to approve");
    eventually(|| async { told(&rig, "marked changes on plan v1") == 1 }).await;
    let said = rig.stand_in.process(0).sent.lock().unwrap().iter().find(|line| line.contains("marked changes")).map(|line| line.replace("\\\"", "\"")).unwrap();
    for part in ["\"Store\" (g1)", "Storage", "\"Record it\" (t3)", "Index it", "after the tables", "proposals"] {
        assert!(said.contains(part), "{part}: {said}");
    }

    let rows = rig.rows(&id).await;
    let lines: Vec<_> = rows
        .iter()
        .filter_map(|row| match row {
            SessionRow::PlanChanges { version, approved, lines, .. } => Some((*version, *approved, lines.len())),
            _ => None,
        })
        .collect();
    assert_eq!(lines, [(1, false, 4)], "one row, and not a second Fleet message beside it");
    assert!(!rows.iter().any(|row| matches!(row, SessionRow::Message { text, .. } if text.contains("marked changes"))));
}

#[tokio::test]
async fn changes_sent_with_approve_approve_the_plan_and_a_mark_naming_nothing_is_refused() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    recorded(&rig, first(&id)).await;
    let nothing = SendPlanChanges {
        session_id: id.clone(),
        version: 1,
        marks: vec![PlanMark::Remove { group: "g9".into(), task: None }],
        approve: true,
    };
    let refused = Arc::clone(&rig.fleet).send_plan_changes(nothing).await;
    assert!(matches!(&refused, Err(Refusal::Unacceptable(why)) if why.code == "fleet.plan_mark_refused"), "{refused:?}");
    assert_eq!(told(&rig, "approved plan"), 0, "nothing was sent");

    let note = PlanMark::Note { id: "n1".into(), group: "g1".into(), task: None, text: "keep it small".into() };
    let sent = SendPlanChanges { session_id: id.clone(), version: 1, marks: vec![note.clone()], approve: true };
    let record = Arc::clone(&rig.fleet).send_plan_changes(sent).await.unwrap();
    assert!(record.plan.unwrap().approved);
    eventually(|| async { told(&rig, "approved plan v1 with changes") == 1 }).await;

    // Marks work on an approved plan too, and stay proposals.
    let later = SendPlanChanges { session_id: id.clone(), version: 1, marks: vec![note], approve: false };
    let record = Arc::clone(&rig.fleet).send_plan_changes(later).await.unwrap();
    assert!(record.plan.unwrap().approved);
    eventually(|| async { told(&rig, "marked changes on plan v1") == 1 }).await;
}

fn task(text: &str, status: ReportedTaskStatus) -> ReportedTask {
    ReportedTask { id: text.to_string(), text: text.to_string(), status, by: None }
}

async fn tasks(rig: &Rig, id: &SessionId, list: Vec<ReportedTask>) -> SessionRecord {
    let harness = read(rig, id).await.harness;
    rig.fleet
        .report_session(SessionReport { harness, session_id: id.clone(), fact: SessionFact::Tasks { tasks: list } })
        .await
        .expect("a report is kept")
}

fn states(record: &SessionRecord) -> Vec<(String, PlanTaskState, bool)> {
    record.plan.iter().flat_map(|p| &p.groups).flat_map(|g| &g.tasks).map(|t| (t.text.clone(), t.state, t.seen)).collect()
}

fn edit(path: &str) -> DroneEvent {
    DroneEvent::Called { tool: testkit::EDITS.into(), call: "toolu_1".into(), detail: CallDetail::of(path) }
}

use PlanTaskState::{Done, HandedIn, Open, Working};
use ReportedTaskStatus::{Completed, InProgress, Pending};

#[tokio::test]
async fn the_agents_task_list_moves_the_plans_tasks_by_text_and_a_claim_alone_is_handed_in() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    recorded(&rig, first(&id)).await;

    // Case and spacing do not matter; an item the plan does not hold changes nothing.
    let record = tasks(
        &rig,
        &id,
        vec![task("add  the TABLES", InProgress), task("Add the migration", Pending), task("Something else", Completed)],
    )
    .await;
    assert_eq!(
        states(&record),
        [("Add the tables".into(), Working, false), ("Add the migration".into(), Open, false), ("Record it".into(), Open, false)]
    );

    // Completed with no edit seen under the group's paths is the agent's word: handed in, not done.
    let record = tasks(&rig, &id, vec![task("Add the tables", Completed)]).await;
    assert_eq!(states(&record)[0], ("Add the tables".into(), HandedIn, false));
}

/// A list heard before any plan is the Session's progress on its own; once a plan is recorded the same
/// kind of list moves the plan's tasks as it always did, and Bridge shows the plan.
#[tokio::test]
async fn a_list_heard_before_a_plan_is_kept_and_the_plan_recorded_later_takes_the_next_list() {
    let rig = rig_placing(None);
    let id = rig.start().await;

    let record = tasks(&rig, &id, vec![task("Add the tables", InProgress), task("Something else", Pending)]).await;
    assert!(record.plan.is_none());
    let build = record.build.expect("kept with no plan");
    assert_eq!(build.pieces.iter().map(|one| (one.text.as_str(), one.working)).collect::<Vec<_>>(), [("Add the tables", true), ("Something else", false)]);

    recorded(&rig, first(&id)).await;
    let record = tasks(&rig, &id, vec![task("Add the tables", Completed), task("Add the migration", InProgress)]).await;
    assert_eq!(
        states(&record),
        [("Add the tables".into(), HandedIn, false), ("Add the migration".into(), Working, false), ("Record it".into(), Open, false)]
    );
}

#[tokio::test]
async fn a_reworded_item_ticks_the_task_it_says_and_an_unrelated_one_ticks_nothing() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    recorded(&rig, first(&id)).await;

    // The plan's words inside a longer line, and the same words in another order with one added.
    let record = tasks(
        &rig,
        &id,
        vec![
            task("Add the tables to the store crate", InProgress),
            task("Write migration, add", Completed),
            task("Mock walk: queue button and model picker", Completed),
            task("Run the tests", InProgress),
        ],
    )
    .await;
    assert_eq!(
        states(&record),
        [("Add the tables".into(), Working, false), ("Add the migration".into(), HandedIn, false), ("Record it".into(), Open, false)],
        "the two reworded items tick their tasks; the two that say something else tick nothing"
    );
}

#[tokio::test]
async fn an_item_that_shares_only_a_verb_with_a_task_is_not_that_task() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    recorded(&rig, first(&id)).await;

    // "Add" and "the" are in every task of the first group; one word of two is not enough.
    let record = tasks(&rig, &id, vec![task("Add the docs", Completed), task("Add the dispatcher", Completed)]).await;
    assert!(states(&record).iter().all(|(_, state, _)| *state == Open), "{:?}", states(&record));
}

#[tokio::test]
async fn two_items_cannot_claim_one_task_and_the_closer_one_does() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    recorded(&rig, first(&id)).await;

    // Both carry "Add the tables". The closer says it exactly once reduced to its words; the other is
    // ignored, whichever the agent listed first, and does not go on to claim a second task.
    for list in [
        vec![task("Add the tables to the store crate", Completed), task("Add tables", InProgress)],
        vec![task("Add tables", InProgress), task("Add the tables to the store crate", Completed)],
    ] {
        let record = tasks(&rig, &id, list).await;
        assert_eq!(
            states(&record),
            [("Add the tables".into(), Working, false), ("Add the migration".into(), Open, false), ("Record it".into(), Open, false)]
        );
    }
}

#[tokio::test]
async fn an_exact_item_keeps_its_task_when_a_longer_one_also_carries_the_words() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    recorded(&rig, first(&id)).await;

    let record = tasks(
        &rig,
        &id,
        vec![task("Add the tables and the migration", Completed), task("Add the migration", InProgress), task("Add the tables", Pending)],
    )
    .await;
    assert_eq!(
        states(&record),
        [("Add the tables".into(), Open, false), ("Add the migration".into(), Working, false), ("Record it".into(), Open, false)],
        "each exact item has its own task; the long one that holds both is left over"
    );
}

#[tokio::test]
async fn an_edit_seen_under_a_groups_paths_marks_the_group_worked_and_makes_its_completion_done() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    recorded(&rig, first(&id)).await;
    tasks(&rig, &id, vec![task("Add the tables", InProgress)]).await;

    // An edit outside both groups' paths moves neither.
    rig.stand_in.says(0, vec![edit("docs/other.md")]);
    // One under the store's paths marks that group, and only it.
    rig.stand_in.says(0, vec![edit("crates/store/src/plan.rs")]);
    eventually(|| async {
        read(&rig, &id).await.plan.is_some_and(|plan| plan.groups[0].worked_at.is_some())
    })
    .await;
    let record = read(&rig, &id).await;
    let plan = record.plan.as_ref().unwrap();
    assert!(plan.groups[1].worked_at.is_none(), "Fleet's group was not touched");
    assert_eq!(states(&record)[0], ("Add the tables".into(), Working, true), "the edit is seen for the task in progress");

    let record = tasks(&rig, &id, vec![task("Add the tables", Completed)]).await;
    assert_eq!(states(&record)[0], ("Add the tables".into(), Done, true), "completed with an edit seen is done");
}

/// A hosted stream, as the process says it: the agent's line that starts a subagent, and the
/// subagent's own, which names that call as its parent.
const SPAWNING: &str = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"toolu_77","name":"Agent","input":{"subagent_type":"rust-engineer","description":"store"}}]}}"#;
const INSIDE: &str = r#"{"type":"assistant","parent_tool_use_id":"toolu_77","message":{"content":[{"type":"tool_use","id":"toolu_78","name":"Write","input":{"file_path":"crates/store/src/plan.rs"}}]}}"#;

#[tokio::test]
async fn a_touch_is_attributed_to_the_subagent_whose_call_the_line_sat_under() {
    let rig = rig_placing(None);
    let id = running(&rig).await;

    // What the stream reader sends for those two lines.
    let spawned: Vec<(String, String)> = adapters::subagent_calls(SPAWNING).into_iter().map(|one| (one.call, one.kind)).collect();
    assert_eq!(spawned, [("toolu_77".to_string(), "rust-engineer".to_string())]);
    rig.stand_in.hears(0, Heard::Spawned(spawned));
    rig.stand_in.hears(0, Heard::Subagent { events: vec![edit("crates/store/src/plan.rs")], call: adapters::subagent_parent(INSIDE) });
    rig.stand_in.says(0, vec![edit("/not/under/the/session/a.rs"), edit("README.md")]);

    eventually(|| async { read(&rig, &id).await.touches.len() == 2 }).await;
    let touches = read(&rig, &id).await.touches;
    assert_eq!(touches[0].path, "crates/store/src/plan.rs");
    assert_eq!(touches[0].act, TouchAct::Edit);
    let by = touches[0].by.as_ref().expect("a subagent");
    assert_eq!((by.agent.as_str(), by.id.as_str()), ("rust-engineer", "toolu_77"));
    assert_eq!(touches[1].path, "README.md");
    assert!(touches[1].by.is_none(), "the Session's own agent");
}

#[tokio::test]
async fn a_subagent_whose_call_was_never_heard_is_named_a_subagent() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    rig.stand_in.hears(0, Heard::Subagent { events: vec![edit("src/a.rs")], call: Some("toolu_lost".into()) });
    eventually(|| async { read(&rig, &id).await.touches.len() == 1 }).await;
    let by = read(&rig, &id).await.touches[0].by.clone().expect("a subagent");
    assert_eq!((by.agent.as_str(), by.id.as_str()), ("subagent", "toolu_lost"));
}

#[tokio::test]
async fn the_touch_window_is_bounded_and_a_repeat_of_the_latest_touch_is_one() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let most = crate::file_touching::WINDOW;
    {
        let mut store = rig.fleet.store().lock().await;
        for nth in 0..most + 15 {
            let at = format!("2026-10-16T09:{:02}:{:02}.000Z", nth / 60, nth % 60);
            crate::plan_keeping::touch_kept(&mut store, id.as_str(), None, &format!("src/f{nth}.rs"), "edit", &at).unwrap();
        }
        // The same file again and again: one touch, moved to the latest instant.
        for nth in 0..5 {
            let at = format!("2026-10-16T10:00:{nth:02}.000Z");
            crate::plan_keeping::touch_kept(&mut store, id.as_str(), None, "src/same.rs", "edit", &at).unwrap();
        }
    }
    let touches = read(&rig, &id).await.touches;
    assert_eq!(touches.len(), most, "the window holds {most}");
    assert_eq!(touches.last().unwrap().path, "src/same.rs");
    assert_eq!(touches.iter().filter(|one| one.path == "src/same.rs").count(), 1);
    assert!(touches.iter().all(|one| one.path != "src/f0.rs"), "the oldest went first");
}
