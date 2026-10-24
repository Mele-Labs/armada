//! The phone's read routes, each one Fleet's answer narrowed to [`PhoneSession`].

use std::collections::BTreeMap;
use std::convert::Infallible;
use std::time::Duration;

use axum::body::Body;
use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use futures_util::{stream, Stream};
use hyper::body::Bytes;
use ipc::{ManifestSummary, SessionList};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::io::AsyncRead;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{timeout_at, Instant};

use crate::fleet_events::Messages;
use crate::phone_sessions::PhoneSession;
use crate::{fleet_client, live, Gateway};

/// What the owner is told when Fleet cannot be reached. Each says what to do.
const NOT_RUNNING: &str = "Armada is not running on your Mac. Open it there, then try again.";
pub(crate) const NOT_ANSWERING: &str = "Armada is not answering on your Mac. Open it there, then try again.";
pub(crate) const NOT_READABLE: &str =
    "Armada answered in a way this phone cannot read. Update Armada on your Mac.";

pub(crate) fn refusal(status: StatusCode, sentence: &'static str) -> Response {
    (status, sentence).into_response()
}

pub(crate) fn port(gateway: &Gateway) -> Result<u16, Response> {
    (gateway.fleet)().map_err(|_| refusal(StatusCode::SERVICE_UNAVAILABLE, NOT_RUNNING))
}

/// A read of Fleet, decoded as the type Fleet sends. `Ok(None)` is Fleet
/// saying the thing is not there.
async fn read<T: DeserializeOwned>(
    gateway: &Gateway,
    path: &str,
    what: &'static str,
) -> Result<Option<T>, Response> {
    let port = port(gateway)?;
    let answer = fleet_client::send(port, "GET", path, None)
        .await
        .map_err(|_| refusal(StatusCode::SERVICE_UNAVAILABLE, NOT_ANSWERING))?;
    match answer.status {
        200 => ipc::decode(what, &answer.body)
            .map(Some)
            .map_err(|_| refusal(StatusCode::BAD_GATEWAY, NOT_READABLE)),
        404 => Ok(None),
        _ => Err(refusal(StatusCode::BAD_GATEWAY, NOT_ANSWERING)),
    }
}

pub(crate) async fn required<T: DeserializeOwned>(
    gateway: &Gateway,
    path: &str,
    what: &'static str,
) -> Result<T, Response> {
    read(gateway, path, what)
        .await?
        .ok_or_else(|| refusal(StatusCode::BAD_GATEWAY, NOT_READABLE))
}

pub(crate) fn json<T: Serialize>(value: &T) -> Response {
    match ipc::encode(value) {
        Ok(body) => ([(header::CONTENT_TYPE, "application/json")], body).into_response(),
        Err(_) => refusal(StatusCode::INTERNAL_SERVER_ERROR, NOT_READABLE),
    }
}

/// Repository labels by Manifest id. A Fleet that will not list them costs the
/// labels, not the page.
pub(crate) async fn repositories(gateway: &Gateway) -> BTreeMap<String, String> {
    read::<Vec<ManifestSummary>>(gateway, "/manifests", "a manifest list")
        .await
        .ok()
        .flatten()
        .unwrap_or_default()
        .into_iter()
        .map(|manifest| (manifest.id.as_str().to_string(), manifest.repository))
        .collect()
}

#[derive(Serialize)]
struct Needs {
    /// Sessions waiting on the owner as Bridge counts it: an ask (a hosted one
    /// carries it, a Terminal one is waiting and nothing more) or an open item.
    sessions: Vec<PhoneSession>,
}

/// Needs you: the live Sessions waiting on the owner, oldest wait first.
pub async fn needs(State(gateway): State<Gateway>) -> Response {
    let list = match required::<SessionList>(&gateway, "/sessions?state=live", "a job list").await {
        Ok(list) => list.sessions,
        Err(refused) => return refused,
    };
    let labels = repositories(&gateway).await;
    let mut sessions: Vec<PhoneSession> = list
        .iter()
        .map(|record| {
            let repository = record
                .manifest_id
                .as_ref()
                .and_then(|id| labels.get(id.as_str()))
                .map(String::as_str);
            PhoneSession::of(record, repository, gateway.pairing.now())
        })
        .filter(|session| session.waiting)
        .collect();
    sessions.sort_by(|a, b| (&a.waiting_since, &a.id).cmp(&(&b.waiting_since, &b.id)));
    json(&Needs { sessions })
}

