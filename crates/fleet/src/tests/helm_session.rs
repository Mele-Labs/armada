//! Helm as a hosted Session: one per repository, made by the first message the dock sends, reused
//! by the next, ended by Start fresh, told what it is in a note and where the person is in a line,
//! permitted by the Auto gate, and given Helm's door. `docs/concepts/helm.md`.
//!
//! **The process is the stand-in `session_host` has**, so a turn is what a test tells it to say.
//! What a Session does once it exists (a lease, a resume, a quiet spell) is `session_host`'s and is
//! held there; what is held here is what makes this one Helm.

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use api::{Admitting, Conversations, HostedSessions, Refusal, Sessions};
use ipc::{
    AskHelm, HelmContext, HelmScreen, HelmSession, ManifestId, SessionRecord, SessionRow,
    SessionState, SessionVoice, TuneSession,
};

use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use super::peer::Placing;
use super::session_host::{denied, eventually, rig_placing, Rig};
use crate::daemon::Fleet;
use crate::helm::{because_in_a_session, Because};
use crate::session_host::{Process, Processes, Sink, Start};
use crate::repositories::{Located, SetUp};

/// A process that is running as `pid`, and says nothing.
struct Still(u32);

impl Process for Still {
    fn pid(&self) -> Option<u32> {
        Some(self.0)
    }
    fn send(&self, _line: String) {}
    fn end(&self) {}
}

/// Hosts whose every process runs as one pid: **a Helm Session that is running**, which is what
/// places a connection beneath it as Helm's. The door cases need no more of a process than that.
struct Running(u32);

impl Processes for Running {
    fn start(&self, _start: &Start, _sink: Sink) -> Result<Arc<dyn Process>, String> {
        Ok(Arc::new(Still(self.0)))
    }
}

/// `fleet`, hosting Sessions through [`Running`], with its first repository's Helm Session
/// started so that `pid` is Helm's. **Placing the connections beneath it is the caller's.**
pub(super) async fn with_helm_running(
    fleet: Fleet<FakeHarness, FakeVcs, FakeWorkProduct>,
    pid: u32,
) -> Arc<Fleet<FakeHarness, FakeVcs, FakeWorkProduct>> {
    let fleet = Arc::new(fleet.hosting_sessions_on(Arc::new(Running(pid)), Duration::from_secs(600)));
    Arc::clone(&fleet).ask_helm(words("hello"), None).await.expect("Helm is running");
    fleet
}

fn words(text: &str) -> AskHelm {
    AskHelm {
        text: text.to_string(),
        attachments: Vec::new(),
        mentions: Vec::new(),
        context: None,
        model: None,
        effort: None,
    }
}

impl Rig {
    pub(super) async fn ask(&self, text: &str) -> SessionRecord {
        self.asking(words(text), None).await.expect("taken")
    }

    async fn asking(&self, asked: AskHelm, within: Option<ManifestId>) -> Result<SessionRecord, Refusal> {
        Arc::clone(&self.fleet).ask_helm(asked, within).await
    }

    async fn helm(&self) -> HelmSession {
        Arc::clone(&self.fleet).get_helm_session(None).await.expect("read")
    }

    pub(super) async fn helm_rows(&self) -> Vec<SessionRow> {
        self.helm().await.session.map(|thread| thread.rows).unwrap_or_default()
    }
}

#[tokio::test]
async fn the_first_message_starts_a_helm_session_and_the_next_one_reuses_it() {
    let rig = rig_placing(None);
    assert!(rig.helm().await.session.is_none(), "none before the first message");
    assert!(rig.stand_in.starts().is_empty());

    let first = rig.ask("why did 77 stop?").await;
    assert!(first.helm, "marked as Helm");
    assert_eq!(first.title.as_deref(), Some("Helm"), "the first message does not retitle it");
    assert_eq!(first.state, SessionState::Live);
    let second = rig.ask("and 78?").await;
    assert_eq!(first.id, second.id, "the same Session");

    assert_eq!(rig.stand_in.starts().len(), 1, "one process for both");
    let listed = rig.fleet.list_sessions(None, None, None).await.unwrap().sessions;
    assert_eq!(listed.len(), 1);
    assert!(listed[0].helm, "the Sessions list marks it");

    let thread = rig.helm().await.session.expect("found by its repository");
    assert_eq!(thread.session.id, first.id);
    assert!(matches!(
        &thread.rows[..],
        [
            SessionRow::Message { from: SessionVoice::You, text: one, .. },
            SessionRow::Message { from: SessionVoice::You, text: two, .. }
        ] if one == "why did 77 stop?" && two == "and 78?"
    ));
}

