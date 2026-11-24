//! Requests to Fleet on loopback over hyper's bare HTTP/1 client.
//!
//! **Only the headers written here are sent.** hyper's `conn::http1` adds none of its own, and
//! Fleet refuses a request that carries `Origin`; nothing in this module can add one. The
//! approach is `crates/pocket/src/fleet_client.rs`.

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use http_body_util::{BodyExt, Empty, Full};
use hyper::body::Bytes;
use hyper::header::{CONNECTION, CONTENT_TYPE, HOST, UPGRADE};
use hyper::{Request, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;
use tokio::time::timeout;

/// Fleet answered on the port the runtime file named, or it did not.
const CONNECT_PATIENCE: Duration = Duration::from_secs(2);

/// A read of a busy Fleet can be slow; past this the read failed.
const ANSWER_PATIENCE: Duration = Duration::from_secs(30);

/// Requests in a row that got no answer within `ANSWER_PATIENCE`, since Fleet last answered one.
///
/// Fleet's event stream can stay up while its routes hang (19 Nov: seven minutes of "no answer in 30
/// seconds" with the cursor advancing), so the title bar reads this beside the link.
pub struct Slowness(AtomicU32);

/// Timeouts in a row after which Fleet is called slow.
pub const SLOW_AFTER: u32 = 3;

impl Slowness {
    pub const fn new() -> Slowness {
        Slowness(AtomicU32::new(0))
    }

    pub fn timed_out(&self) {
        let _ = self.0.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| Some(n.saturating_add(1)));
    }

    /// Any answer, a refusal too, shows Fleet's routes work.
    pub fn answered(&self) {
        self.0.store(0, Ordering::Relaxed);
    }

    pub fn standing(&self) -> bool {
        self.0.load(Ordering::Relaxed) >= SLOW_AFTER
    }
}

static SLOWNESS: Slowness = Slowness::new();

/// Whether Fleet's routes have timed out `SLOW_AFTER` times running.
pub fn slow() -> bool {
    SLOWNESS.standing()
}

/// Count a request's outcome: only a wait of the ordinary patience says Fleet is slow, since a clone or a
/// model call is meant to wait longer.
fn noted(result: Result<Result<(u16, Vec<u8>), String>, tokio::time::error::Elapsed>, patience: Duration) -> Result<(u16, Vec<u8>), String> {
    match result {
        Ok(answer) => {
            if answer.is_ok() {
                SLOWNESS.answered();
            }
            answer
        }
        Err(_) => {
            if patience == ANSWER_PATIENCE {
                SLOWNESS.timed_out();
            }
            Err(format!("no answer in {} seconds", patience.as_secs()))
        }
    }
}

/// What a route with a model call inside it waits: 150 seconds, deliberately longer than Fleet's
/// own bound on a call (a gate re-run may make several), so Fleet's refusal, with its code and run
/// id, arrives rather than Bridge's abort.
const MODEL_CALL_MS: u64 = 150_000;

/// Bridge names itself on every request so Fleet tells a press from an agent's
/// (`api::CALLER_HEADER`, and `CALLER` in `apps/desktop/src/main/request.ts`).
const CALLER: (&str, &str) = ("x-armada-caller", "bridge");

async fn connect(port: u16) -> Result<TcpStream, String> {
    let at = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    timeout(CONNECT_PATIENCE, TcpStream::connect(at))
        .await
        .map_err(|_| "no connection in 2 seconds".to_string())?
        .map_err(|why| why.to_string())
}

/// What a request ended on when Fleet closed the connection before its answer was whole.
pub const STOPPED: &str = "Fleet stopped before it answered";

/// A request's failure as a person reads it: a connection Fleet closed mid-answer is Fleet stopping, which hyper's
/// own words do not say; anything else keeps hyper's.
fn dropped(why: hyper::Error) -> String {
    if why.is_incomplete_message() || why.is_canceled() {
        STOPPED.to_string()
    } else {
        why.to_string()
    }
}

/// GET `path`; the status and the bytes, nothing interpreted.
pub async fn get(port: u16, path: &str) -> Result<(u16, Vec<u8>), String> {
    let io = TokioIo::new(connect(port).await?);
    let (mut sender, connection) =
        hyper::client::conn::http1::handshake(io).await.map_err(|why| why.to_string())?;
    tokio::spawn(connection);
    let request = Request::builder()
        .uri(path)
        .header(HOST, format!("127.0.0.1:{port}"))
        .header(CALLER.0, CALLER.1)
        .body(Empty::<Bytes>::new())
        .map_err(|why| why.to_string())?;
    let read = async {
        let response = sender.send_request(request).await.map_err(dropped)?;
        let status = response.status().as_u16();
        let body = response.into_body().collect().await.map_err(dropped)?;
        Ok((status, body.to_bytes().to_vec()))
    };
    noted(timeout(ANSWER_PATIENCE, read).await, ANSWER_PATIENCE)
}

/// POST `body`, JSON already encoded, to `path`; the status and the bytes, nothing interpreted.
pub async fn post(port: u16, path: &str, body: String) -> Result<(u16, Vec<u8>), String> {
    send_post(port, path, Some(body), ANSWER_PATIENCE).await
}

/// POST to a route that asks a model inside the request, so it waits `MODEL_CALL_MS` and Fleet's own
/// refusal arrives first.
pub async fn post_to_model(port: u16, path: &str, body: String) -> Result<(u16, Vec<u8>), String> {
    send_post(port, path, Some(body), Duration::from_millis(MODEL_CALL_MS)).await
}

