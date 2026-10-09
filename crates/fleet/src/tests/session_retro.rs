//! A Session's retro, end to end against a stand-in process and a fake model:
//! written on a press from what the Session kept, several to a Session, read
//! back with its items, answered with Agree and Disagree, and listed beside a
//! Job's. `docs/concepts/retro.md`, *A Session's retro*.

use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Retros};
use core_model::{LessonState, Whose};
use ipc::{ManifestId, RetroState, SendSessionMessage, SessionId, StartSession};
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::session_host::{Heard, Process, Processes, Sink, Start};
use crate::tests::tmp::TempDir;

type Hosted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

struct Quiet;

impl Process for Quiet {
    fn pid(&self) -> Option<u32> {
        Some(4242)
    }
    fn send(&self, _line: String) {}
    fn end(&self) {}
}

struct Stand;

impl Processes for Stand {
    fn start(&self, _start: &Start, sink: Sink) -> Result<Arc<dyn Process>, String> {
        let _ = sink.send(Heard::Attached { busy: false });
        Ok(Arc::new(Quiet))
    }
}

const TWO_ITEMS: &str = r#"{"items":[
  {"who":"owner","lands_in":"kit","title":"Walks opened in the browser","what":"The agent ran open on a walk.","fix":"Tell the agent to use show_window.","evidence":["correction:1"]},
  {"who":"agent","lands_in":"kit","title":"A reset waited a long time","what":"The agent sat on an ask.","fix":"Tell the owner when an ask has waited.","evidence":["correction:1","correction:9"]},
  {"who":"agent","lands_in":"kit","title":"Cites nothing real","what":"It cites a row that is not there.","fix":"Drop it.","evidence":["correction:9"]}
]}"#;

async fn rig(judge: FakeJudge) -> (Arc<Hosted>, SessionId, TempDir) {
    let home = TempDir::new();
    let mut fittings = crate::tests::daemon::fittings(&home, FakeWorkProduct::changed(&[]));
    fittings.judge = Arc::new(judge);
    let fleet = Fleet::assembled(fittings)
        .hosting_sessions_on(Arc::new(Stand), Duration::from_secs(600));
    let manifest = ManifestId::carried(
        fleet
            .repositories()
            .first()
            .expect("a served repository")
            .manifest()
            .id()
            .as_str(),
    );
    let fleet = Arc::new(fleet);
    let id = fleet
        .start_session(StartSession {
            manifest_id: manifest,
            title: Some("Fix the flaky test".into()),
            model: None,
            effort: None,
            mode: None,
            pilot: None,
            fork: None,
        })
        .await
        .expect("started")
        .id;
    (fleet, id, home)
}

async fn said(fleet: &Arc<Hosted>, id: &SessionId, text: &str) {
    Arc::clone(fleet)
        .send_session_message(SendSessionMessage {
            session_id: id.clone(),
            text: text.into(),
            attachments: Vec::new(),
            mentions: Vec::new(),
        })
        .await
        .expect("taken");
}

#[tokio::test]
async fn a_press_writes_a_retro_that_is_read_listed_and_answered() {
    let (fleet, id, _home) = rig(FakeJudge::saying(TWO_ITEMS)).await;
    said(&fleet, &id, "no, show walks with show_window").await;

    let retro = Arc::clone(&fleet)
        .write_session_retro(id.as_str().to_string())
        .await
        .expect("written");
    assert_eq!(retro.state, RetroState::Written);
    assert_eq!(retro.session.as_ref().expect("a session").id, id.as_str());
    assert_eq!(retro.record.corrections.len(), 1);
    assert_eq!(retro.items.len(), 2, "the item citing nothing real is dropped");
    assert_eq!(retro.items[1].who.domain(), Whose::Agent);
    assert!(retro.items[0].id.contains("-r1-0"), "{}", retro.items[0].id);

    let read = fleet.get_session_retro(id.as_str().to_string()).await.unwrap();
    assert_eq!(read.items.len(), 2);

    let listed = fleet.list_lessons(None, None, None, 50).await.unwrap().lessons;
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].session.as_ref().expect("a session").id, id.as_str());
    assert!(listed[0].handle.starts_with("s-"));

    let accepted = Arc::clone(&fleet)
        .agree_lesson(retro.items[0].id.clone(), api::Redirector::Person)
        .await
        .expect("kept");
    assert_eq!(accepted.state.domain(), LessonState::Accepted);
    let discarded = fleet
        .disagree_lesson(retro.items[1].id.clone())
        .await
        .expect("discarded");
    assert_eq!(discarded.state.domain(), LessonState::Discarded);
    let still_open = fleet.list_lessons(None, None, None, 50).await.unwrap().lessons;
    assert!(still_open.is_empty());

    // Nothing new since: the last retro answers.
    let again = Arc::clone(&fleet)
        .write_session_retro(id.as_str().to_string())
        .await
        .expect("answered");
    assert_eq!(again.items.len(), 2);

    // Something new makes a second.
    said(&fleet, &id, "and stop opening pages").await;
    let second = Arc::clone(&fleet)
        .write_session_retro(id.as_str().to_string())
        .await
        .expect("written");
    assert!(second.items[0].id.contains("-r2-0"), "{}", second.items[0].id);
    assert_eq!(second.record.corrections.len(), 1, "from the end of the first");
}

#[tokio::test]
async fn a_session_with_nothing_in_it_is_refused() {
    let (fleet, id, _home) = rig(FakeJudge::saying(TWO_ITEMS)).await;
    let refused = Arc::clone(&fleet)
        .write_session_retro(id.as_str().to_string())
        .await
        .expect_err("nothing to write about");
    assert_eq!(refused.error().code, "fleet.session_retro_nothing_new");
}

#[tokio::test]
async fn a_second_press_while_one_is_writing_is_refused() {
    let (fleet, id, _home) =
        rig(FakeJudge::saying(TWO_ITEMS).taking(Duration::from_millis(400))).await;
    said(&fleet, &id, "a first word").await;
    let first = tokio::spawn({
        let fleet = Arc::clone(&fleet);
        let id = id.as_str().to_string();
        async move { fleet.write_session_retro(id).await }
    });
    tokio::time::sleep(Duration::from_millis(100)).await;
    let refused = Arc::clone(&fleet)
        .write_session_retro(id.as_str().to_string())
        .await
        .expect_err("being written");
    assert_eq!(refused.error().code, "fleet.session_retro_being_written");
    first.await.unwrap().expect("the first is written");
}
