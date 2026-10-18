//! **A question asked while the owner walks a page is refused, not held.** An open question keeps
//! the agent's turn open, and his notes and his Approve from the window queue behind it until he
//! answers. So while a walk is open the question is refused to the agent, which is told to end its
//! turn, and kept whole in the thread for the person to answer after. `docs/concepts/session.md`, *Showing a window*.

use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Next, Sessions};
use ipc::{
    Event, RunOrNot, SessionFact, SessionId, SessionReport, SessionRow, SessionVoice, ShowWindow, WindowClosed,
    WindowFailed, WindowTake,
};

use super::session_host::{rig_placing, Rig};

const QUESTION: &str = r#"{"tool_name":"AskUserQuestion","tool_use_id":"t1","input":{"questions":[
  {"question":"Which clock?","header":"Clock","multiSelect":false,
   "options":[{"label":"Fake","description":""},{"label":"Frozen","description":""}]}]}}"#;

const ONE: &str = "http://localhost:5173/one";
const TWO: &str = "http://localhost:5173/two";

async fn show(rig: &Rig, id: &SessionId, url: &str) {
    rig.fleet
        .show_window(None, ShowWindow { url: url.into(), title: None, session_id: Some(id.clone()), variations: Vec::new(), ..Default::default() })
        .await
        .expect("shown");
}

async fn closed(rig: &Rig, id: &SessionId, url: &str) {
    rig.fleet
        .window_closed(WindowClosed { session_id: id.clone(), url: url.into() })
        .await
        .expect("told");
}

async fn walking(rig: &Rig, id: &SessionId) -> Option<String> {
    rig.fleet.walk_open(id.as_str()).await
}

#[tokio::test]
async fn a_question_while_a_walk_is_open_is_refused_with_what_to_do_instead_and_notes_still_reach_the_agent() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    show(&rig, &id, ONE).await;

    let asking = ipc::decode("an ask", QUESTION.as_bytes()).unwrap();
    let RunOrNot::Deny { message } = rig.fleet.session_permission(id.as_str(), asking).await.unwrap() else {
        panic!("a question during a walk is not held for the person");
    };
    assert!(message.contains("Needs you"), "{message}");
    assert!(message.contains("Do not add it to waiting_for"), "{message}");
    assert!(message.contains("End your turn"), "{message}");
    assert!(message.contains("ask_in_window"), "a question about the page has its own way in: {message}");
    let now = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap();
    assert!(now.session.hosted.and_then(|hosted| hosted.asked).is_none(), "no card was put up");
    // The question stays whole, settled so the lapsed reading takes it up: every option and its
    // description, not a bare line of text for a reply box.
    let kept: Vec<_> = now
        .rows
        .iter()
        .filter_map(|row| match row {
            SessionRow::Ask { ask, state, .. } => Some((ask, state)),
            _ => None,
        })
        .collect();
    let [(ask, state)] = kept.as_slice() else { panic!("one ask row keeps the question: {:?}", now.rows) };
    assert_eq!(**state, ipc::SessionAskState::Kept);
    let questions = &ask.questions;
    assert_eq!(questions.len(), 1);
    assert_eq!(questions[0].question, "Which clock?");
    assert_eq!(questions[0].header, "Clock");
    let labels: Vec<&str> = questions[0].options.iter().map(|one| one.label.as_str()).collect();
    assert_eq!(labels, ["Fake", "Frozen"]);
    assert!(rig.fleet.hosts().asks().waiting().is_empty(), "and nothing waits on an answer");

    rig.send(&id, "The header is off\nOn header, top, at http://localhost:5173/one").await;
    assert_eq!(walking(&rig, &id).await.as_deref(), Some(ONE), "a note is not the walk's end");
}

#[tokio::test]
async fn a_walk_is_open_until_it_is_approved_its_window_is_closed_or_a_newer_one_replaces_it() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    assert_eq!(walking(&rig, &id).await, None, "nothing shown, nothing open");

    show(&rig, &id, ONE).await;
    assert_eq!(walking(&rig, &id).await.as_deref(), Some(ONE));
    closed(&rig, &id, ONE).await;
    assert_eq!(walking(&rig, &id).await, None, "the person closed it");

    show(&rig, &id, ONE).await;
    assert_eq!(walking(&rig, &id).await.as_deref(), Some(ONE), "shown again, open again");
    show(&rig, &id, TWO).await;
    // Bridge closes the window it replaces, and says so after the newer one is shown.
    closed(&rig, &id, ONE).await;
    assert_eq!(walking(&rig, &id).await.as_deref(), Some(TWO), "the newer window stands");

    rig.send(&id, &format!("Approved: {TWO}")).await;
    assert_eq!(walking(&rig, &id).await, None, "approved");

    let rows = rig.rows(&id).await;
    let ended = rows
        .iter()
        .filter(|row| matches!(row, SessionRow::Window { ended_at: Some(_), .. }))
        .count();
    assert_eq!(ended, 3, "each walk that ended says so on its row: {rows:?}");
}

