//! Fleet's fold of a thread matching Electron Bridge's, item for item: what it keeps unfolded, what it never
//! draws, where an answered ask goes, and the plan, pinned sketch, revisions and Judge lines it draws.

use ipc::{PlanCardState, SessionAskState, SessionRow, SessionThread, ThreadItem};

use super::thread_view::{agent, ask, at, fleet, held, ids, items, kept_sketch, kinds, record, show_sketch, thread, tool, you, SCENE};
use crate::thread_view::{folded, Revisions};

fn row(text: String) -> SessionRow {
    ipc::decode("a row", text.as_bytes()).expect("a row")
}

fn disputed(id: &str, minute: u32, notice: &str) -> SessionRow {
    row(format!(r#"{{"kind":"disputed","id":"{id}","at":"{}","notice":"{notice}","why":"wrong","spotted":"called"}}"#, at(minute).as_str()))
}

fn window_ask(id: &str, minute: u32) -> SessionRow {
    let place = r#"{"url":"http://localhost/","selector":"main"}"#;
    row(format!(
        r#"{{"kind":"window_ask","id":"{id}","at":"{}","question":"Which?","place":{place},"window":"http://localhost/"}}"#,
        at(minute).as_str()
    ))
}

fn with(fields: &str, rows: Vec<SessionRow>) -> SessionThread {
    SessionThread { session: record(fields), rows }
}

const ASKED: &str = r#""hosted":{"turn":{"state":"idle"},"mode":"auto","running":true,"asked":{"call":"c","manifest_id":"armada","asked_at":"2026-11-03T10:00:00.000Z","tool":"Bash","detail":"ls","truncated":false,"rule":"r","offers":[],"holding_for_seconds":60}}"#;

fn plan(approved: bool, extra: &str) -> String {
    format!(r#""plan":{{"version":2,"recorded_at":"2026-11-03T10:00:00.000Z","approved":{approved},"scope":[],"groups":[]{extra}}}"#)
}

const PLAN_ASKED: &str = r#""waiting_for":[{"id":"plan","text":"Plan","since":"2026-11-03T10:00:00.000Z","source":"plan"}]"#;

fn turn_then_words() -> Vec<SessionRow> {
    vec![
        fleet("f0", 0, "Fleet: checks failed", None),
        tool("t0", 1, "Bash ls"),
        you("y0", 2, "go on"),
        fleet("f1", 3, "Fleet: your turn ended", Some("stop_nudge")),
        tool("t1", 4, "Bash pwd"),
        agent("m1", 5, "Which do you want?"),
    ]
}

#[test]
fn a_disputed_notice_stays_a_row_and_what_followed_it_folds_as_calls_do() {
    let found = items(vec![fleet("f1", 1, "Fleet: you are behind", Some("slot_behind")), tool("t1", 2, "Bash ls"), agent("m1", 3, "x"), disputed("d1", 4, "f1")]);
    assert_eq!(kinds(&found), ["row", "calls", "row", "row"]);
    assert_eq!(ids(&found), ["f1", "t1", "m1", "d1"]);
}

#[test]
fn the_last_fleet_turn_stays_open_while_the_job_asks_him_and_no_other_does() {
    let found = folded(&with(ASKED, turn_then_words()), &[], &Revisions::new()).1;
    assert_eq!(kinds(&found), ["fleet_turn", "row", "row", "calls", "row"], "the earlier turn stays folded");
    assert_eq!(ids(&found), ["f0", "y0", "f1", "t1", "m1"]);
    let calm = folded(&with("", turn_then_words()), &[], &Revisions::new()).1;
    assert_eq!(kinds(&calm), ["fleet_turn", "row", "fleet_turn", "row"], "nothing asked, nothing kept");
    let put = format!("{},{PLAN_ASKED}", plan(false, ""));
    assert_eq!(kinds(&folded(&with(&put, turn_then_words()), &[], &Revisions::new()).1)[2], "row", "a plan put to him asks too");
}

#[test]
fn calls_no_screen_draws_are_left_out_before_the_fold() {
    let found = items(vec![
        tool("t1", 1, "TaskUpdate {}"),
        tool("t2", 2, "Bash ls"),
        tool("t3", 3, "mcp__armada-fleet__show_window {}"),
        tool("t4", 4, "ToolSearch select:Read"),
        agent("m1", 5, "x"),
        fleet("f1", 6, "Fleet: still there?", None),
        tool("t5", 7, "TaskList"),
    ]);
    assert_eq!(kinds(&found), ["calls", "row", "fleet_turn"]);
    let ThreadItem::Calls { rows, names, .. } = &found[0] else { panic!("a run") };
    assert_eq!((rows.len(), names.as_slice()), (1, ["Bash".to_string()].as_slice()));
    let ThreadItem::FleetTurn { handled, .. } = &found[2] else { panic!("a turn") };
    assert!(!handled, "an undrawn call is nothing done");
}

#[test]
fn a_settled_ask_and_a_window_question_fold_into_the_run_of_calls() {
    let found = items(vec![
        tool("t1", 1, "Bash ls"),
        ask("a1", SessionAskState::AllowedOnce, &[]),
        window_ask("w1", 2),
        tool("t2", 3, "Read a.rs"),
    ]);
    assert_eq!(kinds(&found), ["calls"]);
    let ThreadItem::Calls { rows, names, .. } = &found[0] else { panic!("a run") };
    assert_eq!(rows.iter().map(SessionRow::id).collect::<Vec<_>>(), ["t1", "a1", "w1", "t2"]);
    assert_eq!(names, &["Bash", "AskUserQuestion", "Read"], "an ask names its tool, a window question none");
    let turn = items(vec![fleet("f1", 0, "Fleet: go", None), ask("a2", SessionAskState::Refused, &[])]);
    let ThreadItem::FleetTurn { rows, .. } = &turn[0] else { panic!("a turn") };
    assert_eq!(rows.len(), 1, "a Fleet turn takes it as its reply");
}

#[test]
fn a_document_is_one_card_at_its_newest_revision() {
    let page = held(
        "artifact",
        "/w/.armada/documents/a.html",
        &[("form", "document"), ("revisions", r#"["/s/r1.html","/s/r2.html","/s/r3.html"]"#), ("revision", "3"), ("kept_at", "2026-11-03T10:05:00.000Z")],
    );
    let mut one = thread(vec![agent("a", 1, "x"), agent("b", 9, "y")]);
    one.session.attachments = vec![page];
    let found = folded(&one, &[], &Revisions::new()).1;
    assert_eq!(ids(&found), ["a", "document:/w/.armada/documents/a.html:r3", "b"]);
}

#[test]
fn an_approved_plan_stands_after_the_row_before_its_plan_row_and_nowhere_after_a_run_of_calls() {
    let plan_row = |minute| SessionRow::Plan { id: "p".into(), at: at(minute), version: 2 };
    let after_words = with(&plan(true, ""), vec![agent("m1", 1, "here"), plan_row(2), you("y", 3, "ok")]);
    let found = folded(&after_words, &[], &Revisions::new()).1;
    assert_eq!(ids(&found), ["m1", "plan:v2", "y"]);
    let ThreadItem::Plan(card) = &found[1] else { panic!("the plan") };
    assert_eq!((card.version, card.state), (2, PlanCardState::Approved));
    let after_calls = with(&plan(true, ""), vec![tool("t", 1, "mcp__armada-fleet__record_plan {}"), plan_row(2), you("y", 3, "ok")]);
    assert_eq!(ids(&folded(&after_calls, &[], &Revisions::new()).1), ["t", "y"]);
    let pending = with(&plan(false, ""), vec![agent("m1", 1, "here"), plan_row(2)]);
    assert_eq!(ids(&folded(&pending, &[], &Revisions::new()).1), ["m1"], "a pending plan is the head's");
}

#[test]
fn a_plan_put_to_him_or_set_aside_is_the_heads_and_a_held_one_is_nothing() {
    let head = |fields: String| folded(&with(&fields, Vec::new()), &[], &Revisions::new()).0.plan.map(|plan| (plan.id, plan.state));
    assert_eq!(head(format!("{},{PLAN_ASKED}", plan(false, ""))), Some(("plan:v2".into(), PlanCardState::Waiting)));
    let aside = plan(false, r#","set_aside_at":"2026-11-03T10:01:00.000Z""#);
    assert_eq!(head(aside), Some(("plan:v2".into(), PlanCardState::SetAside)));
    assert_eq!(head(plan(false, "")), None, "recorded and held, not put to him yet");
    assert_eq!(head(format!("{},{PLAN_ASKED}", plan(true, ""))), None, "approved is an item");
}

#[test]
fn the_newest_sketch_put_to_him_is_pinned_and_its_card_leaves_the_thread() {
    let asked = kept_sketch("Map", &[("asked", "true"), ("drawn", "s1-20261103T100300000Z-1")]);
    let mut one = thread(vec![show_sketch("s1", 1, "Map", ""), tool("t1", 2, "Bash a"), show_sketch("s2", 3, "Map", ""), tool("t2", 4, "Bash b")]);
    one.session.attachments = vec![asked];
    let (head, found) = folded(&one, &[], &Revisions::new());
    assert_eq!(kinds(&found), ["sketch", "calls"], "the runs either side of the card join");
    let ThreadItem::Sketch(earlier) = &found[0] else { panic!("the earlier card") };
    assert!(earlier.replaced);
    let pin = head.sketch.expect("the pin");
    assert_eq!((pin.sketch.as_str(), pin.title.as_str(), pin.asked), ("sketch:Map", "Map", true));
    let answered = kept_sketch("Map", &[("asked", "true"), ("settled", "approved"), ("settled_at", "2026-11-03T10:09:00.000Z")]);
    one.session.attachments = vec![answered];
    let (head, found) = folded(&one, &[], &Revisions::new());
    assert!(head.sketch.is_none());
    assert_eq!(kinds(&found), ["sketch", "calls", "sketch", "calls"]);
}

#[test]
fn the_latest_sketch_card_carries_every_kept_drawing_once_all_of_them_read() {
    let kept = kept_sketch("Map", &[("revisions", r#"["/k/1.json","/k/2.json"]"#)]);
    let mut one = thread(vec![show_sketch("s1", 1, "Map", ""), agent("m", 2, "x"), show_sketch("s2", 3, "Map", "")]);
    one.session.attachments = vec![kept];
    let scene = ipc::scene_of_detail(SCENE).expect("a scene");
    let mut read = Revisions::new();
    read.insert("/k/1.json".into(), scene.clone());
    let revisions = |read: &Revisions| -> Vec<usize> {
        folded(&one, &[], read).1.iter().filter_map(|item| match item {
            ThreadItem::Sketch(card) => Some(card.revisions.len()),
            _ => None,
        }).collect()
    };
    assert_eq!(revisions(&read), [0, 0], "one still unread offers none");
    read.insert("/k/2.json".into(), scene);
    assert_eq!(revisions(&read), [0, 2], "the latest card only");
}

#[test]
fn a_finished_judge_reading_stands_after_the_last_row_by_then() {
    let reading = |id: &str, state: &str, minute: u32, notes: &str| {
        format!(
            r#"{{"id":"{id}","artifact":{{"kind":"plan","id":"2","title":"Plan v2"}},"prompt":"","state":"{state}","asked_at":"{}","finished_at":"{}","read":false,"notes":[{notes}]}}"#,
            at(0).as_str(),
            at(minute).as_str()
        )
    };
    let note = r#"{"id":"n","claim":"missing a test"}"#;
    let readings = [reading("r1", "done", 2, ""), reading("r2", "done", 0, note), reading("r3", "failed", 2, ""), reading("r4", "reading", 2, "")];
    let fields = format!(r#""judge_readings":[{}]"#, readings.join(","));
    let one = with(&fields, vec![agent("a", 1, "x"), tool("t1", 3, "Bash a"), tool("t2", 4, "Bash b")]);
    let found = folded(&one, &[], &Revisions::new()).1;
    assert_eq!(ids(&found), ["judged:r2", "a", "judged:r1", "t1"]);
    let ThreadItem::Judged { clean, .. } = &found[0] else { panic!("a judged line") };
    assert!(!clean, "it has a note");
}
