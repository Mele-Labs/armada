//! The claim, written before the code: **a recorded plan is put to the person when he can answer
//! it.** Not while a page the Job showed is being changed or was changed and not shown again, and, where it names a walk, not before he
//! has approved that walk. `docs/concepts/session/the-plan.md`.

use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Sessions};
use ipc::{
    AnswerSessionAsk, AskingToRun, DismissWaiting, HelmCallAnswer, PlanGroupInput, PlanTaskInput, QuestionAnswer, RecordPlan,
    SessionFact, SessionId, SessionRecord, SessionReport, ShowWindow, WaitingItem, WaitingSource, WindowClosed, WindowTake,
    WithdrawWindow,
};

use super::session_host::{eventually, rig_placing, Rig};

const ONE: &str = "http://localhost:5173/one";
const TWO: &str = "http://localhost:5173/two";

fn plan(id: &SessionId, walk: Option<&str>) -> RecordPlan {
    RecordPlan {
        summary: "You get a plan.".into(),
        groups: vec![PlanGroupInput {
            id: None,
            title: "Store".into(),
            agent: "rust-engineer".into(),
            paths: vec!["crates/store".into()],
            issue: None,
            gives: None,
            tasks: vec![PlanTaskInput { id: None, title: "Add the table".into() }],
        }],
        sketch: None,
        walk: walk.map(String::from),
        session_id: Some(id.clone()),
    }
}

async fn record(rig: &Rig, id: &SessionId, walk: Option<&str>) {
    rig.fleet.record_plan(None, plan(id, walk)).await.expect("recorded");
}

async fn show(rig: &Rig, id: &SessionId, shown: ShowWindow) {
    rig.fleet.show_window(None, ShowWindow { session_id: Some(id.clone()), ..shown }).await.expect("shown");
}

async fn page(rig: &Rig, id: &SessionId, url: &str) {
    show(rig, id, ShowWindow { url: url.into(), ..Default::default() }).await;
}

async fn told(rig: &Rig, id: &SessionId, fact: SessionFact) {
    let report = SessionReport { harness: adapters::HOSTED_HARNESS.into(), session_id: id.clone(), fact };
    rig.fleet.report_session(report).await.expect("kept");
}

async fn read(rig: &Rig, id: &SessionId) -> SessionRecord {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session
}

async fn call(rig: &Rig, id: &SessionId) -> Option<WaitingItem> {
    read(rig, id).await.waiting_for.into_iter().find(|one| one.source == WaitingSource::Plan)
}

/// The clock reads milliseconds; a pause makes the next instant a later one.
async fn later() {
    tokio::time::sleep(Duration::from_millis(5)).await;
}

#[tokio::test]
async fn a_plan_recorded_while_a_page_changes_waits_until_the_job_shows_it_again_and_he_approves_it() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    page(&rig, &id, ONE).await;
    later().await;
    told(&rig, &id, SessionFact::Wrote).await;

    record(&rig, &id, None).await;
    assert!(call(&rig, &id).await.is_none(), "the page the plan shows is being changed");
    assert!(read(&rig, &id).await.plan.is_some_and(|plan| !plan.approved), "and the plan is kept, pending");

    later().await;
    told(&rig, &id, SessionFact::TurnCompleted { said: None }).await;
    assert!(call(&rig, &id).await.is_none(), "the page is ready, but it was changed and not shown again");
    told(&rig, &id, SessionFact::Wrote).await;
    told(&rig, &id, SessionFact::TurnCompleted { said: None }).await;
    assert!(call(&rig, &id).await.is_none(), "a second fix and a second turn end change nothing");

    later().await;
    page(&rig, &id, ONE).await;
    assert!(call(&rig, &id).await.is_none(), "shown again, it is to be approved again");
    let shown = read(&rig, &id).await.attachments.iter().find(|one| one.kind == "artifact").expect("the window").changed_at.clone();

    later().await;
    rig.send(&id, &format!("Approved: {ONE}")).await;
    let record = read(&rig, &id).await;
    let item = call(&rig, &id).await.expect("approved, so it is asked");
    assert_eq!(item.id, "plan:1");
    assert!(item.since > shown && item.since > record.plan.expect("a plan").recorded_at, "later than the re-show and the recording");

    told(&rig, &id, SessionFact::Wrote).await;
    assert!(call(&rig, &id).await.is_some(), "his Approve stands until the page is shown again");
}

