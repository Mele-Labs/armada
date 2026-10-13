//! The claim, written before the code: **a Session's agent starts or forks a Session, and it waits
//! for the owner.** The new Session is listed at once with no process; the agent's first message sits
//! unsent and reads as a waiting item of its own; his ordinary send sends it and settles both; and the
//! caller is placed by its connection, never named. `docs/concepts/session.md`, *A Session an agent
//! started*. The stand-in process is `session_host`'s.

use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Refusal, Sessions};
use ipc::{
    CloseSession, ForkThisSession, ManifestId, SendSessionMessage, SessionHeld, SessionId, SessionRecord,
    SessionRow, SessionState, SessionVoice, StartAnotherSession, StartSession, WaitingActKind,
    WaitingSource,
};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::peer::Placing;
use crate::tests::session_host::{Shared, StandIn};
use crate::tests::tmp::TempDir;

type Hosted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const FIRST: &str = "Read crates/reader and say why it drops lines.";

struct Rig {
    fleet: Arc<Hosted>,
    stand_in: Arc<StandIn>,
    manifest: ManifestId,
    home: TempDir,
}

fn rig(peers: Option<Arc<Placing>>) -> Rig {
    let home = TempDir::new();
    let stand_in = Arc::new(StandIn::default());
    let mut fittings = crate::tests::daemon::fittings(&home, FakeWorkProduct::changed(&[]));
    if let Some(peers) = peers {
        fittings.peers = peers;
    }
    let fleet = Fleet::assembled(fittings)
        .hosting_sessions_on(Arc::new(Shared(Arc::clone(&stand_in))), Duration::from_secs(600));
    let manifest = ManifestId::carried(fleet.repositories().first().expect("a served repository").manifest().id().as_str());
    Rig { fleet: Arc::new(fleet), stand_in, manifest, home }
}

fn caller() -> Option<api::Caller> {
    Some(api::Caller::at("127.0.0.1:50999".parse().unwrap()))
}

fn begins(rig: &Rig, title: &str, message: &str) -> StartAnotherSession {
    StartAnotherSession { manifest_id: rig.manifest.clone(), title: title.into(), message: message.into() }
}

/// A transcript line the agent CLI wrote: `kind` is `user` or `assistant`.
fn line(kind: &str, uuid: &str, text: &str) -> String {
    let origin = if kind == "user" { r#""origin":{"kind":"human"},"# } else { "" };
    format!(
        r#"{{"type":"{kind}","uuid":"{uuid}","timestamp":"2026-10-12T08:48:10.000Z",{origin}"message":{{"role":"x","content":[{{"type":"text","text":"{text}"}}]}}}}"#
    )
}

impl Rig {
    async fn record(&self, id: &SessionId) -> SessionRecord {
        Arc::clone(&self.fleet).get_session(id.clone()).await.expect("read").session
    }

    async fn rows(&self, id: &SessionId) -> Vec<SessionRow> {
        Arc::clone(&self.fleet).get_session(id.clone()).await.expect("read").rows
    }

    async fn send(&self, id: &SessionId, text: &str) {
        Arc::clone(&self.fleet)
            .send_session_message(SendSessionMessage {
                session_id: id.clone(),
                text: text.into(),
                attachments: Vec::new(),
                mentions: Vec::new(),
            })
            .await
            .expect("taken");
    }

    /// A hosted session with its process up: the agent that makes the calls.
    async fn an_agent(&self) -> SessionId {
        let id = self
            .fleet
            .start_session(StartSession {
                manifest_id: self.manifest.clone(),
                title: Some("The agent".into()),
                model: None,
                effort: None,
                mode: None,
                pilot: None,
                fork: None,
            })
            .await
            .expect("started")
            .id;
        self.send(&id, "go on").await;
        self.stand_in.init(0);
        id
    }

    async fn listed(&self) -> Vec<SessionRecord> {
        Arc::clone(&self.fleet).list_sessions(None, None, None).await.expect("listed").sessions
    }

    /// The ids of the agent's messages in the session's thread, once there are `count`.
    async fn agent_rows(&self, id: &SessionId, count: usize) -> Vec<String> {
        for _ in 0..500 {
            let ids: Vec<String> = self
                .rows(id)
                .await
                .iter()
                .filter_map(|row| match row {
                    SessionRow::Message { id, from: SessionVoice::Agent, .. } => Some(id.clone()),
                    _ => None,
                })
                .collect();
            if ids.len() == count {
                return ids;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("the agent's {count} messages did not reach the thread within five seconds");
    }

    /// The transcript the CLI keeps for session `id`, from these lines.
    fn transcript(&self, id: &SessionId, lines: &[String]) {
        let dir = self.home.path().join(adapters::SESSIONS).join("-somewhere");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("{}.jsonl", id.as_str())), lines.join("\n") + "\n").unwrap();
    }

    /// An agent with two turns of its own said and written, ready to be forked.
    async fn an_agent_that_has_spoken(&self) -> (SessionId, Vec<String>) {
        let id = self.an_agent().await;
        self.stand_in.finishes(0, "Looking now.");
        self.send(&id, "and then").await;
        self.stand_in.finishes(0, "Fixed.");
        let agent = self.agent_rows(&id, 2).await;
        self.transcript(
            &id,
            &[
                line("user", "u0", "go on"),
                line("assistant", "a1", "Looking now."),
                line("user", "u1", "and then"),
                line("assistant", "a2", "Fixed."),
            ],
        );
        (id, agent)
    }
}

fn kind<'a>(record: &'a SessionRecord, kind: &str) -> Option<&'a ipc::Attachment> {
    record.attachments.iter().find(|one| one.kind == kind)
}

