//! The claim, written before the code: **a side question put to a Job is answered on a thrown-away copy of its
//! conversation, and the Job is none the wiser until the owner tells it.** The CLI is a script that records how it
//! was started, so the tests read the real argument list Fleet renders. `docs/concepts/session.md`, *An aside*.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use adapters::HeadlessAgent;
use api::{Asides, Next, Refusal};
use ipc::{AskAside, Aside, AsideState, Event, SessionId, SessionRow, SessionVoice, TellAside};
use store::{aside_state, KeptAside};

use super::agent_starts::{rigged_fitted, Rig};
use super::session_host::eventually;
use super::tmp::TempDir;

const ANSWER: &str = "Because two tasks share it.";
const QUESTION: &str = "why a mutex here?";
const QUOTE: &str = "let held = lock.lock();";

/// The CLI as a script: it writes what it was given where the test reads it, writes the copy it was told to make
/// (which Fleet must remove), and prints the stream a call prints.
fn script(log: &Path) -> String {
    let log = log.display();
    let projects = adapters::SESSIONS;
    format!(
        r#"#!/bin/sh
# Fleet puts other questions to this same client (naming a Job, say). Only a fork of a conversation is an aside.
forked=""; sid=""; prev=""
for a in "$@"; do
  if [ "$a" = "--fork-session" ]; then forked=1; fi
  if [ "$prev" = "--session-id" ]; then sid="$a"; fi
  prev="$a"
done
if [ -z "$forked" ]; then
  /bin/cat > /dev/null
  echo '{{"type":"assistant","message":{{"content":[{{"type":"text","text":"workflow: none"}}]}}}}'
  exit 0
fi
/bin/mkdir -p "{log}"
printf '%s\n' "$@" > "{log}/args"
/bin/cat > "{log}/stdin"
pwd > "{log}/cwd"
/bin/mkdir -p "$HOME/{projects}/-somewhere"
echo '{{}}' > "$HOME/{projects}/-somewhere/$sid.jsonl"
echo "$sid" > "{log}/wrote"
echo '{{"type":"system","subtype":"init"}}'
echo '{{"type":"system","subtype":"status","status":"requesting"}}'
echo '{{"type":"stream_event","event":{{"delta":{{"type":"text_delta","text":"Because "}}}}}}'
/bin/sleep 0.3
echo '{{"type":"stream_event","event":{{"delta":{{"type":"text_delta","text":"two tasks share it."}}}}}}'
echo '{{"type":"assistant","message":{{"content":[{{"type":"text","text":"{ANSWER}"}}]}}}}'
echo '{{"type":"result","total_cost_usd":0.01,"duration_api_ms":5,"is_error":false,"result":"{ANSWER}"}}'
"#
    )
}

struct Asked {
    rig: Rig,
    tools: TempDir,
}

fn rig() -> Asked {
    let tools = TempDir::new();
    let program = tools.path().join("fake-agent");
    std::fs::write(&program, script(&tools.path().join("log"))).expect("the script");
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).expect("executable");
    let agent = program.to_string_lossy().to_string();
    let rig = rigged_fitted(None, None, None, move |fittings| fittings.judge = Arc::new(HeadlessAgent::at(agent)));
    Asked { rig, tools }
}

impl Asked {
    fn logged(&self, name: &str) -> String {
        std::fs::read_to_string(self.tools.path().join("log").join(name)).unwrap_or_default()
    }

    async fn ask(&self, id: &SessionId, question: &str) -> Result<Aside, Refusal> {
        let ask = AskAside { session_id: id.clone(), question: question.into(), quote: Some(QUOTE.into()) };
        Arc::clone(&self.rig.fleet).ask_aside(ask).await
    }

    async fn asides(&self, id: &SessionId) -> Vec<Aside> {
        self.rig.fleet.list_asides(id.clone()).await.expect("listed").asides
    }

    /// The Job's one aside once it has ended.
    async fn ended(&self, id: &SessionId) -> Aside {
        eventually(|| async { self.asides(id).await.first().is_some_and(|one| one.state != AsideState::Asking) }).await;
        self.asides(id).await.remove(0)
    }

    fn job_file(&self, id: &SessionId) -> PathBuf {
        self.rig.home.path().join(adapters::SESSIONS).join("-somewhere").join(format!("{}.jsonl", id.as_str()))
    }

    fn sent_to_the_job(&self, id: &SessionId, containing: &str) -> usize {
        let process = self.rig.stand_in.process_of(id.as_str()).expect("a process");
        let sent = process.sent.lock().unwrap();
        sent.iter().filter(|line| line.contains(containing)).count()
    }
}

