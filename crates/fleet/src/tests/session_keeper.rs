//! A hosted session's agent outlives a Fleet restart.
//!
//! **The keeper runs on a runtime of its own and each Fleet on one it can be
//! thrown away with**, so dropping a Fleet's runtime cuts its connections the
//! way a stopped process does. The agent is a shell script that says what turn
//! it is on and which process it is, so "the same process" is a string to read.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::DroneEvent;
use adapters::HeadlessAgent;
use api::HostedSessions;
use ipc::{ManifestId, SendSessionMessage, SessionRow, SessionVoice, StartSession};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};
use tokio::runtime::{Handle, Runtime};

use crate::daemon::Fleet;
use crate::session_host::keeper::{keep, Keeper};
use crate::session_host::{Heard, Process, Processes, Sink, Start};
use crate::tests::tmp::TempDir;

const AGENT: &str = r#"
while IFS= read -r line; do
  n=$((n+1))
  case "$line" in *die*) echo 'boom: no such session' >&2; exit 3;; esac
  echo '{"type":"system","subtype":"init","session_id":"s","model":"m","mcp_servers":[]}'
  echo '{"type":"assistant","message":{"content":[{"type":"text","text":"pid '$$' before '$n'"}]}}'
  case "$line" in *hold*) while [ ! -f "$1" ]; do sleep 0.05; done;; esac
  echo '{"type":"assistant","message":{"content":[{"type":"text","text":"pid '$$' after '$n'"}]}}'
  echo '{"type":"result","num_turns":1,"total_cost_usd":0.01,"permission_denials":[]}'
done
"#;

static SOCKETS: AtomicUsize = AtomicUsize::new(0);

/// A keeper per session, run in-process on `keepers`, the way `armada
/// session-keep` runs it beside Fleet.
struct Keepers {
    keepers: Handle,
    directory: PathBuf,
    socket: PathBuf,
    go: PathBuf,
    starts: AtomicUsize,
}

impl Keepers {
    fn new(keepers: Handle, directory: &Path) -> Arc<Keepers> {
        std::fs::write(directory.join("agent.sh"), AGENT).unwrap();
        let socket = std::env::temp_dir().join(format!(
            "ak-{}-{}.sock",
            std::process::id(),
            SOCKETS.fetch_add(1, Ordering::SeqCst)
        ));
        Arc::new(Keepers {
            keepers,
            directory: directory.to_path_buf(),
            socket,
            go: directory.join("go"),
            starts: AtomicUsize::new(0),
        })
    }

    fn spool(&self) -> PathBuf {
        self.directory.join("spool")
    }

    fn keeper(&self) -> Keeper {
        Keeper {
            socket: self.socket.clone(),
            spool: self.spool(),
            log: self.directory.join("spool.log"),
            program: String::from("sh"),
            args: vec![
                self.directory.join("agent.sh").to_string_lossy().into_owned(),
                self.go.to_string_lossy().into_owned(),
            ],
            directory: Some(self.directory.clone()),
            alone_for: Duration::from_secs(60),
            ends_turn: ends_turn(),
        }
    }
}

fn ends_turn() -> Arc<dyn Fn(&str) -> bool + Send + Sync> {
    Arc::new(|line| line.contains("\"result\""))
}

/// The same host a Fleet has, with a keeper in place of `armada session-keep`.
struct Through(Arc<Keepers>);