fn code(refused: Result<SessionHeld, Refusal>) -> String {
    format!("{:?}", refused.expect_err("refused"))
}

#[tokio::test]
async fn a_started_session_is_listed_at_once_holding_the_message_and_starts_no_process() {
    let rig = rig(None);
    let agent = rig.an_agent().await;
    let processes = rig.stand_in.starts().len();

    let held = rig.fleet.start_another_session(caller(), begins(&rig, "Check the reader", FIRST)).await.expect("held");
    assert_eq!((held.title.as_str(), held.fork), ("Check the reader", false));

    let listed = rig.listed().await;
    let record = listed.iter().find(|one| one.id == held.session_id).expect("it is listed at once");
    assert_eq!(record.title.as_deref(), Some("Check the reader"));
    assert_eq!(record.state, SessionState::Live);
    assert!(!record.hosted.as_ref().expect("Bridge hosts it").running, "no process");
    assert_eq!(rig.stand_in.starts().len(), processes, "nothing was started");
    assert!(rig.rows(&held.session_id).await.is_empty(), "no turn, no row");

    let unsent = kind(record, "unsent").expect("the message is kept");
    assert_eq!((unsent.target.as_str(), unsent.state), (FIRST, ipc::AttachmentState::Standing));

    let [item] = &record.waiting_for[..] else { panic!("one item waits: {:?}", record.waiting_for) };
    assert_eq!(item.id, format!("unsent:{}", held.session_id.as_str()));
    assert_eq!((item.text.as_str(), item.source), ("First message", WaitingSource::FirstMessage));
    let act = item.act.as_ref().expect("an act of its own");
    assert_eq!((act.kind, act.target.as_str()), (WaitingActKind::Message, held.session_id.as_str()));
    assert!(item.options.is_empty(), "no reply is drawn for it");

    assert_eq!(kind(record, "started_by").expect("it names its starter").target, agent.as_str());
}

#[tokio::test]
async fn the_callers_thread_and_ledger_name_the_session_it_started() {
    let rig = rig(None);
    let agent = rig.an_agent().await;
    let held = rig.fleet.start_another_session(caller(), begins(&rig, "Check the reader", FIRST)).await.expect("held");

    let rows = rig.rows(&agent).await;
    let started = rows
        .iter()
        .find_map(|row| match row {
            SessionRow::Started { session, fork, .. } => Some((session.clone(), *fork)),
            _ => None,
        })
        .expect("the thread says it");
    assert_eq!((started.0.id, started.0.title.as_str(), started.1), (held.session_id.clone(), "Check the reader", false));

    let record = rig.record(&agent).await;
    assert_eq!(kind(&record, "started_to").expect("the caller names it").target, held.session_id.as_str());
    assert_eq!(kind(&record, "started_to").unwrap().state, ipc::AttachmentState::Spent, "a fact that stays");
}

#[tokio::test]
async fn his_ordinary_send_sends_it_starts_the_first_turn_and_settles_both() {
    let rig = rig(None);
    rig.an_agent().await;
    let held = rig.fleet.start_another_session(caller(), begins(&rig, "Check the reader", FIRST)).await.expect("held");
    let before = rig.stand_in.starts().len();

    // He may edit it first: what he sends is the box, not what the agent wrote.
    rig.send(&held.session_id, "Read crates/reader and say why it drops blank lines.").await;

    assert_eq!(rig.stand_in.starts().len(), before + 1, "the first turn starts");
    let record = rig.record(&held.session_id).await;
    assert_eq!(kind(&record, "unsent").expect("the row stays as a fact").state, ipc::AttachmentState::Spent);
    assert!(record.waiting_for.is_empty(), "nothing waits: {:?}", record.waiting_for);
    let sent: Vec<String> = rig
        .rows(&held.session_id)
        .await
        .into_iter()
        .filter_map(|row| match row {
            SessionRow::Message { from: SessionVoice::You, text, .. } => Some(text),
            _ => None,
        })
        .collect();
    assert_eq!(sent, ["Read crates/reader and say why it drops blank lines."]);
}

#[tokio::test]
async fn ending_it_unsent_discards_the_message_and_the_item() {
    let rig = rig(None);
    rig.an_agent().await;
    let held = rig.fleet.start_another_session(caller(), begins(&rig, "Check the reader", FIRST)).await.expect("held");
    let ended = Arc::clone(&rig.fleet)
        .close_session(CloseSession { session_id: held.session_id.clone() })
        .await
        .expect("closed");
    assert!(ended.waiting_for.is_empty());
    assert_eq!(kind(&ended, "unsent").map(|one| one.state), Some(ipc::AttachmentState::GivenBack));
    assert!(rig.stand_in.starts().len() == 1, "and it never ran");
}