fn takes() -> Vec<WindowTake> {
    [("Every state", ONE), ("Needs you first", TWO)]
        .map(|(name, url)| WindowTake { name: name.into(), url: url.into() })
        .into()
}

#[tokio::test]
async fn takes_reach_the_row_bridge_opens_and_the_ledger_a_reopen_reads_and_a_take_that_is_no_page_shows_nothing() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let mut watching = rig.fleet.events().subscribe();
    let shown = ShowWindow { url: ONE.into(), title: None, session_id: Some(id.clone()), variations: takes(), ..Default::default() };
    let record = rig.fleet.show_window(None, shown).await.expect("shown");

    let kept = record
        .attachments
        .iter()
        .find(|one| one.detail.get("form").map(String::as_str) == Some("window"))
        .expect("the window is on the ledger");
    let on_ledger = kept.detail.get("variations").expect("and its takes with it");
    assert!(on_ledger.contains("Needs you first") && on_ledger.contains(TWO), "{on_ledger}");

    let arrived = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match watching.next().await {
                Some(Next::Send(delivered)) => {
                    if let Event::SessionRow(ipc::SessionRowChanged { row: SessionRow::Window { variations, .. }, .. }) = delivered.event {
                        return variations;
                    }
                }
                other => panic!("the stream ended or dropped: {other:?}"),
            }
        }
    })
    .await
    .expect("the window row arrives");
    assert_eq!(arrived, takes(), "Bridge opens the window on the row, with the takes it carries");

    let refused = |url: &str, variations: Vec<WindowTake>| ShowWindow {
        url: url.into(),
        title: None,
        session_id: Some(id.clone()),
        variations,
        ..Default::default()
    };
    let bad = |name: &str, url: &str| WindowTake { name: name.into(), url: url.into() };
    for (url, variations) in [
        (ONE, vec![bad("A", ONE), bad("B", "file:///etc/passwd")]),
        (ONE, vec![bad("A", ONE), bad("B", "javascript:alert(1)")]),
        (ONE, vec![bad("A", ONE), bad("  ", TWO)]),
        (TWO, vec![bad("A", ONE), bad("B", TWO)]),
    ] {
        assert!(rig.fleet.show_window(None, refused(url, variations.clone())).await.is_err(), "{variations:?}");
    }
    let windows = rig.rows(&id).await.into_iter().filter(|row| matches!(row, SessionRow::Window { .. })).count();
    assert_eq!(windows, 1, "a refused call writes no row");
}

#[tokio::test]
async fn approving_any_take_ends_the_walk_and_a_sentence_that_only_begins_with_the_word_does_not() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let shown = ShowWindow { url: ONE.into(), title: None, session_id: Some(id.clone()), variations: takes(), ..Default::default() };
    rig.fleet.show_window(None, shown).await.expect("shown");

    let waiting = || async { Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session.waiting_for.len() };
    assert_eq!(waiting().await, 1, "the walk waits on him");
    rig.send(&id, &format!("Approved the second one: {TWO}")).await;
    assert_eq!(walking(&rig, &id).await.as_deref(), Some(ONE), "his own words are not an Approve");
    rig.send(&id, &format!("Approved B, Needs you first: {TWO}")).await;
    assert_eq!(walking(&rig, &id).await, None, "the take on screen was approved");
    assert_eq!(waiting().await, 0, "and the walk no longer waits on him");
}

#[tokio::test]
async fn a_question_may_be_about_a_take_on_another_origin_than_the_first() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let other = "http://localhost:6006/two";
    let takes = vec![
        WindowTake { name: "A".into(), url: ONE.into() },
        WindowTake { name: "B".into(), url: other.into() },
    ];
    let shown = ShowWindow { url: ONE.into(), title: None, session_id: Some(id.clone()), variations: takes, ..Default::default() };
    rig.fleet.show_window(None, shown).await.expect("shown");
    let ask = |url: &str| ipc::AskInWindow {
        question: "Beside or under?".into(),
        options: Vec::new(),
        url: url.into(),
        selector: "main > h1".into(),
        text: None,
        session_id: Some(id.clone()),
    };
    rig.fleet.ask_in_window(None, ask(other)).await.expect("on a take's origin");
    assert!(rig.fleet.ask_in_window(None, ask("http://localhost:7000/")).await.is_err(), "on no take's origin");
    let window = rig.rows(&id).await.into_iter().find_map(|row| match row {
        SessionRow::WindowAsk { window, .. } => Some(window),
        _ => None,
    });
    assert_eq!(window.as_deref(), Some(ONE), "the window it is asked in is the one the Session showed");
}

