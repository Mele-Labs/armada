//! A Job's thread folded for every client: the rules Bridge worked out in TypeScript, held where Fleet does.

use std::collections::BTreeMap;

use ipc::{
    Attachment, AttachmentState, FleetActKind, FleetActState, Instant, SessionAskState, SessionRecord, SessionRow,
    SessionThread, SessionVoice, ThreadItem, TimelineEvent, TimelineKind, TimelineSubject, TimelineTone,
};

use crate::thread_view::{folded, tail_of, window, Revisions};

pub(super) const SCENE: &str = r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0,"title":"A"}],"edges":[]}"#;

pub(super) fn at(minute: u32) -> Instant {
    Instant::carried(format!("2026-11-03T10:{minute:02}:00.000Z"))
}

pub(super) fn record(fields: &str) -> SessionRecord {
    let more = if fields.is_empty() { String::new() } else { format!(",{fields}") };
    let text = format!(
        r#"{{"id":"s1","harness":"hosted","origin":"bridge","cwd":"/w","state":"live","started_at":"2026-11-03T09:00:00.000Z","last_seen_at":"2026-11-03T09:00:00.000Z","usage":{{}},"attachments":[]{more}}}"#
    );
    ipc::decode("a record", text.as_bytes()).expect("a record")
}

pub(super) fn thread(rows: Vec<SessionRow>) -> SessionThread {
    SessionThread { session: record(""), rows }
}

pub(super) fn tool(id: &str, minute: u32, text: &str) -> SessionRow {
    SessionRow::Tool { id: id.into(), at: at(minute), text: text.into(), description: None, subagent: None, image: None, message: None }
}

pub(super) fn subagent(id: &str, minute: u32) -> SessionRow {
    let text = r#"{"id":"sub","task":"look","state":"running","since":"2026-11-03T10:00:00.000Z"}"#;
    let subagent = ipc::decode("a subagent", text.as_bytes()).expect("a subagent");
    SessionRow::Tool { id: id.into(), at: at(minute), text: "Agent look".into(), description: None, subagent: Some(subagent), image: None, message: None }
}

pub(super) fn message(id: &str, minute: u32, from: SessionVoice, text: &str, notice: Option<&str>) -> SessionRow {
    SessionRow::Message {
        id: id.into(),
        at: at(minute),
        from,
        text: text.into(),
        files: Vec::new(),
        tags: Vec::new(),
        notice: notice.map(str::to_string),
        queued: false,
        edited_at: None,
    }
}

pub(super) fn agent(id: &str, minute: u32, text: &str) -> SessionRow {
    message(id, minute, SessionVoice::Agent, text, None)
}

pub(super) fn you(id: &str, minute: u32, text: &str) -> SessionRow {
    message(id, minute, SessionVoice::You, text, None)
}

pub(super) fn fleet(id: &str, minute: u32, text: &str, notice: Option<&str>) -> SessionRow {
    let from = SessionVoice::Session { id: "fleet".into(), title: "Fleet".into() };
    message(id, minute, from, text, notice)
}

