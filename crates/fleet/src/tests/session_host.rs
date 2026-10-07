//! A session Fleet hosts for Bridge, against a stand-in process: intake to the
//! ledger as one row, the lease taken on the first write and every write held
//! until then, a turn started by another session's message, tune, close, and
//! resume after a quiet spell. Since 23.49.
//!
//! **The stand-in is the process seam and not the agent CLI**: a real one needs
//! a model and an account. It records what Fleet started and wrote, and says
//! what a test tells it to, through the same channel the real reader uses.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use adapter_traits::{CallDetail, DroneEvent, Speaker};
use api::{HostedSessions, Sessions};
use ipc::{
    AnswerSessionAsk, CloseSession, GateAnswer, HelmCallAnswer, ManifestId, SendSessionMessage,
    SessionAskState, SessionFact, SessionGate, SessionId, SessionMode, SessionOrigin,
    SessionReport, SessionRow, SessionState, SessionTurn, SessionVoice, StartSession, TuneSession,
};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::session_host::{address_of, Heard, Process, Processes, Sink, Start};
use crate::tests::daemon::a_fleet;
use crate::tests::tmp::TempDir;

type Hosted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

pub(super) struct Standing {
    pub(super) sent: Mutex<Vec<String>>,
    pub(super) ended: AtomicBool,
    pid: u32,
}

impl Process for Standing {
    fn pid(&self) -> Option<u32> {
        Some(self.pid)
    }
    fn send(&self, line: String) {
        self.sent.lock().unwrap().push(line);
    }
    fn end(&self) {
        self.ended.store(true, Ordering::SeqCst);
    }
}

#[derive(Default)]
pub(super) struct StandIn {
    started: Mutex<Vec<(Start, Sink, Arc<Standing>)>>,
}

pub(super) struct Shared(pub(super) Arc<StandIn>);

impl Processes for Shared {
    fn start(&self, start: &Start, sink: Sink) -> Result<Arc<dyn Process>, String> {
        let mut started = self.0.started.lock().unwrap();
        let process = Arc::new(Standing {
            sent: Mutex::new(Vec::new()),
            ended: AtomicBool::new(false),
            pid: 1000 + started.len() as u32,
        });
        started.push((start.clone(), sink, Arc::clone(&process)));
        Ok(process)
    }
}

impl StandIn {
    pub(super) fn starts(&self) -> Vec<Start> {
        self.started
            .lock()
            .unwrap()
            .iter()
            .map(|one| one.0.clone())
            .collect()
    }
    pub(super) fn process(&self, index: usize) -> Arc<Standing> {
        Arc::clone(&self.started.lock().unwrap()[index].2)
    }
    pub(super) fn says(&self, index: usize, events: Vec<DroneEvent>) {
        let _ = self.started.lock().unwrap()[index]
            .1
            .send(Heard::Events(events));
    }
    pub(super) fn hears(&self, index: usize, heard: Heard) {
        let _ = self.started.lock().unwrap()[index].1.send(heard);
    }
    pub(super) fn init(&self, index: usize) {
        self.says(
            index,
            vec![DroneEvent::Started {
                session: String::new(),
                model: String::from("stand-in"),
                mcp_servers: 1,
            }],
        );
    }
    pub(super) fn finishes(&self, index: usize, text: &str) {
        self.says(
            index,
            vec![
                DroneEvent::Said {
                    text: text.into(),
                    by: Speaker::Drone,
                },
                DroneEvent::Ended {
                    turns: 1,
                    cost_micros: 5_000,
                    refusals: 0,
                },
            ],
        );
    }
}

struct Rig {
    fleet: Arc<Hosted>,
    stand_in: Arc<StandIn>,
    manifest: ManifestId,
    root: String,
    _home: TempDir,
}

fn rig() -> Rig {
    let home = TempDir::new();
    let stand_in = Arc::new(StandIn::default());
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[])).hosting_sessions_on(
        Arc::new(Shared(Arc::clone(&stand_in))),
        Duration::from_secs(600),
    );
    let one = fleet.repositories().first().expect("a served repository");
    let manifest = ManifestId::carried(one.manifest().id().as_str());
    let root = one.root().to_string();
    Rig {
        fleet: Arc::new(fleet),
        stand_in,
        manifest,
        root,
        _home: home,
    }
}