#[tokio::test]
async fn a_plan_with_no_page_shown_is_asked_at_its_recording() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    record(&rig, &id, None).await;
    let record = read(&rig, &id).await;
    let item = call(&rig, &id).await.expect("asked");
    assert_eq!(item.since.as_str(), record.plan.expect("a plan").recorded_at.as_str());
}

#[tokio::test]
async fn a_plan_naming_a_walk_is_asked_only_after_his_approve_on_it() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    record(&rig, &id, Some(ONE)).await;
    assert!(call(&rig, &id).await.is_none(), "the walk was never shown");

    rig.send(&id, &format!("Approved: {ONE}")).await;
    assert!(call(&rig, &id).await.is_none(), "an Approve before the page was shown approves nothing");

    later().await;
    page(&rig, &id, ONE).await;
    assert!(call(&rig, &id).await.is_none(), "shown and not approved");
    rig.send(&id, "Looks close, but the header is off").await;
    rig.send(&id, &format!("Approved: {TWO}")).await;
    assert!(call(&rig, &id).await.is_none(), "an Approve of another page is not this walk's");

    later().await;
    rig.send(&id, &format!("Approved: {ONE}")).await;
    let item = call(&rig, &id).await.expect("approved, so it is asked");
    assert!(item.since.as_str() > read(&rig, &id).await.plan.expect("a plan").recorded_at.as_str());

    later().await;
    page(&rig, &id, ONE).await;
    assert!(call(&rig, &id).await.is_none(), "shown again, it is to be approved again");
}

/// Job 887f02f4, 27 Oct 2026: the plan named the walk, he approved it, and six seconds later the Job
/// edited its design document. The card went back on hold and never reached him.
#[tokio::test]
async fn an_approved_walk_holds_nothing_while_the_job_goes_on_editing() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    record(&rig, &id, Some(ONE)).await;
    later().await;
    page(&rig, &id, ONE).await;
    later().await;
    rig.send(&id, &format!("Approved: {ONE}")).await;
    assert!(call(&rig, &id).await.is_some(), "approved, so it is asked");

    told(&rig, &id, SessionFact::Wrote).await;
    assert!(call(&rig, &id).await.is_some(), "a file written while the turn runs takes nothing back");
    told(&rig, &id, SessionFact::TurnCompleted { said: None }).await;
    assert!(call(&rig, &id).await.is_some(), "nor once the turn ends");
    assert!(read(&rig, &id).await.plan.expect("a plan").held_by.is_empty());

    later().await;
    page(&rig, &id, ONE).await;
    assert!(call(&rig, &id).await.is_none(), "shown again, it is to be approved again");
}

#[tokio::test]
async fn an_approve_of_a_take_of_the_window_is_the_walks() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let takes = [("Every state", ONE), ("Needs you first", TWO)]
        .map(|(name, url)| WindowTake { name: name.into(), url: url.into() })
        .into();
    show(&rig, &id, ShowWindow { url: ONE.into(), variations: takes, ..Default::default() }).await;
    record(&rig, &id, Some(ONE)).await;
    assert!(call(&rig, &id).await.is_none(), "shown and not approved");

    rig.send(&id, &format!("Approved B, Needs you first: {TWO}")).await;
    assert!(call(&rig, &id).await.is_some(), "the second take's Approve approves the window");
}

#[tokio::test]
async fn recording_again_without_a_walk_clears_it() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    record(&rig, &id, Some(ONE)).await;
    assert!(call(&rig, &id).await.is_none());

    record(&rig, &id, None).await;
    assert_eq!(call(&rig, &id).await.map(|item| item.id), Some("plan:2".to_string()));
}

fn takes(urls: &[&str]) -> Vec<WindowTake> {
    urls.iter().zip('A'..).map(|(url, letter)| WindowTake { name: letter.to_string(), url: (*url).into() }).collect()
}

async fn window(rig: &Rig, id: &SessionId, urls: &[&str]) {
    later().await;
    show(rig, id, ShowWindow { url: urls[0].into(), variations: takes(urls), ..Default::default() }).await;
}