/// Two first messages at once make one Session.
#[tokio::test]
async fn two_first_messages_at_once_make_one_session() {
    let rig = rig_placing(None);
    let (a, b) = tokio::join!(rig.asking(words("one"), None), rig.asking(words("two"), None));
    assert_eq!(a.expect("taken").id, b.expect("taken").id);
    assert_eq!(rig.fleet.list_sessions(None, None, None).await.unwrap().sessions.len(), 1);
}

#[tokio::test]
async fn a_second_repository_has_a_helm_session_of_its_own() {
    let rig = rig_placing(None);
    let first = rig.ask("hello").await;

    let served = rig.fleet.repositories().first().expect("a served repository");
    let root = format!("{}-other", rig.root);
    std::fs::create_dir_all(&root).expect("a folder");
    let text = "version: 1\nid: 01OTHERMANIFEST\n";
    let manifest = config::Manifest::parse(&std::path::Path::new(&root).join("armada.yml"), text)
        .expect("a Manifest");
    rig.fleet
        .repositories()
        .add(Located {
            root: root.clone(),
            records_root: root.clone(),
            set_up: Some(SetUp::of(manifest, (*served.workflows()).clone())),
        })
        .expect("served");

    let other = rig
        .asking(words("hello there"), Some(ManifestId::carried("01OTHERMANIFEST")))
        .await
        .expect("taken");
    assert_ne!(first.id, other.id, "its own Session");
    assert!(other.helm);
    assert_eq!(other.manifest_id.as_ref().map(|id| id.as_str()), Some("01OTHERMANIFEST"));
    assert_eq!(rig.ask("back again").await.id, first.id, "the first is still the first's");
}

#[tokio::test]
async fn a_message_with_nothing_in_it_makes_no_session() {
    let rig = rig_placing(None);
    let refused = rig.asking(words("   "), None).await.unwrap_err();
    assert!(format!("{refused:?}").contains("fleet.session_message_empty"), "{refused:?}");
    assert!(rig.helm().await.session.is_none());
    assert!(rig.fleet.list_sessions(None, None, None).await.unwrap().sessions.is_empty());
}

#[tokio::test]
async fn starting_fresh_ends_it_and_the_next_message_starts_a_new_one_on_the_same_tuning() {
    let rig = rig_placing(None);
    let first = rig.ask("hello").await;
    rig.fleet
        .tune_session(TuneSession {
            session_id: first.id.clone(),
            model: Some("opus".into()),
            effort: Some("high".into()),
            mode: ipc::SessionMode::Auto,
        })
        .await
        .expect("tuned");

    let fresh = Arc::clone(&rig.fleet).start_helm_fresh(None).await.expect("ended");
    assert!(fresh.session.is_none());
    assert!(rig.helm().await.session.is_none(), "none live");
    let ended = rig.fleet.list_sessions(None, None, None).await.unwrap().sessions;
    assert_eq!(ended[0].state, SessionState::Ended);
    assert_eq!(ended[0].end_reason.as_deref(), Some("closed"));
    assert!(rig.stand_in.process(0).ended.load(Ordering::SeqCst), "its process is ended");

    let next = rig.ask("again").await;
    assert_ne!(first.id, next.id, "a new Session");
    let starts = rig.stand_in.starts();
    let last = starts.last().expect("started");
    assert_eq!(last.session, next.id.as_str());
    assert_eq!(
        (last.model.as_deref(), last.effort.as_deref()),
        (Some("opus"), Some("high")),
        "kept through Start fresh"
    );

    // And a repository with none live is answered the same, and nothing happens.
    Arc::clone(&rig.fleet).start_helm_fresh(None).await.expect("ended");
    let again = Arc::clone(&rig.fleet).start_helm_fresh(None).await.expect("nothing to end");
    assert!(again.session.is_none());
}

