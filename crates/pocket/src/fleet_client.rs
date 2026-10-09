//! One request to Fleet, on loopback, written by hand.
//!
//! **Only the headers written here are sent.** A library client adds what it
//! likes, and Fleet refuses a request carrying `Origin`; this one has no way to
//! add it.

use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
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

/// One request, one connection, closed by the server.
pub async fn send(
    port: u16,
    method: &str,
    path: &str,
    body: Option<&[u8]>,
) -> Result<Answer, Unreachable> {
    let cut = |cause: String| Unreachable { port, cause };
    let at = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let mut socket = timeout(CONNECT_PATIENCE, TcpStream::connect(at))
        .await
        .map_err(|_| cut("no connection in 2 seconds".into()))?
        .map_err(|why| cut(why.to_string()))?;
    let mut head = format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n");
    if let Some(body) = body {
        head.push_str(&format!(
            "Content-Type: application/json\r\nContent-Length: {}\r\n",
            body.len()
        ));
    }
    head.push_str("\r\n");
    let mut raw = Vec::new();
    socket
        .write_all(head.as_bytes())
        .await
        .map_err(|why| cut(why.to_string()))?;
    socket
        .write_all(body.unwrap_or_default())
        .await
        .map_err(|why| cut(why.to_string()))?;
    socket
        .read_to_end(&mut raw)
        .await
        .map_err(|why| cut(why.to_string()))?;
    answer(&raw).ok_or_else(|| cut("the answer was not HTTP".into()))
}

fn answer(raw: &[u8]) -> Option<Answer> {
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n")?;
    let head = std::str::from_utf8(&raw[..split]).ok()?;
    let status = head.lines().next()?.split_whitespace().nth(1)?.parse().ok()?;
    Some(Answer {
        status,
        body: raw[split + 4..].to_vec(),
    })
}
