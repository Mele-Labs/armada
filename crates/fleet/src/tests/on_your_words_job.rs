//! A Job on your words: started with the flag by Bridge, placed by its connection and given the
//! placed door, and a Job like any other everywhere else.
//!
//! **The process is the stand-in `session_host` has**, so a turn is what a test tells it to say.
//! What a Job does once it exists (a lease, a resume, a quiet spell) is `session_host`'s and is held
//! there; what is held here is what the flag changes.

use std::sync::Arc;
use std::time::Duration;

use api::{Admitting, HostedSessions};
use ipc::{CloseSession, ManifestId, SendSessionMessage, SessionId, StartSession};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use super::peer::Placing;
use super::session_host::{rig_placing, Rig};
use crate::daemon::Fleet;
use crate::session_host::{Process, Processes, Sink, Start};

/// A process that is running as `pid`, and says nothing.
struct Still(u32);

impl Process for Still {
    fn pid(&self) -> Option<u32> {
        Some(self.0)
    }
    fn send(&self, _line: String) {}
    fn end(&self) {}
}

/// Hosts whose every process runs as one pid: **a Job on your words that is running**, which is what
/// places a connection beneath it. The door cases need no more of a process than that.
struct Running(u32);

impl Processes for Running {
    fn start(&self, _start: &Start, _sink: Sink) -> Result<Arc<dyn Process>, String> {
        Ok(Arc::new(Still(self.0)))
    }
}

fn start_of(manifest: ManifestId, on_your_words: bool) -> StartSession {
    StartSession {
        manifest_id: manifest,
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
        on_your_words: on_your_words.then_some(true),
    }
}

/// `fleet`, hosting Jobs through [`Running`], with a Job on your words started and told something
/// so that `pid` is its process. **Placing the connections beneath it is the caller's.**
pub(super) async fn with_a_job_on_your_words_running(
    fleet: Fleet<FakeHarness, FakeVcs, FakeWorkProduct>,
    pid: u32,
) -> Arc<Fleet<FakeHarness, FakeVcs, FakeWorkProduct>> {
    let fleet = Arc::new(fleet.hosting_sessions_on(Arc::new(Running(pid)), Duration::from_secs(600)));
    let one = fleet.repositories().first().expect("a served repository");
    let manifest = ManifestId::carried(one.manifest().id().as_str());
    let made = fleet.start_session(start_of(manifest, true)).await.expect("started");
    Arc::clone(&fleet)
        .send_session_message(SendSessionMessage {
            session_id: made.id,
            text: String::from("hello"),
            attachments: Vec::new(),
            mentions: Vec::new(),
        })
        .await
        .expect("it is running");
    fleet
}

impl Rig {
    async fn start_on_your_words(&self, on_your_words: bool) -> SessionId {
        self.fleet
            .start_session(start_of(self.manifest.clone(), on_your_words))
            .await
            .expect("started")
            .id
    }
}

#[tokio::test]
async fn the_flag_is_on_the_record_of_a_job_started_on_your_words_and_of_no_other() {
    let rig = rig_placing(None);
    let on = rig.start_on_your_words(true).await;
    let plain = rig.start_on_your_words(false).await;
    let of = |id: &SessionId| {
        let id = id.clone();
        let fleet = Arc::clone(&rig.fleet);
        async move { fleet.get_session(id).await.expect("read").session.on_your_words }
    };
    assert!(of(&on).await);
    assert!(!of(&plain).await);
}

/// **Placed by its connection, and given the placed door**: a Job's, plus the acts a person may ask
/// for, and with them the `On your words` tools. Any other Job keeps a Job's.
#[tokio::test]
async fn its_door_is_a_jobs_plus_the_extras_and_another_jobs_is_not() {
    let peers = Placing::nothing();
    let rig = rig_placing(Some(Arc::clone(&peers)));
    let port = 12345;
    // The stand-in's processes are pids 1000 and 1001; each has a relay beneath it.
    peers.holding(2000, 52000, port);
    peers.started(1000, 2000);
    peers.holding(2001, 52001, port);
    peers.started(1001, 2001);
    let on = rig.start_on_your_words(true).await;
    rig.send(&on, "hello").await;
    let ordinary = rig.start().await;
    rig.send(&ordinary, "hi").await;
    let caller = |from: u16| api::Caller::at(format!("127.0.0.1:{from}").parse().unwrap());

    let reach = rig.fleet.words_reach_at(caller(52000)).expect("its connection");
    assert!(!reach.is_a_hosted_session(), "its acts are placed as a Job on your words's");
    for extra in ["tell_session", "ask_the_person", "list_sessions"] {
        assert!(reach.may(extra), "`{extra}` is its");
    }

    let session = rig.fleet.words_reach_at(caller(52001)).expect("a Job's connection");
    assert!(session.is_a_hosted_session());
    assert!(session.may("list_sessions") && session.may("ask_the_person"));
    assert!(!session.may("tell_session"));
}

#[tokio::test]
async fn an_ended_job_on_your_words_places_no_caller() {
    let rig = rig_placing(None);
    let on = rig.start_on_your_words(true).await;
    rig.send(&on, "hello").await;
    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: on }).await.expect("ended");
    let caller = api::Caller::at("127.0.0.1:52000".parse().unwrap());
    assert!(rig.fleet.words_reach_at(caller).is_none(), "no process, no placement");
}
