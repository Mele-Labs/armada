//! The event stream, upgraded and driven over an in-memory pipe.
//!
//! `tokio::io::duplex` is both halves of a connection with no connection: the
//! server side is handed to hyper, the client side to a real WebSocket client,
//! and the upgrade that runs between them is the same code path a Bridge takes.
//! **Nothing here binds a port**, which is the point — a loopback listener
//! would still be a listener, and would still fail in an environment that has
//! no networking at all.

use futures_util::StreamExt;
use ipc::{ProtocolId, StreamMessage};
use std::time::Duration;
use tokio::io::DuplexStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use crate::tests::connected;
use crate::tests::fake::FakeDaemon;
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

/// Every connection opens with current state, then carries events.
#[tokio::test]
async fn a_connection_opens_with_a_resync_and_then_carries_events() {
    let events = Broadcaster::new();
    let daemon = FakeDaemon::new(events.clone());
    let app = router(Served::by(daemon, run_id(), events.clone()));
    let mut socket = connected(app, "/events", 8192).await;

    let StreamMessage::Resync(resync) = read(&mut socket).await else {
        panic!("the first message is the resync — a reconnecting client must not have to ask");
    };
    assert_eq!(resync.protocol_id, ProtocolId::current());
    assert_eq!(resync.cursor.position(), 0);

    // Published only now: the resync having arrived is what says the socket is
    // subscribed, so this cannot race the subscription.
    let cursor = events.publish(an_event());

    let StreamMessage::Event(delivered) = read(&mut socket).await else {
        panic!("an event follows the resync");
    };
    assert_eq!(delivered.cursor, cursor);
    let ipc::Event::StudioDeleted(gone) = delivered.event else {
        panic!("a deletion, not a change");
    };
    assert_eq!(gone.id.as_str(), "01STUDIO");
}

/// A client that cannot keep up loses the oldest events and is **told**, then
/// handed current state. The count alone would leave it patching a history it
/// cannot repair.
#[tokio::test]
async fn a_client_that_cannot_keep_up_is_told_and_resynced() {
    let events = Broadcaster::with_backlog(1);
    let daemon = FakeDaemon::new(events.clone());
    let app = router(Served::by(daemon, run_id(), events.clone()));
    // A pipe small enough that the socket stops draining almost at once.
    let mut socket = connected(app, "/events", 256).await;

    let StreamMessage::Resync(_) = read(&mut socket).await else {
        panic!("the first message is the resync");
    };
    for _ in 0..24 {
        events.publish(an_event());
    }

    let mut told = None;
    let mut resynced_after = false;
    for _ in 0..24 {
        match read(&mut socket).await {
            StreamMessage::Missed(missed) => told = Some(missed.dropped),
            StreamMessage::Resync(_) if told.is_some() => {
                resynced_after = true;
                break;
            }
            _ => continue,
        }
    }
    let dropped = told.expect("the bound dropped events and the client was not told");
    assert!(dropped > 0);
    assert!(
        resynced_after,
        "a drop is always followed by current state, or the client is quietly wrong"
    );
}

fn an_event() -> ipc::Event {
    ipc::Event::StudioDeleted(ipc::StudioDeleted {
        id: ipc::StudioId::carried("01STUDIO"),
        manifest_id: ipc::ManifestId::carried("01MF"),
    })
}

/// A tap is told every event, from any clone of the stream, in the order they were published — and
/// unlike a subscription it does not lose the oldest when more than the backlog arrive at once.
#[test]
fn a_tap_hears_every_event_from_every_clone_even_past_the_backlog() {
    let events = Broadcaster::with_backlog(2);
    let heard = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let into = std::sync::Arc::clone(&heard);
    events.tap(move |event| into.lock().expect("not poisoned").push(event.kind()));

    let other = events.clone();
    for _ in 0..5 {
        other.publish(an_event());
    }
    events.publish(an_event());

    assert_eq!(heard.lock().expect("not poisoned").len(), 6, "a backlog of two lost none");
}

/// A minute that published more than the window holds says how many it could
/// not count, rather than reading as a quieter minute than it was. #1759.
#[test]
fn a_tally_wider_than_its_window_says_how_much_it_could_not_count() {
    let events = Broadcaster::new();
    let from = events.cursor();
    for _ in 0..crate::TALLIED + 10 {
        events.publish(an_event());
    }
    let tally = events.tallied(from);
    let window = crate::TALLIED as u64;
    assert_eq!(tally.count("studio.deleted"), window);
    let line = tally.to_string();
    assert!(line.contains("published 4106 events"), "{line}");
    assert!(
        line.contains("10 beyond the tally window, uncounted"),
        "{line}"
    );
}

/// One line for the stream.
#[test]
fn a_tally_reads_as_one_line_for_the_stream() {
    let events = Broadcaster::new();
    let from = events.cursor();
    events.publish(an_event());
    events.publish(an_event());
    let tally = events.tallied(from);
    assert_eq!(
        tally.to_string(),
        format!(
            "published 2 events (BACKLOG {}): studio.deleted 2",
            crate::BACKLOG
        )
    );
    assert!(events.tallied(tally.upto()).is_empty());
}