/// The model and effort ride on the message that starts the Session, and are checked.
#[tokio::test]
async fn the_message_that_starts_it_may_say_what_it_runs_on() {
    let rig = rig_placing(None);
    let refused = rig
        .asking(AskHelm { effort: Some("extreme".into()), ..words("hi") }, None)
        .await
        .unwrap_err();
    assert!(format!("{refused:?}").contains("fleet.unacceptable_effort"), "{refused:?}");
    assert!(rig.helm().await.session.is_none(), "a refused message makes no Session");

    let made = rig
        .asking(
            AskHelm { model: Some("opus".into()), effort: Some("low".into()), ..words("hi") },
            None,
        )
        .await
        .expect("taken");
    let hosted = made.hosted.expect("hosted facts");
    assert_eq!((hosted.model.as_deref(), hosted.effort.as_deref()), (Some("opus"), Some("low")));
    // A live Session keeps the tuning it has.
    let kept = rig
        .asking(AskHelm { model: Some("sonnet".into()), ..words("again") }, None)
        .await
        .expect("taken");
    assert_eq!(kept.hosted.expect("hosted facts").model.as_deref(), Some("opus"));
}

#[tokio::test]
async fn the_line_saying_where_the_person_is_goes_ahead_of_their_words_and_is_not_on_the_thread() {
    let rig = rig_placing(None);
    let context = HelmContext {
        screen: HelmScreen::Overview,
        picked: Some(ManifestId::carried("armada")),
        studio: None,
        node: None,
    };
    rig.asking(AskHelm { context: Some(context), ..words("why did it stall?") }, None)
        .await
        .expect("taken");

    let line = rig.stand_in.process(0).sent.lock().unwrap()[0].clone();
    let at = line.find("The person is on Overview, picked on armada").expect("the line");
    assert!(at < line.find("why did it stall?").expect("the words"), "ahead of them: {line}");

    let rows = rig.helm_rows().await;
    let [SessionRow::Message { text, .. }] = &rows[..] else {
        panic!("one row: {rows:?}");
    };
    assert_eq!(text, "why did it stall?", "the thread keeps what was typed");

    // No context, no line.
    rig.ask("and now?").await;
    let plain = rig.stand_in.process(0).sent.lock().unwrap()[1].clone();
    assert!(!plain.contains("The person is on"), "{plain}");
}

#[tokio::test]
async fn the_note_is_appended_to_its_system_prompt_and_no_other_sessions_is() {
    let rig = rig_placing(None);
    let helm = rig.ask("hello").await;
    let ordinary = rig.start().await;
    rig.send(&ordinary, "hi").await;

    let starts = rig.stand_in.starts();
    let told = starts[0].note.as_deref().expect("a note");
    assert!(
        told.starts_with("You are Helm, docked in Armada over repository 01FIXTUREMANIFEST"),
        "{told}"
    );
    assert!(told.contains("tell_session") && told.contains("list_jobs"), "{told}");
    assert_eq!(starts[0].session, helm.id.as_str());
    assert!(
        !starts[1].note.as_deref().unwrap_or_default().contains("You are Helm"),
        "an ordinary Session has no Helm note; Armada's session rules are added where the launch is rendered, to every Session"
    );

    // The next process of the same Session is told it again: tuning ends an idle one.
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "hello");
    eventually(|| async { rig.turn(&helm.id).await == ipc::SessionTurn::Idle }).await;
    rig.fleet
        .tune_session(TuneSession {
            session_id: helm.id.clone(),
            model: None,
            effort: None,
            mode: ipc::SessionMode::Auto,
        })
        .await
        .expect("tuned");
    rig.ask("once more").await;
    eventually(|| async { rig.stand_in.starts().len() == 3 }).await;
    assert_eq!(rig.stand_in.starts()[2].session, helm.id.as_str());
    assert_eq!(rig.stand_in.starts()[2].note.as_deref(), Some(told));
}

