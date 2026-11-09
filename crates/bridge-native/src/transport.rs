//! Requests to Fleet on loopback over hyper's bare HTTP/1 client.
//!
//! **Only the headers written here are sent.** hyper's `conn::http1` adds none of its own, and
//! Fleet refuses a request that carries `Origin`; nothing in this module can add one. The
//! approach is `crates/pocket/src/fleet_client.rs`.

use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

use http_body_util::{BodyExt, Empty};
use hyper::body::Bytes;
use hyper::header::{CONNECTION, HOST, UPGRADE};
use hyper::{Request, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;
use tokio::time::timeout;

/// Fleet answered on the port the runtime file named, or it did not.
const CONNECT_PATIENCE: Duration = Duration::from_secs(2);

/// A read of a busy Fleet can be slow; past this the read failed.
const ANSWER_PATIENCE: Duration = Duration::from_secs(30);

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
        let response = sender.send_request(request).await.map_err(|why| why.to_string())?;
        let status = response.status().as_u16();
        let body = response.into_body().collect().await.map_err(|why| why.to_string())?;
        Ok((status, body.to_bytes().to_vec()))
    };
    timeout(ANSWER_PATIENCE, read).await.map_err(|_| "no answer in 30 seconds".to_string())?
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
