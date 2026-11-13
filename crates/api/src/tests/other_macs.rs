//! What this Mac's Phone Gateway reads to send another Mac: `/other_mac/events` is `/events` with each event put
//! to the share filter first.

use std::time::Duration;

use futures_util::StreamExt;
use ipc::{ProtocolId, StreamMessage};
use tokio::io::DuplexStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use crate::tests::connected;
use crate::tests::fake::{FakeDaemon, THE_PRIVATE_STUDIO};
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, Served};

async fn read(socket: &mut WebSocketStream<DuplexStream>) -> StreamMessage {
    let frame = tokio::time::timeout(Duration::from_secs(5), socket.next())
        .await
        .expect("the stream answers")
        .expect("the socket is open")
        .expect("a frame");
    let Message::Text(json) = frame else {
        panic!("the stream is text: {frame:?}");
    };
    ipc::decode("stream message", json.as_bytes()).expect("a stream message")
}

fn deleted(studio: &str) -> ipc::Event {
    ipc::Event::StudioDeleted(ipc::StudioDeleted {
        id: ipc::StudioId::carried(studio),
        manifest_id: ipc::ManifestId::carried("01MF"),
    })
}

#[tokio::test]
async fn the_other_macs_stream_opens_with_a_resync_and_carries_only_what_the_filter_lets_through() {
    let events = Broadcaster::new();
    let app = router(Served::by(FakeDaemon::new(events.clone()), run_id(), events.clone()));
    let mut socket = connected(app, "/other_mac/events", 8192).await;

    let StreamMessage::Resync(resync) = read(&mut socket).await else {
        panic!("the first message is the resync, as on /events");
    };
    assert_eq!(resync.protocol_id, ProtocolId::current());

    events.publish(deleted(THE_PRIVATE_STUDIO));
    let shared = events.publish(deleted("01SHARED"));

    let StreamMessage::Event(delivered) = read(&mut socket).await else {
        panic!("an event follows the resync");
    };
    assert_eq!(delivered.cursor, shared, "the held-back event was never sent; the next keeps its own position");
    assert_eq!(delivered.event, deleted("01SHARED"));
}