/// The Job's own file write, then the end of its turn.
async fn fixed(rig: &Rig, id: &SessionId) {
    later().await;
    told(rig, id, SessionFact::Wrote).await;
    told(rig, id, SessionFact::TurnCompleted { said: None }).await;
}

async fn approve(rig: &Rig, id: &SessionId, take: &str, url: &str) {
    later().await;
    let said = if take.is_empty() { format!("Approved: {url}") } else { format!("Approved {take}: {url}") };
    rig.send(id, &said).await;
}

/// Job 43cd07be, 18–19 Oct 2026: the windows it showed, in order, with their takes, the Job's fixes
/// between them and every Approve the owner gave. He approved every window he had in front of him,
/// and the plan never reached him until he approved two dead ones by hand.
#[tokio::test]
async fn a_plan_is_asked_at_his_last_approve_however_many_windows_the_job_showed_before() {
    const FIRST_RUN: &str = "http://localhost:40118/?walk=first-run";
    const NEW: &str = "http://localhost:40118/?walk=first-run-new";
    const CLAUDE: &str = "http://localhost:40118/?walk=first-run-claude";
    const MENU: &str = "http://localhost:40118/?walk=first-run-menu";
    const HANGAR: &str = "http://localhost:40118/?menu=hangar&walk=first-run-menu";
    const REMEMBER: &str = "http://localhost:40118/?walk=remember-where";
    const AT_SCALE: &str = "http://localhost:40118/?walk=runs-at-scale";
    const PICK: &str = "http://localhost:40128/?walk=pick-without-manifest";
    const HEADER: &str = "http://localhost:40118/?take=header&walk=first-run&y=";
    const THREAD: &str = "http://localhost:40118/?take=thread&walk=first-run&y=";
    const TOOLBAR: &str = "http://localhost:40118/?take=toolbar&walk=first-run&y=";
    let rig = rig_placing(None);
    let id = rig.start().await;

    window(&rig, &id, &[MENU, HANGAR]).await;
    approve(&rig, &id, "A, Map: offset battlefield", MENU).await;
    fixed(&rig, &id).await;
    // remember-where: a walk dropped from the repository the same day.
    window(&rig, &id, &[REMEMBER, FIRST_RUN, CLAUDE]).await;
    window(&rig, &id, &[AT_SCALE, REMEMBER, FIRST_RUN]).await;
    fixed(&rig, &id).await;
    window(&rig, &id, &[PICK]).await;
    approve(&rig, &id, "", PICK).await;
    fixed(&rig, &id).await;
    // Header, then the same takes again with Thread first; the Header take was then deleted.
    window(&rig, &id, &[HEADER, THREAD, TOOLBAR]).await;
    window(&rig, &id, &[THREAD, HEADER, TOOLBAR]).await;
    approve(&rig, &id, "A, Thread", THREAD).await;
    later().await;
    record(&rig, &id, None).await;
    fixed(&rig, &id).await;
    assert!(call(&rig, &id).await.is_none());
    assert_eq!(
        read(&rig, &id).await.plan.expect("a plan").held_by,
        [ipc::PlanHold { url: AT_SCALE.into(), why: ipc::PlanHoldWhy::Changed }],
        "Many workspaces was never approved; the replaced Header and remember-where windows, and the \
         approved menu, pick and Thread windows, hold nothing, though the Job wrote after his Approves"
    );

    window(&rig, &id, &[FIRST_RUN, NEW, CLAUDE, MENU, AT_SCALE]).await;
    assert!(call(&rig, &id).await.is_none(), "the window that replaces them waits on his Approve");
    for (take, url) in [("A, Open a project", FIRST_RUN), ("B, New project", NEW), ("C, Import from Claude Code", CLAUDE), ("D, Start menu", MENU)] {
        approve(&rig, &id, take, url).await;
        assert!(call(&rig, &id).await.is_some(), "{take}: an Approve of any take settles the window");
    }
    approve(&rig, &id, "E, Many workspaces", AT_SCALE).await;
    let rows = rig.fleet.rows_of(id.as_str()).await.expect("the thread");
    let last = rows.iter().rev().find_map(|row| match row {
        ipc::SessionRow::Message { at, text, .. } if text.contains(AT_SCALE) => Some(at.clone()),
        _ => None,
    });
    let item = call(&rig, &id).await.expect("every window he has in front of him is approved, so the plan is asked");
    assert_eq!(Some(item.since), last, "from his last Approve");
}

