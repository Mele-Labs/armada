//! One running Check's log, over the same in-memory pipe the other sockets use.
//!
//! What these prove is the half a reader cannot check from outside: that a
//! viewer gets what the Check printed before it connected **and** what it
//! prints after, from one connection, and is told in a sentence when the Check
//! has ended rather than left watching a file that will not grow again.
//!
//! The reader is a fake, for `crate::tests::journal`'s reason: what
//! `fleet::following` does with a real file is proved over there.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::http::StatusCode;
use axum::Router;
use futures_util::StreamExt;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use ipc::{LandOutputMessage, OutputEnded};
use tokio::io::DuplexStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;
use tower::Service;

use crate::tests::connected;
use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, Follow, Followed, LandOutput, Served};

/// A log a test appends to and then ends, standing in for the file.
#[derive(Default)]
struct Growing {
    written: Mutex<Vec<String>>,
    ended: Mutex<bool>,
}

impl Growing {
    fn wrote(&self, line: &str) {
        self.written
            .lock()
            .expect("not poisoned")
            .push(line.to_string());
    }

    fn ended(&self) {
        *self.ended.lock().expect("not poisoned") = true;
    }
}

impl Follow for Growing {
    fn read(&self, from: u64, _to_the_end: bool) -> Followed {
        let written = self.written.lock().expect("not poisoned");
        Followed {
            lines: written.iter().skip(from as usize).cloned().collect(),
            from: written.len() as u64,
            skipped: 0,
            unreadable: false,
        }
    }

    fn writing(&self) -> bool {
        !*self.ended.lock().expect("not poisoned")
    }
}

const ROOT: &str = "/Users/user/armada";
const BRANCH: &str = "bridge/check-log-sheet";

fn land_wired(log: Arc<Growing>) -> Router {
    let events = Broadcaster::new();
    let daemon = Arc::new(FakeDaemon::new(events.clone()));
    *daemon.land.lock().expect("not poisoned") = Some(LandOutput {
        root: ROOT.to_string(),
        branch: BRANCH.to_string(),
        name: "screens_test".to_string(),
        follow: log,
    });
    router(Served::sharing(daemon, run_id(), events))
}

/// The line's own three words, as a query.
fn land_path(check: &str) -> String {
    format!(
        "/merge_lines/checks/observe?root={}&branch={}&check={}",
        ipc::door::encoded(ROOT),
        ipc::door::encoded(BRANCH),
        ipc::door::encoded(check)
    )
}

async fn read_land(socket: &mut WebSocketStream<DuplexStream>) -> LandOutputMessage {
    let frame = tokio::time::timeout(Duration::from_secs(5), socket.next())
        .await
        .expect("the socket answers")
        .expect("the socket is open")
        .expect("a frame");
    let Message::Text(json) = frame else {
        panic!("the socket is text: {frame:?}");
    };
    ipc::decode("a merge line Check log message", json.as_bytes()).expect("a message")
}

/// **`observe_land_check`'s claim.** A merge line Check's log arrives as it is
/// written, under the line's own names rather than a Job's, and says when the
/// Check has ended.
#[tokio::test]
async fn a_merge_line_checks_log_arrives_as_it_is_written_and_says_when_it_ends() {
    let log = Arc::new(Growing::default());
    log.wrote("RUN  v3.2.4");
    let app = land_wired(Arc::clone(&log));
    let mut socket = connected(app, &land_path("screens_test"), 8192).await;

    let LandOutputMessage::Opened(opened) = read_land(&mut socket).await else {
        panic!("the first message says whose log this is");
    };
    assert_eq!(opened.protocol_id, ipc::ProtocolId::current());
    assert_eq!(
        (
            opened.root.as_str(),
            opened.branch.as_str(),
            opened.name.as_str()
        ),
        (ROOT, BRANCH, "screens_test")
    );
    let LandOutputMessage::Lines(first) = read_land(&mut socket).await else {
        panic!("what was there before the socket opened");
    };
    assert_eq!(first.lines, vec!["RUN  v3.2.4"]);

    log.wrote(" ok src/merge-line.test.ts (12 tests)");
    let LandOutputMessage::Lines(next) = read_land(&mut socket).await else {
        panic!("what is written next");
    };
    assert_eq!(next.lines, vec![" ok src/merge-line.test.ts (12 tests)"]);

    log.ended();
    let mut last = read_land(&mut socket).await;
    if let LandOutputMessage::Lines(_) = last {
        last = read_land(&mut socket).await;
    }
    let LandOutputMessage::Closed(closed) = last else {
        panic!("a closing sentence, not silence: {last:?}");
    };
    assert_eq!(closed.because, OutputEnded::Finished);
}

/// A Check with no log to read is refused at the handshake, before any socket
/// opens: the daemon resolves the names first.
#[tokio::test]
async fn a_merge_line_check_with_no_log_is_refused_before_the_socket_opens() {
    let app = land_wired(Arc::new(Growing::default()));
    let (client_side, server_side) = tokio::io::duplex(8192);
    tokio::spawn(async move {
        let service = service_fn(move |request| app.clone().call(request));
        let _ = hyper::server::conn::http1::Builder::new()
            .serve_connection(TokioIo::new(server_side), service)
            .with_upgrades()
            .await;
    });
    let path = land_path("../outcomes/x");
    let opened =
        tokio_tungstenite::client_async(format!("ws://fleet.invalid{path}"), client_side).await;
    let Err(tokio_tungstenite::tungstenite::Error::Http(answer)) = opened else {
        panic!("the handshake is refused before any socket opens");
    };
    assert_eq!(answer.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
