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
use tokio::time::timeout;

/// Fleet answered on a port the runtime file named, or it did not.
const CONNECT_PATIENCE: Duration = Duration::from_secs(2);

/// What Fleet answered: the status and the bytes, nothing interpreted.
#[derive(Debug)]
pub struct Answer {
    pub status: u16,
    pub body: Vec<u8>,
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

/// One request, one connection.
pub async fn send(
    port: u16,
    method: &str,
    path: &str,
    body: Option<&[u8]>,
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
        .header(CALLER, "phone");
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
    let body = response
        .into_body()
        .collect()
        .await
        .map_err(|why| cut(why.to_string()))?
        .to_bytes()
        .to_vec();
    Ok(Answer { status, body })
}

/// Open Fleet's WebSocket at `path` and hand back the upgraded stream.
pub async fn upgrade(
    port: u16,
    path: &str,
) -> Result<TokioIo<hyper::upgrade::Upgraded>, Unreachable> {
    let cut = |cause: String| Unreachable { port, cause };
    let io = TokioIo::new(connect(port).await?);
    let (mut sender, connection) = hyper::client::conn::http1::handshake(io)
        .await
        .map_err(|why| cut(why.to_string()))?;
    tokio::spawn(connection.with_upgrades());
    let request = Request::builder()
        .uri(path)
        .header(HOST, format!("127.0.0.1:{port}"))
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
        return Err(cut(format!("answered {} to the upgrade", response.status())));
    }
    let upgraded = hyper::upgrade::on(response)
        .await
        .map_err(|why| cut(why.to_string()))?;
    Ok(TokioIo::new(upgraded))
}