fn walks(record: &SessionRecord) -> Vec<String> {
    record.waiting_for.iter().filter(|one| one.source == WaitingSource::Walk).map(|one| one.id.clone()).collect()
}

#[tokio::test]
async fn closing_the_walks_window_does_not_release_the_plan_and_his_approve_does() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    page(&rig, &id, ONE).await;
    record(&rig, &id, None).await;
    assert!(call(&rig, &id).await.is_none(), "a walk waits on him");
    assert_eq!(walks(&read(&rig, &id).await), [format!("walk:{ONE}")]);

    rig.fleet.window_closed(WindowClosed { session_id: id.clone(), url: ONE.into() }).await.expect("told");
    assert!(call(&rig, &id).await.is_none(), "closing the window is not an Approve");
    assert_eq!(walks(&read(&rig, &id).await), [format!("walk:{ONE}")], "and the walk still waits");

    later().await;
    rig.send(&id, &format!("Approved: {ONE}")).await;
    let item = call(&rig, &id).await.expect("approved, so it is asked");
    assert!(item.since.as_str() > read(&rig, &id).await.plan.expect("a plan").recorded_at.as_str());
    assert!(walks(&read(&rig, &id).await).is_empty());
}

#[tokio::test]
async fn an_older_unapproved_walk_still_waits_after_he_approves_a_newer_one_and_after_a_message_or_a_note() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    page(&rig, &id, ONE).await;
    later().await;
    page(&rig, &id, TWO).await;
    assert_eq!(walks(&read(&rig, &id).await).len(), 2, "both wait");

    rig.send(&id, &format!("Approved: {TWO}")).await;
    assert_eq!(walks(&read(&rig, &id).await), [format!("walk:{ONE}")], "the newer one's Approve leaves the older waiting");

    rig.send(&id, "Which of these is the better one?").await;
    assert_eq!(walks(&read(&rig, &id).await), [format!("walk:{ONE}")], "a message settles nothing");
    rig.send(&id, &format!("The header is off\nOn header, top, at {ONE}")).await;
    told(&rig, &id, SessionFact::TurnCompleted { said: None }).await;
    assert!(walks(&read(&rig, &id).await).is_empty(), "a note settles nothing, and the page it is about is now changed until shown again");
    later().await;
    page(&rig, &id, ONE).await;
    assert_eq!(walks(&read(&rig, &id).await), [format!("walk:{ONE}")], "shown again, it waits");

    later().await;
    page(&rig, &id, TWO).await;
    assert_eq!(walks(&read(&rig, &id).await).len(), 2, "shown again after its Approve, it waits again");

    rig.send(&id, &format!("Approved: {ONE}")).await;
    rig.send(&id, &format!("Approved: {TWO}")).await;
    assert!(walks(&read(&rig, &id).await).is_empty());
}

/// On 20 Oct he asked for a bigger change on a walk instead of approving it, the Job planned that
/// change, and the walk he had moved past held the new plan out of his sight.
#[tokio::test]
async fn a_walk_he_answered_with_changes_does_not_hold_the_plan_recorded_after() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    page(&rig, &id, ONE).await;
    later().await;
    rig.send(&id, "Can we make it a stepper instead?").await;
    later().await;
    record(&rig, &id, None).await;

    let record = read(&rig, &id).await;
    let item = call(&rig, &id).await.expect("the plan answers what he said, so it is asked");
    assert_eq!(item.since.as_str(), record.plan.expect("a plan").recorded_at.as_str());
    assert_eq!(walks(&read(&rig, &id).await), [format!("walk:{ONE}")], "the walk still waits on its own Approve");

    later().await;
    page(&rig, &id, ONE).await;
    assert!(call(&rig, &id).await.is_none(), "shown again after the plan, it holds the plan");
}

