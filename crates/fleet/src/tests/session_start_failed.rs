//! A Job whose agent would not start keeps the cause on its record, and what was said to it waits:
//! a second word does not reach the agent while the start still fails, and Start again sends every
//! kept turn in order once the agent comes up. The stand-in is `session_host`'s, which fails here
//! for as long as the test says.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use api::HostedSessions;
use ipc::{ManifestId, SendSessionMessage, SessionId, StartAgain, StartSession};
use testkit::{FakeVcs, FakeWorkProduct};

use super::session_host::{eventually, Shared, StandIn};
use crate::daemon::Fleet;
use crate::session_host::{Process, Processes, Sink, Start};
use crate::tests::tmp::TempDir;

const CAUSE: &str = "claude: command not found";

/// The stand-in process seam, refusing to start while `failing` is set.
struct Flaky {
    failing: AtomicBool,
    tried: AtomicUsize,
    inner: Shared,
}

impl Processes for Flaky {
    fn start(&self, start: &Start, sink: Sink) -> Result<Arc<dyn Process>, String> {
        self.tried.fetch_add(1, Ordering::SeqCst);
        match self.failing.load(Ordering::SeqCst) {
            true => Err(CAUSE.to_string()),
            false => self.inner.start(start, sink),
        }
    }
}

struct Rig {
    fleet: Arc<Fleet<testkit::FakeHarness, FakeVcs, FakeWorkProduct>>,
    host: Arc<Flaky>,
    stand_in: Arc<StandIn>,
    manifest: ManifestId,
    _home: TempDir,
}

fn rig() -> Rig {
    let home = TempDir::new();
    let stand_in = Arc::new(StandIn::default());
    let host = Arc::new(Flaky {
        failing: AtomicBool::new(true),
        tried: AtomicUsize::new(0),
        inner: Shared(Arc::clone(&stand_in)),
    });
    let fittings = crate::tests::daemon::fittings(&home, FakeWorkProduct::changed(&[]));
    let fleet = Fleet::assembled(fittings).hosting_sessions_on(Arc::clone(&host) as Arc<dyn Processes>, Duration::from_secs(600));
    let manifest = ManifestId::carried(fleet.repositories().first().expect("a served repository").manifest().id().as_str());
    Rig { fleet: Arc::new(fleet), host, stand_in, manifest, _home: home }
}

impl Rig {
    async fn job(&self) -> SessionId {
        self.fleet
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
                project_rows: Vec::new(),
            })
            .await
            .expect("started")
            .id
    }

    /// The answer to a message to the Job: the route does not fail because the agent would not start.
    async fn say(&self, id: &SessionId, text: &str) -> ipc::SessionRecord {
        Arc::clone(&self.fleet)
            .send_session_message(SendSessionMessage {
                session_id: id.clone(),
                text: text.into(),
                attachments: Vec::new(),
                mentions: Vec::new(),
            })
            .await
            .expect("the message is taken")
    }

    async fn failed(&self, id: &SessionId) -> Option<ipc::StartFailed> {
        let thread = Arc::clone(&self.fleet).get_session(id.clone()).await.expect("read");
        thread.session.hosted.expect("hosted").start_failed
    }

    /// A Job that has said its first word to an agent that would not start.
    async fn failed_once(&self) -> SessionId {
        let id = self.job().await;
        self.say(&id, "first word").await;
        eventually(|| async { self.failed(&id).await.is_some() }).await;
        id
    }
}

#[tokio::test]
async fn a_job_whose_agent_would_not_start_twice_keeps_the_cause_on_its_record() {
    let rig = rig();
    let id = rig.job().await;

    let answered = rig.say(&id, "first word").await;
    eventually(|| async { rig.failed(&id).await.is_some() }).await;

    let failed = rig.failed(&id).await.expect("the failure is kept");
    assert_eq!(failed.cause, CAUSE);
    assert_eq!(rig.host.tried.load(Ordering::SeqCst), 2, "the start is tried the second time and no more");
    assert!(rig.stand_in.starts().is_empty());
    assert_eq!(answered.id, id, "the route answered with the Job");
}

#[tokio::test]
async fn a_second_word_to_a_job_that_failed_to_start_does_not_reach_the_agent_and_the_failure_stays() {
    let rig = rig();
    let id = rig.failed_once().await;

    let answered = rig.say(&id, "second word").await;

    assert_eq!(answered.id, id, "the route answered with the Job");
    let failed = rig.failed(&id).await.expect("the failure stays");
    assert_eq!(failed.cause, CAUSE);
    assert!(rig.stand_in.starts().is_empty(), "no agent was started for it");
}

#[tokio::test]
async fn start_again_sends_the_kept_turns_in_order_and_clears_the_failure() {
    let rig = rig();
    let id = rig.failed_once().await;
    rig.say(&id, "second word").await;
    assert!(rig.failed(&id).await.is_some());
    rig.host.failing.store(false, Ordering::SeqCst);

    let record = Arc::clone(&rig.fleet).start_again(StartAgain { session_id: id.clone() }).await.expect("started again");

    assert!(record.hosted.expect("hosted").start_failed.is_none());
    assert!(rig.failed(&id).await.is_none());
    let sent = rig.stand_in.process(0).sent.lock().unwrap().clone();
    assert_eq!(sent.len(), 2, "both turns were sent: {sent:?}");
    assert!(sent[0].contains("first word"), "the first goes first: {sent:?}");
    assert!(sent[1].contains("second word"), "the second follows: {sent:?}");
}