/// Server-Sent Events: each Session change Fleet publishes, narrowed.
pub async fn live(State(gateway): State<Gateway>) -> Response {
    let port = match port(&gateway) {
        Ok(port) => port,
        Err(refused) => return refused,
    };
    let upgraded = match fleet_client::upgrade(port, "/events").await {
        Ok(upgraded) => upgraded,
        Err(_) => return refusal(StatusCode::SERVICE_UNAVAILABLE, NOT_ANSWERING),
    };
    let frames = frames(upgraded, BEAT);
    (
        [
            (header::CONTENT_TYPE, "text/event-stream"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        Body::from_stream(frames),
    )
        .into_response()
}

/// Fleet's messages as Server-Sent Events, with a comment line whenever `beat` passes
/// with nothing sent. `Messages::next` reads with `read_exact`, which loses bytes if it
/// is dropped half way, so it is never raced against the timer: a task reads, and the
/// stream waits on its channel.
fn frames<R: AsyncRead + Unpin + Send + 'static>(
    io: R,
    beat: Duration,
) -> impl Stream<Item = Result<Bytes, Infallible>> + Send {
    let (sender, receiver) = mpsc::channel(32);
    let reader = tokio::spawn(async move {
        let mut messages = Messages::new(io);
        while let Some(text) = messages.next().await {
            if sender.send(text).await.is_err() {
                break;
            }
        }
    });
    stream::unfold((receiver, StopReading(reader)), move |(mut receiver, reader)| async move {
        // Fixed before the loop: a message `live::change` drops does not put the beat off.
        let due = Instant::now() + beat;
        loop {
            let frame = match timeout_at(due, receiver.recv()).await {
                Ok(Some(text)) => match live::change(&text).and_then(|change| live::frame(&change)) {
                    Some(frame) => frame,
                    None => continue,
                },
                Ok(None) => return None,
                Err(_) => BEATING.to_string(),
            };
            return Some((Ok(Bytes::from(frame)), (receiver, reader)));
        }
    })
}

/// An SSE comment: clients ignore it, and it keeps the connection seen to be alive.
const BEATING: &str = ": beat\n\n";

/// Ends the reading task when the phone goes away, rather than at Fleet's next message.
struct StopReading(JoinHandle<()>);

impl Drop for StopReading {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// How long `/api/live` stays quiet before it sends a comment line, so the phone
/// can tell a stream that is waiting from one that has died.
const BEAT: Duration = Duration::from_secs(15);

#[cfg(test)]
mod tests {
    use futures_util::StreamExt;
    use tokio::io::AsyncWriteExt;

    use ipc::{Cursor, ProtocolId, Resync, StreamMessage};

    use super::*;

    /// One unmasked text frame, as Fleet sends it.
    fn text(message: &str) -> Vec<u8> {
        let mut out = vec![0x81, 126];
        out.extend_from_slice(&(message.len() as u16).to_be_bytes());
        out.extend_from_slice(message.as_bytes());
        out
    }

    const OTHER: &str = r#"{"message":"event","cursor":3,"event":{"kind":"repositories.changed","repositories":[]}}"#;
    fn resync() -> String {
        let message = StreamMessage::Resync(Resync { protocol_id: ProtocolId::current(), cursor: Cursor::at(9) });
        ipc::encode(&message).unwrap()
    }

    async fn next(frames: &mut (impl Stream<Item = Result<Bytes, Infallible>> + Unpin)) -> Option<String> {
        let item = tokio::time::timeout(Duration::from_secs(5), frames.next()).await.expect("the stream stalled");
        item.map(|bytes| String::from_utf8(bytes.unwrap().to_vec()).unwrap())
    }

    #[tokio::test]
    async fn a_quiet_fleet_gets_a_beat_and_a_message_still_gets_its_frame() {
        let (mut fleet, here) = tokio::io::duplex(4096);
        let mut frames = Box::pin(frames(here, Duration::from_millis(40)));
        assert_eq!(next(&mut frames).await.as_deref(), Some(": beat\n\n"));
        fleet.write_all(&text(&resync())).await.unwrap();
        // The beat is not owed again before the frame: the resync comes next or after one more beat.
        let mut got = next(&mut frames).await;
        while got.as_deref() == Some(": beat\n\n") {
            got = next(&mut frames).await;
        }
        assert_eq!(got.as_deref(), Some("data: {\"change\":\"resync\"}\n\n"));
    }

    #[tokio::test]
    async fn messages_that_are_dropped_do_not_hold_back_the_beat() {
        let (mut fleet, here) = tokio::io::duplex(4096);
        let mut frames = Box::pin(frames(here, Duration::from_millis(60)));
        let keep_talking = tokio::spawn(async move {
            for _ in 0..40 {
                fleet.write_all(&text(OTHER)).await.unwrap();
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            fleet
        });
        assert_eq!(next(&mut frames).await.as_deref(), Some(": beat\n\n"));
        keep_talking.abort();
    }

    #[tokio::test]
    async fn a_fleet_that_went_away_ends_the_stream() {
        let (fleet, here) = tokio::io::duplex(4096);
        let mut frames = Box::pin(frames(here, Duration::from_secs(60)));
        drop(fleet);
        assert_eq!(next(&mut frames).await, None);
    }
}