fn code(refusal: Refusal) -> String {
    refusal.error().code.clone()
}

/// Every file under `dir` with this name.
fn found(dir: &Path, name: &str) -> Vec<PathBuf> {
    let mut hits = Vec::new();
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            hits.extend(found(&path, name));
        } else if path.file_name().is_some_and(|file| file == name) {
            hits.push(path);
        }
    }
    hits
}

fn value_after(args: &[&str], flag: &str) -> Option<String> {
    let at = args.iter().position(|arg| *arg == flag)?;
    args.get(at + 1).map(|value| value.to_string())
}

#[tokio::test]
async fn an_aside_is_answered_on_a_copy_and_leaves_the_jobs_own_conversation_and_thread_as_they_were() {
    let asked = rig();
    let (id, _) = asked.rig.an_agent_that_has_spoken().await;
    let (file, thread, record) = (asked.job_file(&id), asked.rig.rows(&id).await, asked.rig.record(&id).await);
    let transcript = std::fs::read(&file).expect("the Job's transcript");
    let mut watching = asked.rig.fleet.events().subscribe();

    let started = asked.ask(&id, QUESTION).await.expect("asked");
    assert_eq!((started.state, started.answer.as_str(), started.quote.as_deref()), (AsideState::Asking, "", Some(QUOTE)));
    let aside = asked.ended(&id).await;
    assert_eq!((aside.state, aside.answer.as_str(), aside.aside_id.as_str()), (AsideState::Answered, ANSWER, started.aside_id.as_str()));
    assert!(aside.answered_at.is_some() && aside.told_at.is_none());

    // How the CLI was started: a fork of the Job's conversation under an id of its own, cut at its last turn, held to the read tools.
    let args = asked.logged("args");
    let args: Vec<&str> = args.lines().collect();
    let copy = value_after(&args, "--session-id").expect("an id for the copy");
    assert_ne!(copy, id.as_str(), "the copy has an id of its own");
    assert_eq!(value_after(&args, "--resume").as_deref(), Some(id.as_str()));
    assert_eq!(value_after(&args, "--resume-session-at").as_deref(), Some("a2"), "the Job's last finished turn");
    for flag in ["--fork-session", "--no-session-persistence", "--strict-mcp-config"] {
        assert!(args.contains(&flag), "{flag}: {args:?}");
    }
    assert_eq!(value_after(&args, "--tools").as_deref(), Some("Read,Grep,Glob"));
    assert_eq!(value_after(&args, "--allowedTools").as_deref(), Some("Read,Grep,Glob"));
    assert_eq!(value_after(&args, "--permission-mode").as_deref(), Some("dontAsk"));
    assert!(!args.iter().any(|arg| ["--plugin-dir", "--mcp-config"].contains(arg)), "{args:?}");
    let root = asked.rig.fleet.repositories().first().expect("a repository").root().to_string();
    assert_eq!(
        std::fs::canonicalize(asked.logged("cwd").trim()).expect("a directory"),
        std::fs::canonicalize(&root).expect("a root"),
        "it starts in the Job's own directory"
    );
    let said = asked.logged("stdin");
    assert!(said.contains(QUESTION) && said.contains(QUOTE) && said.contains("side question"), "{said}");

    // The Job's conversation and thread are what they were, and the copy the CLI wrote is gone.
    assert_eq!(std::fs::read(&file).expect("still there"), transcript, "the Job's transcript is byte for byte as it was");
    assert_eq!(asked.logged("wrote").trim(), copy, "the stand-in did write a copy");
    assert!(found(asked.rig.home.path(), &format!("{copy}.jsonl")).is_empty(), "the copy was removed");
    assert_eq!(format!("{:?}", asked.rig.rows(&id).await), format!("{thread:?}"), "the thread gained no row");
    assert_eq!(asked.rig.record(&id).await.hosted.map(|hosted| hosted.running), record.hosted.map(|hosted| hosted.running));
    assert_eq!(asked.sent_to_the_job(&id, QUESTION), 0, "the agent was told nothing");

    // It went out as it grew and as it ended, whole each time.
    let mut states = Vec::new();
    while states.last() != Some(&AsideState::Answered) {
        match tokio::time::timeout(Duration::from_secs(5), watching.next()).await.expect("an event") {
            Some(Next::Send(delivered)) => {
                if let Event::SessionAside(one) = delivered.event {
                    assert_eq!(one.aside_id, started.aside_id);
                    states.push(one.state);
                }
            }
            other => panic!("the stream ended: {}", other.is_some()),
        }
    }
    assert_eq!(states.first(), Some(&AsideState::Asking));
}

