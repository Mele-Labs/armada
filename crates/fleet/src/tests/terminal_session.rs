//! A session run in a terminal, opened in Bridge: its thread drawn from the
//! transcript file, each new line following as a row, and a message held for
//! its mod. Since 23.53. **No terminal and no agent**: the transcript is a file
//! the test writes, and the mod is the test asking.

use std::io::Write as _;
use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Next, Sessions};
use ipc::{
    SendSessionMessage, SessionFact, SessionId, SessionOrigin, SessionReport, SessionRow,
    SessionVoice, TakeHeld,
};
use testkit::{FakeVcs, FakeHarness, FakeWorkProduct};

use super::session_host::{Shared, StandIn};
use crate::daemon::Fleet;
use crate::tests::daemon::a_fleet;
use crate::tests::tmp::TempDir;

type Hosted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const ID: &str = "aaaa-1111";

struct Rig {
    fleet: Arc<Hosted>,
    transcript: std::path::PathBuf,
    root: String,
    _home: TempDir,
}

fn said(kind: &str, uuid: &str, origin: &str, text: &str) -> String {
    format!(
        r#"{{"type":"{kind}","uuid":"{uuid}","timestamp":"2026-10-07T08:48:10.000Z",{origin}"message":{{"role":"x","content":[{{"type":"text","text":"{text}"}}]}}}}"#
    )
}

fn rig() -> Rig {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[])).hosting_sessions_on(
        Arc::new(Shared(Arc::new(StandIn::default()))),
        Duration::from_secs(600),
    );
    let root = fleet.repositories().first().expect("a repository").root().to_string();
    let dir = home.path().join(".claude/projects/-somewhere");
    std::fs::create_dir_all(&dir).unwrap();
    let transcript = dir.join(format!("{ID}.jsonl"));
    Rig {
        fleet: Arc::new(fleet),
        transcript,
        root,
        _home: home,
    }
}

impl Rig {
    async fn reports(&self, fact: SessionFact) {
        self.fleet
            .report_session(SessionReport {
                harness: "claude_code".into(),
                session_id: SessionId::carried(ID),
                fact,
            })
            .await
            .unwrap();
    }

    async fn opened(&self) -> Vec<SessionRow> {
        Arc::clone(&self.fleet)
            .get_session(SessionId::carried(ID))
            .await
            .expect("a terminal session reads")
            .rows
    }

    async fn sends(&self, text: &str) -> Result<(), String> {
        Arc::clone(&self.fleet)
            .send_session_message(SendSessionMessage {
                session_id: SessionId::carried(ID),
                text: text.into(),
                attachments: Vec::new(),
                mentions: Vec::new(),
            })
            .await
            .map(|_| ())
            .map_err(|why| format!("{why:?}"))
    }

    async fn mod_asks(&self) -> Vec<String> {
        self.fleet
            .take_held_messages(TakeHeld {
                session_id: ID.into(),
            })
            .await
            .unwrap()
            .messages
    }

    async fn started(&self) {
        self.reports(SessionFact::Started {
            cwd: self.root.clone(),
            title: None,
            origin: SessionOrigin::Terminal,
        })
        .await;
    }
}

/// **The thread is what the transcript says**, read when Bridge opens the
/// session, and each line the CLI writes after that is a `session.row`.
#[tokio::test]
async fn opening_a_terminal_session_reads_its_transcript_and_then_follows_it() {
    let rig = rig();
    rig.started().await;
    std::fs::write(
        &rig.transcript,
        said("user", "u1", r#""origin":{"kind":"human"},"#, "Fix the build") + "\n",
    )
    .unwrap();

    let mut watching = rig.fleet.events().subscribe();
    let rows = rig.opened().await;
    assert!(matches!(
        &rows[..],
        [SessionRow::Message { from: SessionVoice::You, text, .. }] if text == "Fix the build"
    ));

    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&rig.transcript)
        .unwrap();
    writeln!(file, "{}", said("assistant", "a1", "", "Fixed.")).unwrap();
    let arrived = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(Next::Send(delivered)) = watching.next().await {
                if let ipc::Event::SessionRow(change) = delivered.event {
                    return change;
                }
            }
        }
    })
    .await
    .expect("the new line arrives as a row");
    assert_eq!(arrived.session_id.as_str(), ID);
    assert!(matches!(
        arrived.row,
        SessionRow::Message { from: SessionVoice::Agent, ref text, .. } if text == "Fixed."
    ));
}

/// **Reachable means the mod is asking.** A message is refused while nothing
/// has asked, held once something has, and handed over to the one ask.
#[tokio::test]
async fn a_message_is_held_for_the_mod_that_is_asking_and_handed_over_once() {
    let rig = rig();
    rig.started().await;

    let refused = rig.sends("are you there").await.unwrap_err();
    assert!(refused.contains("fleet.terminal_session_unreachable"), "{refused}");

    assert!(rig.mod_asks().await.is_empty(), "nothing yet, and now it is listening");
    rig.sends("run the tests").await.expect("held");
    rig.sends("then lint").await.expect("held");
    assert_eq!(rig.mod_asks().await, vec!["run the tests", "then lint"]);
    assert!(rig.mod_asks().await.is_empty(), "handed over once");
}

/// A terminal session takes words only, and a session that ended takes nothing.
#[tokio::test]
async fn a_terminal_session_takes_words_and_not_a_file_and_not_after_it_ended() {
    let rig = rig();
    rig.started().await;
    rig.mod_asks().await;

    let file = Arc::clone(&rig.fleet)
        .send_session_message(SendSessionMessage {
            session_id: SessionId::carried(ID),
            text: "look".into(),
            attachments: vec![ipc::SessionUpload {
                name: "a.txt".into(),
                media_type: "text/plain".into(),
                data: "aGk=".into(),
            }],
            mentions: Vec::new(),
        })
        .await
        .unwrap_err();
    assert!(format!("{file:?}").contains("fleet.terminal_session_text_only"));

    rig.reports(SessionFact::Ended {
        reason: "prompt_input_exit".into(),
    })
    .await;
    let closed = rig.sends("hello").await.unwrap_err();
    assert!(closed.contains("fleet.session_closed"), "{closed}");
}

/// The mod of a session Fleet hosts asks too, and is handed nothing.
#[tokio::test]
async fn a_session_that_is_not_a_terminal_one_is_handed_nothing() {
    let rig = rig();
    assert!(rig.mod_asks().await.is_empty(), "no such session is not an error");
}
