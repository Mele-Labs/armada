//! A Session's run, as one socket sees it: the output so far, then what follows, then the end.
//!
//! **`watching_run`'s shape without a log.** The run's output is in memory, so the history and the
//! subscription are taken together by whoever holds it (`fleet::session_running`) and arrive here
//! already consistent.

use axum::extract::ws::{Message, WebSocket};
use ipc::{ProtocolId, SessionRunMessage};
use serde::Serialize;
use tokio::sync::broadcast;

/// How many messages a run's channel holds for a viewer not keeping up.
pub const SESSION_RUN_BACKLOG: usize = 256;

/// What a run says after a viewer has subscribed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionRunEvent {
    Lines(Vec<String>),
    /// The run ended. `None` where a signal ended the process group.
    Exited(Option<i32>),
}

/// What one viewer is answered with.
pub struct ObservedSessionRun {
    pub run_id: String,
    /// What the run printed before this viewer subscribed.
    pub history: Vec<String>,
    /// `Some` where the run was still going: the end then arrives on it. `None` where it ended,
    /// and `exited` is the code it ended with.
    pub live: Result<broadcast::Receiver<SessionRunEvent>, Option<i32>>,
}

pub(crate) async fn relay_session_run(mut socket: WebSocket, observed: ObservedSessionRun) {
    let opened = SessionRunMessage::Opened {
        protocol_id: ProtocolId::current(),
        run_id: observed.run_id,
    };
    if !send(&mut socket, &opened).await {
        return;
    }
    if !observed.history.is_empty()
        && !send(&mut socket, &SessionRunMessage::Lines { lines: observed.history }).await
    {
        return;
    }
    match observed.live {
        Err(code) => {
            send(&mut socket, &SessionRunMessage::Exited { code }).await;
        }
        Ok(mut live) => loop {
            let message = match live.recv().await {
                Ok(SessionRunEvent::Lines(lines)) => SessionRunMessage::Lines { lines },
                Ok(SessionRunEvent::Exited(code)) => {
                    send(&mut socket, &SessionRunMessage::Exited { code }).await;
                    break;
                }
                Err(broadcast::error::RecvError::Lagged(dropped)) => {
                    SessionRunMessage::Missed { dropped }
                }
                // The run's record went with no end sent: say so as a signal's.
                Err(broadcast::error::RecvError::Closed) => {
                    send(&mut socket, &SessionRunMessage::Exited { code: None }).await;
                    break;
                }
            };
            if !send(&mut socket, &message).await {
                return;
            }
        },
    }
    let _ = socket.close().await;
}

async fn send<M: Serialize>(socket: &mut WebSocket, message: &M) -> bool {
    let Ok(text) = ipc::encode(message) else {
        return false;
    };
    socket.send(Message::Text(text)).await.is_ok()
}