/// **Auto, as a Session's is**: a call runs, and only what reaches into the owner's files or
/// takes something away outside its slot is refused. Helm's three classes are not asked of it.
#[tokio::test]
async fn its_permission_is_the_auto_gate_and_nothing_waits_on_the_dock() {
    let rig = rig_placing(None);
    let helm = rig.ask("hello").await;
    let hosted = helm.hosted.expect("hosted facts");
    assert_eq!(hosted.mode, ipc::SessionMode::Auto);

    let asking = |tool: &str, input: &str| -> ipc::AskingToRun {
        let body = format!(r#"{{"tool_name":"{tool}","input":{input}}}"#);
        ipc::decode("an ask", body.as_bytes()).expect("an ask")
    };
    let ran = rig
        .fleet
        .session_permission(helm.id.as_str(), asking("Bash", r#"{"command":"ls -la"}"#))
        .await
        .unwrap();
    assert!(matches!(ran, ipc::RunOrNot::Allow { .. }), "it runs unasked");
    let removed = rig
        .fleet
        .session_permission(helm.id.as_str(), asking("Bash", r#"{"command":"rm -rf build"}"#))
        .await
        .unwrap();
    assert!(matches!(removed, ipc::RunOrNot::Deny { .. }), "what removes outside its slot is refused");
    assert!(rig.fleet.helm().asks().waiting().is_empty(), "Helm's dock carries no card");

    // A push is read as the class it is. Whether Auto stops it is a Session's rule, not Helm's.
    assert_eq!(
        because_in_a_session(&asking("Bash", r#"{"command":"git push origin main"}"#), &[]),
        Some(Because::PushesToShared)
    );
}

#[tokio::test]
async fn its_first_write_leases_a_slot_and_an_edit_in_the_slot_runs() {
    let rig = rig_placing(None);
    let helm = rig.ask("change a file").await;
    rig.stand_in.init(0);

    let path = format!("{}/src/lib.rs", rig.root);
    let first = rig.gate(&helm.id, "Edit", &format!(r#"{{"file_path":"{path}"}}"#)).await;
    assert!(denied(&first), "the write into the main checkout is held, and leases");
    let record = Arc::clone(&rig.fleet).get_session(helm.id.clone()).await.unwrap();
    let held: Vec<_> = record
        .session
        .attachments
        .iter()
        .map(|one| (one.kind.as_str(), one.target.as_str()))
        .collect();
    assert!(held.contains(&("slot", "1")), "{held:?}");
    assert!(record.rows.iter().any(|row| matches!(row, SessionRow::Lease { slot: 1, .. })));

    // The turn ends; it resumes in the slot, with its note still on it.
    rig.stand_in.finishes(0, "standing down");
    eventually(|| async { rig.stand_in.starts().len() == 2 }).await;
    let moved = &rig.stand_in.starts()[1];
    assert_eq!(moved.directory, adapter_traits::slot_path(&rig.root, 1));
    assert!(moved.note.is_some(), "the process in the slot is Helm's too");
    rig.stand_in.init(1);

    let inside = format!("{}/src/lib.rs", adapter_traits::slot_path(&rig.root, 1));
    assert!(!denied(&rig.gate(&helm.id, "Edit", &format!(r#"{{"file_path":"{inside}"}}"#)).await));
    let asked: ipc::AskingToRun = ipc::decode(
        "an ask",
        format!(r#"{{"tool_name":"Edit","input":{{"file_path":"{inside}"}}}}"#).as_bytes(),
    )
    .unwrap();
    let ran = rig.fleet.session_permission(helm.id.as_str(), asked).await.unwrap();
    assert!(matches!(ran, ipc::RunOrNot::Allow { .. }), "an Edit in its slot runs");
}

/// **Placed by its connection, and given Helm's door**: a Session's, plus the acts a person may ask
/// Helm for, and with them the `Helm only` tools. Any other Session keeps a Session's.
#[tokio::test]
async fn its_door_is_a_sessions_plus_helms_extras_and_another_sessions_is_not() {
    let peers = Placing::nothing();
    let rig = rig_placing(Some(Arc::clone(&peers)));
    let port = 12345;
    // The stand-in's processes are pids 1000 and 1001; each has a relay beneath it.
    peers.holding(2000, 52000, port);
    peers.started(1000, 2000);
    peers.holding(2001, 52001, port);
    peers.started(1001, 2001);
    let helm = rig.ask("hello").await;
    let ordinary = rig.start().await;
    rig.send(&ordinary, "hi").await;
    let caller = |from: u16| api::Caller::at(format!("127.0.0.1:{from}").parse().unwrap());

    let reach = rig.fleet.helm_at(caller(52000)).expect("Helm's connection");
    assert!(!reach.is_a_hosted_session(), "its acts are Helm's, as they were");
    for extra in ["tell_session", "ask_the_person", "list_sessions"] {
        assert!(reach.may(extra), "`{extra}` is Helm's");
    }

    let session = rig.fleet.helm_at(caller(52001)).expect("a Session's connection");
    assert!(session.is_a_hosted_session());
    assert!(session.may("list_sessions") && session.may("ask_the_person"));
    assert!(!session.may("tell_session"));
    drop(helm);
}

#[tokio::test]
async fn an_ended_helm_session_places_no_caller() {
    let rig = rig_placing(None);
    rig.ask("hello").await;
    Arc::clone(&rig.fleet).start_helm_fresh(None).await.expect("ended");
    let caller = api::Caller::at("127.0.0.1:52000".parse().unwrap());
    assert!(rig.fleet.helm_at(caller).is_none(), "no process, no Helm");
}