#[tokio::test]
async fn a_fork_at_a_message_of_the_callers_own_live_session_waits_the_same_way() {
    let rig = rig(None);
    let (agent, rows) = rig.an_agent_that_has_spoken().await;
    let before = rig.stand_in.starts().len();

    let held = rig
        .fleet
        .fork_this_session(
            caller(),
            ForkThisSession { title: "The other way".into(), message: "Do it with a map.".into(), at: Some(rows[0].clone()) },
        )
        .await
        .expect("a live session forks itself at a message");
    assert!(held.fork);
    assert_eq!(rig.stand_in.starts().len(), before, "no process yet");

    let record = rig.record(&held.session_id).await;
    let from = kind(&record, "forked_from").expect("the fork links back");
    assert_eq!(from.target, agent.as_str());
    assert_eq!(from.detail.get("at").map(String::as_str), Some(rows[0].as_str()));
    assert!(kind(&record, "started_by").is_none(), "a fork's link is the fork's own");
    assert_eq!(kind(&record, "unsent").expect("held").target, "Do it with a map.");
    assert_eq!(record.waiting_for[0].source, WaitingSource::FirstMessage);
    assert!(kind(&rig.record(&agent).await, "forked_to").is_some());
    assert!(
        rig.rows(&agent).await.iter().any(|row| matches!(row, SessionRow::Started { fork: true, .. })),
        "the callout in the caller's thread"
    );

    rig.send(&held.session_id, "Do it with a map.").await;
    let starts = rig.stand_in.starts();
    assert_eq!(starts[before].forking.as_deref(), Some(agent.as_str()));
    assert_eq!(starts[before].fork_cut.as_deref(), Some("a1"), "the copy ends at that line");
}

#[tokio::test]
async fn a_fork_with_no_message_ends_at_the_callers_latest_of_its_own() {
    let rig = rig(None);
    let (agent, rows) = rig.an_agent_that_has_spoken().await;
    let before = rig.stand_in.starts().len();
    let held = rig
        .fleet
        .fork_this_session(caller(), ForkThisSession { title: "Again".into(), message: "Go.".into(), at: None })
        .await
        .expect("forks");
    let from = kind(&rig.record(&held.session_id).await, "forked_from").cloned().expect("link");
    assert_eq!(from.detail.get("at").map(String::as_str), Some(rows[1].as_str()));
    rig.send(&held.session_id, "Go.").await;
    assert_eq!(rig.stand_in.starts()[before].fork_cut.as_deref(), Some("a2"));
    assert_eq!(rig.stand_in.starts()[before].forking.as_deref(), Some(agent.as_str()));
}

#[tokio::test]
async fn a_message_the_conversation_does_not_hold_is_refused_and_nothing_is_written() {
    let rig = rig(None);
    let (agent, _) = rig.an_agent_that_has_spoken().await;
    let listed = rig.listed().await.len();
    let refused = code(
        rig.fleet
            .fork_this_session(caller(), ForkThisSession { title: "t".into(), message: "m".into(), at: Some("nowhere".into()) })
            .await,
    );
    assert!(refused.contains("fleet.fork_point_unwritten"), "{refused}");
    assert_eq!(rig.listed().await.len(), listed, "no session was left behind");
    assert!(kind(&rig.record(&agent).await, "forked_to").is_none());
}

#[tokio::test]
async fn a_call_that_places_no_hosted_session_is_refused_and_nothing_is_made() {
    // Nobody holds the connection: the call comes from no Session's process.
    let rig = rig(Some(Placing::nothing()));
    rig.an_agent().await;
    let listed = rig.listed().await.len();

    let refused = code(rig.fleet.start_another_session(caller(), begins(&rig, "Check the reader", FIRST)).await);
    assert!(refused.contains("fleet.agent_start_unplaced"), "{refused}");
    let refused = code(
        rig.fleet
            .fork_this_session(caller(), ForkThisSession { title: "t".into(), message: "m".into(), at: None })
            .await,
    );
    assert!(refused.contains("fleet.agent_start_unplaced"), "{refused}");
    // And a call with no connection at all, which is what a person's request is.
    let refused = code(rig.fleet.start_another_session(None, begins(&rig, "Check the reader", FIRST)).await);
    assert!(refused.contains("fleet.agent_start_unplaced"), "{refused}");
    assert_eq!(rig.listed().await.len(), listed);
}

#[tokio::test]
async fn a_blank_title_or_message_is_refused() {
    let rig = rig(None);
    rig.an_agent().await;
    for (title, message) in [("  ", FIRST), ("Check", "  ")] {
        let refused = code(rig.fleet.start_another_session(caller(), begins(&rig, title, message)).await);
        assert!(refused.contains("fleet.agent_start_empty"), "{refused}");
    }
}
