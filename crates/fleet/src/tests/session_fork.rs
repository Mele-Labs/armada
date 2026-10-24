//! A session forked from one that is ended or dead: the new one starts as a
//! copy of the old conversation under its own id, and the two ledgers point at
//! each other. Since 23.69. A fork at a message may be taken of a live one,
//! since 23.79. The stand-in process is `session_host`'s.

use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Refusal, Sessions};
use ipc::{
    CloseSession, ForkFrom, ManifestId, MoveSession, SendSessionMessage, SessionFact, SessionId,
    SessionOrigin, SessionRecord, SessionReport, SessionRow, SessionState, SessionVoice, StartSession,
    TakeHeld,
};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::repositories::{Located, SetUp};
use crate::tests::daemon::a_fleet;
use crate::tests::session_host::{Shared, StandIn};
use crate::tests::tmp::TempDir;

type Hosted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const OLD: &str = "aaaa-1111";

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
    async fn reports(&self, fact: SessionFact) {
        self.fleet
            .report_session(SessionReport {
                harness: adapters::HOSTED_HARNESS.into(),
                session_id: SessionId::carried(OLD),
                fact,
            })
            .await
            .unwrap();
    }

    /// A session run in a terminal, with a title.
    async fn a_terminal_session(&self) {
        self.reports(SessionFact::Started {
            cwd: self.root.clone(),
            title: None,
            origin: SessionOrigin::Terminal,
            mod_version: None,
        })
        .await;
        self.reports(SessionFact::Titled {
            title: "Why the reader drops lines".into(),
            named: false,
        })
        .await;
    }

    async fn forks(&self, id: &str) -> Result<SessionRecord, Refusal> {
        self.forks_at(id, None).await
    }

    async fn forks_at(&self, id: &str, at: Option<&str>) -> Result<SessionRecord, Refusal> {
        self.fleet
            .start_session(StartSession {
                manifest_id: self.manifest.clone(),
                title: None,
                model: None,
                effort: None,
                mode: None,
                root: None,
                fork: Some(ForkFrom {
                    session_id: SessionId::carried(id),
                    at: at.map(str::to_string),
                }),
                request: None,
                after: None,
                held: false,
            })
            .await
    }

    async fn record(&self, id: &str) -> SessionRecord {
        Arc::clone(&self.fleet)
            .get_session(SessionId::carried(id))
            .await
            .expect("read")
            .session
    }
}

/// A transcript line the agent CLI wrote: `kind` is `user` or `assistant`.
fn line(kind: &str, uuid: &str, text: &str) -> String {
    let origin = if kind == "user" { r#""origin":{"kind":"human"},"# } else { "" };
    format!(
        r#"{{"type":"{kind}","uuid":"{uuid}","timestamp":"2026-10-12T08:48:10.000Z",{origin}"message":{{"role":"x","content":[{{"type":"text","text":"{text}"}}]}}}}"#
    )
}

impl Rig {
    /// The transcript the CLI keeps for session `id`, from these lines.
    fn transcript(&self, id: &str, lines: &[String]) {
        let dir = self._home.path().join(adapters::SESSIONS).join("-somewhere");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("{id}.jsonl")), lines.join("\n") + "\n").unwrap();
    }

    async fn the_mod_asks(&self) {
        Arc::clone(&self.fleet)
            .take_held_messages(TakeHeld {
                session_id: OLD.into(),
                wait_ms: None,
            })
            .await
            .expect("the mod asks");
    }

    /// A hosted session started and sent its first message, so its process is up.
    async fn a_hosted_session(&self) -> SessionId {
        let hosted = self
            .fleet
            .start_session(StartSession {
                manifest_id: self.manifest.clone(),
                title: None,
                model: None,
                effort: None,
                mode: None,
                root: None,
                fork: None,
                request: None,
                after: None,
                held: false,
            })
            .await
            .expect("started");
        self.first_message(&hosted.id).await;
        self.stand_in.init(0);
        hosted.id
    }

    /// The ids of the agent's messages in the session's thread, once there are `count`.
    async fn agent_rows(&self, id: &SessionId, count: usize) -> Vec<String> {
        for _ in 0..500 {
            let rows = Arc::clone(&self.fleet).get_session(id.clone()).await.expect("read").rows;
            let ids: Vec<String> = rows
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

    /// The fork's first message, so its first process starts.
    async fn first_message(&self, id: &SessionId) {
        // A Session that was not forked is read at its first message, and its turn waits on the reading.
        let runtime = self.fleet.hosts().of(id.as_str());
        let fresh = !runtime.state().proposal.asked;
        self.sent_first(id).await;
        if fresh {
            crate::tests::session_host::eventually(|| async { !runtime.state().proposal.holds_back() }).await;
        }
    }

    async fn sent_first(&self, id: &SessionId) {
        Arc::clone(&self.fleet)
            .send_session_message(SendSessionMessage {
                session_id: id.clone(),
                text: "go on".into(),
                attachments: Vec::new(),
                mentions: Vec::new(),
            })
            .await
            .expect("taken");
    }
}

fn row_of<'a>(record: &'a SessionRecord, kind: &str) -> Option<&'a ipc::Attachment> {
    record.attachments.iter().find(|one| one.kind == kind)
}

