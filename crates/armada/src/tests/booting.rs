//! What serving does when the boot fails, is stopped, or finishes.
//!
//! A real listener on a port the system picks and a plain router, because the
//! claim is about the server's shutdown rather than about any route. A command
//! held by `api::Reconciliation` is stood in for by a handler waiting on a
//! channel whose sender the boot owns; `api`'s own tests cover the real hold.

use std::future::pending;
use std::time::Duration;

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::Router;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot, watch};

use crate::booting::serve_while_booting;

/// What a test waits for before calling the server wedged. Far over what a
/// shutdown takes, so only a shutdown that never comes reaches it.
const PATIENCE: Duration = Duration::from_secs(10);

/// A command that waits for the boot to finish, and is refused if the boot is
/// dropped first, as `api::reconciling::hold_commands` does.
async fn held(
    State((mut over, arrived)): State<(watch::Receiver<bool>, mpsc::UnboundedSender<()>)>,
) -> StatusCode {
    let _ = arrived.send(());
    let finished = over.wait_for(|over| *over).await.is_ok();
    match finished {
        true => StatusCode::OK,
        false => StatusCode::SERVICE_UNAVAILABLE,
    }
}

struct Served {
    listener: TcpListener,
    app: Router,
    port: u16,
    arrived: mpsc::UnboundedReceiver<()>,
    over: watch::Sender<bool>,
}

async fn served() -> Served {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a port to bind");
    let port = listener.local_addr().expect("a bound address").port();
    let (over, watched) = watch::channel(false);
    let (arrived_tx, arrived) = mpsc::unbounded_channel();
    let app = Router::new()
        .route("/held", post(held))
        .with_state((watched, arrived_tx));
    Served {
        listener,
        app,
        port,
        arrived,
        over,
    }
}

/// The status line of the answer to one command, read to the end.
async fn command(port: u16) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("the server to be listening");
    stream
        .write_all(
            b"POST /held HTTP/1.1\r\nHost: test\r\nConnection: close\r\nContent-Length: 0\r\n\r\n",
        )
        .await
        .expect("the request to send");
    let mut answer = String::new();
    stream
        .read_to_string(&mut answer)
        .await
        .expect("the answer to arrive");
    answer.lines().next().unwrap_or_default().to_string()
}

#[tokio::test]
async fn a_boot_that_fails_stops_the_server_and_is_the_error_returned() {
    let Served {
        listener,
        app,
        port,
        mut arrived,
        over,
    } = served().await;
    let (fail, failing) = oneshot::channel::<()>();
    // Owns `over`, as the boot owns the `Reconciliation`: dropped with it.
    let boot = async move {
        let _holding = over;
        let _ = failing.await;
        Err::<(), _>("the store would not open".to_string())
    };
    let failing_once_held = async {
        arrived.recv().await.expect("the command to be held");
        fail.send(()).expect("the boot to be waiting");
    };

    let (served, status, ()) = tokio::time::timeout(PATIENCE, async {
        tokio::join!(
            serve_while_booting(listener, app, pending(), boot),
            command(port),
            failing_once_held,
        )
    })
    .await
    .expect("a failed boot stops the server and answers what it held");

    let error = served.expect_err("the failure is what serve returns");
    assert_eq!(error.to_string(), "the store would not open");
    assert_eq!(status, "HTTP/1.1 503 Service Unavailable");
}

/// Tells the test when the boot future has been dropped.
struct Dropped(Option<oneshot::Sender<()>>);

impl Drop for Dropped {
    fn drop(&mut self) {
        if let Some(sender) = self.0.take() {
            let _ = sender.send(());
        }
    }
}

#[tokio::test]
async fn a_stop_during_the_boot_ends_the_boot_and_does_not_wait_for_it() {
    let served = served().await;
    let (started, begun) = oneshot::channel::<()>();
    let (dropped, ended) = oneshot::channel::<()>();
    let (stop, stopping) = oneshot::channel::<()>();
    // Never finishes: a reconcile stuck in a Job's gate.
    let boot = async move {
        let _ended = Dropped(Some(dropped));
        let _ = started.send(());
        pending::<Result<(), String>>().await
    };
    let stopping_once_begun = async {
        begun.await.expect("the boot to start");
        stop.send(()).expect("the server to be listening for it");
    };

    let (booted, ()) = tokio::time::timeout(PATIENCE, async {
        tokio::join!(
            serve_while_booting(
                served.listener,
                served.app,
                async {
                    let _ = stopping.await;
                },
                boot,
            ),
            stopping_once_begun,
        )
    })
    .await
    .expect("a stop does not wait for the boot");

    let booted = booted.expect("a stop is not an error");
    assert!(
        booted.is_none(),
        "a boot that never finished returned a value"
    );
    tokio::time::timeout(PATIENCE, ended)
        .await
        .expect("the boot is ended where it stands")
        .expect("the boot to be dropped, not completed");
}

#[tokio::test]
async fn a_boot_that_finishes_is_handed_back_when_the_stop_comes() {
    let served = served().await;
    let (done, finished) = oneshot::channel::<()>();
    let (stop, stopping) = oneshot::channel::<()>();
    let boot = async move {
        let _ = done.send(());
        Ok::<_, String>("turning")
    };
    let (over, port) = (served.over, served.port);
    let asking_then_stopping = async {
        finished.await.expect("the boot to finish");
        // Finishing is not a reason to stop: the server still answers.
        assert_eq!(over.send(true), Ok(()));
        assert_eq!(command(port).await, "HTTP/1.1 200 OK");
        stop.send(()).expect("the server to be listening for it");
    };

    let (booted, ()) = tokio::time::timeout(PATIENCE, async {
        tokio::join!(
            serve_while_booting(
                served.listener,
                served.app,
                async {
                    let _ = stopping.await;
                },
                boot,
            ),
            asking_then_stopping,
        )
    })
    .await
    .expect("a stop ends the server");

    assert_eq!(booted.expect("a stop is not an error"), Some("turning"));
}