/// Job 30236153, 26 Oct 2026: he closed the walk's window unwalked, answered the Job's question on
/// where a run is saved, and the plan recorded two minutes later was put to him ahead of the walk.
#[tokio::test]
async fn his_answer_to_the_jobs_question_is_not_words_on_the_walk() {
    const ASK: &str = r#"{"tool_name":"AskUserQuestion","tool_use_id":"toolu_q","input":{"questions":[
      {"question":"Save where?","header":"Save where","multiSelect":false,
       "options":[{"label":"This Mac","description":""},{"label":"The repository","description":""}]}]}}"#;
    let rig = rig_placing(None);
    let id = rig.start().await;
    page(&rig, &id, ONE).await;
    rig.fleet.window_closed(WindowClosed { session_id: id.clone(), url: ONE.into() }).await.expect("told");
    later().await;

    let fleet = Arc::clone(&rig.fleet);
    let session = id.as_str().to_string();
    let asking: AskingToRun = ipc::decode("an ask", ASK.as_bytes()).expect("an ask");
    let held = tokio::spawn(async move { fleet.session_permission(&session, asking).await });
    let minted = std::sync::Mutex::new(String::new());
    eventually(|| async {
        let asked = read(&rig, &id).await.hosted.and_then(|hosted| hosted.asked);
        asked.map(|asked| *minted.lock().unwrap() = asked.call).is_some()
    })
    .await;
    let minted = minted.lock().unwrap().clone();
    Arc::clone(&rig.fleet)
        .answer_session_ask(AnswerSessionAsk {
            session_id: id.clone(),
            call: minted,
            answer: HelmCallAnswer::AllowOnce,
            note: None,
            answers: vec![QuestionAnswer { question: "Save where?".into(), chosen: vec!["This Mac".into()] }],
            folder: None,
            kind: None,
        })
        .await
        .expect("answered");
    held.await.unwrap().expect("decided");
    later().await;
    record(&rig, &id, None).await;

    assert!(call(&rig, &id).await.is_none(), "the walk he never approved holds the plan");
    assert_eq!(
        read(&rig, &id).await.plan.expect("a plan").held_by,
        [ipc::PlanHold { url: ONE.into(), why: ipc::PlanHoldWhy::Unapproved }]
    );
}

#[tokio::test]
async fn a_page_changed_between_fixes_raises_no_walk_call_until_show_window_shows_it_again() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    page(&rig, &id, ONE).await;
    assert_eq!(walks(&read(&rig, &id).await), [format!("walk:{ONE}")], "shown, so it waits on him");

    told(&rig, &id, SessionFact::Wrote).await;
    assert!(walks(&read(&rig, &id).await).is_empty(), "held while the Job changes it");
    later().await;
    told(&rig, &id, SessionFact::TurnCompleted { said: None }).await;
    assert!(walks(&read(&rig, &id).await).is_empty(), "the Job paused between fixes: it is ready, but changed and not shown again");
    told(&rig, &id, SessionFact::Wrote).await;
    told(&rig, &id, SessionFact::TurnCompleted { said: None }).await;
    assert!(walks(&read(&rig, &id).await).is_empty(), "and again after the next fix");

    later().await;
    page(&rig, &id, ONE).await;
    let record = read(&rig, &id).await;
    let shown = record.attachments.iter().find(|one| one.kind == "artifact").expect("the window").changed_at.clone();
    let [walk] = &record.waiting_for[..] else { panic!("one walk waits: {:?}", record.waiting_for) };
    assert_eq!((walk.source, walk.since.as_str()), (WaitingSource::Walk, shown.as_str()), "it waits again from the moment it is shown");
}