#[tokio::test]
async fn a_fork_of_an_ended_terminal_session_starts_from_its_conversation() {
    let rig = rig();
    rig.a_terminal_session().await;
    rig.reports(SessionFact::Ended {
        reason: "prompt_input_exit".into(),
    })
    .await;

    let forked = rig.forks(OLD).await.expect("an ended session forks");
    assert_ne!(forked.id.as_str(), OLD, "its own id");
    assert_eq!(forked.title.as_deref(), Some("Why the reader drops lines"));
    assert!(forked.hosted.is_some(), "Bridge hosts the fork");

    Arc::clone(&rig.fleet)
        .send_session_message(SendSessionMessage {
            session_id: forked.id.clone(),
            text: "go on".into(),
            attachments: Vec::new(),
            mentions: Vec::new(),
        })
        .await
        .expect("taken");
    let starts = rig.stand_in.starts();
    assert_eq!(starts.len(), 1);
    assert_eq!(starts[0].session, forked.id.as_str());
    assert_eq!(starts[0].forking.as_deref(), Some(OLD));
    assert!(!starts[0].resuming);
    assert_eq!(starts[0].directory, rig.root, "from the main checkout");
}

#[tokio::test]
async fn each_ledger_names_the_other_and_the_fork_holds_nothing_of_the_old_ones() {
    let rig = rig();
    rig.a_terminal_session().await;
    rig.reports(SessionFact::Ended {
        reason: "prompt_input_exit".into(),
    })
    .await;
    let forked = rig.forks(OLD).await.expect("forks");

    let old = rig.record(OLD).await;
    let to = row_of(&old, "forked_to").expect("the old one links to the fork");
    assert_eq!(to.target, forked.id.as_str());

    let new = rig.record(forked.id.as_str()).await;
    let from = row_of(&new, "forked_from").expect("the fork links back");
    assert_eq!(from.target, OLD);
    assert!(
        new.attachments.iter().all(|one| one.kind == "forked_from"),
        "no slot, no branch: {:?}",
        new.attachments
    );
}

#[tokio::test]
async fn a_fork_of_a_dead_session_that_never_ended_is_allowed() {
    let rig = rig();
    rig.a_terminal_session().await;
    // Its mod has never asked, so nothing is listening: dead.
    assert!(rig.forks(OLD).await.is_ok());
}

#[tokio::test]
async fn a_session_whose_mod_is_still_asking_is_not_forked() {
    let rig = rig();
    rig.a_terminal_session().await;
    Arc::clone(&rig.fleet)
        .take_held_messages(TakeHeld {
            session_id: OLD.into(),
            wait_ms: None,
        })
        .await
        .expect("the mod asks");
    let refused = format!("{:?}", rig.forks(OLD).await.unwrap_err());
    assert!(refused.contains("fleet.session_fork_live"), "{refused}");
}

#[tokio::test]
async fn a_session_nobody_knows_is_not_forked() {
    let rig = rig();
    let refused = format!("{:?}", rig.forks("nobody").await.unwrap_err());
    assert!(refused.contains("fleet.no_such_session"), "{refused}");
}

