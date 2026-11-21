//! The claim, written before the code: **his comment on a passage of a Job's page opens a margin thread, sent now
//! or held and then sent with the others in one message. The agent answers in the thread, and an answer changes
//! nothing on the page. Make this change asks for an edit; the agent's suggested edit waits on his Accept, and only
//! Accept writes the page, as its next revision. A later revision without the passage keeps the thread's quote and
//! marks it gone, and Settle closes the thread and asks the agent to add the outcome to Decisions. The Job's thread
//! carries one line per round of margin talk, just after the message the round sent, which stays.**
//! `crates/ipc/operations/comment_margin.toml` and its neighbours, `docs/concepts/document.md`, *Margin threads*.

use std::sync::Arc;

use adapter_traits::{CallDetail, DroneEvent};
use api::{HostedSessions, Margins, Refusal};
use ipc::{
    ActOnMargin, AnswerThread, CommentMargin, GetMarginThreads, MarginAct, MarginAnchor, MarginOn, MarginRoundKind, MarginState, MarginThread,
    MarginThreads, SendHeldMargin, SessionId, SessionRow, SuggestEdit, ThreadItem,
};

use super::document_answers::{record_of, revision_kept, sent, the_document};
use super::session_host::{rig_placing, Rig};
use super::tmp::TempDir;
use crate::session_host::page_words::{edit_applied, passage_in};

const TITLE: &str = "Quiet hours for notices";
const WAIT: &str = "Notices wait until eight in the morning.";
const WHO: &str = "Applies to every team on the rota.";

/// The page, with the passage on who it applies to where `who` is given.
fn page(wait: &str, who: Option<&str>) -> String {
    let who = who.map(|who| format!("<h2>Who it applies to</h2><p>{who}</p>")).unwrap_or_default();
    format!(
        "<html><head><title>{TITLE}</title><meta name=\"armada-review\" content=\"approve\"></head><body><h1>{TITLE}</h1>\
         <h2>When notices wait</h2><p>{wait}</p>{who}<h2>Decisions</h2><p>None yet.</p></body></html>"
    )
}

/// The page written by the Job's Write call, which the tool answered.
fn written(rig: &Rig, file: &std::path::Path, html: &str, call: &str) {
    std::fs::write(file, html).unwrap();
    rig.stand_in.says(
        0,
        vec![
            DroneEvent::Called { tool: "Write".into(), call: call.into(), detail: CallDetail::of(&format!("{} +9", file.display())) },
            DroneEvent::Answered { call: call.into(), failed: false },
        ],
    );
}

/// How many lines Fleet sent the Job's process that hold `containing`.
fn told(rig: &Rig, id: &SessionId, containing: &str) -> usize {
    rig.stand_in.process_of(id.as_str()).map_or(0, |process| process.sent.lock().unwrap().iter().filter(|line| line.contains(containing)).count())
}

fn on_passage(id: &SessionId, quote: &str, text: &str, hold: bool) -> CommentMargin {
    CommentMargin {
        session_id: id.clone(),
        document: "quiet-hours.html".into(),
        on: MarginOn::Passage { anchor: MarginAnchor { quote: quote.into(), block: None, section: None } },
        text: text.into(),
        hold,
    }
}

fn act(id: &SessionId, thread: &str, act: MarginAct) -> ActOnMargin {
    ActOnMargin { session_id: id.clone(), thread: thread.into(), act }
}

fn thread<'a>(threads: &'a MarginThreads, id: &str) -> &'a MarginThread {
    threads.threads.iter().find(|one| one.id == id).expect("the thread")
}

async fn threads(rig: &Rig, id: &SessionId) -> MarginThreads {
    rig.fleet.get_margin_threads(GetMarginThreads { session_id: id.clone(), document: "quiet-hours.html".into() }).await.unwrap()
}

fn code(refused: Result<impl std::fmt::Debug, Refusal>) -> String {
    match refused {
        Err(Refusal::Unacceptable(why)) => why.code,
        other => panic!("refused as unacceptable: {other:?}"),
    }
}

/// A Job that wrote the page once, kept as revision 1: its id, the checkout, the page's file there and Fleet's copy.
async fn quiet_hours(rig: &Rig) -> (SessionId, TempDir, std::path::PathBuf, String) {
    let id = rig.start().await;
    rig.send(&id, "write up quiet hours").await;
    rig.stand_in.init(0);
    let work = TempDir::new();
    let file = work.path().join(".armada/documents/quiet-hours.html");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    let path = rig.fleet.page_home(id.as_str(), &file.to_string_lossy());
    written(rig, &file, &page(WAIT, Some(WHO)), "toolu_1");
    revision_kept(rig, &id, &path, "1").await;
    (id, work, file, path)
}