impl Rig {
    async fn start(&self) -> SessionId {
        self.fleet
            .start_session(StartSession {
                manifest_id: self.manifest.clone(),
                title: None,
                model: None,
                effort: None,
                mode: None,
                pilot: None,
                fork: None,
            })
            .await
            .expect("started")
            .id
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

    async fn rows(&self, id: &SessionId) -> Vec<SessionRow> {
        Arc::clone(&self.fleet).get_session(id.clone()).await.expect("read").rows
    }

    async fn gate(&self, id: &SessionId, tool: &str, input: &str) -> GateAnswer {
        let call = format!(
            r#"{{"session_id":"{}","cwd":"{}","tool_name":"{tool}","tool_input":{input}}}"#,
            id.as_str(),
            self.root
        );
        let gate: SessionGate = ipc::decode("a gate", call.as_bytes()).expect("a gate");
        self.fleet.gate_session_call(gate).await.expect("answered")
    }

    async fn turn(&self, id: &SessionId) -> SessionTurn {
        let record = Arc::clone(&self.fleet).get_session(id.clone()).await.unwrap().session;
        record.hosted.expect("hosted").turn
    }
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

fn denied(answer: &GateAnswer) -> bool {
    matches!(answer, GateAnswer::Hold { .. })
}

fn reason(answer: &GateAnswer) -> String {
    match answer {
        GateAnswer::Hold { held } => held.reason.clone(),
        GateAnswer::Pass {} => String::new(),
    }
}

#[tokio::test]
async fn a_hosted_session_is_one_row_that_holds_nothing_until_it_writes() {
    let rig = rig();
    let id = rig.start().await;

    let listed = rig
        .fleet
        .list_sessions(None, None, None)
        .await
        .unwrap()
        .sessions;
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].origin, SessionOrigin::Bridge);
    assert!(
        listed[0].attachments.is_empty(),
        "no slot and no branch at birth"
    );
    let hosted = listed[0].hosted.clone().expect("hosted facts");
    assert_eq!(
        hosted.mode,
        SessionMode::Auto,
        "auto unless the person says"
    );
    assert!(!hosted.running, "no process until the first message");
    assert!(rig.stand_in.starts().is_empty());

    // The mod in a hosted session reports it as a terminal one, and its end.
    let mod_says = |fact| SessionReport {
        harness: adapters::HOSTED_HARNESS.into(),
        session_id: id.clone(),
        fact,
    };
    rig.fleet
        .report_session(mod_says(SessionFact::Started {
            cwd: rig.root.clone(),
            title: Some("a title the mod read".into()),
            origin: SessionOrigin::Terminal,
        }))
        .await
        .unwrap();
    rig.fleet
        .report_session(mod_says(SessionFact::Ended {
            reason: "exit".into(),
        }))
        .await
        .unwrap();
    let after = rig
        .fleet
        .list_sessions(None, None, None)
        .await
        .unwrap()
        .sessions;
    assert_eq!(after.len(), 1, "one row, not two");
    assert_eq!(after[0].origin, SessionOrigin::Bridge);
    assert_eq!(
        after[0].state,
        SessionState::Live,
        "Fleet ends a session it hosts"
    );
}

#[tokio::test]
async fn a_message_starts_the_process_and_the_thread_follows_the_turn() {
    let rig = rig();
    let id = rig.start().await;
    rig.send(&id, "fix the login\nwith care").await;

    let starts = rig.stand_in.starts();
    assert_eq!(starts.len(), 1);
    assert_eq!(starts[0].session, id.as_str());
    assert!(!starts[0].resuming);
    assert_eq!(starts[0].directory, rig.root);
    assert_eq!(starts[0].name, address_of(id.as_str()));
    let line = rig.stand_in.process(0).sent.lock().unwrap()[0].clone();
    assert!(line.contains("fix the login"));
    assert_eq!(rig.turn(&id).await, SessionTurn::Working { woken_by: None });

    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "done");
    eventually(|| async { rig.turn(&id).await == SessionTurn::Idle }).await;

    let thread = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap();
    assert_eq!(thread.session.title.as_deref(), Some("fix the login"));
    assert!(thread.session.last_turn_at.is_some());
    assert!(matches!(
        &thread.rows[..],
        [
            SessionRow::Message { from: SessionVoice::You, .. },
            SessionRow::Message { from: SessionVoice::Agent, text, .. }
        ] if text == "done"
    ));
}