#[tokio::test]
async fn a_closed_hosted_session_forks_and_a_live_one_does_not() {
    let rig = rig();
    let hosted = rig
        .fleet
        .start_session(StartSession {
            manifest_id: rig.manifest.clone(),
            title: Some("A thing".into()),
            model: None,
            effort: None,
            mode: None,
            root: None,
            fork: None,
            request: None,
            after: None,
            held: false,
        })
        .await
        .expect("started");
    let refused = format!("{:?}", rig.forks(hosted.id.as_str()).await.unwrap_err());
    assert!(refused.contains("fleet.session_fork_live"), "{refused}");

    Arc::clone(&rig.fleet)
        .close_session(CloseSession {
            session_id: hosted.id.clone(),
        })
        .await
        .expect("closed");
    let forked = rig.forks(hosted.id.as_str()).await.expect("forks");
    assert_eq!(forked.title.as_deref(), Some("A thing"));
}

#[tokio::test]
async fn a_terminal_record_says_whether_its_mod_is_asking() {
    let rig = rig();
    rig.a_terminal_session().await;
    assert!(rig.record(OLD).await.terminal.is_none(), "not asked yet");
    Arc::clone(&rig.fleet)
        .take_held_messages(TakeHeld {
            session_id: OLD.into(),
            wait_ms: None,
        })
        .await
        .expect("the mod asks");
    let asking = rig.record(OLD).await.terminal.expect("facts");
    assert!(asking.listening);
}

fn conversation() -> Vec<String> {
    vec![
        line("user", "u1", "Why does the reader drop lines"),
        line("assistant", "a1", "It stops at the first blank one."),
        line("user", "u2", "Fix it"),
        line("assistant", "a2", "Done."),
    ]
}

#[tokio::test]
async fn a_live_terminal_session_is_forked_at_a_message_and_goes_on_untouched() {
    let rig = rig();
    rig.a_terminal_session().await;
    rig.transcript(OLD, &conversation());
    rig.the_mod_asks().await;
    assert!(rig.forks(OLD).await.is_err(), "the whole of a live one is still refused");

    let forked = rig.forks_at(OLD, Some("a1")).await.expect("a live session forks at a message");
    assert_ne!(forked.id.as_str(), OLD);
    assert!(forked.hosted.is_some(), "Bridge hosts the fork");

    let new = rig.record(forked.id.as_str()).await;
    let from = row_of(&new, "forked_from").expect("the fork links back");
    assert_eq!(from.target, OLD);
    assert_eq!(from.detail.get("at").map(String::as_str), Some("a1"), "the row Bridge cuts by");

    let old = rig.record(OLD).await;
    assert_eq!(row_of(&old, "forked_to").expect("the old one links to the fork").target, forked.id.as_str());
    assert!(old.terminal.expect("facts").listening, "the old one is still taking messages");

    rig.first_message(&forked.id).await;
    let starts = rig.stand_in.starts();
    assert_eq!(starts[0].forking.as_deref(), Some(OLD));
    assert_eq!(starts[0].fork_cut.as_deref(), Some("a1"), "the copy ends at that line");
}

#[tokio::test]
async fn a_fork_of_a_live_session_is_told_whose_work_is_not_its_own() {
    let rig = rig();
    rig.a_terminal_session().await;
    rig.transcript(OLD, &conversation());
    rig.the_mod_asks().await;

    let forked = rig.forks_at(OLD, Some("a1")).await.expect("forks");
    rig.first_message(&forked.id).await;
    let note = rig.stand_in.starts()[0].fork_note.clone().expect("a fork is told it is one");
    for fact in [
        "This job is a fork",
        "\"Why the reader drops lines\"",
        OLD,
        "is live and keeps working",
        "subagents, background tasks and shells, slots",
        "task-notification",
        "do not act on it, relaunch that work",
    ] {
        assert!(note.contains(fact), "{fact}: {note}");
    }
}