#[tokio::test]
async fn margin_threads_carry_his_comments_the_agents_answers_and_only_his_accept_writes_the_page() {
    let rig = rig_placing(None);
    let (id, _work, file, path) = quiet_hours(&rig).await;
    let fleet = || Arc::clone(&rig.fleet);
    let before = sent(&rig.rows(&id).await).len();

    // Comment held: two threads, nothing sent.
    let held = fleet().comment_margin(on_passage(&id, WAIT, "Eight is late for the night shift.", true)).await.unwrap();
    let m1 = thread(&held, "m1");
    assert_eq!((m1.state, m1.anchor.section.as_deref(), m1.gone), (MarginState::Held, Some("When notices wait"), false));
    let held = fleet().comment_margin(on_passage(&id, WHO, "Contractors too?", true)).await.unwrap();
    assert_eq!(thread(&held, "m2").state, MarginState::Held);
    assert_eq!(sent(&rig.rows(&id).await).len(), before, "a held comment sends nothing");
    assert_eq!(told(&rig, &id, "His margin comments"), 0);
    assert_eq!(code(fleet().comment_margin(on_passage(&id, WAIT, " ", true)).await), "fleet.margin_empty");

    // Send held: one message naming each passage, his words and no path; the agent reads where and how to answer.
    let after = fleet().send_held_margin(SendHeldMargin { session_id: id.clone(), document: "quiet-hours.html".into() }).await.unwrap();
    assert!(after.threads.iter().all(|one| one.state == MarginState::Open && one.messages.iter().all(|said| !said.held)));
    let rows = sent(&rig.rows(&id).await);
    assert_eq!(rows.len(), before + 1, "one message for both: {rows:?}");
    assert_eq!(
        rows.last().unwrap(),
        &format!("Comments on {TITLE}\nOn \"{WAIT}\": Eight is late for the night shift.\nOn \"{WHO}\": Contractors too?")
    );
    assert_eq!(told(&rig, &id, "the page at .armada/documents/quiet-hours.html"), 1);
    assert_eq!(told(&rig, &id, "Thread m1, under \"When notices wait\""), 1);
    assert_eq!(told(&rig, &id, "Thread m2, under \"Who it applies to\""), 1);
    assert_eq!(told(&rig, &id, "answer_thread"), 1);
    assert_eq!(
        code(fleet().send_held_margin(SendHeldMargin { session_id: id.clone(), document: "quiet-hours.html".into() }).await),
        "fleet.margin_nothing_held"
    );

    // The agent answers: the thread holds it, and the page is as it was.
    let answer = AnswerThread { session_id: Some(id.clone()), thread: "m1".into(), text: "Nine would miss the early standup.".into() };
    let answered = fleet().answer_thread(None, answer).await.unwrap();
    assert_eq!((answered.state, answered.messages.len()), (MarginState::Answered, 2));
    assert_eq!(the_document(&record_of(&rig, &id).await, &path).unwrap().detail["revision"], "1", "an answer writes no page");

    // Make this change: the agent is asked for an edit and told not to write the page.
    fleet().act_on_margin_thread(act(&id, "m1", MarginAct::MakeChange)).await.unwrap();
    assert_eq!(told(&rig, &id, "Call suggest_edit with thread m1"), 1);
    assert_eq!(sent(&rig.rows(&id).await).last().unwrap(), &format!("Make this change on \"{WAIT}\" in {TITLE}"));

    // The agent suggests: it must fit the page, and the page is not written.
    let unfit = SuggestEdit { session_id: Some(id.clone()), thread: "m1".into(), struck: "nine".into(), added: "ten".into() };
    assert_eq!(code(fleet().suggest_edit(None, unfit).await), "fleet.margin_edit_unfit");
    let edit = SuggestEdit { session_id: Some(id.clone()), thread: "m1".into(), struck: "eight".into(), added: "eleven".into() };
    let suggested = fleet().suggest_edit(None, edit).await.unwrap();
    assert_eq!(suggested.state, MarginState::Suggested);
    assert_eq!(the_document(&record_of(&rig, &id).await, &path).unwrap().detail["revision"], "1", "a suggestion writes no page");

    // Accept writes revision 2 with the edit, in the checkout too, and tells the agent which revision holds it.
    let accepted = fleet().act_on_margin_thread(act(&id, "m1", MarginAct::Accept)).await.unwrap();
    revision_kept(&rig, &id, &path, "2").await;
    let m1 = thread(&accepted, "m1");
    assert_eq!((m1.anchor.quote.as_str(), m1.gone), ("Notices wait until eleven in the morning.", false));
    assert_eq!(m1.suggestion.as_ref().and_then(|one| one.decided.as_ref()).and_then(|one| one.revision), Some(2));
    assert!(std::fs::read_to_string(&file).unwrap().contains("Notices wait until eleven in the morning."));
    assert_eq!(told(&rig, &id, "Revision 2 holds it"), 1);
    assert_eq!(code(fleet().act_on_margin_thread(act(&id, "m1", MarginAct::Accept)).await), "fleet.margin_no_suggestion");

    // A later revision without the passage on who it applies to: m2 keeps its quote and is gone.
    written(&rig, &file, &page("Notices wait until eleven in the morning.", None), "toolu_2");
    revision_kept(&rig, &id, &path, "3").await;
    let now = threads(&rig, &id).await;
    let m2 = thread(&now, "m2");
    assert_eq!((m2.anchor.quote.as_str(), m2.gone), (WHO, true));
    assert!(!thread(&now, "m1").gone);

    // Settle: the thread closes and the agent is asked to add the outcome to Decisions.
    let settle = MarginAct::Settle { outcome: Some("Notices wait until eleven.".into()) };
    let settled = fleet().act_on_margin_thread(act(&id, "m1", settle)).await.unwrap();
    assert_eq!(thread(&settled, "m1").state, MarginState::Settled);
    assert_eq!(told(&rig, &id, "Add it to the page's Decisions section and write the page again."), 1);
    let reply = CommentMargin { on: MarginOn::Thread { thread: "m1".into() }, ..on_passage(&id, WAIT, "One more thing.", false) };
    assert_eq!(code(fleet().comment_margin(reply).await), "fleet.margin_settled");

    // The Job's thread draws one line per round, in place of each message the round sent.
    let read = ipc::JobThreadRead { session_id: id.clone(), before: None, limit: None, upto: None };
    let items = fleet().get_job_thread(read).await.unwrap().items;
    let cards: Vec<(MarginRoundKind, &str)> = items
        .iter()
        .filter_map(|item| match item {
            ThreadItem::Margin(card) => Some((card.kind, card.summary.as_str())),
            _ => None,
        })
        .collect();
    assert_eq!(
        cards,
        [
            (MarginRoundKind::Comments, "2 comments, 1 answered, 1 passage gone from the page"),
            (MarginRoundKind::Change, "Change asked, accepted"),
            (MarginRoundKind::Accepted, "Change accepted, revision 2"),
            (MarginRoundKind::Settled, "Settled: Notices wait until eleven."),
        ]
    );
    // Each card stands just after the message row it sent, which stays in the thread for a screen that draws no card.
    for (at, item) in items.iter().enumerate() {
        let ThreadItem::Margin(card) = item else { continue };
        let replaces = card.replaces.as_deref().unwrap_or_else(|| panic!("round {} found its row", card.id));
        assert!(
            matches!(items.get(at.wrapping_sub(1)), Some(ThreadItem::Row { row: SessionRow::Message { id, .. } }) if id == replaces),
            "{} stands after {replaces}",
            card.id
        );
    }
}

#[test]
fn a_passage_is_found_under_its_section_and_an_edit_lands_only_in_the_block_that_holds_it() {
    let html = page(WAIT, Some(WHO));
    assert_eq!(passage_in(&html, WAIT), Some(Some("When notices wait".to_string())));
    assert_eq!(passage_in(&html, "Notices  wait\nuntil eight"), Some(Some("When notices wait".to_string())), "whitespace is one space");
    assert_eq!(passage_in(&html, "Notices wait until noon"), None);

    let edited = edit_applied(&html, WHO, "every team", "every team and contractor").unwrap();
    assert!(edited.contains("Applies to every team and contractor on the rota."), "{edited}");
    assert!(edited.contains(WAIT), "only the run asked for changes");
    assert!(edit_applied(&html, WAIT, "", "x").is_err(), "an empty struck is refused");
    assert!(edit_applied(&html, WAIT, "midnight", "noon").is_err(), "struck must stand on the page");
    let amp = page("Fish &amp; chips wait.", None);
    assert!(edit_applied(&amp, "Fish & chips wait.", "Fish & chips", "Tea & toast").unwrap().contains("Tea &amp; toast wait."));
}