#[tokio::test]
async fn a_window_is_working_from_the_sessions_own_write_or_a_note_and_ready_when_it_shows_again_or_its_turn_ends() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let working = || async {
        let kept = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session.attachments;
        let window = kept.iter().find(|one| one.detail.get("form").map(String::as_str) == Some("window")).expect("shown");
        window.detail.get("working").map(String::as_str) == Some("true")
    };
    let told = |fact| async {
        let report = SessionReport { harness: adapters::HOSTED_HARNESS.into(), session_id: id.clone(), fact };
        rig.fleet.report_session(report).await.expect("kept");
    };
    show(&rig, &id, ONE).await;
    assert!(!working().await, "showing a page says it is ready");

    told(SessionFact::Wrote).await;
    assert!(working().await, "the session wrote a file: it is changing the page");
    told(SessionFact::TurnCompleted { said: None }).await;
    assert!(!working().await, "its turn ended");

    told(SessionFact::Wrote).await;
    show(&rig, &id, ONE).await;
    assert!(!working().await, "it showed the page again mid-turn");

    rig.send(&id, "Too slow\nOn header, top, at http://localhost:5173/one").await;
    assert!(working().await, "a note taken in the window: it will change the page");
    rig.send(&id, "Looks fine").await;
    assert!(working().await, "a message that is no note changes nothing");
}

/// The session hands a note to a subagent and ends its turn: the page is still changing, so the
/// window stays working until a turn ends with none standing. A server left running holds nothing.
#[tokio::test]
async fn a_window_stays_working_through_a_turn_that_ends_with_a_subagent_standing() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let working = || async {
        let kept = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session.attachments;
        let window = kept.iter().find(|one| one.detail.get("form").map(String::as_str) == Some("window")).expect("shown");
        window.detail.get("working").map(String::as_str) == Some("true")
    };
    let told = |fact| async {
        let report = SessionReport { harness: adapters::HOSTED_HARNESS.into(), session_id: id.clone(), fact };
        rig.fleet.report_session(report).await.expect("kept");
    };
    let attached = |kind: &str, target: &str| SessionFact::Attached {
        attachment: ipc::AttachmentReport { kind: kind.into(), target: target.into(), detail: Default::default() },
    };
    let settled = |kind: &str, target: &str| SessionFact::Settled {
        attachment: ipc::AttachmentNamed { kind: kind.into(), target: target.into() },
        state: ipc::AttachmentState::Spent,
    };
    show(&rig, &id, ONE).await;

    rig.send(&id, "Too slow\nOn header, top, at http://localhost:5173/one").await;
    told(attached("subagent", "a1")).await;
    told(SessionFact::TurnCompleted { said: None }).await;
    assert!(working().await, "its turn ended, but the subagent is still changing the page");

    told(settled("subagent", "a1")).await;
    assert!(working().await, "the subagent stopping alone does not say the page is done");
    told(SessionFact::TurnCompleted { said: None }).await;
    assert!(!working().await, "the turn that took its report ended with none standing");

    rig.send(&id, "Still off\nOn header, top, at http://localhost:5173/one").await;
    told(attached("server", "5173")).await;
    told(SessionFact::TurnCompleted { said: None }).await;
    assert!(!working().await, "a dev server left running is no work on the page");

    rig.send(&id, "Again\nOn header, top, at http://localhost:5173/one").await;
    told(attached("subagent", "a2")).await;
    show(&rig, &id, ONE).await;
    assert!(!working().await, "showing the page again says it is ready, subagent or not");
}

#[tokio::test]
async fn a_window_that_could_not_show_its_page_tells_the_session_in_fleets_voice_and_the_walk_stays() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    show(&rig, &id, ONE).await;

    let failed = WindowFailed { session_id: id.clone(), url: ONE.into(), why: "ERR_CONNECTION_REFUSED".into() };
    let told = Arc::clone(&rig.fleet).window_failed(failed).await.expect("told");
    assert!(told.told);
    let rows = rig.rows(&id).await;
    let said = rows.iter().find_map(|row| match row {
        SessionRow::Message { from: SessionVoice::Session { title, .. }, text, .. } if title == "Fleet" => Some(text.clone()),
        _ => None,
    });
    let said = said.expect("Fleet said it, not the person");
    assert!(said.contains(ONE) && said.contains("ERR_CONNECTION_REFUSED"), "{said}");
    assert!(said.contains("show_window"), "{said}");
    assert_eq!(walking(&rig, &id).await.as_deref(), Some(ONE), "the window is still on his screen");

    let unknown = WindowFailed { session_id: SessionId::carried("nobody"), url: ONE.into(), why: "HTTP 500".into() };
    assert!(Arc::clone(&rig.fleet).window_failed(unknown).await.is_err());
}
