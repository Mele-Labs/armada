//! A session forked from one that is ended or dead: the new one starts as a
//! copy of the old conversation under its own id, and the two ledgers point at
//! each other. Since 23.69. The stand-in process is `session_host`'s.

use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Refusal, Sessions};
use ipc::{
    CloseSession, ForkFrom, ManifestId, SendSessionMessage, SessionFact, SessionId, SessionOrigin,
    SessionRecord, SessionReport, StartSession, TakeHeld,
};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
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
        self.fleet
            .start_session(StartSession {
                manifest_id: self.manifest.clone(),
                title: None,
                model: None,
                effort: None,
                mode: None,
                pilot: None,
                fork: Some(ForkFrom {
                    session_id: SessionId::carried(id),
                }),
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
            pilot: None,
            fork: None,
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
