//! The upgrades, and what goes down each.
//!
//! `/events` is global, because Bridge holds exactly one connection; a Check's
//! log and a run's output each open deliberately on a socket of their own. A
//! transcript row on the global stream would evict the state changes Bridge
//! draws from, and an eviction there is a full resync rather than a lost row.
//!
//! All of them are extractors in the same `Router`. There is no second port.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use ipc::{Missed, ProtocolId, Resync, StreamMessage};

use crate::answers::refused;
use crate::daemon::Queries;
use crate::served::Served;
use crate::stream::Next;

/// Which merge line Check's log a viewer asked for: the line's own words.
#[derive(serde::Deserialize)]
pub(crate) struct LandCheck {
    root: String,
    branch: String,
    check: String,
}

/// One merge line Check's log, running or ended.
///
/// **Its own socket**, for the reasons at the top of this module. The daemon
/// resolves the three names before the upgrade, so a Check with no log to read
/// is a refusal read at the moment it was asked.
pub(crate) async fn observe_land_check<D: Queries>(
    State(served): State<Served<D>>,
    axum::extract::Query(asked): axum::extract::Query<LandCheck>,
    upgrade: WebSocketUpgrade,
) -> Response {
    match served
        .daemon()
        .observe_land_check(asked.root, asked.branch, asked.check)
        .await
    {
        Ok(land) => upgrade.on_upgrade(move |socket| crate::following::relay_land(socket, land)),
        Err(refusal) => refused(refusal),
    }
}

/// The event stream. **Global, and a client subscribes to nothing** — one
/// socket carries every Job, because Bridge holds exactly one connection.
///
/// Nothing is read from the socket. The stream is one-directional by design:
/// there is no subscribe message to read, and a connection that carried state
/// would be a connection that is expensive to drop and remake.
pub(crate) async fn events<D: Queries>(
    State(served): State<Served<D>>,
    upgrade: WebSocketUpgrade,
) -> Response {
    upgrade.on_upgrade(move |socket| watch(socket, served))
}

async fn watch<D: Queries>(mut socket: WebSocket, served: Served<D>) {
    // Subscribe first, then snapshot. The other order drops whatever lands in
    // between; this order can only repeat, and a repeat is detectable.
    let mut subscription = served.events().subscribe();
    if !resync(&mut socket, &served).await {
        return;
    }
    while let Some(next) = subscription.next().await {
        let delivered = match next {
            Next::Send(delivered) => send(&mut socket, &StreamMessage::Event(delivered)).await,
            // The count alone cannot repair what the client holds, so the drop
            // is always followed by current state.
            Next::Missed(dropped) => {
                send(&mut socket, &StreamMessage::Missed(Missed { dropped })).await
                    && resync(&mut socket, &served).await
            }
        };
        if !delivered {
            return;
        }
    }
}

/// Where the stream is, and what this build speaks. `false` where the socket is
/// gone, and the caller stops. Nothing is read from the daemon: the state a
/// client holds is fetched by its own reads, and this tells it when to.
async fn resync<D: Queries>(socket: &mut WebSocket, served: &Served<D>) -> bool {
    let cursor = served.events().cursor();
    send(
        socket,
        &StreamMessage::Resync(Resync {
            protocol_id: ProtocolId::current(),
            cursor,
        }),
    )
    .await
}

async fn send(socket: &mut WebSocket, message: &StreamMessage) -> bool {
    let Ok(text) = ipc::encode(message) else {
        return false;
    };
    // Awaited, not queued: this is what makes a slow client slow *this* task
    // rather than Fleet's memory, so the bound upstream is the thing that gives.
    socket.send(Message::Text(text)).await.is_ok()
}