#[tokio::test]
async fn a_write_is_held_until_the_first_one_leases_a_slot_and_the_session_moves_into_it() {
    let rig = rig();
    let id = rig.start().await;
    rig.send(&id, "change a file").await;
    rig.stand_in.init(0);

    // Reads and a write outside the repository never lease.
    let read = rig.gate(&id, "Bash", r#"{"command":"git status"}"#).await;
    assert!(!denied(&read));
    let scratch = rig
        .gate(&id, "Write", r#"{"file_path":"/tmp/notes.txt"}"#)
        .await;
    assert!(!denied(&scratch));
    assert!(rig
        .fleet
        .list_sessions(None, None, None)
        .await
        .unwrap()
        .sessions[0]
        .attachments
        .is_empty());

    // The first write into the checkout is held, and leases.
    let path = format!("{}/src/lib.rs", rig.root);
    let first = rig
        .gate(&id, "Edit", &format!(r#"{{"file_path":"{path}"}}"#))
        .await;
    assert!(denied(&first), "the write is held");
    assert!(reason(&first).contains("Slot 1"));
    let record = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap();
    let held: Vec<_> = record
        .session
        .attachments
        .iter()
        .map(|one| (one.kind.as_str(), one.target.as_str()))
        .collect();
    assert!(held.contains(&("slot", "1")), "{held:?}");
    assert!(held
        .iter()
        .any(|(kind, target)| *kind == "branch" && target.contains("session-")));
    assert!(record
        .rows
        .iter()
        .any(|row| matches!(row, SessionRow::Lease { slot: 1, .. })));

    // A second write in the same turn leases nothing more and is held too.
    let again = rig
        .gate(&id, "Write", &format!(r#"{{"file_path":"{path}"}}"#))
        .await;
    assert!(denied(&again));
    assert_eq!(
        rig.fleet
            .list_sessions(None, None, None)
            .await
            .unwrap()
            .sessions[0]
            .attachments
            .iter()
            .filter(|one| one.kind == "slot")
            .count(),
        1
    );

    // The turn ends; Fleet ends the process and resumes it in the slot.
    rig.stand_in.finishes(0, "I will stop here");
    eventually(|| async { rig.stand_in.starts().len() == 2 }).await;
    assert!(rig.stand_in.process(0).ended.load(Ordering::SeqCst));
    let moved = &rig.stand_in.starts()[1];
    assert!(moved.resuming, "the same session, resumed");
    assert_eq!(moved.directory, adapter_traits::slot_path(&rig.root, 1));
    let told = rig.stand_in.process(1).sent.lock().unwrap()[0].clone();
    assert!(told.contains("You are now in"));
    let thread = rig.rows(&id).await;
    assert!(
        !thread.iter().any(
            |row| matches!(row, SessionRow::Message { text, .. } if text == "I will stop here")
        ),
        "what the agent said standing down is not shown"
    );

    // In the slot a write goes through; the main checkout is still refused.
    rig.stand_in.init(1);
    let inside = format!("{}/src/lib.rs", adapter_traits::slot_path(&rig.root, 1));
    assert!(!denied(
        &rig.gate(&id, "Edit", &format!(r#"{{"file_path":"{inside}"}}"#))
            .await
    ));
    let outside = rig
        .gate(&id, "Edit", &format!(r#"{{"file_path":"{path}"}}"#))
        .await;
    assert!(denied(&outside));
    assert!(reason(&outside).contains("main checkout"));
}

#[tokio::test]
async fn another_sessions_message_wakes_the_target_and_names_who() {
    let rig = rig();
    let a = rig.start().await;
    let b = rig.start().await;
    rig.send(&a, "hello a").await;
    rig.send(&b, "hello b").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "a ready");
    rig.stand_in.init(1);
    rig.stand_in.finishes(1, "b ready");
    eventually(|| async { rig.turn(&b).await == SessionTurn::Idle }).await;

    // A writes to B with SendMessage; the CLI delivers it to B's live process.
    rig.stand_in.says(
        0,
        vec![DroneEvent::Called {
            tool: "SendMessage".into(),
            call: "toolu_1".into(),
            detail: CallDetail::of(&format!("to {}: please review", address_of(b.as_str()))),
        }],
    );
    eventually(|| async { rig.rows(&b).await.len() == 3 }).await;
    // B's turn starts with no message of the person's outstanding.
    rig.stand_in.init(1);
    eventually(|| async {
        matches!(
            rig.turn(&b).await,
            SessionTurn::Working { woken_by: Some(_) }
        )
    })
    .await;
    match rig.turn(&b).await {
        SessionTurn::Working { woken_by: Some(by) } => assert_eq!(by.id, a.as_str()),
        other => panic!("{other:?}"),
    }
    let rows = rig.rows(&b).await;
    assert!(matches!(
        &rows[2],
        SessionRow::Message { from: SessionVoice::Session { id, .. }, text, .. }
            if id == a.as_str() && text == "please review"
    ));
    rig.stand_in.finishes(1, "reviewed");
    eventually(|| async { rig.turn(&b).await == SessionTurn::Idle }).await;
}

#[tokio::test]
async fn a_message_to_a_session_whose_process_ended_resumes_it() {
    let rig = rig();
    let a = rig.start().await;
    let b = rig.start().await;
    rig.send(&a, "hello a").await;
    rig.send(&b, "hello b").await;
    rig.stand_in.init(1);
    rig.stand_in.finishes(1, "b ready");
    eventually(|| async { rig.turn(&b).await == SessionTurn::Idle }).await;

    // Tuning ends a process that is idle, as a quiet spell does.
    rig.fleet
        .tune_session(TuneSession {
            session_id: b.clone(),
            model: None,
            effort: None,
            mode: SessionMode::Auto,
        })
        .await
        .unwrap();
    assert!(rig.stand_in.process(1).ended.load(Ordering::SeqCst));

    rig.stand_in.says(
        0,
        vec![DroneEvent::Called {
            tool: "SendMessage".into(),
            call: "toolu_2".into(),
            detail: CallDetail::of(&format!("to {}: are you there", address_of(b.as_str()))),
        }],
    );
    eventually(|| async { rig.stand_in.starts().len() == 3 }).await;
    let resumed = &rig.stand_in.starts()[2];
    assert_eq!(resumed.session, b.as_str());
    assert!(resumed.resuming);
    let line = rig.stand_in.process(2).sent.lock().unwrap()[0].clone();
    assert!(line.contains("are you there"));
}

#[tokio::test]
async fn tuning_ends_an_idle_process_and_the_next_message_resumes_on_the_new_settings() {
    let rig = rig();
    let id = rig.start().await;
    rig.send(&id, "first").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "ok");
    eventually(|| async { rig.turn(&id).await == SessionTurn::Idle }).await;

    let tuned = rig
        .fleet
        .tune_session(TuneSession {
            session_id: id.clone(),
            model: Some("sonnet".into()),
            effort: Some("high".into()),
            mode: SessionMode::Plan,
        })
        .await
        .unwrap();
    let hosted = tuned.hosted.unwrap();
    assert_eq!(
        (
            hosted.model.as_deref(),
            hosted.effort.as_deref(),
            hosted.mode,
            hosted.running
        ),
        (Some("sonnet"), Some("high"), SessionMode::Plan, false)
    );
    assert!(rig.stand_in.process(0).ended.load(Ordering::SeqCst));

    rig.send(&id, "second").await;
    let next = &rig.stand_in.starts()[1];
    assert!(next.resuming);
    assert_eq!(
        (next.model.as_deref(), next.effort.as_deref(), next.mode),
        (Some("sonnet"), Some("high"), SessionMode::Plan)
    );
}

#[tokio::test]
async fn a_quiet_process_is_ended_and_closing_ends_the_row_and_refuses_more() {
    let rig = rig();
    let home = TempDir::new();
    let stand_in = Arc::new(StandIn::default());
    let quick = Arc::new(
        a_fleet(&home, FakeWorkProduct::changed(&[]))
            .hosting_sessions_on(Arc::new(Shared(Arc::clone(&stand_in))), Duration::ZERO),
    );
    let one = quick.repositories().first().unwrap();
    let id = quick
        .start_session(StartSession {
            manifest_id: ManifestId::carried(one.manifest().id().as_str()),
            title: Some("quiet".into()),
            model: None,
            effort: None,
            mode: None,
            pilot: None,
            fork: None,
        })
        .await
        .unwrap()
        .id;
    Arc::clone(&quick)
        .send_session_message(SendSessionMessage {
            session_id: id.clone(),
            text: "go".into(),
            attachments: Vec::new(),
            mentions: Vec::new(),
        })
        .await
        .unwrap();
    stand_in.init(0);
    stand_in.finishes(0, "ok");
    eventually(|| async {
        quick.sweep_quiet().await;
        stand_in.process(0).ended.load(Ordering::SeqCst)
    })
    .await;

    // Resumed by the next message, under the same id.
    Arc::clone(&quick)
        .send_session_message(SendSessionMessage {
            session_id: id.clone(),
            text: "again".into(),
            attachments: Vec::new(),
            mentions: Vec::new(),
        })
        .await
        .unwrap();
    assert!(stand_in.starts()[1].resuming);

    let closed = quick
        .close_session(CloseSession {
            session_id: id.clone(),
        })
        .await
        .unwrap();
    assert_eq!(closed.state, SessionState::Ended);
    assert_eq!(closed.end_reason.as_deref(), Some("closed"));
    assert!(stand_in.process(1).ended.load(Ordering::SeqCst));
    let refused = Arc::clone(&quick)
        .send_session_message(SendSessionMessage {
            session_id: id,
            text: "more".into(),
            attachments: Vec::new(),
            mentions: Vec::new(),
        })
        .await;
    assert!(refused.is_err());
    drop(rig);
}

#[tokio::test]
async fn an_ask_is_on_the_threads_own_row_and_the_answer_goes_back_in_the_call() {
    let rig = rig();
    let id = rig.start().await;
    let asking = |command: &str| -> ipc::AskingToRun {
        let body = format!(r#"{{"tool_name":"Bash","input":{{"command":"{command}"}}}}"#);
        ipc::decode("an ask", body.as_bytes()).expect("an ask")
    };

    // Auto runs what is none of the three classes, unasked.
    let ran = rig
        .fleet
        .session_permission(id.as_str(), asking("ls -la"))
        .await
        .unwrap();
    assert!(matches!(ran, ipc::RunOrNot::Allow { .. }));

    // A destructive one waits on the person.
    let fleet = Arc::clone(&rig.fleet);
    let session = id.as_str().to_string();
    let waiting = tokio::spawn(async move {
        fleet
            .session_permission(&session, asking("rm -rf build"))
            .await
    });
    let call = Mutex::new(String::new());
    eventually(|| async {
        let asked = Arc::clone(&rig.fleet)
            .get_session(id.clone())
            .await
            .unwrap()
            .session
            .hosted
            .unwrap()
            .asked;
        match asked {
            Some(asked) => {
                *call.lock().unwrap() = asked.call;
                true
            }
            None => false,
        }
    })
    .await;
    assert!(matches!(
        &rig.rows(&id).await[..],
        [SessionRow::Ask {
            state: SessionAskState::Waiting,
            ..
        }]
    ));
    assert!(
        rig.fleet.helm().asks().waiting().is_empty(),
        "Helm's dock does not carry it"
    );
    rig.fleet
        .answer_session_ask(AnswerSessionAsk {
            session_id: id.clone(),
            call: call.into_inner().unwrap(),
            answer: HelmCallAnswer::Refuse,
            note: Some("not that".into()),
        })
        .await
        .unwrap();
    let decided = waiting.await.unwrap().unwrap();
    assert!(matches!(decided, ipc::RunOrNot::Deny { message } if message.contains("not that")));
    assert!(matches!(
        &rig.rows(&id).await[..],
        [SessionRow::Ask {
            state: SessionAskState::Refused,
            ..
        }]
    ));
}

#[tokio::test]
async fn a_call_on_a_hosted_sessions_connection_is_its_own_ask_and_its_door_offers_the_permission_tool(
) {
    use api::{Admitting, Conversations};

    let rig = rig();
    let id = rig.start().await;
    let peer: std::net::SocketAddr = "127.0.0.1:50999".parse().unwrap();
    let caller = api::Caller::at(peer);
    assert!(
        rig.fleet.helm_at(caller).is_none(),
        "no process, no session"
    );

    rig.send(&id, "hello").await;
    let reach = rig
        .fleet
        .helm_at(caller)
        .expect("a hosted session's connection");
    assert!(reach.is_a_hosted_session());
    assert!(reach.may("ask_the_person"));
    assert!(reach.may("list_sessions") && reach.may("who_owns"));
    assert!(
        !reach.may("change_slot_pool"),
        "Helm's acts are not offered to it"
    );

    let asking: ipc::AskingToRun = ipc::decode(
        "an ask",
        br#"{"tool_name":"Bash","input":{"command":"ls"}}"#,
    )
    .unwrap();
    let decided = rig
        .fleet
        .ask_the_person_at(asking, None, caller)
        .await
        .unwrap();
    assert!(matches!(decided, ipc::RunOrNot::Allow { .. }));
    assert!(rig.fleet.helm().asks().waiting().is_empty());
}
