//! **A question asked while the owner walks a page is refused, not held.** An open question keeps
//! the agent's turn open, and his notes and his Approve from the window queue behind it until he
//! answers. So while a walk is open the agent is told to put the question under `waiting_for` and
//! end its turn. `docs/concepts/session.md`, *Showing a window*.

use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Next, Sessions};
use ipc::{Event, RunOrNot, SessionId, SessionRow, ShowWindow, WindowClosed, WindowTake};

use super::session_host::{rig_placing, Rig};

const QUESTION: &str = r#"{"tool_name":"AskUserQuestion","tool_use_id":"t1","input":{"questions":[
  {"question":"Which clock?","header":"Clock","multiSelect":false,
   "options":[{"label":"Fake","description":""},{"label":"Frozen","description":""}]}]}}"#;

const ONE: &str = "http://localhost:5173/one";
const TWO: &str = "http://localhost:5173/two";

async fn show(rig: &Rig, id: &SessionId, url: &str) {
    rig.fleet
        .show_window(None, ShowWindow { url: url.into(), title: None, session_id: Some(id.clone()), variations: Vec::new() })
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
    assert!(message.contains("waiting_for"), "{message}");
    assert!(message.contains("end your turn"), "{message}");
    assert!(message.contains("ask_in_window"), "a question about the page has its own way in: {message}");
    let now = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap();
    assert!(now.session.hosted.and_then(|hosted| hosted.asked).is_none(), "no card was put up");
    assert!(!now.rows.iter().any(|row| matches!(row, SessionRow::Ask { .. })), "and no ask row");

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
    let shown = ShowWindow { url: ONE.into(), title: None, session_id: Some(id.clone()), variations: takes() };
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
    let shown = ShowWindow { url: ONE.into(), title: None, session_id: Some(id.clone()), variations: takes() };
    rig.fleet.show_window(None, shown).await.expect("shown");

    let waiting = || async { Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session.waiting_for.len() };
    assert_eq!(waiting().await, 1, "the walk waits on him");
    rig.send(&id, &format!("Approved the second one: {TWO}")).await;
    assert_eq!(walking(&rig, &id).await.as_deref(), Some(ONE), "his own words are not an Approve");
    rig.send(&id, &format!("Approved B, Needs you first: {TWO}")).await;
    assert_eq!(walking(&rig, &id).await, None, "the take on screen was approved");
    assert_eq!(waiting().await, 0, "and the walk no longer waits on him");
}