/// A background subagent the Job started on 18 Oct finished within the hour by its own transcript,
/// and stood on the ledger for a day, so no turn end made a window ready again.
#[tokio::test]
async fn a_subagent_whose_transcript_ended_leaves_no_window_changing() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let subagent = ipc::AttachmentReport { kind: "subagent".into(), target: "x1".into(), detail: Default::default() };
    told(&rig, &id, SessionFact::Attached { attachment: subagent }).await;
    page(&rig, &id, ONE).await;
    let working = |record: &SessionRecord| {
        record.attachments.iter().any(|one| one.kind == "artifact" && one.detail.get("working").is_some_and(|it| it == "true"))
    };

    later().await;
    fixed(&rig, &id).await;
    assert!(working(&read(&rig, &id).await), "a subagent still at work keeps the page changing");

    let dir = std::path::Path::new(&rig.fleet.host().home).join(".claude/projects/-work").join(id.as_str()).join("subagents");
    std::fs::create_dir_all(&dir).unwrap();
    let ended = r#"{"type":"assistant","uuid":"s1","isSidechain":true,"timestamp":"2026-10-18T11:38:00.000Z","message":{"role":"x","stop_reason":"end_turn","content":[{"type":"text","text":"Done."}]}}"#;
    std::fs::write(dir.join("agent-x1.jsonl"), format!("{ended}\n")).unwrap();
    fixed(&rig, &id).await;
    assert!(!working(&read(&rig, &id).await), "its transcript ended, so the turn end makes the page ready");
}

async fn withdraw(rig: &Rig, id: &SessionId, url: &str) -> Result<SessionRecord, api::Refusal> {
    rig.fleet.withdraw_window(None, WithdrawWindow { url: url.into(), session_id: Some(id.clone()) }).await
}

#[tokio::test]
async fn a_walk_the_job_withdraws_holds_nothing_until_it_is_shown_again() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    page(&rig, &id, ONE).await;
    record(&rig, &id, None).await;
    assert_eq!(walks(&read(&rig, &id).await), [format!("walk:{ONE}")]);
    assert!(call(&rig, &id).await.is_none(), "the walk he has not approved holds the plan");

    let record = withdraw(&rig, &id, ONE).await.expect("withdrawn");
    assert!(walks(&record).is_empty(), "the walk left his Needs you list");
    assert!(call(&rig, &id).await.is_some(), "and the plan is put to him");
    assert!(rig.fleet.walk_open(id.as_str()).await.is_none(), "a question of the job's is no longer refused for a walk");

    later().await;
    page(&rig, &id, ONE).await;
    assert_eq!(walks(&read(&rig, &id).await), [format!("walk:{ONE}")], "shown again, it waits again");
    assert!(call(&rig, &id).await.is_none(), "and holds the plan again");
}

#[tokio::test]
async fn a_job_can_only_withdraw_a_walk_it_showed() {
    let rig = rig_placing(None);
    let first = rig.start().await;
    let second = rig.start().await;
    page(&rig, &first, ONE).await;

    let never = withdraw(&rig, &first, TWO).await;
    assert!(matches!(never, Err(api::Refusal::Unacceptable(_))), "a page it never showed: {never:?}");
    let another = withdraw(&rig, &second, ONE).await;
    assert!(matches!(another, Err(api::Refusal::Unacceptable(_))), "another job's window: {another:?}");
    assert_eq!(walks(&read(&rig, &first).await), [format!("walk:{ONE}")], "the first job's walk still waits");
}

/// **A plan he put away holds nothing**, whatever the Job shows after. Recording again is a new
/// version that starts without the mark, and waits as any first recording does.
#[tokio::test]
async fn a_plan_set_aside_holds_nothing_and_the_next_version_waits_again() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    record(&rig, &id, None).await;
    assert!(call(&rig, &id).await.is_some(), "asked at its recording");
    Arc::clone(&rig.fleet)
        .dismiss_waiting(DismissWaiting { session_id: id.clone(), item_id: "plan:1".into(), reason: None, misread: false })
        .await
        .unwrap();

    later().await;
    page(&rig, &id, ONE).await;
    let aside = read(&rig, &id).await.plan.expect("a plan");
    assert!(aside.set_aside_at.is_some());
    assert!(aside.held_by.is_empty(), "a window he has not approved holds nothing of a plan put away: {:?}", aside.held_by);
    assert!(call(&rig, &id).await.is_none());

    later().await;
    record(&rig, &id, None).await;
    let next = read(&rig, &id).await.plan.expect("a plan");
    assert_eq!(next.version, 2);
    assert!(next.set_aside_at.is_none(), "a new version starts without the mark");
    assert!(next.held_by.iter().any(|one| one.url == ONE), "it waits on the window as a first recording does");
    later().await;
    rig.send(&id, &format!("Approved: {ONE}")).await;
    assert_eq!(call(&rig, &id).await.expect("asked again").id, "plan:2");
}
