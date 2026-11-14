//! Requests to Fleet, on loopback, over hyper's bare HTTP/1 client.
//!
//! **Only the headers written here are sent.** hyper's `conn::http1` adds none
//! of its own, and Fleet refuses a request carrying `Origin`; this module has
//! no way to add it. A body is read to its end whether it is sized or chunked.

use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::header::{CONNECTION, CONTENT_TYPE, HOST, UPGRADE};
use hyper::{Request, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;

/// The header Fleet reads to place a request's door; its value here is the
/// phone's. Fleet's own spelling is `api::CALLER_HEADER`, which this crate
/// does not depend on.
const CALLER: &str = "x-armada-caller";

/// What the Gateway says in [`CALLER`] for a call another Mac made, and the header that names that Mac. Fleet's
/// spellings are `api::MAC` and `api::MAC_HEADER`.
const MAC_CALLER: &str = "mac";
const MAC_ID: &str = "x-armada-mac";
use tokio::time::timeout;

/// Fleet answered on a port the runtime file named, or it did not.
const CONNECT_PATIENCE: Duration = Duration::from_secs(2);

/// What Fleet answered: the status and the bytes, nothing interpreted.
#[derive(Debug)]
pub struct Answer {
    pub status: u16,
    pub body: Vec<u8>,
    /// Fleet's `Content-Type`, for a forward that hands the answer back as it came.
    pub content_type: Option<String>,
}

#[derive(Debug)]
pub struct Unreachable {
    pub port: u16,
    pub cause: String,
}

impl std::fmt::Display for Unreachable {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            out,
            "Fleet did not answer on port {}: {}",
            self.port, self.cause
        )
    }
}

impl std::error::Error for Unreachable {}

async fn connect(port: u16) -> Result<TcpStream, Unreachable> {
    let cut = |cause: String| Unreachable { port, cause };
    let at = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    timeout(CONNECT_PATIENCE, TcpStream::connect(at))
        .await
        .map_err(|_| cut("no connection in 2 seconds".into()))?
        .map_err(|why| cut(why.to_string()))
}

/// One request, one connection, as the phone's.
pub async fn send(
    port: u16,
    method: &str,
    path: &str,
    body: Option<&[u8]>,
) -> Result<Answer, Unreachable> {
    send_as(port, method, path, body, None).await
}

/// [`send`] as Mac `mac`, which Fleet records as the door the act came through.
pub async fn send_as_mac(
    port: u16,
    method: &str,
    path: &str,
    body: Option<&[u8]>,
    mac: &str,
) -> Result<Answer, Unreachable> {
    send_as(port, method, path, body, Some(mac)).await
}

async fn send_as(
    port: u16,
    method: &str,
    path: &str,
    body: Option<&[u8]>,
    mac: Option<&str>,
) -> Result<Answer, Unreachable> {
    let cut = |cause: String| Unreachable { port, cause };
    let io = TokioIo::new(connect(port).await?);
    let (mut sender, connection) = hyper::client::conn::http1::handshake(io)
        .await
        .map_err(|why| cut(why.to_string()))?;
    tokio::spawn(connection);
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header(HOST, format!("127.0.0.1:{port}"))
        .header(CALLER, if mac.is_some() { MAC_CALLER } else { "phone" });
    if let Some(mac) = mac {
        request = request.header(MAC_ID, mac);
    }
    if body.is_some() {
        request = request.header(CONTENT_TYPE, "application/json");
    }
    let request = request
        .body(Full::new(Bytes::copy_from_slice(body.unwrap_or_default())))
        .map_err(|why| cut(why.to_string()))?;
    let response = sender
        .send_request(request)
        .await
        .map_err(|why| cut(why.to_string()))?;
    let status = response.status().as_u16();
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let body = response
        .into_body()
        .collect()
        .await
        .map_err(|why| cut(why.to_string()))?
        .to_bytes()
        .to_vec();
    Ok(Answer { status, body, content_type })
}

/// Open Fleet's WebSocket at `path` and hand back the upgraded stream.
pub async fn upgrade(
    port: u16,
    path: &str,
) -> Result<TokioIo<hyper::upgrade::Upgraded>, Unreachable> {
    match upgrade_or_refused(port, path).await? {
        Ok(upgraded) => Ok(upgraded),
        Err(answer) => Err(Unreachable { port, cause: format!("answered {} to the upgrade", answer.status) }),
    }
}

/// [`upgrade`], with Fleet's answer read whole where it refused the upgrade, so its status and words can be
/// passed on as it said them.
pub(crate) async fn upgrade_or_refused(
    port: u16,
    path: &str,
) -> Result<Result<TokioIo<hyper::upgrade::Upgraded>, Answer>, Unreachable> {
    let cut = |cause: String| Unreachable { port, cause };
    let io = TokioIo::new(connect(port).await?);
    let (mut sender, connection) = hyper::client::conn::http1::handshake(io)
        .await
        .map_err(|why| cut(why.to_string()))?;
    tokio::spawn(connection.with_upgrades());
    // Named as the phone, as [`send`] is: Fleet cuts `/events` to what the phone sees.
    let request = Request::builder()
        .uri(path)
        .header(HOST, format!("127.0.0.1:{port}"))
        .header(CALLER, "phone")
        .header(CONNECTION, "Upgrade")
        .header(UPGRADE, "websocket")
        .header("Sec-WebSocket-Version", "13")
        // Fleet does not check the key against anything; the value is the
        // protocol's own worked example.
        .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
        .body(Full::new(Bytes::new()))
        .map_err(|why| cut(why.to_string()))?;
    let response = sender
        .send_request(request)
        .await
        .map_err(|why| cut(why.to_string()))?;
    if response.status() != StatusCode::SWITCHING_PROTOCOLS {
        let status = response.status().as_u16();
        let content_type = response.headers().get(CONTENT_TYPE).and_then(|value| value.to_str().ok()).map(str::to_string);
        let body = response.into_body().collect().await.map_err(|why| cut(why.to_string()))?.to_bytes().to_vec();
        return Ok(Err(Answer { status, body, content_type }));
    }
    let upgraded = hyper::upgrade::on(response)
        .await
        .map_err(|why| cut(why.to_string()))?;
    Ok(Ok(TokioIo::new(upgraded)))
}