pub(super) fn check(id: &str, minute: u32, name: &str) -> SessionRow {
    let text = format!(r#"{{"kind":"check","id":"{id}","at":"{}","name":"{name}","run":1,"state":"passed"}}"#, at(minute).as_str());
    ipc::decode("a check", text.as_bytes()).expect("a check")
}

pub(super) fn ask(id: &str, state: SessionAskState, questions: &[&str]) -> SessionRow {
    let asked = questions
        .iter()
        .map(|question| format!(r#"{{"question":"{question}","header":"H","multi_select":false,"options":[]}}"#))
        .collect::<Vec<_>>()
        .join(",");
    let call = format!(
        r#"{{"call":"{id}","manifest_id":"armada","asked_at":"{}","tool":"AskUserQuestion","detail":"","truncated":false,"rule":"r","offers":["refuse"],"holding_for_seconds":60,"questions":[{asked}]}}"#,
        at(0).as_str()
    );
    SessionRow::Ask { id: id.into(), at: at(0), ask: ipc::decode("a call", call.as_bytes()).expect("a call"), state, via: None }
}

pub(super) fn kinds(items: &[ThreadItem]) -> Vec<&'static str> {
    items
        .iter()
        .map(|item| match item {
            ThreadItem::Row { .. } => "row",
            ThreadItem::Calls { .. } => "calls",
            ThreadItem::Subagent { .. } => "subagent",
            ThreadItem::Checks { .. } => "checks",
            ThreadItem::FleetTurn { .. } => "fleet_turn",
            ThreadItem::Sketch(_) => "sketch",
            ThreadItem::File { .. } => "file",
            ThreadItem::Document(_) => "document",
            ThreadItem::Change { .. } => "change",
            ThreadItem::NotStarted { .. } => "not_started",
            ThreadItem::Plan(_) => "plan",
            ThreadItem::Judged { .. } => "judged",
        })
        .collect()
}

pub(super) fn ids(items: &[ThreadItem]) -> Vec<&str> {
    items.iter().map(ThreadItem::id).collect()
}

pub(super) fn items(rows: Vec<SessionRow>) -> Vec<ThreadItem> {
    folded(&thread(rows), &[], &Revisions::new()).1
}

pub(super) fn held(kind: &str, target: &str, detail: &[(&str, &str)]) -> Attachment {
    Attachment {
        kind: kind.into(),
        manifest_id: None,
        target: target.into(),
        state: AttachmentState::Standing,
        detail: detail.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect::<BTreeMap<_, _>>(),
        since: at(0),
        changed_at: at(0),
    }
}

pub(super) fn kept_sketch(title: &str, extra: &[(&str, &str)]) -> Attachment {
    let mut detail = vec![("form", "sketch"), ("scene", SCENE), ("title", title)];
    detail.extend_from_slice(extra);
    held("artifact", &format!("sketch:{title}"), &detail)
}

pub(super) fn show_sketch(id: &str, minute: u32, title: &str, extra: &str) -> SessionRow {
    tool(id, minute, &format!(r#"mcp__armada-fleet__show_sketch {{"title":"{title}","scene":{SCENE}{extra}}}"#))
}

pub(super) fn with_ledger(rows: Vec<SessionRow>, ledger: Vec<Attachment>) -> Vec<ThreadItem> {
    let mut one = thread(rows);
    one.session.attachments = ledger;
    folded(&one, &[], &Revisions::new()).1
}

// ---- the fold

#[test]
fn a_run_of_calls_is_broken_by_a_message() {
    let found = items(vec![
        tool("t1", 1, "Bash ls"),
        tool("t2", 2, "Read a.rs"),
        tool("t3", 3, "Bash pwd"),
        agent("m1", 4, "done"),
        tool("t4", 5, "Edit b.rs"),
    ]);
    assert_eq!(kinds(&found), ["calls", "row", "calls"]);
    let ThreadItem::Calls { id, names, rows } = &found[0] else { panic!("a run of calls") };
    assert_eq!((id.as_str(), rows.len()), ("t1", 3));
    assert_eq!(names, &["Bash", "Read"], "each tool once, in the order first called");
    let ThreadItem::Calls { names, .. } = &found[2] else { panic!("a run of one") };
    assert_eq!(names, &["Edit"]);
}

#[test]
fn a_check_run_of_one_name_then_another_makes_two_items() {
    let found = items(vec![
        check("c1", 1, "lint"),
        check("c2", 2, "lint"),
        check("c3", 3, "test"),
        tool("t1", 4, "Bash ls"),
        check("c4", 5, "test"),
    ]);
    assert_eq!(kinds(&found), ["checks", "checks", "calls", "checks"]);
    let ThreadItem::Checks { rows, .. } = &found[0] else { panic!("checks") };
    assert_eq!(rows.len(), 2);
    assert_eq!(ids(&found), ["c1", "c3", "t1", "c4"]);
}

#[test]
fn a_subagent_call_stands_alone_and_ends_the_run_around_it() {
    let found = items(vec![tool("t1", 1, "Bash ls"), subagent("t2", 2), tool("t3", 3, "Bash pwd"), tool("t4", 4, "Read a")]);
    assert_eq!(kinds(&found), ["calls", "subagent", "calls"]);
    let ThreadItem::Calls { rows, .. } = &found[2] else { panic!("calls") };
    assert_eq!(rows.len(), 2);
}

#[test]
fn a_fleet_turn_folds_its_work_and_lifts_the_last_agent_message_out() {
    let found = items(vec![
        fleet("f1", 1, "Fleet: your turn ended", Some("stop_nudge")),
        tool("t1", 2, "Bash ls"),
        agent("m1", 3, "looking"),
        tool("t2", 4, "Read a"),
        agent("m2", 5, "all done"),
        you("y1", 6, "thanks"),
    ]);
    assert_eq!(kinds(&found), ["fleet_turn", "row", "row"]);
    let ThreadItem::FleetTurn { id, rows, handled, .. } = &found[0] else { panic!("a turn") };
    assert_eq!((id.as_str(), *handled), ("f1", true));
    assert_eq!(rows.iter().map(SessionRow::id).collect::<Vec<_>>(), ["t1", "m1", "t2"]);
    assert_eq!(found[1].id(), "m2", "the words meant for the person stand outside the fold");
}

#[test]
fn a_fleet_turn_the_agent_did_nothing_after_is_not_handled() {
    let found = items(vec![fleet("f1", 1, "Fleet: still there?", None), you("y1", 2, "yes")]);
    let ThreadItem::FleetTurn { rows, handled, .. } = &found[0] else { panic!("a turn") };
    assert!(rows.is_empty() && !handled);
    assert_eq!(kinds(&found), ["fleet_turn", "row"]);
}

#[test]
fn a_sketch_card_ends_a_run_of_calls() {
    let ledger = vec![kept_sketch("Plan", &[])];
    let found = with_ledger(
        vec![tool("t1", 1, "Bash ls"), show_sketch("s1", 2, "Plan", ""), tool("t2", 3, "Bash pwd"), tool("t3", 4, "Read a")],
        ledger,
    );
    assert_eq!(kinds(&found), ["calls", "sketch", "calls"]);
    let ThreadItem::Sketch(card) = &found[1] else { panic!("a card") };
    assert_eq!((card.id.as_str(), card.sketch.as_str(), card.title.as_str()), ("s1", "sketch:Plan", "Plan"));
    assert!(!card.replaced && !card.asked && card.settled.is_none());
}

#[test]
fn a_show_sketch_the_ledger_does_not_hold_stays_a_tool_row() {
    let refused = with_ledger(vec![show_sketch("s1", 1, "Plan", "")], vec![kept_sketch("Another", &[])]);
    assert_eq!(kinds(&refused), ["calls"]);
    let wont_draw = tool("s2", 2, r#"show_sketch {"title":"Plan","scene":{"nodes":[],"edges":[]}}"#);
    assert_eq!(kinds(&with_ledger(vec![wont_draw], vec![kept_sketch("Plan", &[])])), ["calls"]);
    let another_tool = tool("s3", 3, r#"Write {"title":"Plan"}"#);
    assert_eq!(kinds(&with_ledger(vec![another_tool], vec![kept_sketch("Plan", &[])])), ["calls"]);
}

#[test]
fn an_earlier_card_is_replaced_and_only_the_latest_carries_how_it_was_answered() {
    let ledger = vec![kept_sketch(
        "Plan",
        &[("asked", "true"), ("settled", "sent_back"), ("settled_at", "2026-11-03T10:30:00.000Z"), ("words", "wider")],
    )];
    let found = with_ledger(vec![show_sketch("s1", 1, "Plan", ""), agent("m", 2, "x"), show_sketch("s2", 3, "Plan", "")], ledger);
    let (ThreadItem::Sketch(first), ThreadItem::Sketch(last)) = (&found[0], &found[2]) else { panic!("two cards") };
    assert!(first.replaced && !first.asked && first.settled.is_none());
    assert!(!last.replaced && last.asked);
    let settled = last.settled.as_ref().expect("how he answered");
    assert_eq!((settled.how, settled.words.as_deref()), (ipc::SketchHow::SentBack, Some("wider")));
}

#[test]
fn a_card_read_against_another_is_drawn_against_the_latest_earlier_card_else_the_ledgers_copy() {
    let earlier = r#"{"title":"Base","scene":"{\"nodes\":[{\"id\":\"b\",\"kind\":\"box\",\"x\":1,\"y\":1,\"title\":\"B\"}],\"edges\":[]}"}"#;
    let against = r#","against":"Base""#;
    let ledger = vec![kept_sketch("Base", &[]), kept_sketch("Next", &[("against", earlier)])];
    let found = with_ledger(vec![show_sketch("s1", 1, "Base", ""), show_sketch("s2", 2, "Next", against)], ledger.clone());
    let (ThreadItem::Sketch(base), ThreadItem::Sketch(next)) = (&found[0], &found[1]) else { panic!("two cards") };
    assert_eq!(next.against.as_deref(), Some("Base"));
    assert_eq!(next.against_drawing.as_ref(), Some(&base.drawing), "the card as it was drawn then");
    let alone = with_ledger(vec![show_sketch("s2", 2, "Next", against)], ledger);
    let ThreadItem::Sketch(next) = &alone[0] else { panic!("a card") };
    assert!(next.against_drawing.is_some(), "the scene the ledger kept at the call");
    let plain = with_ledger(vec![show_sketch("s2", 2, "Next", against)], vec![kept_sketch("Next", &[])]);
    let ThreadItem::Sketch(next) = &plain[0] else { panic!("a card") };
    assert!(next.against_drawing.is_none(), "drawn plain where nothing holds what it names");
}

#[test]
fn a_mock_node_takes_the_kept_hash_the_ledger_holds_for_it() {
    let mock = |kept: &str| {
        format!(
            r#"{{"nodes":[{{"id":"m","kind":"mock","x":0,"y":0,"w":200,"h":100,"html":"<p>a</p>","css":""{kept}}}],"edges":[]}}"#
        )
    };
    let hash = r#","kept":"0123456789abcdef0123456789abcdef""#;
    let ledger = vec![held("artifact", "sketch:M", &[("form", "sketch"), ("title", "M"), ("scene", &mock(hash))])];
    let call = tool("s1", 1, &format!(r#"show_sketch {{"title":"M","scene":{}}}"#, mock("")));
    let found = with_ledger(vec![call], ledger);
    let ThreadItem::Sketch(card) = &found[0] else { panic!("a card") };
    let drawn = ipc::encode(&card.drawing).expect("a scene");
    assert!(drawn.contains("0123456789abcdef0123456789abcdef"), "the card has the hash: {drawn}");
}

#[test]
fn a_show_file_the_ledger_holds_is_a_file_opened_by_the_ledgers_own_path() {
    let ledger = vec![held("artifact", "/work/docs/a.md", &[("form", "file"), ("title", "a.md")])];
    let found = with_ledger(vec![tool("f1", 1, "mcp__armada-fleet__show_file ./docs/a.md"), tool("f2", 2, "show_file nothing.md")], ledger);
    assert_eq!(kinds(&found), ["file", "calls"]);
    let ThreadItem::File { id, path, name, .. } = &found[0] else { panic!("a file") };
    assert_eq!((id.as_str(), path.as_str(), name.as_str()), ("f1", "/work/docs/a.md", "a.md"));
}

#[test]
fn a_waiting_ask_a_reminder_a_plan_row_a_delivery_row_and_a_handed_off_link_are_no_items() {
    let reminder = SessionRow::BrowserReminder { id: "b1".into(), at: at(1), tool: "Chrome".into(), address: None };
    let plan = SessionRow::Plan { id: "p1".into(), at: at(2), version: 1 };
    let row = |text: String| -> SessionRow { ipc::decode("a row", text.as_bytes()).expect("a row") };
    let delivery = row(format!(r#"{{"kind":"delivery","id":"d1","at":"{}"}}"#, at(3).as_str()));
    let linked = |id: &str, replaces: &str| {
        row(format!(r#"{{"kind":"linked","id":"{id}","at":"{}","session":{{"id":"s2","title":"Next"}},"event":"done"{replaces}}}"#, at(4).as_str()))
    };
    let found = items(vec![
        reminder,
        ask("a1", SessionAskState::Waiting, &["Which?"]),
        plan,
        delivery,
        linked("l1", r#","replaces":"s0""#),
        linked("l2", ""),
    ]);
    assert_eq!(ids(&found), ["l2"]);
    assert_eq!(kinds(&found), ["row"]);
}

#[test]
fn a_document_card_stands_before_the_first_row_written_after_its_revision() {
    let page = held(
        "artifact",
        "/w/.armada/documents/script.html",
        &[
            ("form", "document"),
            ("title", "Video script"),
            ("revisions", r#"["/s/r1.html","/s/r2.html"]"#),
            ("revision", "2"),
            ("asked", "true"),
            ("kept_at", "2026-11-03T10:03:30.000Z"),
        ],
    );
    let found = with_ledger(vec![agent("a", 2, "x"), tool("w", 3, "Write script.html"), agent("b", 4, "y")], vec![page]);
    assert_eq!(ids(&found), ["a", "w", "document:/w/.armada/documents/script.html:r2", "b"]);
    let ThreadItem::Document(card) = &found[2] else { panic!("a card") };
    assert_eq!((card.revision, card.asked, card.title.as_str()), (2, true, "Video script"));
    assert!(card.settled.is_none());
}

#[test]
fn a_document_is_numbered_by_its_revisions_asks_nothing_and_follows_a_time_that_does_not_read() {
    let page = held(
        "artifact",
        "/w/d.html",
        &[("form", "document"), ("revisions", r#"["/s/r1.html","/s/r2.html"]"#), ("kept_at", "2026-11-03T10:03:30.000Z")],
    );
    let mut odd = agent("b", 0, "y");
    if let SessionRow::Message { at, .. } = &mut odd {
        *at = Instant::carried("10:09:00");
    }
    let found = with_ledger(vec![agent("a", 2, "x"), odd], vec![page]);
    let ThreadItem::Document(card) = found.last().expect("items") else { panic!("the card is last") };
    assert_eq!((card.revision, card.asked), (2, false));
}

#[test]
fn a_settled_document_carries_how_it_was_answered() {
    let page = held(
        "artifact",
        "/w/d.html",
        &[
            ("form", "document"),
            ("revisions", r#"["/s/r1.html"]"#),
            ("revision", "1"),
            ("asked", "true"),
            ("settled", "approved"),
            ("settled_at", "2026-11-03T10:04:00.000Z"),
        ],
    );
    let found = with_ledger(Vec::new(), vec![page]);
    let ThreadItem::Document(card) = &found[0] else { panic!("a card") };
    assert_eq!(card.at, at(0), "revision 1 is placed at when it was first held");
    assert_eq!(card.settled.as_ref().map(|settled| settled.how), Some(ipc::SketchHow::Approved));
}

fn event(id: u64, minute: u32, kind: TimelineKind, session: &str, says: &str) -> TimelineEvent {
    TimelineEvent {
        id,
        at: at(minute),
        kind,
        subject: TimelineSubject::Session { id: ipc::SessionId::carried(session) },
        title: None,
        tone: TimelineTone::Ok,
        says: says.into(),
        detail: None,
        items: Vec::new(),
        call: None,
        to: None,
        manifest_id: None,
        live: false,
        over: false,
    }
}

#[test]
fn a_value_the_person_changed_is_an_item_where_it_happened() {
    let timeline = [
        event(7, 3, TimelineKind::SessionModel, "s1", "model: opus → sonnet"),
        event(8, 3, TimelineKind::SessionEffort, "another", "effort: low -> high"),
        event(9, 3, TimelineKind::SessionStarted, "s1", "Started"),
        event(10, 1, TimelineKind::SessionWorkflow, "s1", "workflow: Feature  →  Fix "),
    ];
    let one = thread(vec![agent("a", 2, "x"), agent("b", 4, "y")]);
    let (_, found) = folded(&one, &timeline, &Revisions::new());
    assert_eq!(ids(&found), ["event:10", "a", "event:7", "b"]);
    let ThreadItem::Change { what, from, to, .. } = &found[2] else { panic!("a change") };
    assert_eq!((*what, from.as_str(), to.as_str()), (ipc::ChangedValue::Model, "opus", "sonnet"));
    let ThreadItem::Change { from, to, .. } = &found[0] else { panic!("a change") };
    assert_eq!((from.as_str(), to.as_str()), ("Feature", "Fix "));
}

#[test]
fn a_job_whose_agent_never_started_ends_in_a_not_started_item() {
    let mut one = thread(vec![you("y1", 5, "hello")]);
    let start_failed = r#"{"cause":"no such binary","log":["boom"]}"#;
    one.session = record(&format!(
        r#""hosted":{{"turn":{{"state":"idle"}},"mode":"auto","running":false,"start_failed":{start_failed}}}"#
    ));
    let (_, found) = folded(&one, &[], &Revisions::new());
    let ThreadItem::NotStarted { id, at: when, cause, log } = found.last().expect("items") else { panic!("not started") };
    assert_eq!((id.as_str(), cause.as_str(), log.len()), ("not-started", "no such binary", 1));
    assert_eq!(*when, at(5), "after the last row");
}

// ---- the head

#[test]
fn the_open_ask_is_hosted_first_and_named_by_the_whole_command_else_detail_else_tool() {
    let call = |detail: &str, whole: &str| {
        let whole = if whole.is_empty() { String::new() } else { format!(r#","whole":"{whole}""#) };
        format!(
            r#"{{"call":"c","manifest_id":"armada","asked_at":"{}","tool":"Bash","detail":"{detail}","truncated":false,"rule":"r","offers":[],"holding_for_seconds":60{whole}}}"#,
            at(0).as_str()
        )
    };
    let hosted = |asked: String| {
        format!(r#""hosted":{{"turn":{{"state":"idle"}},"mode":"auto","running":true,"asked":{asked}}}"#)
    };
    let command = |fields: String| {
        let one = SessionThread { session: record(&fields), rows: Vec::new() };
        folded(&one, &[], &Revisions::new()).0.asked.map(|asked| asked.command)
    };
    assert_eq!(command(hosted(call("ls", "ls -la"))).as_deref(), Some("ls -la"));
    assert_eq!(command(hosted(call("ls", ""))).as_deref(), Some("ls"));
    assert_eq!(command(hosted(call("", ""))).as_deref(), Some("Bash"));
    assert_eq!(command(String::new()), None);
    let terminal = format!(r#""terminal":{{"listening":true,"asked":{}}}"#, call("pwd", ""));
    assert_eq!(command(terminal).as_deref(), Some("pwd"));
}

#[test]
fn an_open_call_is_the_last_row_while_a_turn_runs() {
    let rows = vec![agent("a", 1, "x"), tool("t1", 2, "Bash cargo check")];
    let running = SessionThread { session: record(r#""turn_since":"2026-11-03T10:00:00.000Z""#), rows: rows.clone() };
    let open = folded(&running, &[], &Revisions::new()).0.open_call.expect("an open call");
    assert_eq!((open.text.as_str(), open.since), ("Bash cargo check", at(2)));
    assert!(folded(&thread(rows), &[], &Revisions::new()).0.open_call.is_none(), "no turn runs");
    let spoke = SessionThread { session: record(r#""turn_since":"2026-11-03T10:00:00.000Z""#), rows: vec![tool("t", 1, "Bash ls"), agent("a", 2, "x")] };
    assert!(folded(&spoke, &[], &Revisions::new()).0.open_call.is_none(), "the last row is not a tool row");
}

fn lapsed_in(rows: Vec<SessionRow>) -> Option<(String, Vec<String>)> {
    folded(&thread(rows), &[], &Revisions::new()).0.lapsed.map(|lapsed| (lapsed.id, lapsed.questions.into_iter().map(|one| one.question).collect()))
}

#[test]
fn a_question_nobody_answered_lapses_until_the_person_says_anything() {
    let unanswered = ask("2", SessionAskState::Unanswered, &["Which night?"]);
    assert_eq!(lapsed_in(vec![you("1", 1, "ok"), unanswered.clone()]), Some(("2".into(), vec!["Which night?".into()])));
    assert_eq!(lapsed_in(vec![unanswered, you("3", 3, "ok")]), None);
}

#[test]
fn a_kept_question_outlasts_what_he_says_until_a_message_answers_it() {
    let kept = ask("1", SessionAskState::Kept, &["Which night?"]);
    assert_eq!(lapsed_in(vec![kept.clone(), you("n", 2, "Approved: http://localhost/")]).map(|one| one.0), Some("1".into()));
    let answer = you("r", 3, "Answering your earlier question \"Which night?\": Friday");
    assert_eq!(lapsed_in(vec![kept.clone(), you("n", 2, "ok"), answer]), None);
    assert_eq!(lapsed_in(vec![kept, ask("2", SessionAskState::Waiting, &["Which night?"])]), None, "a newer ask takes it");
}

#[test]
fn earlier_kept_questions_come_oldest_first_under_the_newest_ones_id() {
    let rows = vec![
        ask("1", SessionAskState::Kept, &["Which night?"]),
        ask("2", SessionAskState::Kept, &["Which place?"]),
        you("3", 3, "ok"),
    ];
    assert_eq!(lapsed_in(rows), Some(("2".into(), vec!["Which night?".into(), "Which place?".into()])));
    assert_eq!(lapsed_in(vec![ask("1", SessionAskState::Unanswered, &[])]), None, "a permission is not a question");
}

#[test]
fn an_ended_job_has_nothing_lapsed() {
    let mut one = thread(vec![ask("1", SessionAskState::Kept, &["Q?"])]);
    assert!(folded(&one, &[], &Revisions::new()).0.lapsed.is_some(), "a live Job holds it");
    one.session.state = ipc::SessionState::Ended;
    assert!(folded(&one, &[], &Revisions::new()).0.lapsed.is_none());
}

#[test]
fn fleets_words_are_acts_that_are_over_and_nothing_the_agent_said_is() {
    let rows = vec![
        fleet("r1", 1, "Fleet: your turn ended\nCarry on.", Some("stop_nudge")),
        fleet("r2", 2, "Step moved.", Some("rail_step")),
        agent("r3", 3, "Done."),
    ];
    let acts = folded(&thread(rows), &[], &Revisions::new()).0.fleet_acts;
    let read = acts.iter().map(|act| (act.id.as_str(), act.state, act.kind, act.said.as_str())).collect::<Vec<_>>();
    assert_eq!(
        read,
        [
            ("r1", FleetActState::Over, FleetActKind::Nudge, "your turn ended"),
            ("r2", FleetActState::Over, FleetActKind::Step, "Step moved.")
        ]
    );
}

#[test]
fn a_running_repair_is_read_off_fleets_notes_and_a_coming_nudge_off_the_record() {
    let rows = vec![fleet("r1", 1, "Repair: a command hung", None), agent("r2", 2, "Still going.")];
    let one = SessionThread { session: record(r#""stop_told_by":"2026-11-03T10:01:00.000Z""#), rows };
    let acts = folded(&one, &[], &Revisions::new()).0.fleet_acts;
    assert_eq!(acts.iter().map(|act| (act.state, act.said.as_str())).collect::<Vec<_>>(), [
        (FleetActState::UnderWay, "Repairing: a command hung"),
        (FleetActState::Coming, "Tell it to carry on or name what it waits on"),
    ], "its start note is not drawn again as a thing done");
    assert_eq!(acts[1].until, Some(Instant::carried("2026-11-03T10:01:00.000Z")));
}

#[test]
fn a_repair_whose_end_note_follows_is_over_and_a_later_one_is_under_way() {
    let said = |rows: Vec<SessionRow>| {
        let acts = folded(&thread(rows), &[], &Revisions::new()).0.fleet_acts;
        acts.iter().map(|act| (act.state, act.said.clone())).collect::<Vec<_>>()
    };
    assert_eq!(said(vec![fleet("r1", 1, "Repair: its checkout is broken", None), fleet("r2", 2, "Repaired: it put the merge right", None)]), [
        (FleetActState::Over, "Repair: its checkout is broken".to_string()),
        (FleetActState::Over, "Repaired: it put the merge right".to_string()),
    ]);
    assert_eq!(said(vec![fleet("r1", 1, "Repair: a command hung", None), fleet("r2", 2, "Not repaired: it ran past 20 minutes", None), fleet("r3", 3, "Repair: its process died twice", None)]), [
        (FleetActState::Over, "Repair: a command hung".to_string()),
        (FleetActState::Over, "Not repaired: it ran past 20 minutes".to_string()),
        (FleetActState::UnderWay, "Repairing: its process died twice".to_string()),
    ]);
}

#[test]
fn the_trail_keeps_every_slot_held_and_when_each_pull_request_opened() {
    let mut one = thread(Vec::new());
    let mut given_back = held("slot", "2", &[]);
    (given_back.state, given_back.since, given_back.changed_at) = (AttachmentState::GivenBack, at(20), at(40));
    let mut standing = held("slot", "5", &[]);
    standing.since = at(50);
    let mut pull = held("pr", "1847", &[]);
    pull.since = at(55);
    one.session.attachments = vec![standing, given_back, pull, held("branch", "fix/x", &[])];
    let trail = folded(&one, &[], &Revisions::new()).0.trail;
    assert_eq!(trail.started, Instant::carried("2026-11-03T09:00:00.000Z"));
    assert_eq!(trail.bays.iter().map(|bay| (bay.slot, bay.until.is_some())).collect::<Vec<_>>(), [(2, true), (5, false)]);
    assert_eq!(trail.bays[0].until, Some(at(40)));
    assert_eq!(trail.opened.iter().map(|opened| opened.number).collect::<Vec<_>>(), [1847]);
    let bare = folded(&thread(Vec::new()), &[], &Revisions::new()).0.trail;
    assert!(bare.bays.is_empty() && bare.opened.is_empty());
}

// ---- the window and the tail

fn calls_and_messages(count: u32) -> Vec<SessionRow> {
    (0..count).map(|n| agent(&format!("m{n:03}"), n % 50, "x")).collect()
}

#[test]
fn a_window_takes_the_newest_whole_items_and_names_where_the_older_ones_begin() {
    let all = items(calls_and_messages(10));
    let (newest, older) = window(&all, None, Some(3)).expect("a window");
    assert_eq!((ids(&newest), older.as_deref()), (["m007", "m008", "m009"].to_vec(), Some("m007")));
    let (earlier, older) = window(&all, Some("m007"), Some(3)).expect("a window");
    assert_eq!((ids(&earlier), older.as_deref()), (["m004", "m005", "m006"].to_vec(), Some("m004")));
    let (first, older) = window(&all, Some("m002"), Some(5)).expect("a window");
    assert_eq!((ids(&first), older), (["m000", "m001"].to_vec(), None));
    assert!(window(&all, Some("nowhere"), None).is_none());
}

#[test]
fn a_window_is_never_cut_inside_an_item_and_finds_a_row_folded_into_one() {
    let all = items(vec![agent("a", 1, "x"), tool("t1", 2, "Bash a"), tool("t2", 3, "Bash b"), tool("t3", 4, "Bash c"), agent("b", 5, "y")]);
    let (newest, older) = window(&all, None, Some(2)).expect("a window");
    assert_eq!((kinds(&newest), older.as_deref()), (["calls", "row"].to_vec(), Some("t1")));
    let (before, _) = window(&all, Some("t3"), Some(5)).expect("a row inside the run finds the run");
    assert_eq!(ids(&before), ["a"], "what is before the whole run");
}

#[test]
fn a_tail_starts_at_the_first_item_the_client_holds_that_changed() {
    let before = items(vec![agent("a", 1, "x"), tool("t1", 2, "Bash a")]);
    let appended = items(vec![agent("a", 1, "x"), tool("t1", 2, "Bash a"), agent("b", 3, "y")]);
    let (from, tail) = tail_of(&before, &appended).expect("a change");
    assert_eq!((from, ids(&tail)), (None, vec!["b"]), "past the last item held, items are appended");
    let grown = items(vec![agent("a", 1, "x"), tool("t1", 2, "Bash a"), tool("t2", 3, "Bash b")]);
    let (from, tail) = tail_of(&before, &grown).expect("a change");
    assert_eq!((from.as_deref(), kinds(&tail)), (Some("t1"), vec!["calls"]));
    assert!(tail_of(&before, &before).is_none());
    let (from, tail) = tail_of(&appended, &before).expect("an item went");
    assert_eq!((from.as_deref(), tail.len()), (Some("b"), 0));
}

/// The native thread draws no `reasoning` row yet, so it is held back: two calls with reasoning between them stay one
/// run, and a Fleet turn keeps its calls (the owner, 10 Nov 2026).
#[test]
fn reasoning_is_held_back_and_ends_no_run_of_calls() {
    let reasoning = |id: &str, minute: u32| SessionRow::Reasoning { id: id.into(), at: at(minute), text: "Weighing the two.".into() };
    let found = items(vec![you("y1", 0, "go"), tool("t1", 1, "Bash ls"), reasoning("r1", 2), tool("t2", 3, "Bash pwd")]);
    assert_eq!(kinds(&found), ["row", "calls"]);
    assert_eq!(ids(&found), ["y1", "t1"]);
    let turn = items(vec![fleet("f1", 0, "Fleet: checks failed", None), tool("t1", 1, "Bash ls"), reasoning("r1", 2), tool("t2", 3, "Bash pwd")]);
    assert_eq!(kinds(&turn), ["fleet_turn"], "{turn:?}");
}