#[tokio::test]
async fn telling_an_answered_aside_sends_the_job_one_message_of_his_and_a_second_tell_is_refused() {
    let asked = rig();
    let (id, _) = asked.rig.an_agent_that_has_spoken().await;
    asked.ask(&id, QUESTION).await.expect("asked");
    let aside = asked.ended(&id).await;
    let before = asked.rig.rows(&id).await.len();
    let tell = || TellAside { session_id: id.clone(), aside_id: aside.aside_id.clone() };

    let told = Arc::clone(&asked.rig.fleet).tell_aside(tell()).await.expect("told");
    assert!(told.told_at.is_some());
    let rows = asked.rig.rows(&id).await;
    let message = rows
        .iter()
        .find_map(|row| match row {
            SessionRow::Message { from: SessionVoice::You, text, .. } if text.contains("Aside, answered") => Some(text.clone()),
            _ => None,
        })
        .expect("his message is in the thread");
    for shown in [QUESTION, ANSWER, QUOTE, "read-only copy of this Job"] {
        assert!(message.contains(shown), "{shown}: {message}");
    }
    assert_eq!(rows.len(), before + 1, "one message and nothing else");
    assert_eq!(asked.asides(&id).await[0].told_at, told.told_at, "kept");

    let again = Arc::clone(&asked.rig.fleet).tell_aside(tell()).await.expect_err("refused");
    assert_eq!(code(again), "fleet.aside_told");
    assert_eq!(asked.rig.rows(&id).await.len(), before + 1, "nothing was sent twice");
}

#[tokio::test]
async fn an_aside_still_asking_or_failed_is_not_told() {
    let asked = rig();
    let (id, _) = asked.rig.an_agent_that_has_spoken().await;
    let row = |aside_id: &str, state: &str| KeptAside {
        aside_id: aside_id.into(),
        session_id: id.as_str().into(),
        question: QUESTION.into(),
        quote: None,
        answer: String::new(),
        state: state.into(),
        failed: (state == aside_state::FAILED).then(|| "the call timed out".to_string()),
        asked_at: "2026-11-15T10:00:00.000Z".into(),
        answered_at: None,
        told_at: None,
    };
    {
        let mut store = asked.rig.fleet.store().lock().await;
        store.keep_aside(&row("going", aside_state::ASKING)).unwrap();
        store.keep_aside(&row("broken", aside_state::FAILED)).unwrap();
    }
    let before = asked.rig.rows(&id).await.len();
    for aside_id in ["going", "broken"] {
        let refused = Arc::clone(&asked.rig.fleet).tell_aside(TellAside { session_id: id.clone(), aside_id: aside_id.into() }).await;
        assert_eq!(code(refused.expect_err("refused")), "fleet.aside_not_answered", "{aside_id}");
    }
    let nothing = Arc::clone(&asked.rig.fleet).tell_aside(TellAside { session_id: id.clone(), aside_id: "nope".into() }).await;
    assert_eq!(code(nothing.expect_err("refused")), "fleet.no_such_aside");
    assert_eq!(asked.rig.rows(&id).await.len(), before, "nothing was sent");
    // An aside "asking" that no turn of this Fleet holds ended with the Fleet that ran it.
    let listed = asked.asides(&id).await;
    assert_eq!(listed.iter().map(|one| one.state).collect::<Vec<_>>(), [AsideState::Failed, AsideState::Failed]);
}

#[tokio::test]
async fn a_blank_question_an_unknown_job_and_a_job_with_no_finished_turn_are_refused_and_keep_nothing() {
    let asked = rig();
    let (id, _) = asked.rig.an_agent_that_has_spoken().await;
    assert_eq!(code(asked.ask(&id, "  ").await.expect_err("blank")), "fleet.aside_blank");
    assert_eq!(code(asked.ask(&SessionId::carried("nobody"), QUESTION).await.expect_err("unknown")), "fleet.no_such_session");
    let quiet = asked.rig.an_agent().await;
    assert_eq!(code(asked.ask(&quiet, QUESTION).await.expect_err("nothing to copy")), "fleet.fork_point_unwritten");
    assert!(asked.asides(&id).await.is_empty() && asked.asides(&quiet).await.is_empty());
    assert!(asked.logged("args").is_empty(), "no call was made");
}