#[tokio::test]
async fn a_fork_of_an_ended_session_is_told_it_has_ended() {
    let rig = rig();
    rig.a_terminal_session().await;
    rig.reports(SessionFact::Ended { reason: "prompt_input_exit".into() }).await;

    let forked = rig.forks(OLD).await.expect("forks");
    rig.first_message(&forked.id).await;
    let note = rig.stand_in.starts()[0].fork_note.clone().expect("told");
    assert!(note.contains("has ended") && !note.contains("keeps working"), "{note}");
}

#[tokio::test]
async fn a_session_that_is_no_fork_is_told_nothing_of_one() {
    let rig = rig();
    rig.a_hosted_session().await;
    assert_eq!(rig.stand_in.starts()[0].fork_note, None);
}

#[tokio::test]
async fn a_second_row_of_one_line_cuts_at_the_line() {
    let rig = rig();
    rig.a_terminal_session().await;
    let both = r#"{"type":"assistant","uuid":"a9","timestamp":"2026-10-12T08:48:10.000Z","message":{"role":"x","content":[{"type":"text","text":"First."},{"type":"tool_use","id":"t","name":"Bash","input":{"command":"ls"}},{"type":"text","text":"Second."}]}}"#;
    rig.transcript(OLD, &[line("user", "u1", "Look"), both.to_string()]);
    rig.the_mod_asks().await;

    let forked = rig.forks_at(OLD, Some("a9.2")).await.expect("forks");
    rig.first_message(&forked.id).await;
    assert_eq!(rig.stand_in.starts()[0].fork_cut.as_deref(), Some("a9"));
}

#[tokio::test]
async fn a_message_that_is_not_in_the_conversation_is_refused_and_nothing_is_written() {
    let rig = rig();
    rig.a_terminal_session().await;
    rig.transcript(OLD, &conversation());
    rig.the_mod_asks().await;

    for at in ["nowhere", ""] {
        let refused = format!("{:?}", rig.forks_at(OLD, Some(at)).await.unwrap_err());
        assert!(refused.contains("fleet.fork_point_unwritten"), "{refused}");
    }
    let old = rig.record(OLD).await;
    assert!(row_of(&old, "forked_to").is_none(), "no fork was made: {:?}", old.attachments);
    let nobody = format!("{:?}", rig.forks_at("nobody", Some("a1")).await.unwrap_err());
    assert!(nobody.contains("fleet.no_such_session"), "{nobody}");
}

/// A hosted session's rows are Fleet's own, so the line is found by the words.
#[tokio::test]
async fn a_live_hosted_session_is_forked_at_the_line_its_row_was_written_as() {
    let rig = rig();
    let hosted = rig.a_hosted_session().await;
    rig.stand_in.finishes(0, "Looking now.");
    rig.first_message(&hosted).await;
    rig.stand_in.finishes(0, "Fixed.");
    let agent = rig.agent_rows(&hosted, 2).await;
    let id = hosted.as_str();
    // The CLI also wrote a line the thread never showed, and a copied opening.
    rig.transcript(
        id,
        &[
            line("user", "c0", "go on"),
            line("assistant", "c1", "Looking now."),
            line("user", "u1", "go on"),
            line("assistant", "a0", "Standing down."),
            line("assistant", "a1", "Looking now."),
            line("user", "u2", "The person named: x.\\n\\ngo on"),
            line("assistant", "a2", "Fixed."),
        ],
    );

    let first = rig.forks_at(id, Some(&agent[0])).await.expect("a live hosted session forks at a message");
    rig.first_message(&first.id).await;
    let second = rig.forks_at(id, Some(&agent[1])).await.expect("and at the next");
    rig.first_message(&second.id).await;
    let starts = rig.stand_in.starts();
    assert_eq!(starts[1].fork_cut.as_deref(), Some("a1"), "not the copied line with the same words");
    assert_eq!(starts[2].fork_cut.as_deref(), Some("a2"));
    let from = row_of(&rig.record(first.id.as_str()).await, "forked_from").expect("link").clone();
    assert_eq!(from.detail.get("at").map(String::as_str), Some(agent[0].as_str()));
}

