//! A Session's run, as one socket sees it: the output so far, then what follows, then the end.
//!
//! **`watching_run`'s shape without a log.** The run's output is in memory, so the history and the
//! subscription are taken together by whoever holds it (`fleet::session_running`) and arrive here
//! already consistent. Output is a terminal's bytes as text, not lines: the escapes are intact.

use axum::extract::ws::{Message, WebSocket};
use ipc::{ProtocolId, SessionRunMessage};
use serde::Serialize;
use tokio::sync::broadcast;

/// How many messages a run's channel holds for a viewer not keeping up.
pub const SESSION_RUN_BACKLOG: usize = 256;

/// How a run ended. `code` is `None` where the process group was stopped or a signal ended it,
/// and `stopped` tells those two apart: it is true where Stop was pressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionRunEnd {
    pub code: Option<i32>,
    pub stopped: bool,
}

/// What a run says after a viewer has subscribed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionRunEvent {
    Output(String),
    Exited(SessionRunEnd),
}

/// What one viewer is answered with.
pub struct ObservedSessionRun {
    pub run_id: String,
    /// What the run printed before this viewer subscribed: the held tail.
    pub history: String,
    /// `Ok` where the run was still going: the end then arrives on it. `Err` where it ended,
    /// and holds how.
    pub live: Result<broadcast::Receiver<SessionRunEvent>, SessionRunEnd>,
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
        && !send(&mut socket, &SessionRunMessage::Output { text: observed.history }).await
    {
        return;
    }
    match observed.live {
        Err(end) => {
            send(&mut socket, &exited(end)).await;
        }
        Ok(mut live) => loop {
            let message = match live.recv().await {
                Ok(SessionRunEvent::Output(text)) => SessionRunMessage::Output { text },
                Ok(SessionRunEvent::Exited(end)) => {
                    send(&mut socket, &exited(end)).await;
                    break;
                }
                Err(broadcast::error::RecvError::Lagged(dropped)) => {
                    SessionRunMessage::Missed { dropped }
                }
                // The run's record went with no end sent: say so as a signal's.
                Err(broadcast::error::RecvError::Closed) => {
                    send(&mut socket, &exited(SessionRunEnd { code: None, stopped: false })).await;
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

fn exited(end: SessionRunEnd) -> SessionRunMessage {
    SessionRunMessage::Exited { code: end.code, stopped: end.stopped }
}

async fn send<M: Serialize>(socket: &mut WebSocket, message: &M) -> bool {
    let Ok(text) = ipc::encode(message) else {
        return false;
    };
    socket.send(Message::Text(text)).await.is_ok()
}