/// POST to a route that answers only once its work is done, such as a clone, waiting `patience` for it.
pub async fn post_waiting(port: u16, path: &str, body: String, patience: Duration) -> Result<(u16, Vec<u8>), String> {
    send_post(port, path, Some(body), patience).await
}

/// POST to `path` with no body and no `content-type`, as a route that takes none is asked
/// (`POST /fleet/build/revert`).
pub async fn post_bare(port: u16, path: &str) -> Result<(u16, Vec<u8>), String> {
    send_post(port, path, None, ANSWER_PATIENCE).await
}

/// PUT `body`, JSON already encoded, to `path`; the status and the bytes, nothing interpreted.
pub async fn put(port: u16, path: &str, body: String) -> Result<(u16, Vec<u8>), String> {
    send("PUT", port, path, Some(body), ANSWER_PATIENCE).await
}

/// DELETE `path`, with no body; the status and the bytes, nothing interpreted.
pub async fn delete(port: u16, path: &str) -> Result<(u16, Vec<u8>), String> {
    send("DELETE", port, path, None, ANSWER_PATIENCE).await
}

async fn send_post(port: u16, path: &str, body: Option<String>, patience: Duration) -> Result<(u16, Vec<u8>), String> {
    send("POST", port, path, body, patience).await
}

async fn send(method: &str, port: u16, path: &str, body: Option<String>, patience: Duration) -> Result<(u16, Vec<u8>), String> {
    let io = TokioIo::new(connect(port).await?);
    let (mut sender, connection) =
        hyper::client::conn::http1::handshake(io).await.map_err(|why| why.to_string())?;
    tokio::spawn(connection);
    let request = Request::builder()
        .method(method)
        .uri(path)
        .header(HOST, format!("127.0.0.1:{port}"))
        .header(CALLER.0, CALLER.1);
    let request = match body {
        Some(body) => request.header(CONTENT_TYPE, "application/json").body(Full::new(Bytes::from(body))),
        None => request.body(Full::new(Bytes::new())),
    }
    .map_err(|why| why.to_string())?;
    let read = async {
        let response = sender.send_request(request).await.map_err(dropped)?;
        let status = response.status().as_u16();
        let body = response.into_body().collect().await.map_err(dropped)?;
        Ok((status, body.to_bytes().to_vec()))
    };
    noted(timeout(patience, read).await, patience)
}

/// Open Fleet's WebSocket at `path` and hand back the upgraded stream.
pub async fn upgrade(port: u16, path: &str) -> Result<TokioIo<hyper::upgrade::Upgraded>, String> {
    let io = TokioIo::new(connect(port).await?);
    let (mut sender, connection) =
        hyper::client::conn::http1::handshake(io).await.map_err(|why| why.to_string())?;
    tokio::spawn(connection.with_upgrades());
    let request = Request::builder()
        .uri(path)
        .header(HOST, format!("127.0.0.1:{port}"))
        .header(CONNECTION, "Upgrade")
        .header(UPGRADE, "websocket")
        .header("Sec-WebSocket-Version", "13")
        // Fleet checks the key against nothing; this is the protocol's own worked example.
        .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
        .body(Empty::<Bytes>::new())
        .map_err(|why| why.to_string())?;
    // Fleet binds before it serves, so a booting Fleet accepts and says nothing; the caller's
    // grace timer, not this call, notices that. The upgrade therefore waits as long as Fleet does.
    let response = sender.send_request(request).await.map_err(|why| why.to_string())?;
    if response.status() != StatusCode::SWITCHING_PROTOCOLS {
        return Err(format!("answered {} to the upgrade", response.status()));
    }
    let upgraded = hyper::upgrade::on(response).await.map_err(|why| why.to_string())?;
    Ok(TokioIo::new(upgraded))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fleet_is_slow_after_timeouts_in_a_row_and_an_answer_ends_it() {
        let slowness = Slowness::new();
        for _ in 0..SLOW_AFTER - 1 {
            slowness.timed_out();
        }
        assert!(!slowness.standing(), "two timeouts are not yet slow");
        slowness.timed_out();
        assert!(slowness.standing());
        slowness.answered();
        assert!(!slowness.standing(), "one answer and the run starts again");
    }

    /// Fleet aborted while it handled a dispatch (21 Nov, a stack overflow): the socket closed with the request read
    /// and nothing written back, and the toast said hyper's "connection closed before message completed".
    #[tokio::test]
    async fn a_connection_closed_before_the_answer_reads_as_fleet_stopping() {
        use tokio::io::AsyncReadExt;
        let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.expect("bound");
        let port = listener.local_addr().expect("addr").port();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accepted");
            let mut read = [0u8; 4096];
            let _ = socket.read(&mut read).await;
            drop(socket);
        });

        let failed = post(port, "/sessions/dispatch", "{}".to_string()).await.expect_err("no answer");

        assert_eq!(failed, STOPPED);
    }

    #[test]
    fn only_a_connected_stream_with_slow_routes_raises_the_notice() {
        assert_eq!(crate::titlebar::slow_notice(true).map(|notice| notice.headline.to_string()), Some("Fleet is slow".to_string()));
        assert!(crate::titlebar::slow_notice(false).is_none());
    }
}
