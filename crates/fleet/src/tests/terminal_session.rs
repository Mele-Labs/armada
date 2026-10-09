//! A session run in a terminal, opened in Bridge: its thread drawn from the
//! transcript file, each new line following as a row, and a message held for
//! its mod. Since 23.53. **No terminal and no agent**: the transcript is a file
//! the test writes, and the mod is the test asking.

use std::io::Write as _;
use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Next, Sessions};
use ipc::{
    AnswerSessionAsk, HelmCallAnswer, QuestionAnswer, SendSessionMessage, SessionFact, SessionId, SessionOrigin, SessionReport, SessionRow,
    SessionVoice, TakeHeld,
};
use testkit::{FakeVcs, FakeHarness, FakeWorkProduct};

use super::session_host::{eventually, Shared, StandIn};
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

fn hosted(home: &TempDir) -> Hosted {
    a_fleet(home, FakeWorkProduct::changed(&[])).hosting_sessions_on(
        Arc::new(Shared(Arc::new(StandIn::default()))),
        Duration::from_secs(600),
    )
}

fn rig() -> Rig {
    let home = TempDir::new();
    let fleet = hosted(&home);
    let root = fleet.repositories().first().expect("a repository").root().to_string();
    let dir = home.path().join(adapters::SESSIONS).join("-somewhere");
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
                harness: adapters::HOSTED_HARNESS.into(),
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
                wait_ms: None,
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
            mod_version: None,
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

impl Rig {
    /// The mod asking and willing to wait, as a task, so the test can send while it holds.
    fn mod_holds(&self, wait_ms: u64) -> tokio::task::JoinHandle<Vec<String>> {
        let fleet = Arc::clone(&self.fleet);
        tokio::spawn(async move {
            fleet
                .take_held_messages(TakeHeld {
                    session_id: ID.into(),
                    wait_ms: Some(wait_ms),
                })
                .await
                .unwrap()
                .messages
        })
    }
}

/// **An ask held open is listening, and a message answers it the moment it is held.** The send
/// is refused until something asks; once an ask is held it is accepted, and the ask returns long
/// before its own wait is up.
#[tokio::test]
async fn an_ask_held_open_is_answered_the_moment_a_message_is_held() {
    let rig = rig();
    rig.started().await;
    assert!(rig.sends("early").await.is_err(), "nothing is asking yet");

    let waiting = rig.mod_holds(20_000);
    eventually(|| async { rig.fleet.hosts().terminals().listening(ID) }).await;
    rig.sends("now").await.expect("held, since an ask is open");

    let held = tokio::time::timeout(Duration::from_secs(5), waiting)
        .await
        .expect("answered long before its wait was up")
        .unwrap();
    assert_eq!(held, vec!["now"]);
}

/// A wait that runs out answers empty, and one with no wait never held anything.
#[tokio::test]
async fn an_ask_with_nothing_held_answers_empty_at_its_wait_and_at_once_without_one() {
    let rig = rig();
    rig.started().await;

    let began = std::time::Instant::now();
    let held = rig.mod_holds(300).await.unwrap();
    assert!(held.is_empty());
    assert!(began.elapsed() >= Duration::from_millis(300), "{:?}", began.elapsed());

    let began = std::time::Instant::now();
    assert!(rig.mod_asks().await.is_empty());
    assert!(began.elapsed() < Duration::from_millis(250), "{:?}", began.elapsed());
}

/// A file is saved by Fleet and the terminal is told its path, in the words a
/// hosted session is told; a session that ended takes nothing.
#[tokio::test]
async fn a_file_is_saved_and_sent_as_its_path_and_nothing_goes_to_an_ended_session() {
    let rig = rig();
    rig.started().await;
    rig.mod_asks().await;

    Arc::clone(&rig.fleet)
        .send_session_message(SendSessionMessage {
            session_id: SessionId::carried(ID),
            text: "look".into(),
            attachments: vec![
                ipc::SessionUpload {
                    name: "a.txt".into(),
                    media_type: "text/plain".into(),
                    data: "aGk=".into(),
                },
                ipc::SessionUpload {
                    name: "shot.png".into(),
                    media_type: "image/png".into(),
                    data: "aGk=".into(),
                },
            ],
            mentions: Vec::new(),
        })
        .await
        .expect("held");
    let held = rig.mod_asks().await;
    assert_eq!(held.len(), 1);
    assert!(held[0].starts_with("look"));
    assert_eq!(held[0].matches("Attached file: ").count(), 2, "{}", held[0]);
    assert!(held[0].contains("a.txt") && held[0].contains("shot.png"));

    rig.reports(SessionFact::Ended {
        reason: "prompt_input_exit".into(),
    })
    .await;
    let closed = rig.sends("hello").await.unwrap_err();
    assert!(closed.contains("fleet.session_closed"), "{closed}");
}

/// **The terminal's own settings are what its mod said**, and a model chosen in
/// Bridge goes to the mod as a command once, and not again when the terminal
/// already runs on it. The mode is only shown.
#[tokio::test]
async fn what_the_terminal_runs_on_is_what_its_mod_said_and_a_model_is_run_there() {
    let rig = rig();
    rig.started().await;
    rig.mod_asks().await;
    rig.reports(SessionFact::Tuned {
        model: Some("haiku".into()),
        effort: None,
        mode: Some(ipc::SessionMode::Plan),
        commands: vec![ipc::TerminalCommand {
            name: "review".into(),
            says: "Review the pull request".into(),
        }],
        mod_version: None,
    })
    .await;
    let record = Arc::clone(&rig.fleet)
        .get_session(SessionId::carried(ID))
        .await
        .unwrap()
        .session;
    let facts = record.terminal.expect("terminal facts");
    assert_eq!(facts.model.as_deref(), Some("haiku"));
    assert_eq!(facts.mode, Some(ipc::SessionMode::Plan));
    assert_eq!(facts.commands.len(), 1);

    let tune = |model: &str| ipc::TuneSession {
        session_id: SessionId::carried(ID),
        model: Some(model.into()),
        effort: Some("low".into()),
        mode: ipc::SessionMode::Auto,
    };
    Arc::clone(&rig.fleet).tune_session(tune("haiku")).await.expect("held");
    let held = rig
        .fleet
        .take_held_messages(TakeHeld { session_id: ID.into(), wait_ms: None })
        .await
        .unwrap();
    let said: Vec<(String, String)> = held.commands.into_iter().map(|one| (one.command, one.args)).collect();
    assert_eq!(said, vec![("effort".to_string(), "low".to_string())], "the model it already runs on is not sent");

    Arc::clone(&rig.fleet).tune_session(tune("opus")).await.expect("held");
    let held = rig
        .fleet
        .take_held_messages(TakeHeld { session_id: ID.into(), wait_ms: None })
        .await
        .unwrap();
    assert_eq!(held.commands.len(), 2);
}

/// A terminal session whose mod is older than the repository's, or says none, is marked.
#[tokio::test]
async fn a_session_whose_mod_is_older_than_the_repositorys_is_marked() {
    let rig = rig();
    let manifest = std::path::Path::new(&rig.root).join(adapters::MOD_MANIFEST);
    let record = || async {
        Arc::clone(&rig.fleet)
            .get_session(SessionId::carried(ID))
            .await
            .unwrap()
            .session
    };
    let says = |version: Option<&str>| SessionFact::Started {
        cwd: rig.root.clone(),
        title: None,
        origin: SessionOrigin::Terminal,
        mod_version: version.map(String::from),
    };

    rig.reports(says(None)).await;
    assert!(!record().await.mod_out_of_date, "a repository with no mod has none to be older than");

    std::fs::create_dir_all(manifest.parent().unwrap()).unwrap();
    std::fs::write(&manifest, r#"{"name":"armada","version":"0.10.0"}"#).unwrap();
    assert!(record().await.mod_out_of_date, "a mod that reported nothing is older");

    rig.reports(says(Some("0.9.0"))).await;
    assert!(record().await.mod_out_of_date, "0.9 is older than 0.10, read as numbers");

    rig.reports(says(Some("0.10.0"))).await;
    assert!(!record().await.mod_out_of_date);

    // A fact that carries no version leaves the one kept.
    rig.reports(SessionFact::TurnCompleted).await;
    assert!(!record().await.mod_out_of_date);
}

/// The installed copy of the mod is the one compared against, and the repository's file is the fallback.
#[tokio::test]
async fn the_installed_mod_is_compared_before_the_repositorys() {
    let rig = rig();
    let repo = std::path::Path::new(&rig.root).join(adapters::MOD_MANIFEST);
    std::fs::create_dir_all(repo.parent().unwrap()).unwrap();
    std::fs::write(&repo, r#"{"name":"armada","version":"0.5.0"}"#).unwrap();
    let installed = crate::runtime::mod_dir(rig._home.path().to_str().unwrap()).join(adapters::MOD_INSTALLED_MANIFEST);
    std::fs::create_dir_all(installed.parent().unwrap()).unwrap();
    std::fs::write(&installed, r#"{"name":"armada","version":"0.10.0"}"#).unwrap();
    rig.reports(SessionFact::Started {
        cwd: rig.root.clone(),
        title: None,
        origin: SessionOrigin::Terminal,
        mod_version: Some("0.9.0".into()),
    })
    .await;
    let record = Arc::clone(&rig.fleet).get_session(SessionId::carried(ID)).await.unwrap().session;
    assert!(record.mod_out_of_date, "0.9 is older than the installed 0.10, though the repository says 0.5");
    std::fs::remove_file(&installed).unwrap();
    let record = Arc::clone(&rig.fleet).get_session(SessionId::carried(ID)).await.unwrap().session;
    assert!(!record.mod_out_of_date, "with no installed copy the repository's 0.5 is the one");
}

/// The mod reports a constant, since it cannot read the plugin's manifest while it runs: the two are one number.
#[test]
fn the_version_the_mod_reports_is_the_plugins_own() {
    let reported = include_str!("../../../../plugins/armada/hooks/facts.ts")
        .lines()
        .find_map(|line| line.strip_prefix("export const MOD_VERSION = '"))
        .and_then(|rest| rest.strip_suffix('\''))
        .expect("the mod's version constant");
    let manifest = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(adapters::MOD_MANIFEST)).unwrap();
    assert!(manifest.contains(&format!("\"version\": \"{reported}\"")), "bump both together");
}

/// The mod of a session Fleet hosts asks too, and is handed nothing.
#[tokio::test]
async fn a_session_that_is_not_a_terminal_one_is_handed_nothing() {
    let rig = rig();
    assert!(rig.mod_asks().await.is_empty(), "no such session is not an error");
}

/// A subagent of a terminal session: its transcript beside the session's, read as it stands.
#[tokio::test]
async fn a_subagent_of_a_session_reads_as_its_own_thread() {
    let rig = rig();
    let dir = rig.transcript.with_extension("").join("subagents");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(rig.transcript.clone(), "").unwrap();
    let line = |uuid: &str, reason: &str, content: &str| {
        format!(
            r#"{{"type":"assistant","uuid":"{uuid}","isSidechain":true,"timestamp":"2026-10-07T08:48:10.000Z","message":{{"role":"x","stop_reason":{reason},"content":{content}}}}}"#
        )
    };
    let file = dir.join("agent-x1.jsonl");
    std::fs::write(&file, line("s1", "\"tool_use\"", r#"[{"type":"tool_use","id":"t1","name":"Bash","input":{"command":"ls"}}]"#) + "\n").unwrap();
    let read = |agent: &str| {
        let fleet = Arc::clone(&rig.fleet);
        let agent = agent.to_string();
        async move { fleet.get_session_subagent(SessionId::carried(ID), agent).await }
    };
    let running = read("x1").await.expect("it reads");
    assert!(!running.finished);
    assert_eq!(running.rows.len(), 1);

    let mut open = std::fs::OpenOptions::new().append(true).open(&file).unwrap();
    writeln!(open, "{}", line("s2", "\"end_turn\"", r#"[{"type":"text","text":"Done."}]"#)).unwrap();
    let done = read("x1").await.expect("it reads");
    assert!(done.finished);
    assert_eq!(done.report.as_deref(), Some("Done."));
    assert_eq!(done.rows.len(), 2);

    assert!(read("nope").await.is_err(), "a subagent that wrote nothing is refused");
}

/// The mod never settles a subagent, so Fleet does: a ledger row reads done once the subagent's own
/// transcript shows its turn ended, and stays done after the panel has read it.
#[tokio::test]
async fn a_subagent_whose_transcript_ended_reads_as_done_on_the_ledger() {
    let rig = rig();
    rig.started().await;
    rig.reports(SessionFact::Attached {
        attachment: ipc::AttachmentReport {
            kind: "subagent".into(),
            target: "x1".into(),
            detail: Default::default(),
        },
    })
    .await;
    let state = |rig: &Rig| {
        let fleet = Arc::clone(&rig.fleet);
        async move {
            let record = fleet.get_session(SessionId::carried(ID)).await.unwrap().session;
            record.attachments.iter().find(|one| one.kind == "subagent").map(|one| one.state)
        }
    };
    assert_eq!(state(&rig).await, Some(ipc::AttachmentState::Standing));

    let dir = rig.transcript.with_extension("").join("subagents");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(&rig.transcript, "").unwrap();
    let line = |reason: &str| {
        format!(
            r#"{{"type":"assistant","uuid":"s1","isSidechain":true,"timestamp":"2026-10-07T08:48:10.000Z","message":{{"role":"x","stop_reason":{reason},"content":[{{"type":"text","text":"Done."}}]}}}}"#
        ) + "\n"
    };
    let file = dir.join("agent-x1.jsonl");
    std::fs::write(&file, line("\"tool_use\"")).unwrap();
    assert_eq!(state(&rig).await, Some(ipc::AttachmentState::Standing), "a turn still going is running");

    std::fs::write(&file, line("\"end_turn\"")).unwrap();
    assert_eq!(state(&rig).await, Some(ipc::AttachmentState::Spent), "read as done without a timer");
    Arc::clone(&rig.fleet).get_session_subagent(SessionId::carried(ID), "x1".into()).await.unwrap();
    std::fs::remove_file(&file).unwrap();
    assert_eq!(state(&rig).await, Some(ipc::AttachmentState::Spent), "and settled for good once read");
}

const QUESTION: &str = r#"{"questions":[{"question":"Which size?","header":"Size","multiSelect":false,
  "options":[{"label":"S","description":"Small"},{"label":"L","description":"Large"}]}]}"#;

impl Rig {
    /// Fleet restarted: a new one over the same home and store, nothing in memory.
    fn restarted(&mut self) {
        self.fleet = Arc::new(hosted(&self._home));
    }

    async fn mod_says(&self, ask: String) -> Result<ipc::TerminalAsked, api::Refusal> {
        let ask: ipc::TerminalAsk = ipc::decode("a question", ask.as_bytes()).unwrap();
        self.fleet.ask_from_terminal(ask).await
    }

    /// The mod putting a terminal question to Fleet, which answers its call id.
    async fn mod_asks_a_question(&self) -> String {
        let ask = format!(r#"{{"kind":"asks","session_id":"{ID}","input":{QUESTION}}}"#);
        match self.mod_says(ask).await.unwrap() {
            ipc::TerminalAsked::Asked { call } => call,
            other => panic!("a question is answered with its call: {other:?}"),
        }
    }

    /// One poll of the mod's, held in a task.
    fn mod_waits(
        &self,
        call: &str,
    ) -> tokio::task::JoinHandle<Result<ipc::TerminalAsked, api::Refusal>> {
        let fleet = Arc::clone(&self.fleet);
        let ask = format!(r#"{{"kind":"wait","session_id":"{ID}","call":"{call}"}}"#);
        let ask: ipc::TerminalAsk = ipc::decode("a poll", ask.as_bytes()).unwrap();
        tokio::spawn(async move { fleet.ask_from_terminal(ask).await })
    }

    async fn bridge_answers(&self, call: &str, chosen: &str) -> Result<(), api::Refusal> {
        Arc::clone(&self.fleet)
            .answer_session_ask(AnswerSessionAsk {
                session_id: SessionId::carried(ID),
                call: call.into(),
                answer: HelmCallAnswer::AllowOnce,
                note: None,
                answers: vec![QuestionAnswer {
                    question: "Which size?".into(),
                    chosen: vec![chosen.into()],
                }],
            })
            .await
            .map(|_| ())
    }

    /// The question as the session names it, read from the store and not memory.
    async fn waiting_ask_again(&self, call: &str) -> ipc::HelmCallInFlight {
        let rows = self.opened().await;
        let [SessionRow::Ask { ask, state: ipc::SessionAskState::Waiting, .. }] = &rows[..] else {
            panic!("the card stands after a restart: {rows:?}");
        };
        assert_eq!(ask.call, call);
        ask.clone()
    }

    async fn waiting_ask(&self) -> ipc::HelmCallInFlight {
        let found = std::sync::Mutex::new(None);
        eventually(|| async {
            let asked = Arc::clone(&self.fleet)
                .get_session(SessionId::carried(ID))
                .await
                .unwrap()
                .session
                .terminal
                .and_then(|facts| facts.asked);
            *found.lock().unwrap() = asked.clone();
            asked.is_some()
        })
        .await;
        let asked = found.lock().unwrap().take();
        asked.expect("a question is waiting")
    }
}

/// **A terminal's question is a card in its thread**, answered from Bridge, and
/// the answers go back to the mod in the tool's own shape.
#[tokio::test]
async fn a_terminal_question_is_an_ask_row_and_a_bridge_answer_reaches_the_polling_mod() {
    let rig = rig();
    rig.started().await;
    let call = rig.mod_asks_a_question().await;
    let polling = rig.mod_waits(&call);

    let ask = rig.waiting_ask().await;
    assert_eq!(ask.call, call);
    assert_eq!(ask.tool, "AskUserQuestion");
    assert_eq!(ask.questions[0].options[1].label, "L");
    let rows = rig.opened().await;
    assert!(matches!(
        &rows[..],
        [SessionRow::Ask { state: ipc::SessionAskState::Waiting, .. }]
    ));

    rig.bridge_answers(&call, "L").await.expect("a terminal session's ask is answerable");
    let ipc::TerminalAsked::Answered { updated_input } = polling.await.unwrap().unwrap() else {
        panic!("an answered question comes back answered");
    };
    assert_eq!(updated_input.pointer("/answers/Which size?").unwrap(), "L");
    assert!(rig.opened().await.is_empty(), "no waiting card is left");
}

/// **No poll need be under way when Bridge answers.** The answer is kept, the
/// card closes, and the mod's next poll collects it, however many times.
#[tokio::test]
async fn a_bridge_answer_before_the_mod_polls_is_kept_for_the_next_poll() {
    let rig = rig();
    rig.started().await;
    let call = rig.mod_asks_a_question().await;

    rig.bridge_answers(&call, "S").await.expect("answered while nobody polls");
    assert!(rig.opened().await.is_empty(), "the card closes at the answer");

    for _ in 0..2 {
        let ipc::TerminalAsked::Answered { updated_input } = rig.mod_waits(&call).await.unwrap().unwrap()
        else {
            panic!("the kept answer is collected");
        };
        assert_eq!(updated_input.pointer("/answers/Which size?").unwrap(), "S");
    }
}

/// **A Fleet that restarts between the question and the answer** still takes
/// the answer from Bridge and hands it to the mod's next poll.
#[tokio::test]
async fn a_terminal_question_survives_a_fleet_restart() {
    let mut rig = rig();
    rig.started().await;
    let call = rig.mod_asks_a_question().await;

    rig.restarted();
    let ask = rig.waiting_ask_again(&call).await;
    assert_eq!(ask.call, call);
    rig.bridge_answers(&call, "L").await.expect("answered after the restart");

    rig.restarted();
    let ipc::TerminalAsked::Answered { updated_input } = rig.mod_waits(&call).await.unwrap().unwrap() else {
        panic!("the answer outlives a second restart");
    };
    assert_eq!(updated_input.pointer("/answers/Which size?").unwrap(), "L");
}

/// The terminal's own prompt answered first: the mod says so, the card closes
/// and the mod's poll ends.
#[tokio::test]
async fn a_terminal_answer_settles_the_ask_and_ends_the_poll() {
    let rig = rig();
    rig.started().await;
    let call = rig.mod_asks_a_question().await;
    let polling = rig.mod_waits(&call);
    rig.waiting_ask().await;

    rig.fleet
        .ask_from_terminal(ipc::TerminalAsk::Settled {
            session_id: ID.into(),
            answered: true,
        })
        .await
        .unwrap();
    assert!(matches!(
        polling.await.unwrap().unwrap(),
        ipc::TerminalAsked::Gone {}
    ));
    assert!(rig.opened().await.is_empty(), "the card is closed");
}

/// A session that ended has no prompt left to answer, so its card is closed
/// and Bridge's answer would find nothing to send.
#[tokio::test]
async fn an_ended_session_closes_its_terminal_question() {
    let rig = rig();
    rig.started().await;
    let call = rig.mod_asks_a_question().await;

    rig.reports(SessionFact::Ended { reason: "exit".into() }).await;
    assert!(rig.opened().await.is_empty(), "no dead card is offered");
    assert!(matches!(
        rig.mod_waits(&call).await.unwrap().unwrap(),
        ipc::TerminalAsked::Gone {}
    ));
    assert!(rig.bridge_answers(&call, "L").await.is_err());
}