#[tokio::test]
async fn a_hosted_row_the_transcript_does_not_hold_is_refused() {
    let rig = rig();
    let hosted = rig.a_hosted_session().await;
    rig.stand_in.finishes(0, "Looking now.");
    let agent = rig.agent_rows(&hosted, 1).await;
    rig.transcript(hosted.as_str(), &[line("user", "u1", "go on"), line("assistant", "a1", "Something else.")]);
    let refused = format!("{:?}", rig.forks_at(hosted.as_str(), Some(&agent[0])).await.unwrap_err());
    assert!(refused.contains("fleet.fork_point_unwritten"), "{refused}");
}

const SECOND: &str = "01SECONDMANIFEST";

impl Rig {
    /// A second served repository, its id and its root.
    fn a_second_repository(&self) -> (ManifestId, String) {
        let root = self._home.path().join("second");
        let text = format!("version: 1\nid: {SECOND}\n");
        let manifest = config::Manifest::parse(&root.join("armada.yml"), &text).expect("the second loads");
        self.fleet
            .repositories()
            .add(Located {
                root: root.to_string_lossy().to_string(),
                records_root: self._home.path().join("second-records").to_string_lossy().to_string(),
                set_up: Some(SetUp::of(manifest, Default::default())),
            })
            .expect("the second is served");
        (ManifestId::carried(SECOND), root.to_string_lossy().to_string())
    }

    async fn moves(&self, id: &str, to: &ManifestId) -> Result<SessionRecord, Refusal> {
        Arc::clone(&self.fleet)
            .move_session(MoveSession { session_id: SessionId::carried(id), manifest_id: to.clone() })
            .await
    }
}

#[tokio::test]
async fn a_move_of_a_live_hosted_session_ends_it_and_starts_the_fork_in_the_other_repository() {
    let rig = rig();
    let (second, second_root) = rig.a_second_repository();
    let hosted = rig.a_hosted_session().await;
    let old = rig.record(hosted.as_str()).await;
    assert_eq!(old.state, SessionState::Live);

    let moved = rig.moves(hosted.as_str(), &second).await.expect("a live hosted session moves");
    assert_ne!(moved.id, hosted, "its own id");
    assert_eq!(moved.manifest_id.as_ref().map(|id| id.as_str()), Some(SECOND));
    assert_eq!(moved.title, old.title, "the same title");
    assert!(moved.hosted.is_some(), "Bridge hosts it");

    let ended = rig.record(hosted.as_str()).await;
    assert_eq!(ended.state, SessionState::Ended, "the old one is ended");
    assert_eq!(ended.end_reason.as_deref(), Some("closed"));
    let to = row_of(&ended, "forked_to").expect("the old one links to the new");
    assert_eq!(to.target, moved.id.as_str());
    assert_eq!(to.detail.get("moved").map(String::as_str), Some(SECOND), "a move, not a fork");

    let new = rig.record(moved.id.as_str()).await;
    let from = row_of(&new, "forked_from").expect("the new one links back");
    assert_eq!(from.target, hosted.as_str());
    assert_eq!(from.detail.get("moved").map(String::as_str), Some(rig.manifest.as_str()));
    assert!(from.detail.get("at").is_none(), "the whole conversation");

    rig.first_message(&moved.id).await;
    let starts = rig.stand_in.starts();
    assert_eq!(starts.len(), 2);
    assert_eq!(starts[1].session, moved.id.as_str());
    assert_eq!(starts[1].forking.as_deref(), Some(hosted.as_str()));
    assert_eq!(starts[1].directory, second_root, "in the other repository's checkout");
    let note = starts[1].fork_note.clone().expect("told it was moved");
    for fact in [
        format!("moved from repository {} to repository {second_root}", rig.root).as_str(),
        "has ended",
        "stayed in",
        "Do not write in",
    ] {
        assert!(note.contains(fact), "{fact}: {note}");
    }
    assert!(!note.contains("This job is a fork"), "{note}");
}

#[tokio::test]
async fn a_move_into_the_repository_a_session_is_in_is_refused_and_ends_nothing() {
    let rig = rig();
    rig.a_second_repository();
    let hosted = rig.a_hosted_session().await;
    let refused = format!("{:?}", rig.moves(hosted.as_str(), &rig.manifest).await.unwrap_err());
    assert!(refused.contains("fleet.session_move_same"), "{refused}");
    assert_eq!(rig.record(hosted.as_str()).await.state, SessionState::Live);
}