impl Processes for Through {
    fn start(&self, _start: &Start, sink: Sink) -> Result<Arc<dyn Process>, String> {
        self.0.starts.fetch_add(1, Ordering::SeqCst);
        self.0.keepers.spawn(keep(self.0.keeper()));
        let stream = loop {
            if let Some(stream) = crate::session_host::kept::connect(&self.0.socket) {
                break stream;
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        crate::session_host::kept::attached(
            stream,
            HeadlessAgent::at(String::new()),
            sink,
            &self.0.socket,
            &self.0.spool(),
        )
    }

    fn reattach(&self, _session: &str, sink: Sink) -> Option<Arc<dyn Process>> {
        let stream = crate::session_host::kept::connect(&self.0.socket)?;
        crate::session_host::kept::attached(
            stream,
            HeadlessAgent::at(String::new()),
            sink,
            &self.0.socket,
            &self.0.spool(),
        )
        .ok()
    }
}

type Hosted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// `later` moves the fixture clock on, so a row this Fleet writes does not take
/// the id of one the last Fleet wrote at the same fixture second. A real clock
/// does not repeat.
fn a_fleet(home: &TempDir, keepers: &Arc<Keepers>, later: u32) -> Arc<Hosted> {
    let mut fittings = crate::tests::daemon::fittings(home, FakeWorkProduct::changed(&[]));
    let clock = crate::tests::planted::Ticking::from_nine();
    for _ in 0..later {
        crate::clock::Clock::now(&clock);
    }
    fittings.clock = Arc::new(clock);
    Arc::new(
        Fleet::assembled(fittings)
            .hosting_sessions_on(Arc::new(Through(Arc::clone(keepers))), Duration::from_secs(600)),
    )
}

fn manifest_of(fleet: &Hosted) -> ManifestId {
    let one = fleet.repositories().first().expect("a served repository");
    ManifestId::carried(one.manifest().id().as_str())
}

async fn said(fleet: &Arc<Hosted>, id: &ipc::SessionId) -> Vec<String> {
    Arc::clone(fleet)
        .get_session(id.clone())
        .await
        .expect("read")
        .rows
        .into_iter()
        .filter_map(|row| match row {
            SessionRow::Message {
                from: SessionVoice::Agent,
                text,
                ..
            } => Some(text),
            _ => None,
        })
        .collect()
}

async fn eventually<F: std::future::Future<Output = bool>>(mut check: impl FnMut() -> F) {
    for _ in 0..500 {
        if check().await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("it did not come true within five seconds");
}

fn pid_in(text: &str) -> String {
    text.split_whitespace().nth(1).unwrap_or_default().to_string()
}

#[test]
fn a_fleet_restart_leaves_the_agent_running_and_the_thread_continues() {
    let keeper_side = Runtime::new().unwrap();
    let home = TempDir::new();
    let keepers = Keepers::new(keeper_side.handle().clone(), home.path());

    let first = Runtime::new().unwrap();
    let (id, pid) = first.block_on(async {
        let fleet = a_fleet(&home, &keepers, 0);
        let id = fleet
            .start_session(StartSession {
                manifest_id: manifest_of(&fleet),
                title: None,
                model: None,
                effort: None,
                mode: None,
                pilot: None,
                fork: None,
            })
            .await
            .expect("started")
            .id;
        Arc::clone(&fleet)
            .send_session_message(SendSessionMessage {
                session_id: id.clone(),
                text: "hold".into(),
                attachments: Vec::new(),
                mentions: Vec::new(),
            })
            .await
            .expect("taken");
        eventually(|| async { said(&fleet, &id).await.len() == 1 }).await;
        // The acknowledgement follows the line it is for.
        tokio::time::sleep(Duration::from_millis(100)).await;
        let pid = pid_in(&said(&fleet, &id).await[0]);
        (id, pid)
    });
    // The Fleet is gone mid-turn: the agent is parked on the flag.
    drop(first);
    std::fs::write(&keepers.go, "").unwrap();
    std::thread::sleep(Duration::from_millis(300));

    let second = Runtime::new().unwrap();
    second.block_on(async {
        let fleet = a_fleet(&home, &keepers, 1000);
        assert_eq!(Arc::clone(&fleet).reattached_sessions().await, 1);
        eventually(|| async { said(&fleet, &id).await.len() == 2 }).await;
        let rows = said(&fleet, &id).await;
        assert!(rows[0].contains("before 1"), "{rows:?}");
        assert!(
            rows[1].contains("after 1"),
            "the turn's later output lands: {rows:?}"
        );
        assert_eq!(pid_in(&rows[1]), pid, "the same process finished the turn");

        Arc::clone(&fleet)
            .send_session_message(SendSessionMessage {
                session_id: id.clone(),
                text: "again".into(),
                attachments: Vec::new(),
                mentions: Vec::new(),
            })
            .await
            .expect("taken");
        eventually(|| async { said(&fleet, &id).await.len() == 4 }).await;
        let rows = said(&fleet, &id).await;
        assert!(rows[3].contains("after 2"), "{rows:?}");
        assert_eq!(pid_in(&rows[3]), pid, "the next message went to the same process");
    });
    assert_eq!(
        keepers.starts.load(Ordering::SeqCst),
        1,
        "one process was ever started: the second Fleet resumed nothing"
    );
}

async fn heard_until_ended(heard: &mut tokio::sync::mpsc::UnboundedReceiver<Heard>) -> Vec<String> {
    let mut texts = Vec::new();
    loop {
        match tokio::time::timeout(Duration::from_secs(5), heard.recv()).await {
            Ok(Some(Heard::Events(events))) => {
                for event in events {
                    match event {
                        DroneEvent::Said { text, .. } => texts.push(text),
                        DroneEvent::Ended { .. } => return texts,
                        _ => {}
                    }
                }
            }
            Ok(Some(_)) => {}
            _ => panic!("the turn did not end: {texts:?}"),
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_keeper_replays_from_what_was_acknowledged_and_no_earlier() {
    let home = TempDir::new();
    let keepers = Keepers::new(Handle::current(), home.path());
    let (sink, mut heard) = tokio::sync::mpsc::unbounded_channel();
    let through = Through(Arc::clone(&keepers));
    let start = Start {
        directory: String::new(),
        session: String::new(),
        resuming: false,
        forking: None,
        name: String::new(),
        model: None,
        effort: None,
        mode: ipc::SessionMode::Auto,
        readable: Vec::new(),
    };
    let first = through.start(&start, sink).unwrap();
    first.send(String::from("hold"));
    // The first line of the turn is taken and acknowledged, then the client goes.
    loop {
        if let Some(Heard::Events(events)) = heard.recv().await {
            if events
                .iter()
                .any(|event| matches!(event, DroneEvent::Said { .. }))
            {
                break;
            }
        }
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
    drop(first);
    drop(heard);
    tokio::time::sleep(Duration::from_millis(100)).await;

    let (sink, mut mid) = tokio::sync::mpsc::unbounded_channel();
    let midway = through.reattach("", sink).expect("the keeper answers");
    let Some(Heard::Attached { busy }) = mid.recv().await else {
        panic!("attach comes first");
    };
    assert!(busy, "the turn was mid-flight when the client went");
    drop(midway);
    drop(mid);
    tokio::time::sleep(Duration::from_millis(100)).await;
    std::fs::write(&keepers.go, "").unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;

    let (sink, mut heard) = tokio::sync::mpsc::unbounded_channel();
    let again = through.reattach("", sink).expect("the keeper answers");
    let Some(Heard::Attached { busy }) = heard.recv().await else {
        panic!("attach comes first");
    };
    assert!(!busy, "the turn ended while nobody was attached");
    let texts = heard_until_ended(&mut heard).await;
    assert_eq!(texts.len(), 1, "only what had not been taken: {texts:?}");
    assert!(texts[0].contains("after 1"), "{texts:?}");
    drop(again);
}

/// A process Fleet let go is not the process the next message reaches. The
/// keeper it ordered to end was still answering on its socket, so the next
/// message reattached to an agent that was already gone and the session never
/// ran again.
#[test]
fn a_message_after_the_process_was_let_go_starts_a_new_one() {
    let keeper_side = Runtime::new().unwrap();
    let home = TempDir::new();
    let keepers = Keepers::new(keeper_side.handle().clone(), home.path());
    let fleet_side = Runtime::new().unwrap();
    fleet_side.block_on(async {
        let fleet = a_fleet(&home, &keepers, 0);
        let id = fleet
            .start_session(StartSession {
                manifest_id: manifest_of(&fleet),
                title: None,
                model: None,
                effort: None,
                mode: None,
                pilot: None,
                fork: None,
            })
            .await
            .expect("started")
            .id;
        let say = |text: &str| {
            Arc::clone(&fleet).send_session_message(SendSessionMessage {
                session_id: id.clone(),
                text: text.into(),
                attachments: Vec::new(),
                mentions: Vec::new(),
            })
        };
        say("one").await.expect("taken");
        eventually(|| async { said(&fleet, &id).await.len() == 2 }).await;
        let first = pid_in(&said(&fleet, &id).await[0]);
        // The turn is over, so a tune, a quiet spell or a move ends the process.
        tokio::time::sleep(Duration::from_millis(100)).await;
        Hosted::let_go(&mut fleet.hosts().of(id.as_str()).state());
        say("two").await.expect("taken");
        eventually(|| async { said(&fleet, &id).await.len() == 4 }).await;
        let rows = said(&fleet, &id).await;
        assert_ne!(pid_in(&rows[2]), first, "a new process answered: {rows:?}");
        assert_eq!(keepers.starts.load(Ordering::SeqCst), 2);
    });
}

#[test]
fn a_process_that_dies_says_why_in_the_thread() {
    let keeper_side = Runtime::new().unwrap();
    let home = TempDir::new();
    let keepers = Keepers::new(keeper_side.handle().clone(), home.path());
    let fleet_side = Runtime::new().unwrap();
    fleet_side.block_on(async {
        let fleet = a_fleet(&home, &keepers, 0);
        let id = fleet
            .start_session(StartSession {
                manifest_id: manifest_of(&fleet),
                title: None,
                model: None,
                effort: None,
                mode: None,
                pilot: None,
                fork: None,
            })
            .await
            .expect("started")
            .id;
        Arc::clone(&fleet)
            .send_session_message(SendSessionMessage {
                session_id: id.clone(),
                text: "die".into(),
                attachments: Vec::new(),
                mentions: Vec::new(),
            })
            .await
            .expect("taken");
        eventually(|| async {
            ended_rows(&fleet, &id)
                .await
                .iter()
                .any(|text| text.contains("boom: no such session") && text.contains("exit status: 3"))
        })
        .await;
    });
}

async fn ended_rows(fleet: &Arc<Hosted>, id: &ipc::SessionId) -> Vec<String> {
    Arc::clone(fleet)
        .get_session(id.clone())
        .await
        .expect("read")
        .rows
        .into_iter()
        .filter_map(|row| match row {
            SessionRow::Tool { text, .. } if text.starts_with("the session's process ended") => {
                Some(text)
            }
            _ => None,
        })
        .collect()
}