#[tokio::test]
async fn a_move_of_a_terminal_session_whose_mod_is_asking_is_refused() {
    let rig = rig();
    let (second, _) = rig.a_second_repository();
    rig.a_terminal_session().await;
    rig.the_mod_asks().await;
    let refused = format!("{:?}", rig.moves(OLD, &second).await.unwrap_err());
    assert!(refused.contains("fleet.session_move_listening"), "{refused}");
    let old = rig.record(OLD).await;
    assert!(row_of(&old, "forked_to").is_none(), "{:?}", old.attachments);
}

#[tokio::test]
async fn a_move_to_a_repository_nobody_serves_or_of_a_session_nobody_knows_is_a_404() {
    let rig = rig();
    let (second, _) = rig.a_second_repository();
    let hosted = rig.a_hosted_session().await;
    let nowhere = rig.moves(hosted.as_str(), &ManifestId::carried("01NOWHERE")).await.unwrap_err();
    assert_eq!(nowhere.status(), 404);
    assert!(format!("{nowhere:?}").contains("fleet.no_such_manifest"), "{nowhere:?}");
    assert_eq!(rig.record(hosted.as_str()).await.state, SessionState::Live);

    let nobody = rig.moves("nobody", &second).await.unwrap_err();
    assert_eq!(nobody.status(), 404);
    assert!(format!("{nobody:?}").contains("fleet.no_such_session"), "{nobody:?}");
}

#[tokio::test]
async fn a_move_of_a_dead_terminal_session_needs_no_end_and_is_told_the_branch_stayed() {
    let rig = rig();
    let (second, second_root) = rig.a_second_repository();
    rig.a_terminal_session().await;
    // The mod has never asked, so nothing is listening; a branch it held is on its ledger.
    {
        let mut store = rig.fleet.store().lock().await;
        store
            .attach(
                &store::KeptAttachment {
                    holder: store::Holder::session(OLD),
                    kind: "branch".into(),
                    manifest_id: rig.manifest.as_str().into(),
                    target: "armada/the-reader".into(),
                    state: store::AttachmentState::Standing,
                    detail: Default::default(),
                    since: "2026-10-15T09:00:00.000Z".into(),
                    changed_at: "2026-10-15T09:00:00.000Z".into(),
                },
                false,
            )
            .expect("kept");
    }
    let moved = rig.moves(OLD, &second).await.expect("a dead terminal session moves");
    rig.first_message(&moved.id).await;
    let note = rig.stand_in.starts()[0].fork_note.clone().expect("told");
    assert!(note.contains("its branch (armada/the-reader)"), "{note}");
    assert!(note.contains(&second_root), "{note}");
    let from = row_of(&rig.record(moved.id.as_str()).await, "forked_from").expect("link").clone();
    assert_eq!(from.detail.get("branch").map(String::as_str), Some("armada/the-reader"));
}

#[tokio::test]
async fn a_move_starts_the_new_job_while_the_old_ones_park_is_held() {
    let rig = rig();
    let (second, _) = rig.a_second_repository();
    let hosted = rig.a_hosted_session().await;
    let at = adapter_traits::slot_path(&rig.root, 1);
    std::fs::create_dir_all(&at).expect("a slot");
    std::fs::write(format!("{at}.lease"), format!("branch sessions/own\nholder job {}\nsince 1\n", hosted.as_str()))
        .expect("a lease record");
    let gate = rig.fleet.vcs().hold_parks();

    let moved = tokio::time::timeout(Duration::from_secs(2), rig.moves(hosted.as_str(), &second))
        .await
        .expect("the move does not wait on the park")
        .expect("moved");

    assert_ne!(moved.id, hosted);
    assert_eq!(rig.record(hosted.as_str()).await.state, SessionState::Ended);
    assert!(rig.fleet.vcs().parked_slots().is_empty(), "the old one's park is still held");
    gate.open();
    crate::tests::session_host::eventually(|| async { !rig.fleet.vcs().parked_slots().is_empty() }).await;
}
